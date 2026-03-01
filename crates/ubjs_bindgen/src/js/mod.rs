use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use uniffi_bindgen::interface::{AsType, ComponentInterface, Type};

use crate::cli::GenerateArgs;

pub mod config;

pub fn generate_bindings(args: &GenerateArgs) -> Result<()> {
    let cfg = config::load(args)?;
    let namespace = namespace_from_source(&args.source)?;
    let metadata = parse_udl_metadata(&args.source)?;

    let module_name = cfg
        .module_name
        .clone()
        .unwrap_or_else(|| pascal_case(&namespace));
    let library_name = cfg
        .library_name
        .clone()
        .unwrap_or_else(|| format!("uniffi_{}", namespace.replace('-', "_")));

    let out_file = args.out_dir.join(format!("{namespace}.ts"));
    fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("failed to create output dir: {}", args.out_dir.display()))?;

    let content = render_ts(&module_name, &library_name, &namespace, &metadata, &cfg);
    fs::write(&out_file, &content)
        .with_context(|| format!("failed to write: {}", out_file.display()))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Internal data types extracted from UDL
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct UdlFunction {
    name: String,
    args: Vec<UdlArg>,
    return_type: Option<Type>,
    #[allow(dead_code)] // used in Phase 2+ (error handling)
    throws_type: Option<Type>,
    #[allow(dead_code)] // used in Phase 5+ (async)
    is_async: bool,
}

#[derive(Debug)]
struct UdlArg {
    name: String,
    type_: Type,
}

#[derive(Debug, Default)]
struct UdlMetadata {
    #[allow(dead_code)] // stored for future use by sub-modules
    namespace: String,
    functions: Vec<UdlFunction>,
}

// ---------------------------------------------------------------------------
// UDL parsing via uniffi_bindgen ComponentInterface
// ---------------------------------------------------------------------------

fn parse_udl_metadata(source: &Path) -> Result<UdlMetadata> {
    if source.extension().and_then(|e| e.to_str()) != Some("udl") {
        return Ok(UdlMetadata::default());
    }

    let udl = fs::read_to_string(source)
        .with_context(|| format!("failed to read UDL: {}", source.display()))?;
    let ci = ComponentInterface::from_webidl(&udl, "crate_name")
        .with_context(|| format!("failed to parse UDL: {}", source.display()))?;

    let functions = ci
        .function_definitions()
        .iter()
        .map(|f| UdlFunction {
            name: f.name().to_string(),
            args: f
                .arguments()
                .into_iter()
                .map(|a| UdlArg {
                    name: a.name().to_string(),
                    type_: a.as_type(),
                })
                .collect(),
            return_type: f.return_type().cloned(),
            throws_type: f.throws_type().cloned(),
            is_async: f.is_async(),
        })
        .collect();

    Ok(UdlMetadata {
        namespace: ci.namespace().to_string(),
        functions,
    })
}

// ---------------------------------------------------------------------------
// Namespace extraction (fast path without full parse)
// ---------------------------------------------------------------------------

fn namespace_from_source(source: &Path) -> Result<String> {
    if let Some(ns) = extract_namespace_from_udl(source) {
        return Ok(ns);
    }
    source
        .file_stem()
        .and_then(|s| s.to_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow::anyhow!("source path must have a valid UTF-8 file stem"))
}

fn extract_namespace_from_udl(source: &Path) -> Option<String> {
    if source.extension().and_then(|e| e.to_str()) != Some("udl") {
        return None;
    }
    let udl = fs::read_to_string(source).ok()?;
    let marker = "namespace";
    let start = udl.find(marker)?;
    let mut chars = udl[start + marker.len()..].chars().peekable();
    while matches!(chars.peek(), Some(c) if c.is_whitespace()) {
        chars.next();
    }
    let mut ns = String::new();
    while matches!(chars.peek(), Some(c) if c.is_ascii_alphanumeric() || *c == '_') {
        ns.push(chars.next()?);
    }
    if ns.is_empty() {
        None
    } else {
        Some(ns)
    }
}

// ---------------------------------------------------------------------------
// TypeScript code generation
// ---------------------------------------------------------------------------

