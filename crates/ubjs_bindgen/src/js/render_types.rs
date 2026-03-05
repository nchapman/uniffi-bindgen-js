// ---------------------------------------------------------------------------
// Type rendering: errors, enums, records, callbacks
// ---------------------------------------------------------------------------

use super::config;
use super::ffi;
use super::naming::{camel_case, safe_js_identifier};
use super::render_helpers::{
    duration_annotations, duration_return_annotation, render_jsdoc, render_jsdoc_with_throws,
    render_literal, render_param, ts_return_type, ts_type_str, type_name,
};
use super::types::*;

// ---------------------------------------------------------------------------
// Error class generation
// ---------------------------------------------------------------------------

pub(super) fn render_error_class(
    e: &ErrorDef,
    cfg: &config::JsBindingsConfig,
    namespace: &str,
) -> String {
    let mut out = String::new();
    let name = &e.name;

    if e.is_flat {
        // Flat error: single `tag` string property, no variant fields
        let mut tag_parts: Vec<String> =
            e.variants.iter().map(|v| format!("'{}'", v.name)).collect();
        if e.is_non_exhaustive {
            tag_parts.push("(string & {})".to_string());
        }

        out.push_str(&render_jsdoc(e.docstring.as_deref(), ""));
        out.push_str(&format!("export class {name} extends Error {{\n"));
        out.push_str(&format!("  override readonly name = '{name}' as const;\n"));
        out.push_str(&format!(
            "  constructor(public readonly tag: {}) {{\n",
            tag_parts.join(" | ")
        ));
        out.push_str("    super(tag);\n");
        out.push_str("  }\n");
        for v in &e.variants {
            out.push_str(&render_jsdoc(v.docstring.as_deref(), "  "));
            let factory_name = safe_js_identifier(&v.name);
            out.push_str(&format!(
                "  static {factory_name}(): {name} {{ return new {name}('{}'); }}\n",
                v.name
            ));
        }
        out.push_str(&render_constructors_on_class_ffi(
            &e.constructors,
            name,
            namespace,
            cfg,
        ));
        out.push_str(&render_methods_on_class_ffi(
            &e.methods, name, namespace, cfg,
        ));
        out.push_str("}\n");
    } else {
        // Rich error: each variant may have different fields; use a discriminated
        // union stored in `variant` and expose a flat set of optional field getters.
        let variant_type = format!("{name}Variant");
        let last_known = e.variants.len().saturating_sub(1);
        out.push_str(&format!("export type {variant_type} =\n"));
        for (i, v) in e.variants.iter().enumerate() {
            let sep = if !e.is_non_exhaustive && i == last_known {
                ";"
            } else {
                ""
            };
            if v.fields.is_empty() {
                out.push_str(&format!("  | {{ tag: '{}' }}{sep}\n", v.name));
            } else {
                let fields: Vec<String> = v
                    .fields
                    .iter()
                    .map(|f| {
                        format!(
                            "{}: {}",
                            safe_js_identifier(&camel_case(&f.name)),
                            ts_type_str(&f.type_)
                        )
                    })
                    .collect();
                out.push_str(&format!(
                    "  | {{ tag: '{}', {} }}{sep}\n",
                    v.name,
                    fields.join(", ")
                ));
            }
        }
        if e.is_non_exhaustive {
            out.push_str("  | { tag: string & {}; [key: string]: unknown };\n");
        }
        out.push('\n');
        out.push_str(&render_jsdoc(e.docstring.as_deref(), ""));
        out.push_str(&format!("export class {name} extends Error {{\n"));
        out.push_str(&format!("  override readonly name = '{name}' as const;\n"));
        out.push_str(&format!(
            "  constructor(public readonly variant: {variant_type}) {{\n"
        ));
        out.push_str("    super(variant.tag);\n");
        out.push_str("  }\n");
        for v in &e.variants {
            let params: Vec<String> = v
                .fields
                .iter()
                .map(|f| {
                    format!(
                        "{}: {}",
                        safe_js_identifier(&camel_case(&f.name)),
                        ts_type_str(&f.type_)
                    )
                })
                .collect();
            // Object literal uses camelCase shorthand (param names match property names).
            let obj_fields: Vec<String> = v
                .fields
                .iter()
                .map(|f| safe_js_identifier(&camel_case(&f.name)))
                .collect();
            let variant_obj = if v.fields.is_empty() {
                format!("{{ tag: '{}' }}", v.name)
            } else {
                format!("{{ tag: '{}', {} }}", v.name, obj_fields.join(", "))
            };
            out.push_str(&render_jsdoc(v.docstring.as_deref(), "  "));
            let factory_name = safe_js_identifier(&v.name);
            out.push_str(&format!(
                "  static {factory_name}({}): {name} {{ return new {name}({variant_obj}); }}\n",
                params.join(", ")
            ));
        }
        out.push_str(&render_constructors_on_class_ffi(
            &e.constructors,
            name,
            namespace,
            cfg,
        ));
        out.push_str(&render_methods_on_class_ffi(
            &e.methods, name, namespace, cfg,
        ));
        out.push_str("}\n");
    }

    out
}

