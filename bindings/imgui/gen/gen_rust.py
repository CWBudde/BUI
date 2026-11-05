"""Rust emitter of gen.py: blenderui-imgui's src/sys.rs (plain FFI over cimgui.h, no
bindgen) and src/safe.rs (a safe layer: &str, &mut for in/out values, slices, methods on
the structs).

SPDX-License-Identifier: GPL-2.0-or-later
"""

from __future__ import annotations

import re

RUST_KEYWORDS = {
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for", "if", "impl",
    "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static",
    "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while", "async", "await", "dyn",
    "abstract", "become", "box", "do", "final", "macro", "override", "priv", "typeof", "unsized", "virtual",
    "yield", "try", "gen",
}
NO_RAW = {"crate", "self", "Self", "super"}

BUILTIN_RS = {
    "bool": "bool", "char": "c_char", "signed char": "i8", "unsigned char": "u8", "short": "i16",
    "signed short": "i16", "unsigned short": "u16", "int": "c_int", "signed int": "c_int",
    "unsigned int": "c_uint", "long long": "i64", "signed long long": "i64", "unsigned long long": "u64",
    "float": "f32", "double": "f64", "size_t": "usize", "void": "c_void", "va_list": "VaList",
}


class Skip(Exception):
    pass


def ident(n):
    if n in NO_RAW:
        return n + "_"
    return "r#" + n if n in RUST_KEYWORDS else n


def rs_base(base, model, g):
    m = re.match(r"ImVector<(.+)>$", base)
    if m:
        return "ImVector<%s>" % rs_type(m.group(1).strip(), model, g)
    m = re.match(r"ImSmallStack_<\s*(\w+)\s*,\s*(\d+)\s*>$", base)
    if m:
        return "ImSmallStack_%s_%s" % (m.group(1), m.group(2))
    if base in BUILTIN_RS:
        return BUILTIN_RS[base]
    return base


def rs_type(t, model, g, array_param=False):
    """C/C++ type spelling (no refs) -> Rust type."""
    c = g.parse_ctype(t)
    if c["ref"]:
        raise Skip("ref")
    base = c["base"]
    if base == "va_list":
        raise Skip("va_list")
    r = rs_base(base, model, g)
    if not c["ptr"]:
        return r
    # innermost pointee constness = base const; outer ones from the pointer qualifiers
    pointee_const = c["const"]
    for k, pc in enumerate(c["ptr"]):
        r = ("*const " if pointee_const else "*mut ") + r
        pointee_const = pc
    return r


def rs_fnptr(fp, model, g):
    args = []
    for a in fp["args"]:
        args.append(rs_param_type(a, model, g))
    ret = fp["ret"]
    r = "" if ret in ("void", "") else " -> " + rs_type(ret, model, g)
    return 'Option<unsafe extern "C" fn(%s)%s>' % (", ".join(args), r)


def rs_param_type(a, model, g):
    if a["type"] == "fnptr":
        return rs_fnptr(a["fnptr"], model, g)
    t = g.c_param_type(a, model)
    if a.get("array") is not None:
        elem = g.parse_ctype(t)
        # T v[N] -> pointer to T (const T v[] -> *const T)
        return ("*const " if (elem["const"] and not elem["ptr"]) or (elem["ptr"] and t.rstrip().endswith("const")) else "*mut ") + rs_type(re.sub(r"\s*const$", "", t), model, g)
    return rs_type(t, model, g)


def rs_ret_type(f, model, g):
    t = g.c_ret_type(f, model)
    if t == "void":
        return ""
    return " -> " + rs_type(t, model, g)


def enum_rust_type(e, model):
    n = e["name"]
    if n.endswith("_") and n[:-1] in model.typedefs:
        return n[:-1]
    if n and not n.endswith("_"):
        return n
    return "c_int"


def const_value(v, rtype):
    if rtype in ("u8",) or rtype == "ImGuiSortDirection":
        return str(v)
    if v >= 2 ** 31:
        v -= 2 ** 32
    return str(v)


def field_rs(fd, model, g):
    if fd["type"] == "fnptr":
        return rs_fnptr(fd["fnptr"], model, g)
    t = rs_type(fd["type"], model, g)
    for d in reversed(g.array_dims(fd, model)):
        t = "[%s; %d]" % (t, d)
    return t