fn render_ts(
    module_name: &str,
    library_name: &str,
    namespace: &str,
    metadata: &UdlMetadata,
    cfg: &config::JsBindingsConfig,
) -> String {
    let visible_fns: Vec<&UdlFunction> = metadata
        .functions
        .iter()
        .filter(|f| !cfg.exclude.contains(&f.name))
        .collect();

    let needs_strings = visible_fns.iter().any(|f| {
        f.return_type.as_ref().is_some_and(is_string_type)
            || f.args.iter().any(|a| is_string_type(&a.type_))
    });

    let mut out = String::new();

    out.push_str(
        "// Generated by uniffi-bindgen-js. DO NOT EDIT.\n\nimport koffi from 'koffi';\n\n",
    );

    // UniFFI ABI structs (always emitted; every call needs them)
    out.push_str("const _RustBuffer = koffi.struct('_RustBuffer', {\n");
    out.push_str("  capacity: 'uint64',\n");
    out.push_str("  len: 'uint64',\n");
    out.push_str("  data: koffi.pointer('uint8'),\n");
    out.push_str("});\n\n");
    out.push_str("const _RustCallStatus = koffi.struct('_RustCallStatus', {\n");
    out.push_str("  code: 'int8',\n");
    out.push_str("  error_buf: '_RustBuffer',\n");
    out.push_str("});\n\n");

    // String helpers
    if needs_strings {
        out.push_str("function _liftString(buf: { len: bigint; data: unknown }): string {\n");
        out.push_str("  if (buf.len === 0n) return '';\n");
        out.push_str("  const bytes = koffi.decode(buf.data, 'uint8', Number(buf.len));\n");
        out.push_str("  return Buffer.from(bytes as Uint8Array).toString('utf8');\n");
        out.push_str("}\n\n");
        out.push_str(
            "function _lowerString(s: string): { capacity: bigint; len: bigint; data: Buffer } {\n",
        );
        out.push_str("  const bytes = Buffer.from(s, 'utf8');\n");
        out.push_str(
            "  return { capacity: BigInt(bytes.length), len: BigInt(bytes.length), data: bytes };\n",
        );
        out.push_str("}\n\n");
    }

    // Native library loading
    out.push_str("const _lib = (() => {\n");
    out.push_str("  const ext = process.platform === 'darwin' ? 'dylib' : 'so';\n");
    out.push_str(&format!(
        "  return koffi.load(`lib{library_name}.${{ext}}`);\n"
    ));
    out.push_str("})();\n\n");

    // FFI function declarations
    for f in &visible_fns {
        let sym = format!("uniffi_{}_fn_func_{}", namespace, f.name);
        let ret_ty = match &f.return_type {
            Some(t) => ffi_type_str(t),
            None => "'void'".to_string(),
        };
        let mut param_tys: Vec<String> = f.args.iter().map(|a| ffi_type_str(&a.type_)).collect();
        param_tys.push("koffi.out(koffi.pointer('_RustCallStatus'))".to_string());

        out.push_str(&format!(
            "const _fn_{} = _lib.func({}, {}, [\n",
            f.name,
            quoted(&sym),
            ret_ty
        ));
        for p in &param_tys {
            out.push_str(&format!("  {p},\n"));
        }
        out.push_str("]);\n\n");
    }

    // Exported namespace
    out.push_str(&format!("export namespace {module_name} {{\n"));
    for f in &visible_fns {
        let exported = cfg
            .rename
            .get(&f.name)
            .cloned()
            .unwrap_or_else(|| camel_case(&f.name));
        let ts_params: Vec<String> = f
            .args
            .iter()
            .map(|a| format!("{}: {}", camel_case(&a.name), ts_type_str(&a.type_)))
            .collect();
        let ts_ret = f
            .return_type
            .as_ref()
            .map(ts_type_str)
            .unwrap_or_else(|| "void".to_string());

        out.push_str(&format!(
            "  export function {}({}): {} {{\n",
            exported,
            ts_params.join(", "),
            ts_ret
        ));
        out.push_str(
            "    const _cs = { code: 0, error_buf: { capacity: 0n, len: 0n, data: null } };\n",
        );

        let call_args: Vec<String> = f
            .args
            .iter()
            .map(|a| {
                let id = camel_case(&a.name);
                if is_string_type(&a.type_) {
                    format!("_lowerString({id})")
                } else {
                    id
                }
            })
            .chain(std::iter::once("_cs".to_string()))
            .collect();

        if f.return_type.is_some() {
            out.push_str(&format!(
                "    const _result = _fn_{}({});\n",
                f.name,
                call_args.join(", ")
            ));
        } else {
            out.push_str(&format!("    _fn_{}({});\n", f.name, call_args.join(", ")));
        }

        out.push_str("    if (_cs.code !== 0) {\n");
        out.push_str(&format!(
            "      throw new Error(`uniffi: {exported} failed (status ${{_cs.code}})`);\n"
        ));
        out.push_str("    }\n");

        if let Some(ret_t) = &f.return_type {
            if is_string_type(ret_t) {
                out.push_str(
                    "    return _liftString(_result as { len: bigint; data: unknown });\n",
                );
            } else {
                out.push_str("    return _result;\n");
            }
        }

        out.push_str("  }\n");
    }
    out.push_str("}\n");

    out
}