/// Render constructors as static methods on a class using FFI calls.
fn render_constructors_on_class_ffi(
    constructors: &[CtorDef],
    type_name: &str,
    namespace: &str,
    cfg: &config::JsBindingsConfig,
) -> String {
    render_constructors_ffi(constructors, type_name, namespace, cfg, "static")
}

/// Render constructors as static functions in a companion namespace using FFI calls.
fn render_constructors_in_namespace_ffi(
    constructors: &[CtorDef],
    type_name: &str,
    namespace: &str,
    cfg: &config::JsBindingsConfig,
) -> String {
    render_constructors_ffi(constructors, type_name, namespace, cfg, "export function")
}

/// Shared implementation for rendering constructors via FFI.
fn render_constructors_ffi(
    constructors: &[CtorDef],
    parent_name: &str,
    namespace: &str,
    cfg: &config::JsBindingsConfig,
    decl_kind: &str,
) -> String {
    let mut out = String::new();
    for ctor in constructors {
        let key = format!("{parent_name}.{}", ctor.name);
        if cfg.exclude.contains(&key) {
            continue;
        }
        let exported = cfg
            .rename
            .get(&key)
            .map(|s| safe_js_identifier(s))
            .unwrap_or_else(|| safe_js_identifier(&camel_case(&ctor.name)));
        let params: Vec<String> = ctor.args.iter().map(render_param).collect();
        let async_kw = if ctor.is_async { "async " } else { "" };
        let ret_type = if ctor.is_async {
            format!("Promise<{parent_name}>")
        } else {
            parent_name.to_string()
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
            "  {decl_kind} {async_kw}{exported}({}): {ret_type} {{\n",
            params.join(", ")
        ));

        // Build FFI call
        let ffi_name = ffi::ffibuf_fn_constructor(namespace, parent_name, &ctor.name);
        let js_arg_names: Vec<String> = ctor
            .args
            .iter()
            .map(|a| safe_js_identifier(&camel_case(&a.name)))
            .collect();
        let arg_pairs: Vec<(&str, &uniffi_bindgen::interface::Type)> = js_arg_names
            .iter()
            .zip(ctor.args.iter())
            .map(|(name, a)| (name.as_str(), &a.type_))
            .collect();

        // Constructor returns the type via RustBuffer (for enums/records/errors).
        // Use a Record type to get RustBuffer return handling — this causes
        // gen_ffi_call to emit `_lift{parent_name}(r)`. The corresponding
        // _liftFoo helper is emitted conditionally in render_ts() when
        // the type has constructors.
        let return_type = uniffi_bindgen::interface::Type::Record {
            name: parent_name.to_string(),
            module_path: String::new(),
        };

        let body = if ctor.is_async {
            ffi::gen_async_ffi_call(
                &ffi_name,
                namespace,
                &arg_pairs,
                Some(&return_type),
                throws_name.as_deref(),
                "    ",
            )
        } else {
            ffi::gen_ffi_call(
                &ffi_name,
                namespace,
                &arg_pairs,
                Some(&return_type),
                throws_name.as_deref(),
                "    ",
            )
        };
        out.push_str(&body);
        out.push_str("\n  }\n");
    }
    out
}

