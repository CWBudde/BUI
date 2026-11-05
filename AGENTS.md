These must be strictly followed:
- Always use DRY principles. Never ever make another function for the same things that can be re-used.
- Never regress the perf, unless absolutely necessary
- Follow the full Blender's source code. DO NOT Deviate from it.
- Use the blender's source code wherever you want. Always follow it, it's the single source of truth, and the god.
- Never have duplicate code cross frameworks unless necessary, one source of truth is absolutely necessary.
- NEVER HAVE THESE:
- implicit coupling
- layer-first folders
- duplication
- representable illegal states
- god files
- shotgun surgery
- layering violations
- pass-through wrappers
- errors / validation not at the boundary

These are recommended:
- In order to prevent memory leaks, and crashes, highly recommended to follow the blender's source code as a base and build upon in rust.

If writing Rust:
- STRICTLY DO NOT HAVE Rc<Refcell>. DO IT COMPILE TIME!! It just swaps gc for runtime borrow checks and refcount churn. you won't get real rust perf without a second pass flattening entitices into an arena or ECS with indices.
- Please have the clippy lints: all = { level = "warn", priority = -2 }, pedantic = { level = "warn", priority = -2 }, perf = { level = "warn", priority = -1 }, complexity = { level = "warn", priority = -1 }, inline_always = "allow", unseparated_literal_suffix = "allow", similar_names = "allow", missing_panics_doc = "allow", missing_errors_doc = "allow", absolute_paths = "allow", module_name_repetitions = "allow", doc_markdown = "allow", too_many_lines = "allow", unused_self = "allow", cast_possible_truncation = "allow", cast_sign_loss = "allow", cast_precision_loss = "allow", unreadable_literal = "allow", implicit_hasher = "allow", significant_drop_tightening = "warn", redundant_clone = "warn", use_self = "warn", redundant_else = "warn", or_fun_call = "warn", missing_const_for_fn = "warn", derive_partial_eq_without_eq = "warn"
- Always NEVER ALLOCATE, try to use the refs, never clone unless necessary. even for strings, use CoW and refs.
