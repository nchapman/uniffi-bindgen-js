// ---------------------------------------------------------------------------
// Rendering helpers: JSDoc, parameters, literals, type strings
// ---------------------------------------------------------------------------

use uniffi_bindgen::interface::{DefaultValue, Literal, Type};

use super::naming::{camel_case, safe_js_identifier};
use super::types::ArgDef;

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
    let ts_name = safe_js_identifier(&camel_case(&arg.name));
    let ts_type = ts_type_str(&arg.type_);
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

/// Map a UniFFI type to its TypeScript representation.
///
/// `Sequence<i64/u64>` maps to `bigint[]` (FFI-direct deserialization via `readSequence`).
pub(super) fn ts_type_str(t: &Type) -> String {
    match t {
        Type::String => "string".to_string(),
        Type::Boolean => "boolean".to_string(),
        Type::Int8 | Type::Int16 | Type::Int32 => "number".to_string(),
        Type::UInt8 | Type::UInt16 | Type::UInt32 => "number".to_string(),
        Type::Int64 | Type::UInt64 => "bigint".to_string(),
        Type::Float32 | Type::Float64 => "number".to_string(),
        Type::Bytes => "Uint8Array".to_string(),
        Type::Duration => "number".to_string(),
        Type::Timestamp => "Date".to_string(),
        Type::Optional { inner_type } => {
            format!("{} | null", ts_type_str(inner_type))
        }
        Type::Sequence { inner_type } => {
            let inner = ts_type_str(inner_type);
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
            ts_type_str(key_type),
            ts_type_str(value_type)
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