def emit_struct(s, model, g, L, unions):
    w = L.append
    name = s["name"]
    derive = {
        "ImVec2": "#[derive(Clone, Copy, Debug, Default, PartialEq)]",
        "ImVec4": "#[derive(Clone, Copy, Debug, Default, PartialEq)]",
        "ImTextureRef": "#[derive(Clone, Copy, Debug)]",
        "ImColor": "#[derive(Clone, Copy, Debug, Default, PartialEq)]",
    }.get(name)
    fields = []
    bf_unit = None
    nbf = 0
    for fd in s["fields"]:
        if fd["type"] == "union":
            un = "%s__Union%d" % (name, len([u for u in unions if u[0].startswith(name + "__")]))
            members = [(ident(m["name"]), field_rs(m, model, g)) for m in fd["union"]]
            unions.append((un, members))
            fields.append((ident(fd["name"].lstrip("_")), un))
            bf_unit = None
            continue
        if "bitfield" in fd:
            base = g.parse_ctype(fd["type"])["base"]
            bits = fd["bitfield"]
            size = {"unsigned int": 32, "int": 32, "unsigned short": 16, "unsigned char": 8}.get(base, 32)
            if bf_unit is None or bf_unit[0] != base or bf_unit[1] + bits > size:
                bf_unit = [base, 0]
                fields.append(("_bitfield%d" % nbf, rs_type(base, model, g)))
                nbf += 1
            bf_unit[1] += bits
            continue
        bf_unit = None
        fields.append((ident(fd["name"]), field_rs(fd, model, g)))
    w("#[repr(C)]")
    if derive:
        w(derive)
    w("pub struct %s {" % name)
    if not fields:
        w("    _empty: u8,")
    for n, t in fields:
        w("    pub %s: %s," % (n, t))
    w("}")


def rs_default(expr, rtype):
    """A C++ default argument as a Rust expression (the few forms the safe layer needs)."""
    e = expr.strip()
    if e == "NULL":
        return "core::ptr::null_mut()"
    if e in ("true", "false"):
        return e
    if re.match(r"^-?\d+$", e):
        return e
    if re.match(r"^-?[\d.]+f?$", e):
        return e.rstrip("f") if "." in e else e.rstrip("f") + ".0"
    m = re.match(r"^ImVec2\((.+),(.+)\)$", e.replace(" ", ""))
    if m:
        vals = [rs_default(x, "f32") for x in m.groups()]
        vals = [v if "." in v or "::" in v else v + ".0" for v in vals]
        return "ImVec2 { x: %s, y: %s }" % tuple(vals)
    if e == "-FLT_MIN":
        return "-f32::MIN_POSITIVE"
    if e == "FLT_MAX":
        return "f32::MAX"
    if re.match(r"^Im\w+_\w+$", e):
        return "sys::" + e
    raise Skip("default " + e)