/// Render methods on an error/enum class as instance methods using FFI calls.
fn render_methods_on_class_ffi(
    methods: &[MethodDef],
    parent_name: &str,
    namespace: &str,
    cfg: &config::JsBindingsConfig,
) -> String {
    // Error enum methods would require lowering `this` as an enum/error value.
    // This is uncommon — no existing fixture exercises it. If needed, generate
    // a _lower function for the error and pass `this` as the first FFI arg.
    if methods.is_empty() {
        return String::new();
    }
    let _ = (parent_name, namespace, cfg);
    // TODO: implement error enum instance methods via FFI when a test case arises
    String::new()
}

/// Render methods in a companion namespace using FFI calls.
fn render_companion_methods_ffi(
    methods: &[MethodDef],
    parent_name: &str,
    self_type: &uniffi_bindgen::interface::Type,
    namespace: &str,
    cfg: &config::JsBindingsConfig,
) -> String {
    let mut out = String::new();
    for m in methods {
        if cfg.exclude.contains(&format!("{parent_name}.{}", m.name)) {
            continue;
        }
        let exported = cfg
            .rename
            .get(&format!("{parent_name}.{}", m.name))
            .map(|s| safe_js_identifier(s))
            .unwrap_or_else(|| safe_js_identifier(&camel_case(&m.name)));
        let value_param = format!("value: {parent_name}");
        let other_params: Vec<String> = m.args.iter().map(render_param).collect();
        let all_params = if other_params.is_empty() {
            value_param
        } else {
            format!("{value_param}, {}", other_params.join(", "))
        };
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
            "  export {async_kw}function {exported}({all_params}): {ts_ret} {{\n",
        ));

        // Build FFI call: self value as first arg + user args
        let ffi_name = ffi::ffibuf_fn_method(namespace, parent_name, &m.name);

        let js_arg_names: Vec<String> = m
            .args
            .iter()
            .map(|a| safe_js_identifier(&camel_case(&a.name)))
            .collect();

        let mut arg_pairs: Vec<(&str, &uniffi_bindgen::interface::Type)> =
            vec![("value", self_type)];
        for (name, a) in js_arg_names.iter().zip(m.args.iter()) {
            arg_pairs.push((name.as_str(), &a.type_));
        }

        let body = if m.is_async {
            ffi::gen_async_ffi_call(
                &ffi_name,
                namespace,
                &arg_pairs,
                m.return_type.as_ref(),
                throws_name.as_deref(),
                "    ",
            )
        } else {
            ffi::gen_ffi_call(
                &ffi_name,
                namespace,
                &arg_pairs,
                m.return_type.as_ref(),
                throws_name.as_deref(),
                "    ",
            )
        };
        out.push_str(&body);
        out.push_str("\n  }\n");
    }
    out
}

