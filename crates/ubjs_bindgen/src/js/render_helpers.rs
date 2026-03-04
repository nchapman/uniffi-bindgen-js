// ---------------------------------------------------------------------------
// Rendering helpers: JSDoc, parameters, literals, type strings
// ---------------------------------------------------------------------------

use uniffi_bindgen::interface::{DefaultValue, Literal, Type};

use super::naming::{camel_case, is_js_reserved, safe_js_identifier};
use super::types::ArgDef;

/// Build a member access + call expression, using bracket notation for reserved words.
///
/// Returns `obj.name(args)` for normal names, or `obj["name"](args)` when
/// `name` is a JavaScript reserved word (which cannot appear after `.`).
pub(super) fn member_call(obj: &str, name: &str, args: &str) -> String {
    if is_js_reserved(name) {
        format!("{obj}[\"{name}\"]({args})")
    } else {
        format!("{obj}.{name}({args})")
    }
}

/// Build a call to a top-level WASM export.
///
/// wasm-pack prefixes reserved-word module exports with `_` (e.g. `class` → `_class`),
/// so we mirror that convention here. Normal names use `__bg.name(args)`.
pub(super) fn wasm_call(name: &str, args: &str) -> String {
    if is_js_reserved(name) {
        format!("__bg._{name}({args})")
    } else {
        format!("__bg.{name}({args})")
    }
}

/// Render an optional UDL docstring as a JSDoc block comment.
///
/// Returns an empty string when `docstring` is `None` or blank, so callers can
/// unconditionally prepend the result without introducing extra blank lines.
/// `indent` is prepended to every line (e.g. `""` for top-level, `"  "` for members).
pub(super) fn render_jsdoc(docstring: Option<&str>, indent: &str) -> String {
    render_jsdoc_with_throws(docstring, None, &[], indent)
}

/// Render a JSDoc block with optional `@throws` and `@param` annotations.
///
/// When `throws` is `Some`, a `@throws {Type}` tag is appended.
/// `extra_annotations` contains additional lines to append (e.g. `@param name - Duration in seconds.`).
pub(super) fn render_jsdoc_with_throws(
    docstring: Option<&str>,
    throws: Option<&str>,
    extra_annotations: &[String],
    indent: &str,
) -> String {
    let raw = docstring.map(str::trim).unwrap_or("");
    let has_content = !raw.is_empty();
    let has_throws = throws.is_some();
    let has_extras = !extra_annotations.is_empty();

    if !has_content && !has_throws && !has_extras {
        return String::new();
    }

    // Collect all doc lines
    let doc_lines: Vec<String> = if has_content {
        raw.lines()
            .map(|l| l.trim().replace("*/", "*\\/"))
            .collect()
    } else {
        vec![]
    };

    // Collect annotation lines
    let mut annotations: Vec<String> = Vec::new();
    for ann in extra_annotations {
        annotations.push(ann.clone());
    }
    if let Some(throws_type) = throws {
        annotations.push(format!("@throws {{{throws_type}}}"));
    }

    let total_lines = doc_lines.len() + annotations.len();

    // Try single-line format when everything fits on one line
    if total_lines == 1 {
        let single = if !doc_lines.is_empty() {
            &doc_lines[0]
        } else {
            &annotations[0]
        };
        let single_len = indent.len() + "/** ".len() + single.len() + " */".len();
        if single_len <= 80 {
            return format!("{indent}/** {single} */\n");
        }
    }

    // Multi-line format
    let mut out = format!("{indent}/**\n");
    for line in &doc_lines {
        if line.is_empty() {
            out.push_str(&format!("{indent} *\n"));
        } else {
            out.push_str(&format!("{indent} * {line}\n"));
        }
    }
    for ann in &annotations {
        out.push_str(&format!("{indent} * {ann}\n"));
    }
    out.push_str(&format!("{indent} */\n"));
    out
}

/// Render a function/method parameter as `name: Type`, `name: Type = default`,
/// or `name?: Type` (when the default is unspecified).
pub(super) fn render_param(arg: &ArgDef) -> String {
    render_param_inner(arg, false)
}

/// Like [`render_param`] but for FFI-mode output.
pub(super) fn render_param_ffi(arg: &ArgDef) -> String {
    render_param_inner(arg, true)
}