// ---------------------------------------------------------------------------
// Type helpers
// ---------------------------------------------------------------------------

fn is_string_type(t: &Type) -> bool {
    matches!(t, Type::String)
}

fn ffi_type_str(t: &Type) -> String {
    match t {
        Type::String | Type::Bytes => "'_RustBuffer'".to_string(),
        Type::Boolean => "'bool'".to_string(),
        Type::Int8 => "'int8'".to_string(),
        Type::UInt8 => "'uint8'".to_string(),
        Type::Int16 => "'int16'".to_string(),
        Type::UInt16 => "'uint16'".to_string(),
        Type::Int32 => "'int32'".to_string(),
        Type::UInt32 => "'uint32'".to_string(),
        Type::Int64 => "'int64'".to_string(),
        Type::UInt64 => "'uint64'".to_string(),
        Type::Float32 => "'float'".to_string(),
        Type::Float64 => "'double'".to_string(),
        _ => "'_RustBuffer'".to_string(), // complex types use RustBuffer serialization
    }
}

fn ts_type_str(t: &Type) -> String {
    match t {
        Type::String => "string".to_string(),
        Type::Boolean => "boolean".to_string(),
        Type::Int8 | Type::Int16 | Type::Int32 => "number".to_string(),
        Type::UInt8 | Type::UInt16 | Type::UInt32 => "number".to_string(),
        Type::Int64 | Type::UInt64 => "bigint".to_string(),
        Type::Float32 | Type::Float64 => "number".to_string(),
        Type::Bytes => "Uint8Array".to_string(),
        Type::Optional { inner_type } => format!("{} | null", ts_type_str(inner_type)),
        Type::Sequence { inner_type } => format!("{}[]", ts_type_str(inner_type)),
        Type::Map {
            key_type,
            value_type,
        } => {
            format!(
                "Map<{}, {}>",
                ts_type_str(key_type),
                ts_type_str(value_type)
            )
        }
        Type::Enum { name, .. }
        | Type::Record { name, .. }
        | Type::Object { name, .. }
        | Type::CallbackInterface { name, .. } => pascal_case(name),
        _ => "unknown".to_string(),
    }
}

fn quoted(s: &str) -> String {
    format!("'{s}'")
}

// ---------------------------------------------------------------------------
// Identifier helpers
// ---------------------------------------------------------------------------

fn camel_case(input: &str) -> String {
    let mut out = String::new();
    let mut capitalize_next = false;
    for ch in input.chars() {
        if ch == '_' || ch == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            out.push(ch.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn pascal_case(input: &str) -> String {
    let mut out = String::new();
    for part in input.split(['_', '-']) {
        if part.is_empty() {
            continue;
        }
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.push_str(chars.as_str());
        }
    }
    if out.is_empty() {
        "UniffiBindings".to_string()
    } else {
        out
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camel_case_handles_underscores() {
        assert_eq!(camel_case("ping"), "ping");
        assert_eq!(camel_case("broken_greet"), "brokenGreet");
        assert_eq!(camel_case("async_greet"), "asyncGreet");
    }

    #[test]
    fn pascal_case_handles_common_cases() {
        assert_eq!(pascal_case("simple_bindings"), "SimpleBindings");
        assert_eq!(pascal_case("simple-bindings"), "SimpleBindings");
    }
}