/// Render synthesised trait methods (Display, Debug, Eq, Hash, Ord) in a companion namespace
/// using FFI calls.
fn render_trait_methods_ffi(
    traits: &SynthesisedTraits,
    parent_name: &str,
    self_type: &uniffi_bindgen::interface::Type,
    namespace: &str,
) -> String {
    let mut out = String::new();

    if let Some(method_name) = &traits.display {
        out.push_str(&render_trait_method_ffi(
            "toString",
            method_name,
            parent_name,
            self_type,
            namespace,
            &uniffi_bindgen::interface::Type::String,
            false,
        ));
    }

    if let Some(method_name) = &traits.debug {
        out.push_str(&render_trait_method_ffi(
            "toDebugString",
            method_name,
            parent_name,
            self_type,
            namespace,
            &uniffi_bindgen::interface::Type::String,
            false,
        ));
    }

    if let Some(method_name) = &traits.eq {
        out.push_str(&render_trait_eq_ffi(
            method_name,
            parent_name,
            self_type,
            namespace,
        ));
    }

    if let Some(method_name) = &traits.hash {
        out.push_str(&render_trait_method_ffi(
            "hashCode",
            method_name,
            parent_name,
            self_type,
            namespace,
            &uniffi_bindgen::interface::Type::UInt64,
            false,
        ));
    }

    if let Some(method_name) = &traits.ord {
        out.push_str(&render_trait_ord_ffi(
            method_name,
            parent_name,
            self_type,
            namespace,
        ));
    }

    out
}

/// Render a single trait method (one self arg, one return) as an FFI call.
fn render_trait_method_ffi(
    exported: &str,
    ffi_method_name: &str,
    parent_name: &str,
    self_type: &uniffi_bindgen::interface::Type,
    namespace: &str,
    return_type: &uniffi_bindgen::interface::Type,
    _is_async: bool,
) -> String {
    let mut out = String::new();
    let ffi_name = ffi::ffibuf_fn_method(namespace, parent_name, ffi_method_name);
    let ts_ret = ts_type_str(return_type);

    out.push_str(&format!(
        "  export function {exported}(value: {parent_name}): {ts_ret} {{\n"
    ));

    let arg_pairs: Vec<(&str, &uniffi_bindgen::interface::Type)> = vec![("value", self_type)];

    let body = ffi::gen_ffi_call(
        &ffi_name,
        namespace,
        &arg_pairs,
        Some(return_type),
        None,
        "    ",
    );
    out.push_str(&body);
    out.push_str("\n  }\n");
    out
}

/// Render the Eq trait method (two self args, boolean return).
fn render_trait_eq_ffi(
    ffi_method_name: &str,
    parent_name: &str,
    self_type: &uniffi_bindgen::interface::Type,
    namespace: &str,
) -> String {
    let mut out = String::new();
    let ffi_name = ffi::ffibuf_fn_method(namespace, parent_name, ffi_method_name);

    out.push_str(&format!(
        "  export function equals(value: {parent_name}, other: {parent_name}): boolean {{\n"
    ));

    let arg_pairs: Vec<(&str, &uniffi_bindgen::interface::Type)> =
        vec![("value", self_type), ("other", self_type)];

    let body = ffi::gen_ffi_call(
        &ffi_name,
        namespace,
        &arg_pairs,
        Some(&uniffi_bindgen::interface::Type::Boolean),
        None,
        "    ",
    );
    out.push_str(&body);
    out.push_str("\n  }\n");
    out
}

/// Render the Ord trait method (two self args, number return).
fn render_trait_ord_ffi(
    ffi_method_name: &str,
    parent_name: &str,
    self_type: &uniffi_bindgen::interface::Type,
    namespace: &str,
) -> String {
    let mut out = String::new();
    let ffi_name = ffi::ffibuf_fn_method(namespace, parent_name, ffi_method_name);

    out.push_str(&format!(
        "  export function compareTo(value: {parent_name}, other: {parent_name}): number {{\n"
    ));

    let arg_pairs: Vec<(&str, &uniffi_bindgen::interface::Type)> =
        vec![("value", self_type), ("other", self_type)];

    let body = ffi::gen_ffi_call(
        &ffi_name,
        namespace,
        &arg_pairs,
        Some(&uniffi_bindgen::interface::Type::Int8),
        None,
        "    ",
    );
    out.push_str(&body);
    out.push_str("\n  }\n");
    out
}

// ---------------------------------------------------------------------------
// Record interface generation
// ---------------------------------------------------------------------------