fn render_param_inner(arg: &ArgDef, ffi_mode: bool) -> String {
    let ts_name = safe_js_identifier(&camel_case(&arg.name));
    let ts_type = ts_type_str_inner(&arg.type_, ffi_mode);
    match &arg.default {
        Some(DefaultValue::Literal(lit)) => {
            format!("{ts_name}: {ts_type} = {}", render_literal(lit))
        }
        Some(DefaultValue::Default) => {
            // "unspecified default" — the Rust side uses a type-level default.
            // Make the parameter optional so the caller can omit it.
            format!("{ts_name}?: {ts_type}")
        }
        None => format!("{ts_name}: {ts_type}"),
    }
}

/// Build `@param` annotations for Duration-typed parameters.
pub(super) fn duration_annotations(args: &[ArgDef]) -> Vec<String> {
    args.iter()
        .filter(|a| matches!(a.type_, Type::Duration))
        .map(|a| {
            let ts_name = safe_js_identifier(&camel_case(&a.name));
            format!("@param {ts_name} - Duration in seconds.")
        })
        .collect()
}

/// Build a `@returns` annotation when the return type is Duration.
pub(super) fn duration_return_annotation(return_type: Option<&Type>) -> Option<String> {
    match return_type {
        Some(Type::Duration) => Some("@returns Duration in seconds.".to_string()),
        _ => None,
    }
}

pub(super) fn render_literal(lit: &Literal) -> String {
    match lit {
        Literal::Boolean(b) => b.to_string(),
        Literal::String(s) => format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'")),
        Literal::UInt(v, _, t) => {
            if matches!(t, Type::Int64 | Type::UInt64) {
                format!("{v}n")
            } else {
                v.to_string()
            }
        }
        Literal::Int(v, _, t) => {
            if matches!(t, Type::Int64 | Type::UInt64) {
                format!("{v}n")
            } else {
                v.to_string()
            }
        }
        Literal::Float(s, _) => s.clone(),
        Literal::Enum(variant_name, _) => format!("'{variant_name}'"),
        Literal::EmptySequence => "[]".to_string(),
        Literal::EmptyMap => "new Map()".to_string(),
        Literal::None => "null".to_string(),
        Literal::Some { inner } => match inner.as_ref() {
            DefaultValue::Default => "undefined".to_string(),
            DefaultValue::Literal(lit) => render_literal(lit),
        },
    }
}

pub(super) fn ts_type_str(t: &Type) -> String {
    ts_type_str_inner(t, false)
}

/// Like [`ts_type_str`] but for FFI-mode output.
///
/// In FFI mode `Sequence<i64/u64>` is `bigint[]` (deserialized via `readSequence`).
/// Legacy mode maps these to `BigInt64Array`/`BigUint64Array` to match wasm-bindgen.
pub(super) fn ts_type_str_ffi(t: &Type) -> String {
    ts_type_str_inner(t, true)
}

fn ts_type_str_inner(t: &Type, ffi_mode: bool) -> String {
    match t {
        Type::String => "string".to_string(),
        Type::Boolean => "boolean".to_string(),
        Type::Int8 | Type::Int16 | Type::Int32 => "number".to_string(),
        Type::UInt8 | Type::UInt16 | Type::UInt32 => "number".to_string(),
        Type::Int64 | Type::UInt64 => "bigint".to_string(),
        Type::Float32 | Type::Float64 => "number".to_string(),
        Type::Bytes => "Uint8Array".to_string(),
        // Duration serialises via serde as { secs: number, nanos: number }.
        // We emit `number` (seconds, float) as the idiomatic JS representation;
        // the wasm fixture crate is responsible for converting to/from f64 seconds.
        Type::Duration => "number".to_string(),
        // Timestamp serialises via serde as a duration-since-epoch.
        // We emit `Date` as the idiomatic JS type; the wasm fixture crate is
        // responsible for converting to/from a JS Date object.
        Type::Timestamp => "Date".to_string(),
        Type::Optional { inner_type } => {
            format!("{} | null", ts_type_str_inner(inner_type, ffi_mode))
        }
        Type::Sequence { inner_type } => {
            // wasm-bindgen maps Box<[i64]> / Box<[u64]> to typed arrays, not plain arrays.
            // In FFI mode, readSequence always returns T[], so skip this.
            if !ffi_mode {
                match inner_type.as_ref() {
                    Type::Int64 => return "BigInt64Array".to_string(),
                    Type::UInt64 => return "BigUint64Array".to_string(),
                    _ => {}
                }
            }
            let inner = ts_type_str_inner(inner_type, ffi_mode);
            // Parenthesize compound inner types to avoid precedence issues
            // e.g. `(string | null)[]` not `string | null[]`
            if matches!(
                inner_type.as_ref(),
                Type::Optional { .. } | Type::Map { .. }
            ) {
                format!("({inner})[]")
            } else {
                format!("{inner}[]")
            }
        }
        Type::Map {
            key_type,
            value_type,
        } => format!(
            "Map<{}, {}>",
            ts_type_str_inner(key_type, ffi_mode),
            ts_type_str_inner(value_type, ffi_mode)
        ),
        Type::Enum { name, .. }
        | Type::Record { name, .. }
        | Type::Object { name, .. }
        | Type::CallbackInterface { name, .. } => name.clone(),
        // Custom types appear in signatures by their user-facing name; the WASM
        // boundary passes the underlying builtin transparently.
        Type::Custom { name, .. } => name.clone(),
    }
}

