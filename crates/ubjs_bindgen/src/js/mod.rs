use std::collections::HashSet;
use std::fs;

use anyhow::{Context, Result};

use crate::cli::GenerateArgs;

pub mod config;

mod external_types;
pub(crate) mod ffi;
mod naming;
mod parsing;
mod render_helpers;
mod render_types;
pub mod runtime_ts;
mod types;
mod wasm_metadata;

use external_types::collect_external_imports;
use naming::pascal_case;
use parsing::{namespace_from_source, parse_metadata};
use render_helpers::{render_jsdoc, ts_type_str, type_name};
use render_types::{
    render_callback_interface, render_enum_type, render_error_class, render_record_interface,
};
use types::*;

/// Walk all type positions in `metadata` and collect error names that appear
/// as value types (e.g. as record fields, enum variant fields, function
/// args/returns, etc.).  These errors need `_lower{Name}` / `_lift{Name}`
/// helpers even if they have no constructors or methods of their own.
fn collect_errors_as_value_types(metadata: &BindingsMetadata) -> HashSet<String> {
    use uniffi_bindgen::interface::Type;

    let error_names: HashSet<&str> = metadata.errors.iter().map(|e| e.name.as_str()).collect();
    let mut used: HashSet<String> = HashSet::new();

    fn visit(t: &Type, error_names: &HashSet<&str>, used: &mut HashSet<String>) {
        match t {
            Type::Enum { name, .. } if error_names.contains(name.as_str()) => {
                used.insert(name.clone());
            }
            Type::Optional { inner_type } | Type::Sequence { inner_type } => {
                visit(inner_type, error_names, used);
            }
            Type::Map {
                key_type,
                value_type,
            } => {
                visit(key_type, error_names, used);
                visit(value_type, error_names, used);
            }
            _ => {}
        }
    }

    macro_rules! visit_type {
        ($t:expr) => {
            visit($t, &error_names, &mut used)
        };
    }

    for f in &metadata.functions {
        for a in &f.args {
            visit_type!(&a.type_);
        }
        if let Some(r) = &f.return_type {
            visit_type!(r);
        }
    }
    for r in &metadata.records {
        for f in &r.fields {
            visit_type!(&f.type_);
        }
        for c in &r.constructors {
            for a in &c.args {
                visit_type!(&a.type_);
            }
        }
        for m in &r.methods {
            for a in &m.args {
                visit_type!(&a.type_);
            }
            if let Some(r) = &m.return_type {
                visit_type!(r);
            }
        }
    }
    for e in &metadata.enums {
        for v in &e.variants {
            for f in &v.fields {
                visit_type!(&f.type_);
            }
        }
        for c in &e.constructors {
            for a in &c.args {
                visit_type!(&a.type_);
            }
        }
        for m in &e.methods {
            for a in &m.args {
                visit_type!(&a.type_);
            }
            if let Some(r) = &m.return_type {
                visit_type!(r);
            }
        }
    }
    for e in &metadata.errors {
        for v in &e.variants {
            for f in &v.fields {
                visit_type!(&f.type_);
            }
        }
        for c in &e.constructors {
            for a in &c.args {
                visit_type!(&a.type_);
            }
        }
        for m in &e.methods {
            for a in &m.args {
                visit_type!(&a.type_);
            }
            if let Some(r) = &m.return_type {
                visit_type!(r);
            }
        }
    }
    for o in &metadata.objects {
        for c in &o.constructors {
            for a in &c.args {
                visit_type!(&a.type_);
            }
        }
        for m in &o.methods {
            for a in &m.args {
                visit_type!(&a.type_);
            }
            if let Some(r) = &m.return_type {
                visit_type!(r);
            }
        }
    }
    for cb in &metadata.callback_interfaces {
        for m in &cb.methods {
            for a in &m.args {
                visit_type!(&a.type_);
            }
            if let Some(r) = &m.return_type {
                visit_type!(r);
            }
        }
    }

    used
}

fn source_extension(source: &std::path::Path) -> Option<&str> {
    source.extension().and_then(|e| e.to_str())
}

/// Returns `true` when the source is a compiled native cdylib (`.dylib`, `.so`, `.dll`).
fn source_is_native_library(source: &std::path::Path) -> bool {
    matches!(source_extension(source), Some("dylib" | "so" | "dll"))
}

