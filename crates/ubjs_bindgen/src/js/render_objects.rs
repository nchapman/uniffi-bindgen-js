// ---------------------------------------------------------------------------
// Object class and top-level function generation
// ---------------------------------------------------------------------------

use std::collections::HashMap;

use uniffi_bindgen::interface::Type;

use super::config::{self, CustomTypeConfig};
use super::naming::{camel_case, safe_js_identifier, snake_case};
use super::render_helpers::{
    duration_annotations, duration_return_annotation, member_call, render_call_body, render_jsdoc,
    render_jsdoc_with_throws, render_param, ts_return_type, type_name, wasm_call,
};
use super::type_lifting::lift_return;
use super::types::*;

/// Render a single constructor (primary or named) as a static factory method.
pub(super) fn render_ctor(
    ctor: &UdlConstructor,
    class_name: &str,
    call_prefix: &str,
    exported: &str,
) -> String {
    let mut out = String::new();
    let params: Vec<String> = ctor.args.iter().map(render_param).collect();
    let args: Vec<String> = ctor
        .args
        .iter()
        .map(|a| safe_js_identifier(&camel_case(&a.name)))
        .collect();
    let inner_expr = format!("{call_prefix}({})", args.join(", "));

    let async_kw = if ctor.is_async { "async " } else { "" };
    let await_kw = if ctor.is_async { "await " } else { "" };
    let ret_type = if ctor.is_async {
        format!("Promise<{class_name}>")
    } else {
        class_name.to_string()
    };

    let throws_name = ctor.throws_type.as_ref().map(type_name);
    let mut annotations = duration_annotations(&ctor.args);
    if let Some(ann) = duration_return_annotation(None) {
        annotations.push(ann);
    }
    out.push_str(&render_jsdoc_with_throws(
        ctor.docstring.as_deref(),
        throws_name.as_deref(),
        &annotations,
        "  ",
    ));
    if let Some(throws) = &ctor.throws_type {
        let lift = format!("_lift{}", type_name(throws));
        out.push_str(&format!(
            "  static {async_kw}{exported}({}): {ret_type} {{\n    try {{ return {class_name}._fromInner({await_kw}{inner_expr}); }} catch (e) {{ return {lift}(e); }}\n  }}\n",
            params.join(", ")
        ));
    } else {
        out.push_str(&format!(
            "  static {async_kw}{exported}({}): {ret_type} {{ return {class_name}._fromInner({await_kw}{inner_expr}); }}\n",
            params.join(", ")
        ));
    }
    out
}