pub(super) fn type_name(t: &Type) -> String {
    match t {
        Type::Enum { name, .. }
        | Type::Record { name, .. }
        | Type::Object { name, .. }
        | Type::CallbackInterface { name, .. } => name.clone(),
        _ => ts_type_str(t),
    }
}

/// Build the TypeScript return-type annotation for a function/method.
///
/// Given `return_type` and `is_async`, produces e.g. `"string"`, `"void"`,
/// `"Promise<string>"`, or `"Promise<void>"`.
pub(super) fn ts_return_type(return_type: Option<&Type>, is_async: bool) -> String {
    let base = return_type
        .map(ts_type_str)
        .unwrap_or_else(|| "void".to_string());
    if is_async {
        format!("Promise<{base}>")
    } else {
        base
    }
}

/// Like [`ts_return_type`] but for FFI-mode output.
pub(super) fn ts_return_type_ffi(return_type: Option<&Type>, is_async: bool) -> String {
    let base = return_type
        .map(ts_type_str_ffi)
        .unwrap_or_else(|| "void".to_string());
    if is_async {
        format!("Promise<{base}>")
    } else {
        base
    }
}

/// Render the body of a function/method call, handling throws and return.
///
/// `call_expr` is the (possibly lifted) call expression.
/// `has_return` controls whether a `return` keyword is emitted.
/// `throws` is the optional error type name (triggers try/catch wrapping).
/// `preamble` is an optional statement emitted before the call (e.g. `"this._assertLive();"`)
/// for object methods that guard against use-after-free.
///
/// Returns the content to place between `{` and `}` of the method declaration:
/// - When no preamble and no throws: a single-line inline body with surrounding spaces
///   (suitable for `{ return foo(); }` on one line).
/// - Otherwise: a multi-line body starting with `\n` and ending with `\n  `
///   (suitable for a `{\n  ...\n}` block).
///
/// Note: constructor rendering (`render_ctor`, `render_enum_constructors`) is intentionally
/// out of scope — constructors wrap the inner call with `_fromInner(...)` which requires
/// a different body shape.
pub(super) fn render_call_body(
    call_expr: &str,
    has_return: bool,
    throws: Option<&str>,
    preamble: Option<&str>,
) -> String {
    let ret_kw = if has_return { "return " } else { "" };
    match (throws, preamble) {
        (Some(throws_name), Some(pre)) => {
            let lift = format!("_lift{throws_name}");
            format!(
                "\n    {pre}\n    try {{ {ret_kw}{call_expr}; }} catch (e) {{ {ret_kw}{lift}(e); }}\n  "
            )
        }
        (Some(throws_name), None) => {
            let lift = format!("_lift{throws_name}");
            format!("\n    try {{ {ret_kw}{call_expr}; }} catch (e) {{ {ret_kw}{lift}(e); }}\n  ")
        }
        (None, Some(pre)) => {
            format!("\n    {pre}\n    {ret_kw}{call_expr};\n  ")
        }
        (None, None) => {
            format!(" {ret_kw}{call_expr}; ")
        }
    }
}
