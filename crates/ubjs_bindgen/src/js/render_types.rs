// ---------------------------------------------------------------------------
// Type rendering: errors, enums, records, callbacks, lift functions
// ---------------------------------------------------------------------------

use std::collections::HashMap;

use uniffi_bindgen::interface::Type;

use super::config::{self, CustomTypeConfig};
use super::naming::{camel_case, safe_js_identifier, snake_case};
use super::render_helpers::{
    render_call_body, render_jsdoc, render_literal, render_param, ts_return_type, ts_type_str,
    type_name,
};
use super::type_lifting::lift_return;
use super::types::*;

/// Apply custom type `lower` to an argument expression if configured.
fn lower_custom_arg(
    arg_expr: &str,
    arg_type: &Type,
    custom_types: &HashMap<String, CustomTypeConfig>,
) -> String {
    if let Type::Custom { name, .. } = arg_type {
        if let Some(ct_cfg) = custom_types.get(name.as_str()) {
            return ct_cfg.lower_expr(arg_expr);
        }
    }
    arg_expr.to_string()
}

/// Wrap a call expression with custom type `lift` for the return type if configured.
fn lift_custom_return(
    call_expr: &str,
    return_type: Option<&Type>,
    custom_types: &HashMap<String, CustomTypeConfig>,
) -> String {
    if let Some(Type::Custom { name, .. }) = return_type {
        if let Some(ct_cfg) = custom_types.get(name.as_str()) {
            return ct_cfg.lift_expr(call_expr);
        }
    }
    call_expr.to_string()
}

// ---------------------------------------------------------------------------
// Error class generation
// ---------------------------------------------------------------------------