pub(super) fn render_object_class(
    o: &UdlObject,
    cfg: &config::JsBindingsConfig,
    local_crate: &str,
) -> String {
    let mut out = String::new();
    let name = &o.name;
    let bg_name = snake_case(name); // wasm-bindgen uses the Rust struct name

    out.push_str(&render_jsdoc(o.docstring.as_deref(), ""));
    if o.is_error {
        out.push_str(&format!("export class {name} extends Error {{\n"));
    } else {
        out.push_str(&format!("export class {name} {{\n"));
    }
    out.push_str(&format!("  private readonly _inner: __bg.{name};\n"));
    out.push_str("  private _freed = false;\n");
    out.push_str(&format!(
        "  private _assertLive(): void {{\n    if (this._freed) throw new Error('{name} object has been freed');\n  }}\n"
    ));

    // Constructors — the primary constructor wraps the wasm-bindgen `new` call.
    // Named constructors become static factory methods.
    let primary_ctor = o.constructors.iter().find(|c| c.name == "new");
    let named_ctors: Vec<&UdlConstructor> =
        o.constructors.iter().filter(|c| c.name != "new").collect();

    // Private base constructor — always present for internal use
    out.push_str(&format!("  private constructor(inner: __bg.{name}) {{\n"));
    if o.is_error {
        out.push_str(&format!("    super('{name}');\n"));
        out.push_str(&format!(
            "    Object.defineProperty(this, 'name', {{ value: '{name}' }});\n"
        ));
    }
    out.push_str("    this._inner = inner;\n");
    out.push_str("  }\n");
    // Internal factory used when lifting an object returned by a WASM function or method.
    out.push_str("  /** @internal */\n");
    out.push_str(&format!(
        "  static _fromInner(inner: __bg.{name}): {name} {{ return new {name}(inner); }}\n"
    ));

    if let Some(ctor) = primary_ctor {
        out.push_str(&render_ctor(
            ctor,
            name,
            &format!("new __bg.{name}"),
            "create",
        ));
    }

    for ctor in named_ctors {
        let exported = cfg
            .rename
            .get(&format!("{}.{}", name, ctor.name))
            .map(|s| safe_js_identifier(s))
            .unwrap_or_else(|| safe_js_identifier(&camel_case(&ctor.name)));
        let ctor_fn = format!("{bg_name}_{}", ctor.name);
        out.push_str(&render_ctor(
            ctor,
            name,
            &format!("__bg.{ctor_fn}"),
            &exported,
        ));
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
        // wasm-bindgen preserves Rust method names verbatim (snake_case) in its JS glue.
        // The public TypeScript name is camelCase (via `exported`), but the inner call
        // must match the wasm-pack output exactly. Use bracket notation for reserved words.
        let raw_call = member_call("this._inner", &m.name, &call_args.join(", "));
        let lifted = lift_return(&raw_call, m.return_type.as_ref(), m.is_async, local_crate);
        let call_expr = lift_custom_return(&lifted.expr, m.return_type.as_ref(), &cfg.custom_types);

        let async_kw = if m.is_async { "async " } else { "" };
        let throws_name = m.throws_type.as_ref().map(type_name);
        // Merge lift preamble with the assertLive preamble
        let preamble = match &lifted.preamble {
            Some(lift_pre) => format!("this._assertLive();\n    {lift_pre}"),
            None => "this._assertLive();".to_string(),
        };
        let body = render_call_body(
            &call_expr,
            m.return_type.is_some(),
            throws_name.as_deref(),
            Some(&preamble),
        );

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
            "  {async_kw}{exported}({}): {ts_ret} {{{body}}}\n",
            params.join(", ")
        ));
    }

    // Synthesised trait methods (instance methods on the class)
    if let Some(method_name) = &o.traits.display {
        out.push_str(&format!(
            "  toString(): string {{ this._assertLive(); return __bg.{bg_name}_{method_name}(this._inner); }}\n"
        ));
    }
    if let Some(method_name) = &o.traits.debug {
        out.push_str(&format!(
            "  toDebugString(): string {{ this._assertLive(); return __bg.{bg_name}_{method_name}(this._inner); }}\n"
        ));
    }
    if let Some(method_name) = &o.traits.eq {
        out.push_str(&format!(
            "  equals(other: {name}): boolean {{ this._assertLive(); return __bg.{bg_name}_{method_name}(this._inner, other._inner); }}\n"
        ));
    }
    if let Some(method_name) = &o.traits.hash {
        out.push_str(&format!(
            "  hashCode(): bigint {{ this._assertLive(); return __bg.{bg_name}_{method_name}(this._inner); }}\n"
        ));
    }
    if let Some(method_name) = &o.traits.ord {
        out.push_str(&format!(
            "  compareTo(other: {name}): number {{ this._assertLive(); return __bg.{bg_name}_{method_name}(this._inner, other._inner); }}\n"
        ));
    }

    // free() — wasm-bindgen generates this on all object classes.
    // Guarded against double-free; marks the object as freed.
    out.push_str("  /** Releases the underlying WASM resource. Safe to call more than once. */\n");
    out.push_str("  free(): void {\n    if (this._freed) return;\n    this._freed = true;\n    this._inner.free();\n  }\n");

    // Symbol.dispose — enables `using obj = Foo.new(...)` for automatic cleanup.
    out.push_str("  [Symbol.dispose](): void { this.free(); }\n");

    out.push_str("}\n");
    out
}

// ---------------------------------------------------------------------------
// Top-level function generation
// ---------------------------------------------------------------------------

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

pub(super) fn render_function(
    f: &UdlFunction,
    cfg: &config::JsBindingsConfig,
    local_crate: &str,
) -> String {
    let mut out = String::new();

    let exported = cfg
        .rename
        .get(&f.name)
        .map(|s| safe_js_identifier(s))
        .unwrap_or_else(|| safe_js_identifier(&camel_case(&f.name)));

    let params: Vec<String> = f.args.iter().map(render_param).collect();
    let ts_ret = ts_return_type(f.return_type.as_ref(), f.is_async);

    let call_args: Vec<String> = f
        .args
        .iter()
        .map(|a| {
            let base = safe_js_identifier(&camel_case(&a.name));
            lower_custom_arg(&base, &a.type_, &cfg.custom_types)
        })
        .collect();
    // wasm-pack exports top-level functions under their original snake_case Rust names;
    // object methods are camelCase in the JS glue. Do not apply camel_case here.
    let raw_call = wasm_call(&f.name, &call_args.join(", "));
    let lifted = lift_return(&raw_call, f.return_type.as_ref(), f.is_async, local_crate);
    let call_expr = lift_custom_return(&lifted.expr, f.return_type.as_ref(), &cfg.custom_types);

    let async_kw = if f.is_async { "async " } else { "" };
    let throws_name = f.throws_type.as_ref().map(type_name);
    let body = render_call_body(
        &call_expr,
        f.return_type.is_some(),
        throws_name.as_deref(),
        lifted.preamble.as_deref(),
    );

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
    out.push_str(&format!(
        "  export {async_kw}function {exported}({}): {ts_ret} {{{body}}}\n",
        params.join(", ")
    ));

    out
}