pub(super) fn render_record_interface(
    r: &RecordDef,
    cfg: &config::JsBindingsConfig,
    namespace: &str,
) -> String {
    let mut out = String::new();
    out.push_str(&render_jsdoc(r.docstring.as_deref(), ""));
    out.push_str(&format!("export interface {} {{\n", r.name));
    for f in &r.fields {
        let ts_name = safe_js_identifier(&camel_case(&f.name));
        let ts_type = ts_type_str(&f.type_);
        out.push_str(&render_jsdoc(f.docstring.as_deref(), "  "));
        // Fields with defaults are optional (callers may omit them).
        let optional = if f.default.is_some() { "?" } else { "" };
        out.push_str(&format!("  {ts_name}{optional}: {ts_type};\n"));
    }
    out.push_str("}\n");

    // Record methods, constructors, and trait methods are emitted in a companion namespace
    // (TS declaration merging allows a namespace with the same name as an interface).
    let has_traits = r.traits.display.is_some()
        || r.traits.debug.is_some()
        || r.traits.eq.is_some()
        || r.traits.hash.is_some()
        || r.traits.ord.is_some();
    let has_companion = !r.methods.is_empty() || !r.constructors.is_empty() || has_traits;
    if has_companion {
        let name = &r.name;
        let self_type = uniffi_bindgen::interface::Type::Record {
            name: name.clone(),
            module_path: String::new(),
        };
        out.push_str(&format!("export namespace {name} {{\n"));

        // Constructors (static factory functions)
        out.push_str(&render_constructors_in_namespace_ffi(
            &r.constructors,
            name,
            namespace,
            cfg,
        ));

        // Synthesised trait methods
        out.push_str(&render_trait_methods_ffi(
            &r.traits, name, &self_type, namespace,
        ));

        // Methods (instance-style, take `self` as first param)
        out.push_str(&render_companion_methods_ffi(
            &r.methods, name, &self_type, namespace, cfg,
        ));

        out.push_str("}\n");
    }

    out
}

// ---------------------------------------------------------------------------
// Enum type generation
// ---------------------------------------------------------------------------