pub fn generate_bindings(args: &GenerateArgs) -> Result<()> {
    let cfg = config::load(args)?;
    let source_ext = source_extension(&args.source);
    let is_wasm_source = source_ext == Some("wasm");

    // When source IS a .wasm file, extract metadata directly from it.
    let metadata = if is_wasm_source {
        parsing::parse_wasm_source(&args.source, args.crate_name.as_deref())?
    } else {
        let library_mode = source_is_native_library(&args.source);
        parse_metadata(&args.source, args.crate_name.as_deref(), library_mode)?
    };
    let namespace = if metadata.namespace.is_empty() {
        namespace_from_source(&args.source)?
    } else {
        metadata.namespace.clone()
    };

    let module_name = cfg
        .module_name
        .clone()
        .unwrap_or_else(|| pascal_case(&namespace));
    let ffi_namespace = metadata.ffi_namespace.clone();

    fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("failed to create output dir: {}", args.out_dir.display()))?;

    // Always generate FFI-direct output.
    let content = render_ts(&module_name, &namespace, &ffi_namespace, &metadata, &cfg)?;
    let out_file = args.out_dir.join(format!("{namespace}.ts"));
    fs::write(&out_file, &content)
        .with_context(|| format!("failed to write: {}", out_file.display()))?;

    // Emit the shared runtime
    let runtime_file = args.out_dir.join("uniffi_runtime.ts");
    fs::write(&runtime_file, runtime_ts::RUNTIME_TS)
        .with_context(|| format!("failed to write: {}", runtime_file.display()))?;

    // Copy .wasm file to output directory (skip if source == destination)
    if is_wasm_source {
        let wasm_filename = format!("{namespace}.wasm");
        let dest = args.out_dir.join(&wasm_filename);
        let same_file = args.source.canonicalize().ok() == dest.canonicalize().ok();
        if !same_file {
            fs::copy(&args.source, &dest)
                .with_context(|| format!("failed to copy WASM: {}", args.source.display()))?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// FFI-direct TypeScript code generation
// ---------------------------------------------------------------------------

fn render_ts(
    module_name: &str,
    namespace: &str,
    ffi_namespace: &str,
    metadata: &BindingsMetadata,
    cfg: &config::JsBindingsConfig,
) -> Result<String> {
    let mut out = String::new();

    // Header
    out.push_str("// Generated by uniffi-bindgen-js. DO NOT EDIT.\n");
    out.push_str(
        "import { UniffiRuntime, UniFFIWriter, UniFFIReader } from './uniffi_runtime.js';\n",
    );
    out.push('\n');

    // Top-level await: auto-load the co-located .wasm file.
    // The second arg is the FFI namespace (crate name) used to construct
    // `ffi_{ns}_rustbuffer_*` export names — must match the Rust scaffolding.
    out.push_str(&format!(
        "const _rt = await UniffiRuntime.load(new URL('./{namespace}.wasm', import.meta.url), '{ffi_namespace}');\n"
    ));

    // External type imports
    let external_imports =
        collect_external_imports(metadata, &cfg.external_packages, &metadata.local_crate)?;
    for (import_path, type_names) in &external_imports {
        let names: Vec<&str> = type_names.iter().map(String::as_str).collect();
        out.push_str(&format!(
            "import {{ {} }} from '{import_path}';\n",
            names.join(", ")
        ));
    }

    // Custom type aliases (top-level, before everything else)
    for ct in &metadata.custom_types {
        if !cfg.exclude.contains(&ct.name) {
            let exported = cfg
                .rename
                .get(&ct.name)
                .cloned()
                .unwrap_or_else(|| ct.name.clone());
            let ts_type = cfg
                .custom_types
                .get(&ct.name)
                .and_then(|c| c.type_name.as_deref())
                .map(|s| s.to_string())
                .unwrap_or_else(|| ts_type_str(&ct.builtin));
            out.push('\n');
            out.push_str(&format!("export type {} = {};\n", exported, ts_type));
        }
    }

    // Extra imports from custom type configs
    let mut extra_imports: Vec<String> = Vec::new();
    for ct in &metadata.custom_types {
        if let Some(ct_cfg) = cfg.custom_types.get(&ct.name) {
            if let Some(imports) = &ct_cfg.imports {
                for imp in imports {
                    if !extra_imports.contains(imp) {
                        extra_imports.push(imp.clone());
                    }
                }
            }
        }
    }
    for imp in &extra_imports {
        out.push_str(&format!("import {imp};\n"));
    }

    // Error classes
    for e in &metadata.errors {
        if !cfg.exclude.contains(&e.name) {
            out.push('\n');
            out.push_str(&render_error_class(e, cfg, ffi_namespace));
        }
    }

    // Record interfaces
    for r in &metadata.records {
        if !cfg.exclude.contains(&r.name) {
            out.push('\n');
            out.push_str(&render_record_interface(r, cfg, ffi_namespace));
        }
    }

    // Enum types
    for e in &metadata.enums {
        if !cfg.exclude.contains(&e.name) {
            out.push('\n');
            out.push_str(&render_enum_type(e, cfg, ffi_namespace));
        }
    }

    // Callback interfaces (TypeScript interface declarations)
    for cb in &metadata.callback_interfaces {
        if !cfg.exclude.contains(&cb.name) {
            out.push('\n');
            out.push_str(&render_callback_interface(cb, cfg));
        }
    }

    // Callback VTable registration (must run at module init, before any calls that use callbacks)
    for cb in &metadata.callback_interfaces {
        if !cfg.exclude.contains(&cb.name) {
            out.push('\n');
            out.push_str(&ffi::gen_callback_vtable_registration(cb, ffi_namespace, cfg));
        }
    }

    // Object classes — handle-based with FFI calls
    let visible_objects: Vec<&ObjectDef> = metadata
        .objects
        .iter()
        .filter(|o| !cfg.exclude.contains(&o.name))
        .collect();
    for o in &visible_objects {
        out.push('\n');
        out.push_str(&render_object_class(o, ffi_namespace, cfg));
    }

    // Serialization helpers for compound types (records, enums, errors)
    out.push('\n');
    out.push_str("// --- Serialization helpers ---\n");
    for r in &metadata.records {
        if cfg.exclude.contains(&r.name) {
            continue;
        }
        out.push('\n');
        out.push_str(&ffi::gen_record_lower_fn(r, ffi_namespace, cfg));
        out.push_str(&ffi::gen_record_lift_fn(r, cfg));
    }
    for e in &metadata.enums {
        if cfg.exclude.contains(&e.name) {
            continue;
        }
        out.push('\n');
        if e.is_flat {
            out.push_str(&ffi::gen_flat_enum_lower_fn(e, ffi_namespace));
            out.push_str(&ffi::gen_flat_enum_lift_fn(e));
        } else {
            out.push_str(&ffi::gen_data_enum_lower_fn(e, ffi_namespace, cfg));
            out.push_str(&ffi::gen_data_enum_lift_fn(e, cfg));
        }
    }
    // Error value-type serialization helpers. Emitted when the error:
    // - has constructors (return type via RustBuffer needs _liftFoo)
    // - has methods (need _lowerFoo to serialize `this`)
    // - is used as a value type in record fields, enum variants, etc.
    let errors_as_value_types = collect_errors_as_value_types(metadata);
    for error in &metadata.errors {
        if cfg.exclude.contains(&error.name) {
            continue;
        }
        let has_ctors = !error.constructors.is_empty();
        let has_methods = !error.methods.is_empty();
        let is_value_type = errors_as_value_types.contains(&error.name);
        let needs_lower = has_methods || is_value_type;
        let needs_lift = has_ctors || is_value_type;
        if !needs_lower && !needs_lift {
            continue;
        }
        out.push('\n');
        if error.is_flat {
            if needs_lower {
                out.push_str(&ffi::gen_flat_error_value_lower_fn(error, ffi_namespace));
            }
            if needs_lift {
                out.push_str(&ffi::gen_flat_error_value_lift_fn(error));
            }
        } else {
            if needs_lower {
                out.push_str(&ffi::gen_rich_error_value_lower_fn(error, ffi_namespace, cfg));
            }
            if needs_lift {
                out.push_str(&ffi::gen_rich_error_value_lift_fn(error, cfg));
            }
        }
    }

    // Error lift helpers (for RustCallStatus error deserialization)
    let all_throws: HashSet<String> = {
        let mut names: HashSet<String> = HashSet::new();
        for f in &metadata.functions {
            if let Some(t) = &f.throws_type {
                names.insert(type_name(t));
            }
        }
        for o in &metadata.objects {
            for c in &o.constructors {
                if let Some(t) = &c.throws_type {
                    names.insert(type_name(t));
                }
            }
            for m in &o.methods {
                if let Some(t) = &m.throws_type {
                    names.insert(type_name(t));
                }
            }
        }
        for e in &metadata.enums {
            for c in &e.constructors {
                if let Some(t) = &c.throws_type {
                    names.insert(type_name(t));
                }
            }
            for m in &e.methods {
                if let Some(t) = &m.throws_type {
                    names.insert(type_name(t));
                }
            }
        }
        for e in &metadata.errors {
            for c in &e.constructors {
                if let Some(t) = &c.throws_type {
                    names.insert(type_name(t));
                }
            }
            for m in &e.methods {
                if let Some(t) = &m.throws_type {
                    names.insert(type_name(t));
                }
            }
        }
        for r in &metadata.records {
            for c in &r.constructors {
                if let Some(t) = &c.throws_type {
                    names.insert(type_name(t));
                }
            }
            for m in &r.methods {
                if let Some(t) = &m.throws_type {
                    names.insert(type_name(t));
                }
            }
        }
        names
    };
    for error in &metadata.errors {
        if all_throws.contains(&error.name) {
            out.push('\n');
            if error.is_flat {
                out.push_str(&ffi::gen_flat_error_lift_fn(error));
            } else {
                out.push_str(&ffi::gen_rich_error_lift_fn(error, cfg));
            }
        }
    }
    // Object-based errors (is_error objects) need _liftError for RustCallStatus.
    for o in &visible_objects {
        if o.is_error && all_throws.contains(&o.name) {
            out.push('\n');
            out.push_str(&ffi::gen_object_error_lift_fn(&o.name));
        }
    }

    // Namespace with top-level functions
    let visible_fns: Vec<&FnDef> = metadata
        .functions
        .iter()
        .filter(|f| !cfg.exclude.contains(&f.name))
        .collect();

    if !visible_fns.is_empty() {
        out.push('\n');
        out.push_str(&render_jsdoc(metadata.namespace_docstring.as_deref(), ""));
        out.push_str(&format!("export namespace {module_name} {{\n"));
        for f in &visible_fns {
            out.push_str(&render_function(f, ffi_namespace, cfg));
        }
        out.push_str("}\n");
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// FFI-direct function rendering
// ---------------------------------------------------------------------------

use naming::{camel_case, safe_js_identifier};
use render_helpers::{
    duration_annotations, duration_return_annotation, render_jsdoc_with_throws, render_param,
    ts_return_type,
};

/// Emit clone calls for Object-typed arguments and build the arg pairs
/// for gen_ffi_call / gen_async_ffi_call.
///
/// For each Object-typed arg, emits a clone line and replaces the arg name
/// with the cloned handle variable (a raw bigint). Non-object args pass through.
fn prepare_object_args(
    args: &[ArgDef],
    js_arg_names: &[String],
    ffi_namespace: &str,
    indent: &str,
    out: &mut String,
) -> Vec<(String, uniffi_bindgen::interface::Type)> {
    let mut result = Vec::new();
    for (name, a) in js_arg_names.iter().zip(args.iter()) {
        if let uniffi_bindgen::interface::Type::Object { name: obj_name, .. } = &a.type_ {
            let clone_fn = ffi::fn_clone(ffi_namespace, obj_name);
            let clone_var = format!("_clone_{name}");
            out.push_str(&format!(
                "{indent}const {clone_var} = _rt.cloneObjectHandle('{clone_fn}', {name}._handle);\n"
            ));
            result.push((clone_var, a.type_.clone()));
        } else {
            result.push((name.clone(), a.type_.clone()));
        }
    }
    result
}

/// Apply return wrapping to generated FFI call body.
///
/// Replaces `return _result;` in the body emitted by `gen_ffi_call` /
/// `gen_async_ffi_call` with a wrapped version:
/// - Object returns: wrap with `_fromHandle()`
/// - Custom returns with lift config: wrap with the lift expression
///
/// Panics in debug builds if the sentinel `return _result;` is not found.
fn apply_return_wrap(
    body: &mut String,
    return_type: Option<&uniffi_bindgen::interface::Type>,
    cfg: &config::JsBindingsConfig,
) {
    match return_type {
        Some(uniffi_bindgen::interface::Type::Object { name, .. }) => {
            assert!(
                body.contains("return _result;"),
                "apply_return_wrap: expected 'return _result;' in body for Object '{name}'"
            );
            *body = body.replace(
                "return _result;",
                &format!("return {name}._fromHandle(_result);"),
            );
        }
        Some(uniffi_bindgen::interface::Type::Custom { name, .. }) => {
            if let Some(ct_cfg) = cfg.custom_types.get(name) {
                let lifted = ct_cfg.lift_expr("_result");
                if lifted != "_result" {
                    assert!(
                        body.contains("return _result;"),
                        "apply_return_wrap: expected 'return _result;' in body for Custom '{name}'"
                    );
                    *body = body.replace("return _result;", &format!("return {lifted};"));
                }
            }
        }
        _ => {}
    }
}

/// Lower custom-typed args before FFI calls.
///
/// For each Custom-typed arg with a `lower` config, emits `const _ct_{name} = lower(name);`
/// and returns updated arg names with the lowered variable name.
fn apply_custom_type_arg_lowering(
    args: &[ArgDef],
    js_arg_names: &[String],
    cfg: &config::JsBindingsConfig,
    indent: &str,
    out: &mut String,
) -> Vec<String> {
    js_arg_names
        .iter()
        .zip(args.iter())
        .map(|(name, a)| {
            if let uniffi_bindgen::interface::Type::Custom { name: ct_name, .. } = &a.type_ {
                if let Some(ct_cfg) = cfg.custom_types.get(ct_name) {
                    let lowered = ct_cfg.lower_expr(name);
                    if lowered != *name {
                        let var = format!("_ct_{name}");
                        out.push_str(&format!("{indent}const {var} = {lowered};\n"));
                        return var;
                    }
                }
            }
            name.clone()
        })
        .collect()
}

fn render_function(f: &FnDef, ffi_namespace: &str, cfg: &config::JsBindingsConfig) -> String {
    let mut out = String::new();

    let exported = cfg
        .rename
        .get(&f.name)
        .map(|s| safe_js_identifier(s))
        .unwrap_or_else(|| safe_js_identifier(&camel_case(&f.name)));

    let params: Vec<String> = f.args.iter().map(render_param).collect();
    let ts_ret = ts_return_type(f.return_type.as_ref(), f.is_async);

    let ffi_name = ffi::ffibuf_fn_func(ffi_namespace, &f.name);

    let throws_name = f.throws_type.as_ref().map(type_name);

    let mut annotations = duration_annotations(&f.args);
    if let Some(ann) = duration_return_annotation(f.return_type.as_ref()) {
        annotations.push(ann);
    }
    out.push_str(&render_jsdoc_with_throws(
        f.docstring.as_deref(),
        throws_name.as_deref(),
        &annotations,
        "  ",
    ));

    let async_kw = if f.is_async { "async " } else { "" };
    out.push_str(&format!(
        "  export {async_kw}function {exported}({}): {ts_ret} {{\n",
        params.join(", ")
    ));

    // Build the JS variable names for args (camelCase, safe)
    let js_arg_names: Vec<String> = f
        .args
        .iter()
        .map(|a| safe_js_identifier(&camel_case(&a.name)))
        .collect();

    // Apply custom type lowering (transforms custom → builtin before serialization)
    let js_arg_names =
        apply_custom_type_arg_lowering(&f.args, &js_arg_names, cfg, "    ", &mut out);

    // Clone Object-typed args (FFI scaffolding consumes handles)
    let prepared = prepare_object_args(&f.args, &js_arg_names, ffi_namespace, "    ", &mut out);
    let arg_pairs: Vec<(&str, &uniffi_bindgen::interface::Type)> = prepared
        .iter()
        .map(|(name, t)| (name.as_str(), t))
        .collect();

    let mut body = if f.is_async {
        ffi::gen_async_ffi_call(
            &ffi_name,
            ffi_namespace,
            &arg_pairs,
            f.return_type.as_ref(),
            throws_name.as_deref(),
            "    ",
            cfg,
        )
    } else {
        ffi::gen_ffi_call(
            &ffi_name,
            ffi_namespace,
            &arg_pairs,
            f.return_type.as_ref(),
            throws_name.as_deref(),
            "    ",
            cfg,
        )
    };

    // Wrap Object/Custom returns
    apply_return_wrap(&mut body, f.return_type.as_ref(), cfg);

    out.push_str(&body);
    out.push_str("\n  }\n");

    out
}

// ---------------------------------------------------------------------------
// FFI-direct object class rendering
// ---------------------------------------------------------------------------

fn render_object_class(o: &ObjectDef, ffi_namespace: &str, cfg: &config::JsBindingsConfig) -> String {
    let mut out = String::new();
    let name = &o.name;

    out.push_str(&render_jsdoc(o.docstring.as_deref(), ""));
    if o.is_error {
        out.push_str(&format!("export class {name} extends Error {{\n"));
    } else {
        out.push_str(&format!("export class {name} {{\n"));
    }

    // Handle-based internals
    out.push_str("  /** @internal */\n");
    out.push_str("  readonly _handle: bigint;\n");
    out.push_str("  private _freed = false;\n");
    out.push_str(&format!(
        "  private _assertLive(): void {{\n    if (this._freed) throw new Error('{name} object has been freed');\n  }}\n"
    ));

    // Private constructor (from handle)
    out.push_str("  private constructor(handle: bigint) {\n");
    if o.is_error {
        out.push_str(&format!("    super('{name}');\n"));
        out.push_str(&format!(
            "    Object.defineProperty(this, 'name', {{ value: '{name}' }});\n"
        ));
    }
    out.push_str("    this._handle = handle;\n");
    let free_fn = ffi::fn_free(ffi_namespace, name);
    out.push_str(&format!(
        "    _rt.registerPointer(this, '{free_fn}', handle);\n"
    ));
    out.push_str("  }\n");

    // Internal factory from handle
    out.push_str("  /** @internal */\n");
    out.push_str(&format!(
        "  static _fromHandle(handle: bigint): {name} {{ return new {name}(handle); }}\n"
    ));

    // Constructors (skipped for [Trait] interfaces — they're only created by Rust)
    if !o.is_trait {
        let primary_ctor = o.constructors.iter().find(|c| c.name == "new");
        let named_ctors: Vec<&CtorDef> =
            o.constructors.iter().filter(|c| c.name != "new").collect();

        if let Some(ctor) = primary_ctor {
            out.push_str(&render_ctor(ctor, name, ffi_namespace, "create", cfg));
        }

        for ctor in named_ctors {
            let exported = cfg
                .rename
                .get(&format!("{}.{}", name, ctor.name))
                .map(|s| safe_js_identifier(s))
                .unwrap_or_else(|| safe_js_identifier(&camel_case(&ctor.name)));
            out.push_str(&render_ctor(ctor, name, ffi_namespace, &exported, cfg));
        }
    }

    // Methods
    for m in &o.methods {
        if cfg.exclude.contains(&format!("{}.{}", name, m.name)) {
            continue;
        }
        let exported = cfg
            .rename
            .get(&format!("{}.{}", name, m.name))
            .map(|s| safe_js_identifier(s))
            .unwrap_or_else(|| safe_js_identifier(&camel_case(&m.name)));
        out.push_str(&render_method(m, name, ffi_namespace, &exported, cfg));
    }

    // Synthesised trait methods (instance methods on the class)
    out.push_str(&render_object_trait_methods(o, ffi_namespace, cfg));

    // free()
    out.push_str("  /** Releases the underlying WASM resource. Safe to call more than once. */\n");
    out.push_str("  free(): void {\n");
    out.push_str("    if (this._freed) return;\n");
    out.push_str("    this._freed = true;\n");
    out.push_str("    _rt.unregisterPointer(this);\n");
    let free_fn = ffi::fn_free(ffi_namespace, name);
    out.push_str(&format!("    _rt.callFree('{free_fn}', this._handle);\n"));
    out.push_str("  }\n");

    out.push_str("}\n");

    // Symbol.dispose — guarded for pre-ES2025 engines (matches wasm-bindgen pattern)
    out.push_str(&format!(
        "if (Symbol.dispose) ({name} as any).prototype[Symbol.dispose] = {name}.prototype.free;\n"
    ));
    out
}

/// Render synthesised trait methods (Display, Debug, Eq, Hash, Ord)
/// as instance methods on an object class using FFI calls.
fn render_object_trait_methods(
    o: &ObjectDef,
    ffi_namespace: &str,
    cfg: &config::JsBindingsConfig,
) -> String {
    let mut out = String::new();
    let name = &o.name;

    if let Some(method_name) = &o.traits.display {
        out.push_str(&render_object_trait_method(
            "toString",
            method_name,
            name,
            ffi_namespace,
            &uniffi_bindgen::interface::Type::String,
            false, // has_other
            cfg,
        ));
    }

    if let Some(method_name) = &o.traits.debug {
        out.push_str(&render_object_trait_method(
            "toDebugString",
            method_name,
            name,
            ffi_namespace,
            &uniffi_bindgen::interface::Type::String,
            false,
            cfg,
        ));
    }

    if let Some(method_name) = &o.traits.eq {
        let ffi_name = ffi::ffibuf_fn_method(ffi_namespace, name, method_name);
        let handle_type = uniffi_bindgen::interface::Type::Object {
            name: name.to_string(),
            module_path: String::new(),
            imp: uniffi_bindgen::interface::ObjectImpl::Struct,
        };
        let clone_fn = ffi::fn_clone(ffi_namespace, name);

        out.push_str(&format!("  equals(other: {name}): boolean {{\n"));
        out.push_str("    this._assertLive();\n");
        out.push_str(&format!(
            "    const _clonedSelf = _rt.cloneObjectHandle('{clone_fn}', this._handle);\n"
        ));
        out.push_str(&format!(
            "    const _clonedOther = _rt.cloneObjectHandle('{clone_fn}', other._handle);\n"
        ));

        let arg_pairs: Vec<(&str, &uniffi_bindgen::interface::Type)> = vec![
            ("_clonedSelf", &handle_type),
            ("_clonedOther", &handle_type),
        ];

        let body = ffi::gen_ffi_call(
            &ffi_name,
            ffi_namespace,
            &arg_pairs,
            Some(&uniffi_bindgen::interface::Type::Boolean),
            None,
            "    ",
            cfg,
        );
        out.push_str(&body);
        out.push_str("\n  }\n");
    }

    if let Some(method_name) = &o.traits.hash {
        out.push_str(&render_object_trait_method(
            "hashCode",
            method_name,
            name,
            ffi_namespace,
            &uniffi_bindgen::interface::Type::UInt64,
            false,
            cfg,
        ));
    }

    if let Some(method_name) = &o.traits.ord {
        let ffi_name = ffi::ffibuf_fn_method(ffi_namespace, name, method_name);
        let handle_type = uniffi_bindgen::interface::Type::Object {
            name: name.to_string(),
            module_path: String::new(),
            imp: uniffi_bindgen::interface::ObjectImpl::Struct,
        };
        let clone_fn = ffi::fn_clone(ffi_namespace, name);

        out.push_str(&format!("  compareTo(other: {name}): number {{\n"));
        out.push_str("    this._assertLive();\n");
        out.push_str(&format!(
            "    const _clonedSelf = _rt.cloneObjectHandle('{clone_fn}', this._handle);\n"
        ));
        out.push_str(&format!(
            "    const _clonedOther = _rt.cloneObjectHandle('{clone_fn}', other._handle);\n"
        ));

        let arg_pairs: Vec<(&str, &uniffi_bindgen::interface::Type)> = vec![
            ("_clonedSelf", &handle_type),
            ("_clonedOther", &handle_type),
        ];

        let body = ffi::gen_ffi_call(
            &ffi_name,
            ffi_namespace,
            &arg_pairs,
            Some(&uniffi_bindgen::interface::Type::Int8),
            None,
            "    ",
            cfg,
        );
        out.push_str(&body);
        out.push_str("\n  }\n");
    }

    out
}

/// Render a single-self-arg object trait method (Display, Debug, Hash).
fn render_object_trait_method(
    exported: &str,
    ffi_method_name: &str,
    class_name: &str,
    ffi_namespace: &str,
    return_type: &uniffi_bindgen::interface::Type,
    _has_other: bool,
    cfg: &config::JsBindingsConfig,
) -> String {
    let mut out = String::new();
    let ts_ret = ts_type_str(return_type);
    let ffi_name = ffi::ffibuf_fn_method(ffi_namespace, class_name, ffi_method_name);
    let clone_fn = ffi::fn_clone(ffi_namespace, class_name);

    let handle_type = uniffi_bindgen::interface::Type::Object {
        name: class_name.to_string(),
        module_path: String::new(),
        imp: uniffi_bindgen::interface::ObjectImpl::Struct,
    };

    out.push_str(&format!("  {exported}(): {ts_ret} {{\n"));
    out.push_str("    this._assertLive();\n");
    out.push_str(&format!(
        "    const _clonedHandle = _rt.cloneObjectHandle('{clone_fn}', this._handle);\n"
    ));

    let arg_pairs: Vec<(&str, &uniffi_bindgen::interface::Type)> =
        vec![("_clonedHandle", &handle_type)];

    let body = ffi::gen_ffi_call(
        &ffi_name,
        ffi_namespace,
        &arg_pairs,
        Some(return_type),
        None,
        "    ",
        cfg,
    );
    out.push_str(&body);
    out.push_str("\n  }\n");
    out
}

fn render_ctor(
    ctor: &CtorDef,
    class_name: &str,
    ffi_namespace: &str,
    exported: &str,
    cfg: &config::JsBindingsConfig,
) -> String {
    let mut out = String::new();

    let params: Vec<String> = ctor.args.iter().map(render_param).collect();
    let async_kw = if ctor.is_async { "async " } else { "" };
    let ret_type = if ctor.is_async {
        format!("Promise<{class_name}>")
    } else {
        class_name.to_string()
    };

    let throws_name = ctor.throws_type.as_ref().map(type_name);
    let annotations = duration_annotations(&ctor.args);
    out.push_str(&render_jsdoc_with_throws(
        ctor.docstring.as_deref(),
        throws_name.as_deref(),
        &annotations,
        "  ",
    ));
    out.push_str(&format!(
        "  static {async_kw}{exported}({}): {ret_type} {{\n",
        params.join(", ")
    ));

    let ffi_name = ffi::ffibuf_fn_constructor(ffi_namespace, class_name, &ctor.name);
    let js_arg_names: Vec<String> = ctor
        .args
        .iter()
        .map(|a| safe_js_identifier(&camel_case(&a.name)))
        .collect();

    // Clone Object-typed args (FFI scaffolding consumes handles)
    let prepared = prepare_object_args(&ctor.args, &js_arg_names, ffi_namespace, "    ", &mut out);
    let arg_pairs: Vec<(&str, &uniffi_bindgen::interface::Type)> = prepared
        .iter()
        .map(|(name, t)| (name.as_str(), t))
        .collect();

    // Constructor returns a Handle (Object type)
    let handle_type = uniffi_bindgen::interface::Type::Object {
        name: class_name.to_string(),
        module_path: String::new(),
        imp: uniffi_bindgen::interface::ObjectImpl::Struct,
    };

    let body = if ctor.is_async {
        ffi::gen_async_ffi_call(
            &ffi_name,
            ffi_namespace,
            &arg_pairs,
            Some(&handle_type),
            throws_name.as_deref(),
            "    ",
            cfg,
        )
    } else {
        ffi::gen_ffi_call(
            &ffi_name,
            ffi_namespace,
            &arg_pairs,
            Some(&handle_type),
            throws_name.as_deref(),
            "    ",
            cfg,
        )
    };
    // Replace the generic return with constructing the class from handle
    let body = body.replace(
        "return _result;",
        &format!("return new {class_name}(_result);"),
    );
    out.push_str(&body);
    out.push_str("\n  }\n");
    out
}

fn render_method(
    m: &MethodDef,
    class_name: &str,
    ffi_namespace: &str,
    exported: &str,
    cfg: &config::JsBindingsConfig,
) -> String {
    let mut out = String::new();

    let params: Vec<String> = m.args.iter().map(render_param).collect();
    let ts_ret = ts_return_type(m.return_type.as_ref(), m.is_async);
    let async_kw = if m.is_async { "async " } else { "" };

    let throws_name = m.throws_type.as_ref().map(type_name);
    let mut annotations = duration_annotations(&m.args);
    if let Some(ann) = duration_return_annotation(m.return_type.as_ref()) {
        annotations.push(ann);
    }
    out.push_str(&render_jsdoc_with_throws(
        m.docstring.as_deref(),
        throws_name.as_deref(),
        &annotations,
        "  ",
    ));
    out.push_str(&format!(
        "  {async_kw}{exported}({}): {ts_ret} {{\n",
        params.join(", ")
    ));
    out.push_str("    this._assertLive();\n");

    // Clone the handle before passing to FFI — the scaffolding consumes handles
    let clone_fn = ffi::fn_clone(ffi_namespace, class_name);
    out.push_str(&format!(
        "    const _clonedHandle = _rt.cloneObjectHandle('{clone_fn}', this._handle);\n"
    ));

    let ffi_name = ffi::ffibuf_fn_method(ffi_namespace, class_name, &m.name);

    // Method args: self handle (cloned) + user args
    let handle_type = uniffi_bindgen::interface::Type::Object {
        name: class_name.to_string(),
        module_path: String::new(),
        imp: uniffi_bindgen::interface::ObjectImpl::Struct,
    };

    let js_arg_names: Vec<String> = m
        .args
        .iter()
        .map(|a| safe_js_identifier(&camel_case(&a.name)))
        .collect();

    // Apply custom type lowering
    let js_arg_names =
        apply_custom_type_arg_lowering(&m.args, &js_arg_names, cfg, "    ", &mut out);

    // Clone Object-typed user args (FFI scaffolding consumes handles)
    let prepared = prepare_object_args(&m.args, &js_arg_names, ffi_namespace, "    ", &mut out);

    let mut arg_pairs: Vec<(&str, &uniffi_bindgen::interface::Type)> =
        vec![("_clonedHandle", &handle_type)];
    for (name, t) in &prepared {
        arg_pairs.push((name.as_str(), t));
    }

    let mut body = if m.is_async {
        ffi::gen_async_ffi_call(
            &ffi_name,
            ffi_namespace,
            &arg_pairs,
            m.return_type.as_ref(),
            throws_name.as_deref(),
            "    ",
            cfg,
        )
    } else {
        ffi::gen_ffi_call(
            &ffi_name,
            ffi_namespace,
            &arg_pairs,
            m.return_type.as_ref(),
            throws_name.as_deref(),
            "    ",
            cfg,
        )
    };

    // Wrap Object/Custom returns
    apply_return_wrap(&mut body, m.return_type.as_ref(), cfg);

    out.push_str(&body);
    out.push_str("\n  }\n");
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use uniffi_bindgen::interface::{DefaultValue, Literal, Type};

    use super::naming::*;
    use super::render_helpers::*;

    #[test]
    fn camel_case_handles_underscores() {
        assert_eq!(camel_case("ping"), "ping");
        assert_eq!(camel_case("broken_greet"), "brokenGreet");
        assert_eq!(camel_case("async_greet"), "asyncGreet");
    }

    #[test]
    fn safe_js_identifier_escapes_reserved_words() {
        assert_eq!(safe_js_identifier("class"), "class_");
        assert_eq!(safe_js_identifier("return"), "return_");
        assert_eq!(safe_js_identifier("delete"), "delete_");
        assert_eq!(safe_js_identifier("void"), "void_");
        assert_eq!(safe_js_identifier("yield"), "yield_");
        assert_eq!(safe_js_identifier("async"), "async_");
        assert_eq!(safe_js_identifier("await"), "await_");
        assert_eq!(safe_js_identifier("typeof"), "typeof_");
        assert_eq!(safe_js_identifier("catch"), "catch_");
        assert_eq!(safe_js_identifier("finally"), "finally_");
        assert_eq!(safe_js_identifier("static"), "static_");
        // Non-reserved words should pass through unchanged
        assert_eq!(safe_js_identifier("name"), "name");
        assert_eq!(safe_js_identifier("count"), "count");
        assert_eq!(safe_js_identifier("value"), "value");
    }

    #[test]
    fn pascal_case_handles_common_cases() {
        assert_eq!(pascal_case("simple_bindings"), "SimpleBindings");
        assert_eq!(pascal_case("simple-bindings"), "SimpleBindings");
    }

    #[test]
    fn render_jsdoc_none_returns_empty() {
        assert_eq!(render_jsdoc(None, ""), "");
    }

    #[test]
    fn render_jsdoc_blank_returns_empty() {
        assert_eq!(render_jsdoc(Some("   "), ""), "");
        assert_eq!(render_jsdoc(Some(""), ""), "");
    }

    #[test]
    fn render_jsdoc_single_line() {
        assert_eq!(render_jsdoc(Some("Hello."), ""), "/** Hello. */\n");
    }

    #[test]
    fn render_jsdoc_single_line_with_indent() {
        assert_eq!(render_jsdoc(Some("Hello."), "  "), "  /** Hello. */\n");
    }

    #[test]
    fn render_jsdoc_multi_line() {
        let doc = "First line.\nSecond line.";
        let expected = "/**\n * First line.\n * Second line.\n */\n";
        assert_eq!(render_jsdoc(Some(doc), ""), expected);
    }

    #[test]
    fn render_jsdoc_multi_line_with_blank() {
        let doc = "First.\n\nSecond.";
        let expected = "/**\n * First.\n *\n * Second.\n */\n";
        assert_eq!(render_jsdoc(Some(doc), ""), expected);
    }

    #[test]
    fn render_jsdoc_escapes_comment_close() {
        assert_eq!(
            render_jsdoc(Some("Returns a*/ value."), ""),
            "/** Returns a*\\/ value. */\n"
        );
    }

    #[test]
    fn render_jsdoc_long_single_line_uses_block_format() {
        let doc = "This is a very long docstring that exceeds the eighty character threshold.";
        let result = render_jsdoc(Some(doc), "");
        assert!(
            result.starts_with("/**\n"),
            "expected block format, got: {result}"
        );
    }

    // -----------------------------------------------------------------------
    // render_literal tests
    // -----------------------------------------------------------------------

    #[test]
    fn render_literal_boolean() {
        assert_eq!(render_literal(&Literal::Boolean(true)), "true");
        assert_eq!(render_literal(&Literal::Boolean(false)), "false");
    }

    #[test]
    fn render_literal_string() {
        assert_eq!(render_literal(&Literal::String("hello".into())), "'hello'");
        assert_eq!(
            render_literal(&Literal::String("it's a \\path".into())),
            "'it\\'s a \\\\path'"
        );
    }

    #[test]
    fn render_literal_uint() {
        assert_eq!(
            render_literal(&Literal::UInt(
                42,
                uniffi_bindgen::interface::Radix::Decimal,
                Type::UInt32
            )),
            "42"
        );
        assert_eq!(
            render_literal(&Literal::UInt(
                100,
                uniffi_bindgen::interface::Radix::Decimal,
                Type::UInt64
            )),
            "100n"
        );
        assert_eq!(
            render_literal(&Literal::UInt(
                7,
                uniffi_bindgen::interface::Radix::Decimal,
                Type::Int64
            )),
            "7n"
        );
    }

    #[test]
    fn render_literal_int() {
        assert_eq!(
            render_literal(&Literal::Int(
                -5,
                uniffi_bindgen::interface::Radix::Decimal,
                Type::Int32
            )),
            "-5"
        );
        assert_eq!(
            render_literal(&Literal::Int(
                -99,
                uniffi_bindgen::interface::Radix::Decimal,
                Type::Int64
            )),
            "-99n"
        );
    }

    #[test]
    fn render_literal_float() {
        assert_eq!(
            render_literal(&Literal::Float("3.14".into(), Type::Float64)),
            "3.14"
        );
    }

    #[test]
    fn render_literal_enum() {
        assert_eq!(
            render_literal(&Literal::Enum("North".into(), Type::String)),
            "'North'"
        );
    }

    #[test]
    fn render_literal_empty_sequence() {
        assert_eq!(render_literal(&Literal::EmptySequence), "[]");
    }

    #[test]
    fn render_literal_empty_map() {
        assert_eq!(render_literal(&Literal::EmptyMap), "new Map()");
    }

    #[test]
    fn render_literal_none() {
        assert_eq!(render_literal(&Literal::None), "null");
    }

    #[test]
    fn render_literal_some() {
        assert_eq!(
            render_literal(&Literal::Some {
                inner: Box::new(DefaultValue::Literal(Literal::String("x".into())))
            }),
            "'x'"
        );
    }

    // -----------------------------------------------------------------------
    // render_param tests
    // -----------------------------------------------------------------------

    #[test]
    fn render_param_no_default() {
        let arg = super::types::ArgDef {
            name: "user_name".into(),
            type_: Type::String,
            default: None,
        };
        assert_eq!(render_param(&arg), "userName: string");
    }

    #[test]
    fn render_param_literal_default() {
        let arg = super::types::ArgDef {
            name: "count".into(),
            type_: Type::Int32,
            default: Some(DefaultValue::Literal(Literal::UInt(
                0,
                uniffi_bindgen::interface::Radix::Decimal,
                Type::Int32,
            ))),
        };
        assert_eq!(render_param(&arg), "count: number = 0");
    }

    #[test]
    fn render_param_unspecified_default() {
        let arg = super::types::ArgDef {
            name: "count".into(),
            type_: Type::Int32,
            default: Some(DefaultValue::Default),
        };
        assert_eq!(render_param(&arg), "count?: number");
    }

    // -----------------------------------------------------------------------
    // ts_type_str tests
    // -----------------------------------------------------------------------

    #[test]
    fn ts_type_str_primitives() {
        assert_eq!(ts_type_str(&Type::String), "string");
        assert_eq!(ts_type_str(&Type::Boolean), "boolean");
        assert_eq!(ts_type_str(&Type::Int32), "number");
        assert_eq!(ts_type_str(&Type::Float64), "number");
        assert_eq!(ts_type_str(&Type::Int64), "bigint");
    }

    #[test]
    fn ts_type_str_optional() {
        assert_eq!(
            ts_type_str(&Type::Optional {
                inner_type: Box::new(Type::String)
            }),
            "string | null"
        );
    }

    #[test]
    fn ts_type_str_sequence() {
        assert_eq!(
            ts_type_str(&Type::Sequence {
                inner_type: Box::new(Type::String)
            }),
            "string[]"
        );
        // FFI mode: Sequence<u64> → bigint[], not BigUint64Array
        assert_eq!(
            ts_type_str(&Type::Sequence {
                inner_type: Box::new(Type::UInt64)
            }),
            "bigint[]"
        );
    }

    #[test]
    fn ts_type_str_sequence_parenthesizes_optional_inner() {
        let t = Type::Sequence {
            inner_type: Box::new(Type::Optional {
                inner_type: Box::new(Type::String),
            }),
        };
        assert_eq!(ts_type_str(&t), "(string | null)[]");
    }

    #[test]
    fn ts_type_str_map() {
        let t = Type::Map {
            key_type: Box::new(Type::String),
            value_type: Box::new(Type::Int32),
        };
        assert_eq!(ts_type_str(&t), "Map<string, number>");
    }

    #[test]
    fn ts_type_str_named_types() {
        assert_eq!(
            ts_type_str(&Type::Enum {
                name: "MyEnum".into(),
                module_path: "crate".into()
            }),
            "MyEnum"
        );
        assert_eq!(
            ts_type_str(&Type::Record {
                name: "Point".into(),
                module_path: "crate".into()
            }),
            "Point"
        );
    }

    // -----------------------------------------------------------------------
    // ts_return_type tests
    // -----------------------------------------------------------------------

    #[test]
    fn ts_return_type_sync() {
        assert_eq!(ts_return_type(Some(&Type::String), false), "string");
        assert_eq!(ts_return_type(None, false), "void");
    }

    #[test]
    fn ts_return_type_async() {
        assert_eq!(ts_return_type(Some(&Type::String), true), "Promise<string>");
        assert_eq!(ts_return_type(None, true), "Promise<void>");
    }

    // -----------------------------------------------------------------------
    // External imports tests
    // -----------------------------------------------------------------------

    #[test]
    fn external_imports_deterministic_order() {
        use super::types::*;
        use std::collections::HashMap;

        let metadata = BindingsMetadata {
            enums: vec![
                EnumDef {
                    name: "ZetaEnum".into(),
                    variants: vec![],
                    is_flat: true,
                    is_non_exhaustive: false,
                    docstring: None,
                    methods: vec![],
                    constructors: vec![],
                    traits: SynthesisedTraits::default(),
                },
                EnumDef {
                    name: "AlphaEnum".into(),
                    variants: vec![],
                    is_flat: true,
                    is_non_exhaustive: false,
                    docstring: None,
                    methods: vec![],
                    constructors: vec![],
                    traits: SynthesisedTraits::default(),
                },
            ],
            functions: vec![FnDef {
                name: "use_ext".into(),
                args: vec![
                    super::types::ArgDef {
                        name: "z".into(),
                        type_: Type::Enum {
                            name: "ZetaEnum".into(),
                            module_path: "ext_crate::sub".into(),
                        },
                        default: None,
                    },
                    super::types::ArgDef {
                        name: "a".into(),
                        type_: Type::Enum {
                            name: "AlphaEnum".into(),
                            module_path: "ext_crate::sub".into(),
                        },
                        default: None,
                    },
                ],
                return_type: None,
                throws_type: None,
                is_async: false,
                docstring: None,
            }],
            ..Default::default()
        };
        let mut ext_pkg = HashMap::new();
        ext_pkg.insert(
            "ext_crate".to_string(),
            "./ext_crate_bindings.js".to_string(),
        );
        let cfg = super::config::JsBindingsConfig {
            external_packages: ext_pkg,
            ..Default::default()
        };

        let imports = super::external_types::collect_external_imports(
            &metadata,
            &cfg.external_packages,
            &metadata.local_crate,
        )
        .unwrap();

        assert_eq!(imports.len(), 1);
        let (path, names) = imports.iter().next().unwrap();
        assert_eq!(path, "./ext_crate_bindings.js");
        let names_vec: Vec<&String> = names.iter().collect();
        assert_eq!(names_vec, vec!["AlphaEnum", "ZetaEnum"]);
    }
}
