"""Python emitter of gen.py: bindings/python/src/buiimgui_gen.inc (C++, CPython API).

Each imgui function becomes a pyimgui-style snake_case function: in/out pointers are
arguments whose new values come back after the return value as a tuple
(`changed, value = imgui.slider_float("x", value, 0, 1)`), ImVec2/ImVec4 are tuples,
`fmt, ...` functions take one str, lists replace (pointer, count) pairs, structs are
wrapper objects with snake_case (and original) field names. Overloads share one name that
tries them in order, and each also has its own name (push_id_int...).

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

import keyword
import re

PY_KEYWORDS = set(keyword.kwlist) | {"print", "exec"}
STRUCT_PREFIX_SKIP = ()


class Unsupported(Exception):
    pass


def count_param(a):
    n = a["name"]
    return a["type"] in ("int", "size_t") and ("count" in n or n.startswith("num_") or n in ("size",))


def classify(f, model, g):
    """-> list of (param, kind, extra) ; raises Unsupported."""
    args = f["args"]
    out = []
    skip = set()
    has_cb = any(a["type"] == "ImGuiInputTextCallback" for a in args)
    for k, a in enumerate(args):
        if k in skip:
            continue
        t = a["type"]
        d = "default" in a
        if t == "...":
            out.append((a, "varargs", None))
            continue
        if t == "fnptr" or t == "va_list":
            if d:
                out.append((a, "hidden", None))
                continue
            raise Unsupported("callback")
        c = g.parse_ctype(t)
        base = c["base"]
        sk = model.scalar_kind(base)
        is_scalar = sk is not None
        if model.typedefs.get(base, {}).get("type") == "fnptr":
            if d:
                out.append((a, "hidden", None))
                continue
            raise Unsupported("callback typedef")
        nxt = args[k + 1] if k + 1 < len(args) else None
        if a.get("array") is not None:
            arr = a["array"]
            if arr == "":
                # const char* const items[] + int items_count
                if nxt is not None and count_param(nxt):
                    elem = t.replace(" const", "").strip() if t.endswith("const") else t
                    out.append((a, "seq", {"elem": elem.replace("const char*", "const char*")}))
                    skip.add(k + 1)
                    out.append((nxt, "count", {"of": a["name"]}))
                    continue
                raise Unsupported("unsized array")
            if (is_scalar or base in ("ImVec2", "ImVec4")) and not c["ptr"]:
                out.append((a, "array_in" if c["const"] else "array", {"n": g.eval_dim(arr, model)}))
                continue
            raise Unsupported("array of " + t)
        if c["ref"]:
            if c["ptr"]:
                raise Unsupported("ref to pointer")
            if c["const"] and (is_scalar or base in g.VALUE_STRUCTS):
                out.append((a, "value", None))
                continue
            if is_scalar and not c["const"]:
                out.append((a, "out", None))
                continue
            if model.is_struct(base):
                out.append((a, "structref", None))
                continue
            raise Unsupported("ref " + t)
        if not c["ptr"]:
            if is_scalar or base in g.VALUE_STRUCTS or base in model.structs:
                out.append((a, "value", None))
                continue
            raise Unsupported("value " + t)
        if len(c["ptr"]) == 1:
            if base == "char" and c["const"]:
                if a["name"] == "fmt" and k + 1 < len(args) and args[k + 1]["type"] == "...":
                    out.append((a, "fmt", None))
                else:
                    out.append((a, "str", None))
                continue
            if base == "char" and not c["const"]:
                if nxt is not None and nxt["type"] == "size_t" and has_cb:
                    out.append((a, "buf", None))
                    skip.add(k + 1)
                    out.append((nxt, "bufsize", {"of": a["name"]}))
                    continue
                raise Unsupported("char buffer")
            if base == "void":
                if c["const"] and nxt is not None and nxt["type"] == "size_t":
                    out.append((a, "bytes", None))
                    skip.add(k + 1)
                    out.append((nxt, "bytesize", {"of": a["name"]}))
                    continue
                if d:
                    out.append((a, "hidden", None))
                    continue
                raise Unsupported("void*")
            if is_scalar or base in ("ImVec2", "ImVec4"):
                if c["const"]:
                    if nxt is not None and count_param(nxt):
                        out.append((a, "seq", {"elem": base}))
                        skip.add(k + 1)
                        out.append((nxt, "count", {"of": a["name"]}))
                        continue
                    if d:
                        out.append((a, "hidden", None))
                        continue
                    raise Unsupported("const scalar pointer")
                out.append((a, "inout", None))
                continue
            if model.is_struct(base):
                out.append((a, "structptr", None))
                continue
            raise Unsupported("pointer " + t)
        if d:
            out.append((a, "hidden", None))
            continue
        raise Unsupported("type " + t)
    return out


def ret_kind(f, model, g):
    t = f["ret"]
    if f.get("ctor"):
        return "ctor"
    if not t or t == "void":
        return "void"
    c = g.parse_ctype(t)
    base = c["base"]
    sk = model.scalar_kind(base)
    if c["ref"]:
        if model.is_struct(base) and not c["ptr"]:
            return "structref"
        if sk and not c["ptr"]:
            return "value"
        raise Unsupported("ret ref")
    if not c["ptr"]:
        if sk or base in ("ImVec2", "ImVec4"):
            return "value"
        if base in model.structs:
            return "structval"
        raise Unsupported("ret " + t)
    if len(c["ptr"]) == 1:
        if base == "char":
            return "str"
        if base == "void":
            return "voidptr"
        if model.is_struct(base) or sk:
            return "ptr"
    if len(c["ptr"]) == 2 and base == "void":
        return "voidptr"
    raise Unsupported("ret " + t)


def local_type(t):
    t = t.strip()
    if t.endswith("&"):
        t = t[:-1].strip()
    if t.startswith("const ") and "*" not in t:
        t = t[6:]
    return t


def pyname(n):
    n = re.sub(r"^_+", "", n) or n
    return n + "_" if n in PY_KEYWORDS else n


def emit_variant(f, model, g, lines, fid):
    """Emit one C++ variant function; returns (python kw names, doc signature)."""
    cls = classify(f, model, g)
    rk = ret_kind(f, model, g)
    scope = f["scope"]
    is_method = scope != "ImGui" and not f.get("static") and not f.get("ctor")
    exposed = [(a, k, x) for (a, k, x) in cls if k not in ("hidden", "out", "count", "bufsize", "bytesize", "varargs")]
    names = []
    for a, k, x in exposed:
        names.append("text" if k == "fmt" else pyname(a["name"]))
    # required = up to the first defaulted exposed param (inout pointers count as required unless defaulted)
    nreq = 0
    for a, k, x in exposed:
        if "default" in a:
            break
        nreq += 1
    w = lines.append
    w("static PyObject* %s(PyObject* self, PyObject* args, PyObject* kw, int* argfail)" % fid)
    w("{")
    w("    (void)self;")
    if names:
        w("    static const char* const kwl[] = { %s, nullptr };" % ", ".join('"%s"' % n for n in names))
        w("    PyObject* o[%d] = {};" % len(names))
        w("    if (!bi_parse(args, kw, kwl, %d, %d, o, argfail)) return nullptr;" % (len(names), nreq))
    else:
        w("    if (!bi_parse(args, kw, nullptr, 0, 0, nullptr, argfail)) return nullptr;")
    if is_method:
        cq = "const " if f.get("const") else ""
        w("    %s%s* self_p = (%s%s*)((BiObj*)self)->ptr;" % (cq, scope, cq, scope))
        w("    if (self_p == nullptr) { PyErr_SetString(PyExc_ValueError, \"NULL object\"); return nullptr; }")
    call_args = []
    outs = []
    oi = 0
    for a, k, x in cls:
        v = "a_" + a["name"]
        dflt = a.get("default")
        t = a["type"]
        if k == "varargs":
            continue
        if k in ("value", "str", "fmt"):
            lt = local_type(t)
            w("    %s %s%s;" % (lt, v, " = " + dflt if dflt is not None else "{}"))
            w("    BI_GET(%d, %s);" % (oi, v))
            oi += 1
            if k == "fmt":
                call_args.append('"%s"')
            call_args.append(v)
        elif k == "hidden":
            if t == "fnptr":
                w("    %s = %s;" % (g.c_fnptr_decl(a["fnptr"], v, model), dflt))
            elif a.get("array") is not None:
                raise Unsupported("hidden array")
            else:
                w("    %s %s = %s;" % (local_type(t) if not t.endswith("&") else t[:-1], v, dflt))
            call_args.append(v)
        elif k == "inout":
            lt = local_type(t)[:-1].strip()
            w("    %s %s_v{}; %s* %s = %s;" % (lt, v, lt, v, dflt if dflt is not None else "nullptr"))
            w("    if (o[%d] && o[%d] != Py_None) { if (!bi_from(o[%d], %s_v)) BI_FAIL; %s = &%s_v; }" % (oi, oi, oi, v, v, v))
            oi += 1
            call_args.append(v)
            outs.append("bi_opt(%s)" % v)
        elif k == "out":
            lt = local_type(t)
            w("    %s %s{};" % (lt, v))
            call_args.append(v)
            outs.append("bi_to(%s)" % v)
        elif k in ("array", "array_in"):
            base = g.parse_ctype(t)["base"]
            w("    %s %s[%d] = {};" % (base, v, x["n"]))
            w("    BI_GET(%d, %s);" % (oi, v))
            oi += 1
            call_args.append(v)
            if k == "array":
                outs.append("bi_to(%s)" % v)
        elif k == "seq":
            elem = x["elem"]
            w("    BiSeq<%s> %s;" % (elem, v))
            w("    BI_GET(%d, %s);" % (oi, v))
            oi += 1
            call_args.append("%s.data()" % v)
        elif k == "count":
            call_args.append("(%s)a_%s.size()" % (t, x["of"]))
        elif k == "bytes":
            w("    BiBytes %s;" % v)
            w("    BI_GET(%d, %s);" % (oi, v))
            oi += 1
            call_args.append("%s.data()" % v)
        elif k == "bytesize":
            call_args.append("a_%s.size()" % x["of"])
        elif k == "buf":
            w("    BiStrBuf %s;" % v)
            w("    BI_GET(%d, %s);" % (oi, v))
            oi += 1
            call_args.append("%s.data()" % v)
            outs.append("bi_to(%s)" % v)
        elif k == "bufsize":
            call_args.append("a_%s.capacity()" % x["of"])
        elif k in ("structptr", "structref"):
            c = g.parse_ctype(t)
            pt = ("const " if c["const"] else "") + c["base"] + "*"
            w("    %s %s = %s;" % (pt, v, dflt if (dflt is not None and k == "structptr") else "nullptr"))
            w("    BI_GETP(%d, %s);" % (oi, v))
            oi += 1
            if k == "structref":
                w("    if (%s == nullptr) { PyErr_SetString(PyExc_TypeError, \"%s: an object is required\"); BI_FAIL; }" % (v, a["name"]))
                call_args.append("*" + v)
            else:
                call_args.append(v)
        else:
            raise Unsupported(k)
    # InputText*: grow the buffer through the resize callback
    if any(k == "buf" for a, k, x in cls):
        for a, k, x in cls:
            if a["type"] == "ImGuiInputTextCallback":
                idx = cls.index((a, k, x))
                call_args[idx_of(cls, a)] = "bi_inputtext_resize"
        bufv = next("a_" + a["name"] for a, k, x in cls if k == "buf")
        for a, k, x in cls:
            if a["name"] == "user_data":
                call_args[idx_of(cls, a)] = "&" + bufv
            if a["name"] == "flags":
                call_args[idx_of(cls, a)] = "(a_flags | ImGuiInputTextFlags_CallbackResize)"
    args_s = ", ".join(call_args)
    if scope == "ImGui":
        call = "ImGui::%s(%s)" % (f["name"], args_s)
    elif f.get("ctor"):
        call = "IM_NEW(%s)(%s)" % (scope, args_s)
    elif f.get("static"):
        call = "%s::%s(%s)" % (scope, f["name"], args_s)
    else:
        call = "self_p->%s(%s)" % (f["name"], args_s)
    if rk == "void":
        w("    %s;" % call)
        items = outs
    elif rk == "ctor":
        w("    return bi_owned(%s);" % call)
        w("}")
        return names, exposed
    else:
        if rk in ("structref",):
            w("    auto& r = %s;" % call)
        else:
            w("    auto r = %s;" % call)
        conv = {
            "value": "bi_to(r)", "str": "bi_to((const char*)r)", "voidptr": "bi_voidptr((const void*)r)",
            "ptr": "bi_ptr_result(r)", "structref": "bi_wrap(&r)", "structval": "bi_copy(r)",
        }[rk]
        items = [conv] + outs
    if not items:
        w("    Py_RETURN_NONE;")
    else:
        w("    return bi_result({ %s });" % ", ".join(items))
    w("}")
    return names, exposed


def idx_of(cls, a):
    """Index in call_args of parameter a (each classified param maps to one call arg, except varargs)."""
    i = 0
    for b, k, x in cls:
        if k == "varargs":
            continue
        if b is a:
            return i
        i += 1 + (1 if k == "fmt" else 0)
    raise KeyError


def doc_sig(name, f, exposed, rk):
    parts = []
    for a, k, x in exposed:
        n = "text" if k == "fmt" else pyname(a["name"])
        if "default" in a:
            parts.append("%s=%s" % (n, a["default"]))
        elif k == "inout":
            parts.append("%s" % n)
        else:
            parts.append(n)
    return "%s(%s)" % (name, ", ".join(parts))


def py_const_name(vname):
    if "_" not in vname:
        return None
    prefix, rest = vname.split("_", 1)
    p = re.sub(r"^ImGui|^Im", "", prefix)
    p = re.sub(r"Flags$", "", p)
    if not rest:
        return None

    def up(s):
        s = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", s)
        s = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", s)
        return s.upper()
    return (up(p) + "_" + up(rest)).replace("__", "_") if p else up(rest)


def emit(api, model, g):
    L = []
    w = L.append
    w("// buiimgui_gen.inc: Dear ImGui %s for Python (included by buiimgui_module.cpp)." % api["version"])
    w("// " + g.GEN_NOTE)
    w("// SPDX-License-Identifier: GPL-2.0-or-later")
    w("")
    stats = {"namespace_functions": 0, "methods_and_ctors": 0, "missing": []}
    total_ns = 0
    module_defs = []   # (pyname, cfunc, doc)
    type_methods = {}  # struct -> list of (pyname, cfunc, flags, doc)
    type_new = {}

    def group_key(f):
        return (f["scope"], "new" if f.get("ctor") else f["name"])

    groups = {}
    for f in g.all_functions(api):
        if f["obsolete"] or f.get("va_list"):
            continue
        groups.setdefault(group_key(f), []).append(f)

    for (scope, fname), fs in groups.items():
        variants = []  # (cfunc id, own python name, doc)
        for f in fs:
            fid = "biv_" + f["c_full"]
            body = []
            try:
                names, exposed = emit_variant(f, model, g, body, fid)
            except Unsupported as e:
                stats["missing"].append("%s::%s (%s)" % (scope, f["name"], e))
                continue
            L.extend(body)
            prefix = "ImGui_" if scope == "ImGui" else scope + "_"
            own = g.snake(f["c_base"][len(prefix):])
            if f.get("ctor"):
                own = g.snake(f["c_base"][len(prefix):]).replace("new", "", 1).strip("_")
            variants.append((fid, own, f))
            if scope == "ImGui":
                stats["namespace_functions"] += 1
            else:
                stats["methods_and_ctors"] += 1
        if not variants:
            continue
        base = g.snake(fname) if fname != "new" else "new"
        rk_doc = []
        for fid, own, f in variants:
            try:
                cls = classify(f, model, g)
                exposed = [(a, k, x) for (a, k, x) in cls if k not in ("hidden", "out", "count", "bufsize", "bytesize", "varargs")]
                rk_doc.append(doc_sig(base, f, exposed, ret_kind(f, model, g)) + "  ->  " + ("%s::%s" % (scope, f["name"]) if scope != "ImGui" else "ImGui::" + f["name"]))
            except Unsupported:
                pass
        doc = "\\n".join(rk_doc).replace('"', '\\"')
        disp = "bid_%s_%s" % (scope, base)
        is_static = fs[0].get("static")
        if fname == "new":
            w("static PyObject* %s(PyTypeObject*, PyObject* args, PyObject* kw)" % disp)
        else:
            w("static PyObject* %s(PyObject* self, PyObject* args, PyObject* kw)" % disp)
        w("{")
        self_expr = "nullptr" if fname == "new" else "self"
        if len(variants) == 1:
            w("    return bi_call1(%s, %s, args, kw);" % (variants[0][0], self_expr))
        else:
            w("    static const BiVariant v[] = { %s };" % ", ".join(v[0] for v in variants))
            w("    return bi_dispatch(%s, args, kw, v, %d);" % (self_expr, len(variants)))
        w("}")
        if fname == "new":
            type_new[scope] = disp
            continue
        entries = [(base, disp, doc)]
        if len(variants) > 1:
            for fid, own, f in variants:
                if own != base:
                    one = "bid1_" + f["c_full"]
                    w("static PyObject* %s(PyObject* self, PyObject* args, PyObject* kw) { return bi_call1(%s, self, args, kw); }" % (one, fid))
                    entries.append((own, one, doc))
        if scope == "ImGui":
            module_defs.extend(entries)
        else:
            for e in entries:
                type_methods.setdefault(scope, []).append(e + (is_static,))

    # module method table
    w("")
    w("static PyMethodDef bi_module_methods_gen[] = {")
    seen = set()
    for n, fn, doc in module_defs:
        if n in seen:
            continue
        seen.add(n)
        w('    { "%s", (PyCFunction)(void (*)(void))%s, METH_VARARGS | METH_KEYWORDS, "%s" },' % (n, fn, doc))
    w("    { nullptr, nullptr, 0, nullptr },")
    w("};")
    stats["python_names"] = len(seen)

    # struct types
    field_tables = set()
    all_types = list(api["opaque"]) + [s["name"] for s in api["structs"] if s["name"] not in ("ImVec2", "ImVec4")]
    for name in all_types:
        st = model.structs.get(name)
        mnames = set()
        if name in type_methods:
            w("static PyMethodDef bi_methods_%s[] = {" % name)
            for n, fn, doc, is_static in type_methods[name]:
                if n in mnames:
                    continue
                mnames.add(n)
                flags = "METH_VARARGS | METH_KEYWORDS" + (" | METH_STATIC" if is_static else "")
                w('    { "%s", (PyCFunction)(void (*)(void))%s, %s, "%s" },' % (n, fn, flags, doc))
            w("    { nullptr, nullptr, 0, nullptr },")
            w("};")
        fields = []
        if st:
            for fd in st["fields"]:
                group = fd["union"] if fd["type"] == "union" else [fd]
                for gf in group:
                    if "bitfield" in gf or gf["type"] == "fnptr":
                        continue
                    for n in dict.fromkeys([g.snake(gf["name"]).lstrip("_") if not gf["name"].startswith("_") else "_" + g.snake(gf["name"]).lstrip("_"), gf["name"]]):
                        if n in mnames or n == "address":
                            continue
                        fields.append('BI_FIELD(%s, %s, "%s")' % (name, gf["name"], n))
        if fields:
            field_tables.add(name)
            w("static BiField bi_fields_%s[] = {" % name)
            for fl in fields:
                w("    %s," % fl)
            w("};")
    w("")
    w("static BiStructInfo bi_structs[] = {")
    for name in all_types:
        st = model.structs.get(name)
        has_fields = st is not None and any(True for fd in st["fields"])
        nf = "bi_fields_%s, (int)(sizeof(bi_fields_%s) / sizeof(BiField))" % (name, name) if name in field_tables else "nullptr, 0"
        w('    { "%s", &BiType<%s>::type, %s, %s, %s },' % (
            name, name, ("bi_methods_" + name) if name in type_methods else "nullptr", nf,
            ("(newfunc)" + type_new[name]) if name in type_new else "nullptr"))
    w("};")

    # constants
    w("")
    w("static const BiConst bi_consts[] = {")
    used = set()
    nconst = 0
    for e in api["enums"]:
        for v in e["values"]:
            if v["obsolete"]:
                continue
            for n in (v["name"], py_const_name(v["name"])):
                if n and n not in used:
                    used.add(n)
                    w('    { "%s", %dLL },' % (n, v["value"]))
                    nconst += 1
    for k, v in api["defines"].items():
        if not v.startswith('"') and k.startswith(("IMGUI_", "IM_")) and k not in used:
            w('    { "%s", %sLL },' % (k, v))
            used.add(k)
    w("    { nullptr, 0 },")
    w("};")
    w("static const char* const bi_str_consts[][2] = {")
    for k, v in api["defines"].items():
        if v.startswith('"'):
            w('    { "%s", %s },' % (k, v))
    w("    { nullptr, nullptr },")
    w("};")
    g.OUT_PY.write_text("\n".join(L) + "\n", encoding="utf-8", newline="\n")
    stats["constants"] = nconst
    return stats


def _has_py_fields(st):
    for fd in st["fields"]:
        group = fd["union"] if fd["type"] == "union" else [fd]
        for gf in group:
            if "bitfield" not in gf and gf["type"] != "fnptr":
                return True
    return False