def safe_variant(f, cname, nargs, model, g, out, used_names, impl_of=None):
    """Emit one safe wrapper for C function `cname` (taking the first nargs C++ args)."""
    allargs = [a for a in f["args"] if a["type"] != "..."]
    args = allargs[:nargs]
    has_cb = any(a["type"] == "ImGuiInputTextCallback" for a in allargs)
    buf_short = False
    call_name = cname
    # InputText* short variants: call the full one (resize callback) with the defaults
    if any(g.parse_ctype(a["type"])["base"] == "char" and not g.parse_ctype(a["type"])["const"] and g.parse_ctype(a["type"])["ptr"] for a in args) and has_cb and nargs < len(allargs):
        buf_short = True
        call_name = f["c_full"]
    params = []   # rust "name: Type"
    pre = []      # statements before the call
    cargs = []    # C call arguments
    post = []
    skip = set()
    is_method = f["scope"] != "ImGui" and not f.get("static") and not f.get("ctor")
    generics = 0
    for k, a in enumerate(allargs):
        if k in skip:
            continue
        exposed_arg = k < nargs
        n = ident(a["name"])
        t = a["type"]
        dflt = a.get("default")
        if not exposed_arg:
            if not buf_short:
                break
            # defaults of a buf-short call
            if t == "ImGuiInputTextCallback":
                cargs.append("Some(crate::input_text_resize)")
            elif a["name"] == "user_data":
                cargs.append("(&mut __buf) as *mut crate::InputBuf as *mut c_void")
            elif a["name"] == "flags":
                cargs.append("sys::ImGuiInputTextFlags_CallbackResize")
            else:
                cargs.append(rs_default(dflt, ""))
            continue
        if t == "fnptr" or g.parse_ctype(t)["base"] == "va_list":
            if dflt == "NULL":
                cargs.append("None")
                continue
            raise Skip("callback")
        c = g.parse_ctype(t)
        base = c["base"]
        sk = model.scalar_kind(base)
        nxt = allargs[k + 1] if k + 1 < len(allargs) else None
        if model.typedefs.get(base, {}).get("type") == "fnptr":
            if t == "ImGuiInputTextCallback" and has_cb and any(x == "buf" for x in [p[0] for p in []]):
                pass
            if t == "ImGuiInputTextCallback" and "__buf" in "".join(pre):
                cargs.append("Some(crate::input_text_resize)")
                continue
            if dflt == "NULL":
                cargs.append("None")
                continue
            raise Skip("callback typedef")
        if a.get("array") is not None:
            arr = a["array"]
            if arr == "":
                if nxt is not None and nxt["type"] == "int" and "count" in nxt["name"] and k + 1 < nargs:
                    params.append("%s: &[&str]" % n)
                    pre.append("let __%s_c: Vec<CString> = %s.iter().map(|s| crate::cs(s)).collect();" % (a["name"], n))
                    pre.append("let __%s_p: Vec<*const c_char> = __%s_c.iter().map(|s| s.as_ptr()).collect();" % (a["name"], a["name"]))
                    cargs.append("__%s_p.as_ptr()" % a["name"])
                    cargs.append("__%s_p.len() as c_int" % a["name"])
                    skip.add(k + 1)
                    continue
                raise Skip("unsized array")
            elem = rs_type(re.sub(r"\s*const$", "", t), model, g)
            N = g.eval_dim(arr, model)
            if c["const"]:
                params.append("%s: &[%s; %d]" % (n, elem, N))
                cargs.append("%s.as_ptr()" % n)
            else:
                params.append("%s: &mut [%s; %d]" % (n, elem, N))
                cargs.append("%s.as_mut_ptr()" % n)
            continue
        if c["ref"] and not c["ptr"]:
            if c["const"] and base in ("ImVec2", "ImVec4", "ImTextureRef"):
                params.append("%s: impl Into<%s>" % (n, base))
                cargs.append("%s.into()" % n)
                continue
            if c["const"] and sk:
                params.append("%s: %s" % (n, rs_type(base, model, g)))
                cargs.append(n)
                continue
            if sk:
                params.append("%s: &mut %s" % (n, rs_type(base, model, g)))
                cargs.append(n)
                continue
            if model.is_struct(base):
                if c["const"]:
                    params.append("%s: &%s" % (n, base))
                    cargs.append("%s as *const %s" % (n, base))
                else:
                    params.append("%s: &mut %s" % (n, base))
                    cargs.append("%s as *mut %s" % (n, base))
                continue
            raise Skip("ref")
        if not c["ptr"]:
            if base in ("ImVec2", "ImVec4", "ImTextureRef"):
                params.append("%s: impl Into<%s>" % (n, base))
                cargs.append("%s.into()" % n)
                continue
            if sk or base in model.structs:
                params.append("%s: %s" % (n, rs_type(base, model, g)))
                if a["name"] == "flags" and "__buf" in "".join(pre):
                    cargs.append("%s | sys::ImGuiInputTextFlags_CallbackResize" % n)
                else:
                    cargs.append(n)
                continue
            raise Skip("value")
        if len(c["ptr"]) == 1:
            nullable = dflt == "NULL"
            if base == "char" and c["const"]:
                if a["name"] == "fmt" and f.get("variadic"):
                    params.append("text: &str")
                    pre.append("let __text = crate::cs(text);")
                    cargs.append('c"%s".as_ptr()')
                    post_variadic = "__text.as_ptr()"
                    cargs.append(post_variadic)
                    continue
                if nullable:
                    params.append("%s: Option<&str>" % n)
                    pre.append("let __%s = %s.map(crate::cs);" % (a["name"], n))
                    cargs.append("__%s.as_ref().map_or(core::ptr::null(), |s| s.as_ptr())" % a["name"])
                else:
                    params.append("%s: &str" % n)
                    pre.append("let __%s = crate::cs(%s);" % (a["name"], n))
                    cargs.append("__%s.as_ptr()" % a["name"])
                continue
            if base == "char" and not c["const"]:
                if nxt is not None and nxt["type"] == "size_t" and has_cb:
                    params.append("%s: &mut String" % n)
                    pre.append("let mut __buf = crate::InputBuf::new(%s);" % n)
                    cargs.append("__buf.ptr()")
                    cargs.append("__buf.capacity()")
                    post.append("__buf.finish(%s);" % n)
                    skip.add(k + 1)
                    continue
                raise Skip("char buffer")
            if base == "void":
                if c["const"] and nxt is not None and nxt["type"] == "size_t" and k + 1 < nargs:
                    params.append("%s: &[u8]" % n)
                    cargs.append("%s.as_ptr() as *const c_void" % n)
                    cargs.append("%s.len()" % n)
                    skip.add(k + 1)
                    continue
                if a["name"] == "user_data" and "__buf" in "".join(pre):
                    cargs.append("(&mut __buf) as *mut crate::InputBuf as *mut c_void")
                    continue
                if nullable:
                    cargs.append("core::ptr::null_mut()")
                    continue
                raise Skip("void*")
            if sk or base in ("ImVec2", "ImVec4"):
                rt = rs_type(base, model, g)
                if c["const"]:
                    if nxt is not None and nxt["type"] == "int" and ("count" in nxt["name"] or nxt["name"].startswith("num_")) and k + 1 < nargs:
                        params.append("%s: &[%s]" % (n, rt))
                        cargs.append("%s.as_ptr()" % n)
                        cargs.append("%s.len() as c_int" % n)
                        skip.add(k + 1)
                        continue
                    if nullable:
                        params.append("%s: Option<&%s>" % (n, rt))
                        cargs.append("%s.map_or(core::ptr::null(), |r| r as *const %s)" % (n, rt))
                        continue
                    raise Skip("const scalar ptr")
                if nullable:
                    params.append("%s: Option<&mut %s>" % (n, rt))
                    cargs.append("%s.map_or(core::ptr::null_mut(), |r| r as *mut %s)" % (n, rt))
                else:
                    params.append("%s: &mut %s" % (n, rt))
                    cargs.append(n)
                continue
            if model.is_struct(base):
                if c["const"]:
                    if nullable:
                        params.append("%s: Option<&%s>" % (n, base))
                        cargs.append("%s.map_or(core::ptr::null(), |r| r as *const %s)" % (n, base))
                    else:
                        params.append("%s: &%s" % (n, base))
                        cargs.append("%s as *const %s" % (n, base))
                else:
                    if nullable:
                        params.append("%s: Option<&mut %s>" % (n, base))
                        cargs.append("%s.map_or(core::ptr::null_mut(), |r| r as *mut %s)" % (n, base))
                    else:
                        params.append("%s: &mut %s" % (n, base))
                        cargs.append("%s as *mut %s" % (n, base))
                continue
        if dflt == "NULL":
            cargs.append("core::ptr::null_mut()")
            continue
        raise Skip("type " + t)
    # return
    rt = f["ret"]
    if f.get("ctor"):
        ret_s = " -> crate::Owned<%s>" % f["scope"]
        wrap = "crate::Owned::from_raw(%s)"
    elif not rt or rt == "void":
        ret_s, wrap = "", "%s"
    else:
        c = g.parse_ctype(rt)
        base = c["base"]
        sk = model.scalar_kind(base)
        if c["ref"] and not c["ptr"]:
            if model.is_struct(base):
                ret_s = " -> &'static %s%s" % ("" if c["const"] else "mut ", base)
                wrap = "&%s*%%s" % ("" if c["const"] else "mut ")
            else:
                raise Skip("ret ref")
        elif not c["ptr"]:
            if sk or base in ("ImVec2", "ImVec4") or base in model.structs:
                ret_s = " -> " + rs_type(base, model, g)
                wrap = "%s"
            else:
                raise Skip("ret")
        elif len(c["ptr"]) == 1 and base == "char":
            ret_s = " -> &'static str"
            wrap = "crate::from_cstr(%s)"
        elif len(c["ptr"]) == 1 and model.is_struct(base):
            ret_s = " -> Option<&'static %s%s>" % ("" if c["const"] else "mut ", base)
            wrap = "%s.%s()" % ("%s", "as_ref" if c["const"] else "as_mut")
        else:
            ret_s = " -> " + rs_type(g.c_ret_type(f, model), model, g)
            wrap = "%s"
    # name
    if impl_of:
        prefix = impl_of + "_"
        rname = g.snake(cname[len(prefix):])
        if f.get("ctor"):
            rname = rname.replace("new", "new", 1)
    else:
        rname = g.snake(cname[len("ImGui_"):])
    rname = ident(rname)
    if rname in used_names:
        raise Skip("name clash " + rname)
    used_names.add(rname)
    self_p = []
    if is_method:
        if f.get("const"):
            self_p = ["&self"]
            cargs.insert(0, "self as *const Self")
        else:
            self_p = ["&mut self"]
            cargs.insert(0, "self as *mut Self")
    call = "sys::%s(%s)" % (call_name, ", ".join(cargs))
    body = []
    body.extend(pre)
    if post:
        body.append("let __r = unsafe { %s };" % call)
        body.extend(post)
        body.append(wrap % "__r" if "%s" in wrap else "__r")
    else:
        body.append("unsafe { %s }" % (wrap % call))
    doc = "%s::%s" % (f["scope"], f["name"]) if f["scope"] != "ImGui" else "ImGui::" + f["name"]
    vis = "pub fn"
    out.append("    /// `%s`" % doc if impl_of else "/// `%s`" % doc)
    ind = "    " if impl_of else ""
    sig = ", ".join(self_p + params)
    out.append("%s#[allow(clippy::too_many_arguments, unused_unsafe)]" % ind)
    out.append("%s%s %s(%s)%s {" % (ind, vis, rname, sig, ret_s))
    for b in body:
        out.append(ind + "    " + b)
    out.append(ind + "}")
    return rname