pub(super) fn render_enum_type(
    e: &EnumDef,
    cfg: &config::JsBindingsConfig,
    namespace: &str,
) -> String {
    let mut out = String::new();
    if e.is_flat {
        // Flat enum → TypeScript union of string literals.
        let has_variant_docs = e
            .variants
            .iter()
            .any(|v| v.docstring.is_some() || v.discr.is_some());
        let doc = if has_variant_docs {
            let base = e.docstring.as_deref().unwrap_or("").trim().to_string();
            let bullets: Vec<String> = e
                .variants
                .iter()
                .filter_map(|v| {
                    let doc = v.docstring.as_deref().map(|d| d.trim().to_string());
                    let discr_info = v
                        .discr
                        .as_ref()
                        .map(|l| format!("(= {})", render_literal(l)));
                    match (doc, discr_info) {
                        (Some(d), Some(disc)) => Some(format!("- `{}` {}: {}", v.name, disc, d)),
                        (Some(d), None) => Some(format!("- `{}`: {}", v.name, d)),
                        (None, Some(disc)) => Some(format!("- `{}` {}", v.name, disc)),
                        (None, None) => None,
                    }
                })
                .collect();
            let joined = bullets.join("\n");
            if base.is_empty() {
                Some(joined)
            } else {
                Some(format!("{base}\n{joined}"))
            }
        } else {
            e.docstring.clone()
        };
        out.push_str(&render_jsdoc(doc.as_deref(), ""));
        let mut parts: Vec<String> = e.variants.iter().map(|v| format!("'{}'", v.name)).collect();
        if e.is_non_exhaustive {
            parts.push("(string & {})".to_string());
        }
        out.push_str(&format!(
            "export type {} = {};\n",
            e.name,
            parts.join(" | ")
        ));
        // If variants have explicit discriminant values, emit a companion const
        // object mapping variant names to their numeric values.
        let has_discrs = e.variants.iter().all(|v| v.discr.is_some());
        if has_discrs {
            out.push_str(&format!(
                "/** Discriminant values for {{@link {}}}. */\n",
                e.name,
            ));
            out.push_str(&format!("export const {name}Values = {{\n", name = e.name,));
            for v in &e.variants {
                if let Some(lit) = &v.discr {
                    out.push_str(&format!("  {}: {},\n", v.name, render_literal(lit),));
                }
            }
            out.push_str("} as const;\n");
        }
    } else {
        // Data enum → discriminated union.
        out.push_str(&render_jsdoc(e.docstring.as_deref(), ""));
        out.push_str(&format!("export type {} =\n", e.name));
        let last_known = e.variants.len().saturating_sub(1);
        for (i, v) in e.variants.iter().enumerate() {
            let sep = if !e.is_non_exhaustive && i == last_known {
                ";"
            } else {
                ""
            };
            if v.fields.is_empty() {
                out.push_str(&format!("  | {{ tag: '{}' }}{sep}\n", v.name));
            } else {
                let fields: Vec<String> = v
                    .fields
                    .iter()
                    .map(|f| {
                        format!(
                            "{}: {}",
                            safe_js_identifier(&camel_case(&f.name)),
                            ts_type_str(&f.type_)
                        )
                    })
                    .collect();
                out.push_str(&format!(
                    "  | {{ tag: '{}', {} }}{sep}\n",
                    v.name,
                    fields.join(", ")
                ));
            }
        }
        if e.is_non_exhaustive {
            out.push_str("  | { tag: string & {}; [key: string]: unknown };\n");
        }
    }

    // Enum methods, constructors, and trait methods are emitted in a companion namespace
    let has_traits = e.traits.display.is_some()
        || e.traits.debug.is_some()
        || e.traits.eq.is_some()
        || e.traits.hash.is_some()
        || e.traits.ord.is_some();
    let has_companion = !e.methods.is_empty() || !e.constructors.is_empty() || has_traits;
    if has_companion {
        let name = &e.name;
        let self_type = uniffi_bindgen::interface::Type::Enum {
            name: name.clone(),
            module_path: String::new(),
        };
        out.push_str(&format!("export namespace {name} {{\n"));

        // Constructors (static factory functions)
        out.push_str(&render_constructors_in_namespace_ffi(
            &e.constructors,
            name,
            namespace,
            cfg,
        ));

        // Synthesised trait methods
        out.push_str(&render_trait_methods_ffi(
            &e.traits, name, &self_type, namespace,
        ));

        // Methods (instance-style, take `self` as first param)
        out.push_str(&render_companion_methods_ffi(
            &e.methods, name, &self_type, namespace, cfg,
        ));

        out.push_str("}\n");
    }

    out
}

// ---------------------------------------------------------------------------
// Callback interface generation
// ---------------------------------------------------------------------------

pub(super) fn render_callback_interface(
    cb: &CallbackInterfaceDef,
    cfg: &config::JsBindingsConfig,
) -> String {
    let mut out = String::new();
    let name = &cb.name;
    out.push_str(&render_jsdoc(cb.docstring.as_deref(), ""));
    out.push_str(&format!("export interface {name} {{\n"));
    for m in &cb.methods {
        // Per-method exclude uses the "InterfaceName.method_name" key (same convention as objects).
        if cfg.exclude.contains(&format!("{name}.{}", m.name)) {
            continue;
        }
        let exported = cfg
            .rename
            .get(&format!("{name}.{}", m.name))
            .map(|s| safe_js_identifier(s))
            .unwrap_or_else(|| safe_js_identifier(&camel_case(&m.name)));
        let params: Vec<String> = m.args.iter().map(render_param).collect();
        let ts_ret = ts_return_type(m.return_type.as_ref(), m.is_async);
        out.push_str(&render_jsdoc(m.docstring.as_deref(), "  "));
        out.push_str(&format!("  {exported}({}): {ts_ret};\n", params.join(", ")));
    }
    out.push_str("}\n");
    out
}