pub(super) fn render_error_class(
    e: &UdlError,
    cfg: &config::JsBindingsConfig,
    local_crate: &str,
) -> String {
    let mut out = String::new();
    let name = &e.name;

    if e.is_flat {
        // Flat error: single `tag` string property, no variant fields
        let mut tag_parts: Vec<String> =
            e.variants.iter().map(|v| format!("'{}'", v.name)).collect();
        if e.is_non_exhaustive {
            tag_parts.push("string".to_string());
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
        out.push_str(&render_enum_constructors_on_class(
            &e.constructors,
            name,
            cfg,
        ));
        out.push_str(&render_enum_methods_on_class(
            &e.methods,
            name,
            cfg,
            local_crate,
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
            out.push_str("  | { tag: string; [key: string]: unknown };\n");
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
        out.push_str(&render_enum_constructors_on_class(
            &e.constructors,
            name,
            cfg,
        ));
        out.push_str(&render_enum_methods_on_class(
            &e.methods,
            name,
            cfg,
            local_crate,
        ));
        out.push_str("}\n");
    }

    out
}

/// Render enum constructors as static methods on a class or in a companion namespace.
/// Enum constructors are exported by wasm-bindgen as `{snake_case_enum}_{ctor_name}(...)`.
pub(super) fn render_enum_constructors_on_class(
    constructors: &[UdlConstructor],
    enum_name: &str,
    cfg: &config::JsBindingsConfig,
) -> String {
    render_enum_constructors(constructors, enum_name, cfg, "static")
}

/// Render enum constructors as static functions in a companion namespace.
pub(super) fn render_enum_constructors_in_namespace(
    constructors: &[UdlConstructor],
    enum_name: &str,
    cfg: &config::JsBindingsConfig,
) -> String {
    render_enum_constructors(constructors, enum_name, cfg, "export function")
}

/// Shared implementation for rendering enum constructors.
/// `decl_kind` is either `"static"` (for class bodies) or `"export function"` (for namespaces).
fn render_enum_constructors(
    constructors: &[UdlConstructor],
    enum_name: &str,
    cfg: &config::JsBindingsConfig,
    decl_kind: &str,
) -> String {
    let mut out = String::new();
    let bg_name = snake_case(enum_name);
    for ctor in constructors {
        let key = format!("{enum_name}.{}", ctor.name);
        if cfg.exclude.contains(&key) {
            continue;
        }
        let exported = cfg
            .rename
            .get(&key)
            .map(|s| safe_js_identifier(s))
            .unwrap_or_else(|| safe_js_identifier(&camel_case(&ctor.name)));
        let params: Vec<String> = ctor.args.iter().map(render_param).collect();
        let args: Vec<String> = ctor
            .args
            .iter()
            .map(|a| safe_js_identifier(&camel_case(&a.name)))
            .collect();
        let async_kw = if ctor.is_async { "async " } else { "" };
        let await_kw = if ctor.is_async { "await " } else { "" };
        let ret_type = if ctor.is_async {
            format!("Promise<{enum_name}>")
        } else {
            enum_name.to_string()
        };
        let ctor_fn = if ctor.name == "new" {
            format!("new __bg.{bg_name}")
        } else {
            format!("__bg.{bg_name}_{}", ctor.name)
        };
        let inner_call = format!("{ctor_fn}({})", args.join(", "));

        out.push_str(&render_jsdoc(ctor.docstring.as_deref(), "  "));
        if let Some(throws) = &ctor.throws_type {
            let lift = format!("_lift{}", type_name(throws));
            out.push_str(&format!(
                "  {decl_kind} {async_kw}{exported}({}): {ret_type} {{\n    try {{ return {await_kw}{inner_call}; }} catch (e) {{ return {lift}(e); }}\n  }}\n",
                params.join(", ")
            ));
        } else {
            out.push_str(&format!(
                "  {decl_kind} {async_kw}{exported}({}): {ret_type} {{ return {await_kw}{inner_call}; }}\n",
                params.join(", ")
            ));
        }
    }
    out
}

/// Render methods on an error class (instance methods that delegate to wasm-bindgen).
/// Error enum methods are exported by wasm-bindgen as `{snake_case_enum}_{method_name}(self, ...)`.
pub(super) fn render_enum_methods_on_class(
    methods: &[UdlMethod],
    enum_name: &str,
    cfg: &config::JsBindingsConfig,
    local_crate: &str,
) -> String {
    let mut out = String::new();
    let bg_name = snake_case(enum_name);
    for m in methods {
        if cfg.exclude.contains(&format!("{enum_name}.{}", m.name)) {
            continue;
        }
        let exported = cfg
            .rename
            .get(&format!("{enum_name}.{}", m.name))
            .map(|s| safe_js_identifier(s))
            .unwrap_or_else(|| safe_js_identifier(&camel_case(&m.name)));
        let params: Vec<String> = m.args.iter().map(render_param).collect();
        let ts_ret = ts_return_type(m.return_type.as_ref(), m.is_async);
        let call_args: Vec<String> = m
            .args
            .iter()
            .map(|a| {
                let base = safe_js_identifier(&camel_case(&a.name));
                lower_custom_arg(&base, &a.type_, &cfg.custom_types)
            })
            .collect();
        let self_plus_args = if call_args.is_empty() {
            "this".to_string()
        } else {
            format!("this, {}", call_args.join(", "))
        };
        let raw_call = format!("__bg.{bg_name}_{}({self_plus_args})", m.name);
        let call_expr = lift_return(&raw_call, m.return_type.as_ref(), m.is_async, local_crate);
        let call_expr = lift_custom_return(&call_expr, m.return_type.as_ref(), &cfg.custom_types);
        let async_kw = if m.is_async { "async " } else { "" };
        let throws_name = m.throws_type.as_ref().map(type_name);
        let body = render_call_body(
            &call_expr,
            m.return_type.is_some(),
            throws_name.as_deref(),
            None,
        );

        out.push_str(&render_jsdoc(m.docstring.as_deref(), "  "));
        out.push_str(&format!(
            "  {async_kw}{exported}({}): {ts_ret} {{{body}}}\n",
            params.join(", ")
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// Error lift helper generation
// ---------------------------------------------------------------------------

pub(super) fn render_lift_fn(e: &UdlError) -> String {
    let mut out = String::new();
    let name = &e.name;
    let fn_name = format!("_lift{name}");

    out.push_str(&format!("function {fn_name}(e: unknown): never {{\n"));

    if e.is_flat {
        out.push_str(
            "  const tag = typeof e === 'string' ? e : (e instanceof Error ? e.message : null);\n",
        );
        for v in &e.variants {
            out.push_str(&format!(
                "  if (tag === '{}') throw new {name}('{}');\n",
                v.name, v.name
            ));
        }
        if e.is_non_exhaustive {
            out.push_str(&format!(
                "  if (typeof tag === 'string') throw new {name}(tag);\n"
            ));
        }
    } else {
        // Rich error: Rust serialises variant as a JSON string {"tag":"...","field":val}
        out.push_str("  try {\n");
        out.push_str("    const raw = typeof e === 'string' ? JSON.parse(e) : (e instanceof Error ? JSON.parse(e.message) : e);\n");
        out.push_str("    const tag = raw?.tag as string | undefined;\n");
        for v in &e.variants {
            if v.fields.is_empty() {
                out.push_str(&format!(
                    "    if (tag === '{}') throw {name}.{}();\n",
                    v.name, v.name
                ));
            } else {
                let args: Vec<String> = v
                    .fields
                    .iter()
                    .map(|f| format!("raw.{}", safe_js_identifier(&camel_case(&f.name))))
                    .collect();
                out.push_str(&format!(
                    "    if (tag === '{}') throw {name}.{}({});\n",
                    v.name,
                    v.name,
                    args.join(", ")
                ));
            }
        }
        if e.is_non_exhaustive {
            out.push_str(&format!(
                "    if (typeof tag === 'string') throw new {name}(raw as {name}Variant);\n"
            ));
        }
        out.push_str("  } catch (inner) {\n");
        out.push_str(&format!("    if (inner instanceof {name}) throw inner;\n"));
        out.push_str("  }\n");
    }

    out.push_str("  throw e;\n");
    out.push_str("}\n");
    out
}

/// Render a lift function for an object type used as an error.
///
/// When wasm-bindgen throws an object error, the thrown value is already the
/// wasm-bindgen wrapper class. We lift it into our wrapper class and re-throw.
pub(super) fn render_object_error_lift_fn(name: &str) -> String {
    let fn_name = format!("_lift{name}");
    let mut out = String::new();
    out.push_str(&format!("function {fn_name}(e: unknown): never {{\n"));
    out.push_str(&format!("  if (e instanceof {name}) throw e;\n"));
    out.push_str(&format!(
        "  if (e instanceof __bg.{name}) throw {name}._fromInner(e);\n"
    ));
    out.push_str("  throw e;\n");
    out.push_str("}\n");
    out
}

// ---------------------------------------------------------------------------
// Record interface generation
// ---------------------------------------------------------------------------

pub(super) fn render_record_interface(
    r: &UdlRecord,
    cfg: &config::JsBindingsConfig,
    local_crate: &str,
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
    let has_traits = r.traits.display.is_some() || r.traits.eq.is_some() || r.traits.hash.is_some();
    let has_companion = !r.methods.is_empty() || !r.constructors.is_empty() || has_traits;
    if has_companion {
        let name = &r.name;
        let bg_name = snake_case(name);
        out.push_str(&format!("export namespace {name} {{\n"));

        // Constructors (static factory functions)
        out.push_str(&render_enum_constructors_in_namespace(
            &r.constructors,
            name,
            cfg,
        ));

        // Synthesised trait methods
        out.push_str(&render_trait_methods(&r.traits, name, &bg_name));

        // Methods (instance-style, take `self` as first param)
        for m in &r.methods {
            if cfg.exclude.contains(&format!("{name}.{}", m.name)) {
                continue;
            }
            let exported = cfg
                .rename
                .get(&format!("{name}.{}", m.name))
                .map(|s| safe_js_identifier(s))
                .unwrap_or_else(|| safe_js_identifier(&camel_case(&m.name)));
            let self_param = format!("self: {name}");
            let other_params: Vec<String> = m.args.iter().map(render_param).collect();
            let all_params = if other_params.is_empty() {
                self_param
            } else {
                format!("{self_param}, {}", other_params.join(", "))
            };
            let ts_ret = ts_return_type(m.return_type.as_ref(), m.is_async);
            let call_args: Vec<String> = m
                .args
                .iter()
                .map(|a| {
                    let base = safe_js_identifier(&camel_case(&a.name));
                    lower_custom_arg(&base, &a.type_, &cfg.custom_types)
                })
                .collect();
            let self_plus_args = if call_args.is_empty() {
                "self".to_string()
            } else {
                format!("self, {}", call_args.join(", "))
            };
            let raw_call = format!("__bg.{bg_name}_{}({self_plus_args})", m.name);
            let call_expr = lift_return(&raw_call, m.return_type.as_ref(), m.is_async, local_crate);
            let call_expr =
                lift_custom_return(&call_expr, m.return_type.as_ref(), &cfg.custom_types);
            let async_kw = if m.is_async { "async " } else { "" };
            let body = render_call_body(&call_expr, m.return_type.is_some(), None, None);

            out.push_str(&render_jsdoc(m.docstring.as_deref(), "  "));
            out.push_str(&format!(
                "  export {async_kw}function {exported}({all_params}): {ts_ret} {{{body}}}\n",
            ));
        }
        out.push_str("}\n");
    }

    out
}

// ---------------------------------------------------------------------------
// Enum type generation
// ---------------------------------------------------------------------------

pub(super) fn render_enum_type(
    e: &UdlEnum,
    cfg: &config::JsBindingsConfig,
    local_crate: &str,
) -> String {
    let mut out = String::new();
    if e.is_flat {
        // Flat enum → TypeScript union of string literals.
        // Individual variant docstrings have no JSDoc anchor in a union type, so
        // any variant docs are folded into the parent type's JSDoc as a bullet list.
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
            parts.push("string".to_string());
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
        // Variant docstrings have no per-member anchor in a union type; the type-level
        // docstring is the only JSDoc anchor available.
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
            out.push_str("  | { tag: string; [key: string]: unknown };\n");
        }
    }

    // Enum methods, constructors, and trait methods are emitted in a companion namespace
    // (TS declaration merging allows a namespace with the same name as a type alias).
    let has_traits = e.traits.display.is_some() || e.traits.eq.is_some() || e.traits.hash.is_some();
    let has_companion = !e.methods.is_empty() || !e.constructors.is_empty() || has_traits;
    if has_companion {
        let name = &e.name;
        let bg_name = snake_case(name);
        out.push_str(&format!("export namespace {name} {{\n"));

        // Constructors (static factory functions)
        out.push_str(&render_enum_constructors_in_namespace(
            &e.constructors,
            name,
            cfg,
        ));

        // Synthesised trait methods
        out.push_str(&render_trait_methods(&e.traits, name, &bg_name));

        // Methods (instance-style, take `self` as first param)
        for m in &e.methods {
            if cfg.exclude.contains(&format!("{name}.{}", m.name)) {
                continue;
            }
            let exported = cfg
                .rename
                .get(&format!("{name}.{}", m.name))
                .map(|s| safe_js_identifier(s))
                .unwrap_or_else(|| safe_js_identifier(&camel_case(&m.name)));
            let self_param = format!("self: {name}");
            let other_params: Vec<String> = m.args.iter().map(render_param).collect();
            let all_params = if other_params.is_empty() {
                self_param
            } else {
                format!("{self_param}, {}", other_params.join(", "))
            };
            let ts_ret = ts_return_type(m.return_type.as_ref(), m.is_async);
            let call_args: Vec<String> = m
                .args
                .iter()
                .map(|a| {
                    let base = safe_js_identifier(&camel_case(&a.name));
                    lower_custom_arg(&base, &a.type_, &cfg.custom_types)
                })
                .collect();
            let self_plus_args = if call_args.is_empty() {
                "self".to_string()
            } else {
                format!("self, {}", call_args.join(", "))
            };
            let raw_call = format!("__bg.{bg_name}_{}({self_plus_args})", m.name);
            let call_expr = lift_return(&raw_call, m.return_type.as_ref(), m.is_async, local_crate);
            let call_expr =
                lift_custom_return(&call_expr, m.return_type.as_ref(), &cfg.custom_types);
            let async_kw = if m.is_async { "async " } else { "" };
            let body = render_call_body(&call_expr, m.return_type.is_some(), None, None);

            out.push_str(&render_jsdoc(m.docstring.as_deref(), "  "));
            out.push_str(&format!(
                "  export {async_kw}function {exported}({all_params}): {ts_ret} {{{body}}}\n",
            ));
        }
        out.push_str("}\n");
    }

    out
}

// ---------------------------------------------------------------------------
// Callback interface generation
// ---------------------------------------------------------------------------

pub(super) fn render_callback_interface(
    cb: &UdlCallbackInterface,
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

// ---------------------------------------------------------------------------
// Synthesised trait method generation
// ---------------------------------------------------------------------------

/// Render synthesised trait methods (Display, Eq, Hash) as functions in a companion namespace.
/// These call into wasm-bindgen exports named by the UniFFI trait convention.
fn render_trait_methods(traits: &SynthesisedTraits, type_name: &str, bg_name: &str) -> String {
    let mut out = String::new();

    if let Some(method_name) = &traits.display {
        out.push_str(&format!(
            "  export function toString(self: {type_name}): string {{ return __bg.{bg_name}_{method_name}(self); }}\n"
        ));
    }

    if let Some(method_name) = &traits.eq {
        out.push_str(&format!(
            "  export function equals(self: {type_name}, other: {type_name}): boolean {{ return __bg.{bg_name}_{method_name}(self, other); }}\n"
        ));
    }

    if let Some(method_name) = &traits.hash {
        out.push_str(&format!(
            "  export function hashCode(self: {type_name}): bigint {{ return __bg.{bg_name}_{method_name}(self); }}\n"
        ));
    }

    out
}