def emit(api, model, g):
    S = []
    w = S.append
    w("//! Raw FFI to Dear ImGui %s through its C API (bindings/imgui/c/cimgui.h)." % api["version"])
    w("//! " + g.GEN_NOTE)
    w("#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code, clippy::all, improper_ctypes)]")
    w("")
    w("use core::ffi::{c_char, c_int, c_uint, c_void};")
    w("")
    w("/// `va_list` functions (`ImGui_TextV`...) are not declared: use the variadic ones.")
    w("pub type VaList = *mut c_void;")
    w("")
    w("#[repr(C)]")
    w("pub struct ImVector<T> {")
    w("    pub Size: c_int,")
    w("    pub Capacity: c_int,")
    w("    pub Data: *mut T,")
    w("}")
    w("impl<T> ImVector<T> {")
    w("    /// The elements (empty when Data is null).")
    w("    pub fn as_slice(&self) -> &[T] {")
    w("        if self.Data.is_null() || self.Size <= 0 { &[] } else { unsafe { core::slice::from_raw_parts(self.Data, self.Size as usize) } }")
    w("    }")
    w("}")
    w("")
    # typedefs
    emitted = set()
    for t in api["typedefs"]:
        if t["obsolete"] or t["name"] in model.structs:
            continue
        if t["type"] == "fnptr":
            w("pub type %s = %s;" % (t["name"], rs_fnptr(t["fnptr"], model, g)))
        else:
            w("pub type %s = %s;" % (t["name"], rs_type(t["type"], model, g)))
        emitted.add(t["name"])
    for e in api["enums"]:
        n = e["name"]
        if n and not n.endswith("_") and n not in emitted:
            w("pub type %s = %s;" % (n, BUILTIN_RS.get(e["underlying"] or "int", e["underlying"] or "c_int")))
            emitted.add(n)
    w("")
    nconst = 0
    for e in api["enums"]:
        rt = enum_rust_type(e, model)
        for v in e["values"]:
            if v["obsolete"]:
                continue
            w("pub const %s: %s = %s;" % (v["name"], rt, const_value(v["value"], rt)))
            nconst += 1
    for k, v in api["defines"].items():
        if k.startswith(("IMGUI_", "IM_")):
            if v.startswith('"'):
                w("pub const %s: &core::ffi::CStr = c%s;" % (k, v))
            else:
                w("pub const %s: i64 = %s;" % (k, v))
    w("")
    for n in api["opaque"]:
        w("#[repr(C)]")
        w("pub struct %s {" % n)
        w("    _opaque: [u8; 0],")
        w("}")
    unions = []
    for sn, (inner, cap) in sorted(g.c_smallstacks(api).items()):
        w("#[repr(C)]")
        w("pub struct %s {" % sn)
        w("    pub LocalData: [%s; %d]," % (rs_type(inner, model, g), cap))
        w("    pub Size: u16,")
        w("}")
    struct_lines = []
    for s in api["structs"]:
        emit_struct(s, model, g, struct_lines, unions)
    for un, members in unions:
        w("#[repr(C)]")
        w("#[derive(Clone, Copy)]")
        w("pub union %s {" % un)
        for n, t in members:
            w("    pub %s: %s," % (n, t))
        w("}")
    S.extend(struct_lines)
    w("")
    w('unsafe extern "C" {')
    nffi = 0
    missing = []
    for f in g.all_functions(api):
        if f["obsolete"] or not g.c_supported(f, model):
            continue
        for cname, nargs in g.c_variants(f):
            try:
                params = []
                if f["scope"] != "ImGui" and not f.get("static") and not f.get("ctor"):
                    params.append("self_: %s%s" % ("*const " if f.get("const") else "*mut ", f["scope"]))
                for a in [a for a in f["args"] if a["type"] != "..."][:nargs]:
                    params.append("%s: %s" % (ident(a["name"]), rs_param_type(a, model, g)))
                if f.get("variadic"):
                    params.append("...")
                line = "    pub fn %s(%s)%s;" % (cname, ", ".join(params), rs_ret_type(f, model, g))
            except Skip as e:
                missing.append("%s (%s)" % (cname, e))
                continue
            w(line)
            nffi += 1
    for s in api["structs"]:
        if s["ctors"]:
            w("    pub fn %s_Delete(self_: *mut %s);" % (s["name"], s["name"]))
    w("    pub fn ImTextureRef_FromID(tex_id: ImTextureID) -> ImTextureRef;")
    w("    pub fn cImGui_SizeOf(type_name: *const c_char) -> usize;")
    w("}")
    w("")
    w("/// Structs whose Rust size differs from C++ (`sizeof`): empty when the layouts match.")
    w("pub fn layout_mismatches() -> Vec<(&'static str, usize, usize)> {")
    w("    let mut bad = Vec::new();")
    for s in api["structs"]:
        w('    let c = unsafe { cImGui_SizeOf(c"%s".as_ptr()) };' % s["name"])
        w('    if c != core::mem::size_of::<%s>() { bad.push(("%s", core::mem::size_of::<%s>(), c)); }' % (s["name"], s["name"], s["name"]))
    w("    bad")
    w("}")
    g.OUT_RS_SYS.parent.mkdir(parents=True, exist_ok=True)
    g.OUT_RS_SYS.write_text("\n".join(S) + "\n", encoding="utf-8", newline="\n")

    # ---- safe layer
    F = []
    w = F.append
    w("//! The safe layer over sys.rs: snake_case functions taking &str, &mut for in/out")
    w("//! values, slices for (pointer, count) pairs; methods on the imgui structs.")
    w("//! `button(label)` takes the required arguments, `button_ex(label, size)` all of them.")
    w("//! " + g.GEN_NOTE)
    w("#![allow(clippy::all, unused_mut, unused_variables)]")
    w("")
    w("use crate::sys::{self, *};")
    w("use core::ffi::{c_char, c_int, c_uint, c_void};")
    w("use std::ffi::CString;")
    w("")
    stats = {"namespace_functions": 0, "methods_and_ctors": 0, "ffi_functions": nffi, "ffi_missing": missing, "missing": []}
    used = set()
    ns_done = set()
    for f in api["functions"]:
        if f["obsolete"] or not g.c_supported(f, model) or f.get("va_list"):
            continue
        ok = False
        for cname, nargs in g.c_variants(f):
            body = []
            try:
                safe_variant(f, cname, nargs, model, g, body, used)
                F.extend(body)
                ok = True
            except Skip as e:
                stats["missing"].append("%s (%s)" % (cname, e))
        if ok:
            stats["namespace_functions"] += 1
    impls = {}
    for s in api["structs"]:
        used_m = set()
        body_all = []
        for f in s["ctors"] + s["methods"]:
            if f["obsolete"] or not g.c_supported(f, model) or f.get("va_list"):
                continue
            ok = False
            for cname, nargs in g.c_variants(f):
                body = []
                try:
                    safe_variant(f, cname, nargs, model, g, body, used_m, impl_of=s["name"])
                    body_all.extend(body)
                    ok = True
                except Skip as e:
                    stats["missing"].append("%s (%s)" % (cname, e))
            if ok:
                stats["methods_and_ctors"] += 1
        if body_all:
            w("impl %s {" % s["name"])
            F.extend(body_all)
            w("}")
        if s["ctors"]:
            w("impl crate::Delete for %s {" % s["name"])
            w("    unsafe fn delete(p: *mut Self) { unsafe { sys::%s_Delete(p) } }" % s["name"])
            w("}")
    g.OUT_RS_SAFE.write_text("\n".join(F) + "\n", encoding="utf-8", newline="\n")
    stats["constants"] = nconst
    return stats
