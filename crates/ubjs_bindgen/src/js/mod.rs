use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use uniffi_bindgen::interface::{AsType, ComponentInterface, DefaultValue, Literal, Type};

use crate::cli::GenerateArgs;

pub mod config;

pub fn generate_bindings(args: &GenerateArgs) -> Result<()> {
    let cfg = config::load(args)?;
    let metadata = parse_udl_metadata(&args.source)?;
    let namespace = if metadata.namespace.is_empty() {
        namespace_from_source(&args.source)?
    } else {
        metadata.namespace.clone()
    };

    let module_name = cfg
        .module_name
        .clone()
        .unwrap_or_else(|| pascal_case(&namespace));

    let out_file = args.out_dir.join(format!("{namespace}.ts"));
    fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("failed to create output dir: {}", args.out_dir.display()))?;

    let content = render_ts(&module_name, &namespace, &metadata, &cfg)?;
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
    throws_type: Option<Type>,
    is_async: bool,
    docstring: Option<String>,
}

#[derive(Debug)]
struct UdlArg {
    name: String,
    type_: Type,
    default: Option<DefaultValue>,
}

/// A variant field (used in rich error variants and data enum variants),
/// or a record field (used in dictionary declarations).
#[derive(Debug)]
struct UdlField {
    name: String,
    type_: Type,
    docstring: Option<String>,
    default: Option<DefaultValue>,
}

/// One variant of an enum or error type.
#[derive(Debug)]
struct UdlVariant {
    name: String,
    /// Empty for flat variants (no associated data).
    fields: Vec<UdlField>,
    docstring: Option<String>,
    /// Explicit discriminant value (e.g. `= 10`), if declared.
    discr: Option<Literal>,
}

/// A [Error] enum — generates a TypeScript error class.
#[derive(Debug)]
struct UdlError {
    name: String,
    variants: Vec<UdlVariant>,
    is_flat: bool,
    docstring: Option<String>,
    /// Methods declared on the error enum.
    methods: Vec<UdlMethod>,
}

/// A plain enum or [Enum] interface — generates a TypeScript union type.
#[derive(Debug)]
struct UdlEnum {
    name: String,
    variants: Vec<UdlVariant>,
    /// true ↔ all variants are unit variants (no fields); serialises as a string.
    is_flat: bool,
    docstring: Option<String>,
    /// Methods declared on the enum (from `impl` blocks).
    methods: Vec<UdlMethod>,
}

/// A `dictionary` declaration — generates a TypeScript interface.
#[derive(Debug)]
struct UdlRecord {
    name: String,
    fields: Vec<UdlField>,
    docstring: Option<String>,
}

/// A constructor of an `interface` object.
#[derive(Debug)]
struct UdlConstructor {
    /// Exported name in JS.  Usually "new".
    name: String,
    args: Vec<UdlArg>,
    throws_type: Option<Type>,
    is_async: bool,
    docstring: Option<String>,
}

/// A method on an `interface` object.
#[derive(Debug)]
struct UdlMethod {
    name: String,
    args: Vec<UdlArg>,
    return_type: Option<Type>,
    throws_type: Option<Type>,
    is_async: bool,
    docstring: Option<String>,
}

/// An `interface` declaration — generates a TypeScript class.
#[derive(Debug)]
struct UdlObject {
    name: String,
    constructors: Vec<UdlConstructor>,
    methods: Vec<UdlMethod>,
    docstring: Option<String>,
}

/// A `[Custom]` typedef — generates a TypeScript type alias.
#[derive(Debug)]
struct UdlCustomType {
    /// The custom type name (e.g. `Url`).
    name: String,
    /// The underlying builtin type (e.g. `Type::String`).
    builtin: Type,
    /// The `module_path` from the source `Type::Custom` — used to detect external custom types.
    module_path: String,
}

/// A method on a `callback interface` — generates a method signature in a TS interface.
///
/// `throws_type` is intentionally absent: UniFFI UDL does not support `[Throws]` on
/// callback interface methods (errors flow outward from the JS implementor into Rust,
/// not inward from the generated binding).
///
/// `is_async` IS expressible in UDL (`[Async]` on a callback method). The generator
/// emits `Promise<T>` for the method return type, which is the correct TypeScript
/// contract. Wasm fixture crates must use `wasm_bindgen_futures` and return a
/// `js_sys::Promise` to back async callback methods at runtime.
#[derive(Debug)]
struct UdlCallbackMethod {
    name: String,
    args: Vec<UdlArg>,
    return_type: Option<Type>,
    is_async: bool,
    docstring: Option<String>,
}

/// A `callback interface` declaration — generates a TypeScript interface.
#[derive(Debug)]
struct UdlCallbackInterface {
    name: String,
    methods: Vec<UdlCallbackMethod>,
    docstring: Option<String>,
}

#[derive(Debug, Default)]
struct UdlMetadata {
    namespace: String,
    namespace_docstring: Option<String>,
    functions: Vec<UdlFunction>,
    errors: Vec<UdlError>,
    enums: Vec<UdlEnum>,
    records: Vec<UdlRecord>,
    objects: Vec<UdlObject>,
    custom_types: Vec<UdlCustomType>,
    callback_interfaces: Vec<UdlCallbackInterface>,
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
    let ci = ComponentInterface::from_webidl(&udl, LOCAL_CRATE_SENTINEL)
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
                    default: a.default_value().cloned(),
                })
                .collect(),
            return_type: f.return_type().cloned(),
            throws_type: f.throws_type().cloned(),
            is_async: f.is_async(),
            docstring: f.docstring().map(ToOwned::to_owned),
        })
        .collect();

    let (errors, enums) = parse_enums(&ci);

    let records = ci
        .record_definitions()
        .iter()
        .map(|r| UdlRecord {
            name: r.name().to_string(),
            fields: r
                .fields()
                .iter()
                .map(|f| UdlField {
                    name: f.name().to_string(),
                    type_: f.as_type(),
                    docstring: f.docstring().map(ToOwned::to_owned),
                    default: f.default_value().cloned(),
                })
                .collect(),
            docstring: r.docstring().map(ToOwned::to_owned),
        })
        .collect();

    let objects = ci
        .object_definitions()
        .iter()
        .map(|o| UdlObject {
            name: o.name().to_string(),
            constructors: o
                .constructors()
                .iter()
                .map(|c| UdlConstructor {
                    name: c.name().to_string(),
                    args: c
                        .arguments()
                        .into_iter()
                        .map(|a| UdlArg {
                            name: a.name().to_string(),
                            type_: a.as_type(),
                            default: a.default_value().cloned(),
                        })
                        .collect(),
                    throws_type: c.throws_type().cloned(),
                    is_async: c.is_async(),
                    docstring: c.docstring().map(ToOwned::to_owned),
                })
                .collect(),
            methods: {
                let ms: Vec<_> = o.methods().into_iter().cloned().collect();
                parse_methods(&ms)
            },
            docstring: o.docstring().map(ToOwned::to_owned),
        })
        .collect();

    let callback_interfaces = ci
        .callback_interface_definitions()
        .iter()
        .map(|cb| UdlCallbackInterface {
            name: cb.name().to_string(),
            methods: cb
                .methods()
                .iter()
                .map(|m| UdlCallbackMethod {
                    name: m.name().to_string(),
                    args: m
                        .arguments()
                        .into_iter()
                        .map(|a| UdlArg {
                            name: a.name().to_string(),
                            type_: a.as_type(),
                            default: a.default_value().cloned(),
                        })
                        .collect(),
                    return_type: m.return_type().cloned(),
                    is_async: m.is_async(),
                    docstring: m.docstring().map(ToOwned::to_owned),
                })
                .collect(),
            docstring: cb.docstring().map(ToOwned::to_owned),
        })
        .collect();

    // Collect all [Custom] typedefs from the type universe, sorted by name for
    // deterministic output (iter_local_types order is not guaranteed by uniffi-bindgen).
    let mut seen_custom: HashSet<String> = HashSet::new();
    let mut custom_types: Vec<UdlCustomType> = Vec::new();
    for t in ci.iter_local_types() {
        if let Type::Custom {
            name,
            builtin,
            module_path,
        } = t
        {
            if seen_custom.insert(name.clone()) {
                custom_types.push(UdlCustomType {
                    name: name.clone(),
                    builtin: *builtin.clone(),
                    module_path: module_path.clone(),
                });
            }
        }
    }
    custom_types.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(UdlMetadata {
        namespace: ci.namespace().to_string(),
        namespace_docstring: ci.namespace_docstring().map(ToOwned::to_owned),
        functions,
        errors,
        enums,
        records,
        objects,
        custom_types,
        callback_interfaces,
    })
}

fn parse_methods(methods: &[uniffi_bindgen::interface::Method]) -> Vec<UdlMethod> {
    methods
        .iter()
        .map(|m| UdlMethod {
            name: m.name().to_string(),
            args: m
                .arguments()
                .into_iter()
                .map(|a| UdlArg {
                    name: a.name().to_string(),
                    type_: a.as_type(),
                    default: a.default_value().cloned(),
                })
                .collect(),
            return_type: m.return_type().cloned(),
            throws_type: m.throws_type().cloned(),
            is_async: m.is_async(),
            docstring: m.docstring().map(ToOwned::to_owned),
        })
        .collect()
}

fn parse_enums(ci: &ComponentInterface) -> (Vec<UdlError>, Vec<UdlEnum>) {
    let mut errors = Vec::new();
    let mut enums = Vec::new();

    for e in ci.enum_definitions() {
        let has_discr = e.variant_discr_type().is_some();
        let variants: Vec<UdlVariant> = e
            .variants()
            .iter()
            .enumerate()
            .map(|(i, v)| UdlVariant {
                name: v.name().to_string(),
                fields: v
                    .fields()
                    .iter()
                    .map(|f| UdlField {
                        name: f.name().to_string(),
                        type_: f.as_type(),
                        docstring: f.docstring().map(ToOwned::to_owned),
                        default: None,
                    })
                    .collect(),
                docstring: v.docstring().map(ToOwned::to_owned),
                discr: if has_discr {
                    e.variant_discr(i).ok()
                } else {
                    None
                },
            })
            .collect();

        let methods = parse_methods(e.methods());

        if ci.is_name_used_as_error(e.name()) {
            errors.push(UdlError {
                name: e.name().to_string(),
                variants,
                is_flat: e.is_flat(),
                docstring: e.docstring().map(ToOwned::to_owned),
                methods,
            });
        } else {
            enums.push(UdlEnum {
                name: e.name().to_string(),
                variants,
                is_flat: e.is_flat(),
                docstring: e.docstring().map(ToOwned::to_owned),
                methods,
            });
        }
    }

    (errors, enums)
}

// ---------------------------------------------------------------------------
// Namespace extraction fallback (non-UDL sources only)
// ---------------------------------------------------------------------------

fn namespace_from_source(source: &Path) -> Result<String> {
    source
        .file_stem()
        .and_then(|s| s.to_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow::anyhow!("source path must have a valid UTF-8 file stem"))
}

// ---------------------------------------------------------------------------
// TypeScript code generation
// ---------------------------------------------------------------------------

fn render_ts(
    module_name: &str,
    namespace: &str,
    metadata: &UdlMetadata,
    cfg: &config::JsBindingsConfig,
) -> Result<String> {
    let mut out = String::new();

    // Header
    out.push_str("// Generated by uniffi-bindgen-js. DO NOT EDIT.\n");
    out.push_str(&format!(
        "import __init, * as __bg from './{namespace}_bg.js';\n"
    ));
    out.push_str("export { __init as init };\n");

    // External type imports — one import statement per external package, types sorted for determinism.
    let external_imports = collect_external_imports(metadata, &cfg.external_packages)?;
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
            out.push('\n');
            out.push_str(&format!(
                "export type {} = {};\n",
                exported,
                ts_type_str(&ct.builtin)
            ));
        }
    }

    // Error classes (top-level, before namespace)
    for e in &metadata.errors {
        if !cfg.exclude.contains(&e.name) {
            out.push('\n');
            out.push_str(&render_error_class(e, cfg));
        }
    }

    // Record interfaces (top-level)
    for r in &metadata.records {
        if !cfg.exclude.contains(&r.name) {
            out.push('\n');
            out.push_str(&render_record_interface(r));
        }
    }

    // Enum types (top-level)
    for e in &metadata.enums {
        if !cfg.exclude.contains(&e.name) {
            out.push('\n');
            out.push_str(&render_enum_type(e, cfg));
        }
    }

    // Callback interfaces (structural TypeScript interfaces)
    for cb in &metadata.callback_interfaces {
        if !cfg.exclude.contains(&cb.name) {
            out.push('\n');
            out.push_str(&render_callback_interface(cb, cfg));
        }
    }

    // Object re-exports (top-level)
    let visible_objects: Vec<&UdlObject> = metadata
        .objects
        .iter()
        .filter(|o| !cfg.exclude.contains(&o.name))
        .collect();
    if !visible_objects.is_empty() {
        out.push('\n');
        for o in &visible_objects {
            out.push_str(&render_object_class(o, namespace, cfg));
        }
    }

    // Error lift helpers (private, before namespace)
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
            for m in &e.methods {
                if let Some(t) = &m.throws_type {
                    names.insert(type_name(t));
                }
            }
        }
        for e in &metadata.errors {
            for m in &e.methods {
                if let Some(t) = &m.throws_type {
                    names.insert(type_name(t));
                }
            }
        }
        names
    };
    // Collect unique error names that need lift helpers, looking them up in errors
    let mut rendered_lifts: Vec<String> = Vec::new();
    for error in &metadata.errors {
        if all_throws.contains(&error.name) {
            let lift = render_lift_fn(error);
            if !lift.is_empty() {
                rendered_lifts.push(lift);
            }
        }
    }
    if !rendered_lifts.is_empty() {
        out.push('\n');
        for lift in rendered_lifts {
            out.push_str(&lift);
        }
    }

    // Namespace with top-level functions — omit entirely if there are none.
    let visible_fns: Vec<&UdlFunction> = metadata
        .functions
        .iter()
        .filter(|f| !cfg.exclude.contains(&f.name))
        .collect();

    if !visible_fns.is_empty() {
        out.push('\n');
        out.push_str(&render_jsdoc(metadata.namespace_docstring.as_deref(), ""));
        out.push_str(&format!("export namespace {module_name} {{\n"));
        for f in &visible_fns {
            out.push_str(&render_function(f, cfg));
        }
        out.push_str("}\n");
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// Error class generation
// ---------------------------------------------------------------------------

fn render_error_class(e: &UdlError, cfg: &config::JsBindingsConfig) -> String {
    let mut out = String::new();
    let name = &e.name;

    if e.is_flat {
        // Flat error: single `tag` string property, no variant fields
        let tag_union: Vec<String> = e.variants.iter().map(|v| format!("'{}'", v.name)).collect();

        out.push_str(&render_jsdoc(e.docstring.as_deref(), ""));
        out.push_str(&format!("export class {name} extends Error {{\n"));
        out.push_str(&format!("  override readonly name = '{name}' as const;\n"));
        out.push_str(&format!(
            "  constructor(public readonly tag: {}) {{\n",
            tag_union.join(" | ")
        ));
        out.push_str("    super(tag);\n");
        out.push_str("  }\n");
        for v in &e.variants {
            out.push_str(&render_jsdoc(v.docstring.as_deref(), "  "));
            out.push_str(&format!(
                "  static {}(): {name} {{ return new {name}('{}'); }}\n",
                v.name, v.name
            ));
        }
        out.push_str(&render_enum_methods_on_class(&e.methods, name, cfg));
        out.push_str("}\n");
    } else {
        // Rich error: each variant may have different fields; use a discriminated
        // union stored in `variant` and expose a flat set of optional field getters.
        let variant_type = format!("{name}Variant");
        out.push_str(&format!("export type {variant_type} =\n"));
        for (i, v) in e.variants.iter().enumerate() {
            let sep = if i == e.variants.len() - 1 { ";" } else { "" };
            if v.fields.is_empty() {
                out.push_str(&format!("  | {{ tag: '{}' }}{sep}\n", v.name));
            } else {
                let fields: Vec<String> = v
                    .fields
                    .iter()
                    .map(|f| format!("{}: {}", camel_case(&f.name), ts_type_str(&f.type_)))
                    .collect();
                out.push_str(&format!(
                    "  | {{ tag: '{}', {} }}{sep}\n",
                    v.name,
                    fields.join(", ")
                ));
            }
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
                .map(|f| format!("{}: {}", camel_case(&f.name), ts_type_str(&f.type_)))
                .collect();
            // Object literal uses camelCase shorthand (param names match property names).
            let obj_fields: Vec<String> = v.fields.iter().map(|f| camel_case(&f.name)).collect();
            let variant_obj = if v.fields.is_empty() {
                format!("{{ tag: '{}' }}", v.name)
            } else {
                format!("{{ tag: '{}', {} }}", v.name, obj_fields.join(", "))
            };
            out.push_str(&render_jsdoc(v.docstring.as_deref(), "  "));
            out.push_str(&format!(
                "  static {}({}): {name} {{ return new {name}({variant_obj}); }}\n",
                v.name,
                params.join(", ")
            ));
        }
        out.push_str(&render_enum_methods_on_class(&e.methods, name, cfg));
        out.push_str("}\n");
    }

    out
}

/// Render methods on an error class (instance methods that delegate to wasm-bindgen).
/// Error enum methods are exported by wasm-bindgen as `{snake_case_enum}_{method_name}(self, ...)`.
fn render_enum_methods_on_class(
    methods: &[UdlMethod],
    enum_name: &str,
    cfg: &config::JsBindingsConfig,
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
            .cloned()
            .unwrap_or_else(|| camel_case(&m.name));
        let params: Vec<String> = m.args.iter().map(render_param).collect();
        let ts_ret = if m.is_async {
            format!(
                "Promise<{}>",
                m.return_type
                    .as_ref()
                    .map(ts_type_str)
                    .unwrap_or_else(|| "void".to_string())
            )
        } else {
            m.return_type
                .as_ref()
                .map(ts_type_str)
                .unwrap_or_else(|| "void".to_string())
        };
        let call_args: Vec<String> = m.args.iter().map(|a| camel_case(&a.name)).collect();
        let self_plus_args = if call_args.is_empty() {
            "this".to_string()
        } else {
            format!("this, {}", call_args.join(", "))
        };
        let raw_call = format!("__bg.{bg_name}_{}({self_plus_args})", m.name);
        let call_expr = lift_return(&raw_call, m.return_type.as_ref(), m.is_async);
        let async_kw = if m.is_async { "async " } else { "" };

        out.push_str(&render_jsdoc(m.docstring.as_deref(), "  "));
        if let Some(throws) = &m.throws_type {
            let lift = format!("_lift{}", type_name(throws));
            if m.return_type.is_some() {
                out.push_str(&format!(
                    "  {async_kw}{exported}({}): {ts_ret} {{\n    try {{ return {call_expr}; }} catch (e) {{ return {lift}(e); }}\n  }}\n",
                    params.join(", ")
                ));
            } else {
                out.push_str(&format!(
                    "  {async_kw}{exported}({}): {ts_ret} {{\n    try {{ {call_expr}; }} catch (e) {{ {lift}(e); }}\n  }}\n",
                    params.join(", ")
                ));
            }
        } else if m.return_type.is_some() {
            out.push_str(&format!(
                "  {async_kw}{exported}({}): {ts_ret} {{ return {call_expr}; }}\n",
                params.join(", ")
            ));
        } else {
            out.push_str(&format!(
                "  {async_kw}{exported}({}): {ts_ret} {{ {call_expr}; }}\n",
                params.join(", ")
            ));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Error lift helper generation
// ---------------------------------------------------------------------------

fn render_lift_fn(e: &UdlError) -> String {
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
                    // Rust serde serialises field names as-is (snake_case by default).
                    .map(|f| format!("raw.{}", f.name))
                    .collect();
                out.push_str(&format!(
                    "    if (tag === '{}') throw {name}.{}({});\n",
                    v.name,
                    v.name,
                    args.join(", ")
                ));
            }
        }
        out.push_str("  } catch (inner) {\n");
        out.push_str(&format!("    if (inner instanceof {name}) throw inner;\n"));
        out.push_str("  }\n");
    }

    out.push_str("  throw e;\n");
    out.push_str("}\n");
    out
}

// ---------------------------------------------------------------------------
// Record interface generation
// ---------------------------------------------------------------------------

fn render_record_interface(r: &UdlRecord) -> String {
    let mut out = String::new();
    out.push_str(&render_jsdoc(r.docstring.as_deref(), ""));
    out.push_str(&format!("export interface {} {{\n", r.name));
    for f in &r.fields {
        let ts_name = camel_case(&f.name);
        let ts_type = ts_type_str(&f.type_);
        out.push_str(&render_jsdoc(f.docstring.as_deref(), "  "));
        // Fields with defaults are optional (callers may omit them).
        let optional = if f.default.is_some() { "?" } else { "" };
        out.push_str(&format!("  {ts_name}{optional}: {ts_type};\n"));
    }
    out.push_str("}\n");
    out
}

// ---------------------------------------------------------------------------
// Enum type generation
// ---------------------------------------------------------------------------

fn render_enum_type(e: &UdlEnum, cfg: &config::JsBindingsConfig) -> String {
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
        let variants: Vec<String> = e.variants.iter().map(|v| format!("'{}'", v.name)).collect();
        out.push_str(&format!(
            "export type {} = {};\n",
            e.name,
            variants.join(" | ")
        ));
    } else {
        // Data enum → discriminated union.
        // Variant docstrings have no per-member anchor in a union type; the type-level
        // docstring is the only JSDoc anchor available.
        out.push_str(&render_jsdoc(e.docstring.as_deref(), ""));
        out.push_str(&format!("export type {} =\n", e.name));
        for (i, v) in e.variants.iter().enumerate() {
            let sep = if i == e.variants.len() - 1 { ";" } else { "" };
            if v.fields.is_empty() {
                out.push_str(&format!("  | {{ tag: '{}' }}{sep}\n", v.name));
            } else {
                let fields: Vec<String> = v
                    .fields
                    .iter()
                    .map(|f| format!("{}: {}", camel_case(&f.name), ts_type_str(&f.type_)))
                    .collect();
                out.push_str(&format!(
                    "  | {{ tag: '{}', {} }}{sep}\n",
                    v.name,
                    fields.join(", ")
                ));
            }
        }
    }

    // Enum methods are emitted as functions in a companion namespace (TS declaration
    // merging allows a namespace with the same name as a type alias).
    if !e.methods.is_empty() {
        let name = &e.name;
        let bg_name = snake_case(name);
        out.push_str(&format!("export namespace {name} {{\n"));
        for m in &e.methods {
            if cfg.exclude.contains(&format!("{name}.{}", m.name)) {
                continue;
            }
            let exported = cfg
                .rename
                .get(&format!("{name}.{}", m.name))
                .cloned()
                .unwrap_or_else(|| camel_case(&m.name));
            let self_param = format!("self: {name}");
            let other_params: Vec<String> = m.args.iter().map(render_param).collect();
            let all_params = if other_params.is_empty() {
                self_param
            } else {
                format!("{self_param}, {}", other_params.join(", "))
            };
            let ts_ret = if m.is_async {
                format!(
                    "Promise<{}>",
                    m.return_type
                        .as_ref()
                        .map(ts_type_str)
                        .unwrap_or_else(|| "void".to_string())
                )
            } else {
                m.return_type
                    .as_ref()
                    .map(ts_type_str)
                    .unwrap_or_else(|| "void".to_string())
            };
            let call_args: Vec<String> = m.args.iter().map(|a| camel_case(&a.name)).collect();
            let self_plus_args = if call_args.is_empty() {
                "self".to_string()
            } else {
                format!("self, {}", call_args.join(", "))
            };
            let raw_call = format!("__bg.{bg_name}_{}({self_plus_args})", m.name);
            let call_expr = lift_return(&raw_call, m.return_type.as_ref(), m.is_async);
            let async_kw = if m.is_async { "async " } else { "" };

            out.push_str(&render_jsdoc(m.docstring.as_deref(), "  "));
            if m.return_type.is_some() {
                out.push_str(&format!(
                    "  export {async_kw}function {exported}({all_params}): {ts_ret} {{ return {call_expr}; }}\n",
                ));
            } else {
                out.push_str(&format!(
                    "  export {async_kw}function {exported}({all_params}): {ts_ret} {{ {call_expr}; }}\n",
                ));
            }
        }
        out.push_str("}\n");
    }

    out
}

// ---------------------------------------------------------------------------
// Callback interface generation
// ---------------------------------------------------------------------------

fn render_callback_interface(cb: &UdlCallbackInterface, cfg: &config::JsBindingsConfig) -> String {
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
            .cloned()
            .unwrap_or_else(|| camel_case(&m.name));
        let params: Vec<String> = m.args.iter().map(render_param).collect();
        let ts_ret = if m.is_async {
            format!(
                "Promise<{}>",
                m.return_type
                    .as_ref()
                    .map(ts_type_str)
                    .unwrap_or_else(|| "void".to_string())
            )
        } else {
            m.return_type
                .as_ref()
                .map(ts_type_str)
                .unwrap_or_else(|| "void".to_string())
        };
        out.push_str(&render_jsdoc(m.docstring.as_deref(), "  "));
        out.push_str(&format!("  {exported}({}): {ts_ret};\n", params.join(", ")));
    }
    out.push_str("}\n");
    out
}

// ---------------------------------------------------------------------------
// Object class generation
// ---------------------------------------------------------------------------

/// Render a single constructor (primary or named) as a static factory method.
fn render_ctor(
    ctor: &UdlConstructor,
    class_name: &str,
    call_prefix: &str,
    exported: &str,
    _bg_name: &str,
    _cfg: &config::JsBindingsConfig,
) -> String {
    let mut out = String::new();
    let params: Vec<String> = ctor.args.iter().map(render_param).collect();
    let args: Vec<String> = ctor.args.iter().map(|a| camel_case(&a.name)).collect();
    let inner_expr = format!("{call_prefix}({})", args.join(", "));

    let async_kw = if ctor.is_async { "async " } else { "" };
    let await_kw = if ctor.is_async { "await " } else { "" };
    let ret_type = if ctor.is_async {
        format!("Promise<{class_name}>")
    } else {
        class_name.to_string()
    };

    out.push_str(&render_jsdoc(ctor.docstring.as_deref(), "  "));
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

fn render_object_class(o: &UdlObject, _namespace: &str, cfg: &config::JsBindingsConfig) -> String {
    let mut out = String::new();
    let name = &o.name;
    let bg_name = snake_case(name); // wasm-bindgen uses the Rust struct name

    out.push_str(&render_jsdoc(o.docstring.as_deref(), ""));
    out.push_str(&format!("export class {name} {{\n"));
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
            "new",
            &bg_name,
            cfg,
        ));
    }

    for ctor in named_ctors {
        let exported = cfg
            .rename
            .get(&format!("{}.{}", name, ctor.name))
            .cloned()
            .unwrap_or_else(|| camel_case(&ctor.name));
        let ctor_fn = format!("{bg_name}_{}", ctor.name);
        out.push_str(&render_ctor(
            ctor,
            name,
            &format!("__bg.{ctor_fn}"),
            &exported,
            &bg_name,
            cfg,
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
            .cloned()
            .unwrap_or_else(|| camel_case(&m.name));
        let params: Vec<String> = m.args.iter().map(render_param).collect();
        let ts_ret = if m.is_async {
            format!(
                "Promise<{}>",
                m.return_type
                    .as_ref()
                    .map(ts_type_str)
                    .unwrap_or_else(|| "void".to_string())
            )
        } else {
            m.return_type
                .as_ref()
                .map(ts_type_str)
                .unwrap_or_else(|| "void".to_string())
        };
        let call_args: Vec<String> = m.args.iter().map(|a| camel_case(&a.name)).collect();
        // wasm-bindgen preserves Rust method names verbatim (snake_case) in its JS glue.
        // The public TypeScript name is camelCase (via `exported`), but the inner call
        // must match the wasm-pack output exactly.
        let raw_call = format!("this._inner.{}({})", m.name, call_args.join(", "));
        let call_expr = lift_return(&raw_call, m.return_type.as_ref(), m.is_async);

        let async_kw = if m.is_async { "async " } else { "" };

        out.push_str(&render_jsdoc(m.docstring.as_deref(), "  "));
        if let Some(throws) = &m.throws_type {
            let lift = format!("_lift{}", type_name(throws));
            if m.return_type.is_some() {
                out.push_str(&format!(
                    "  {async_kw}{exported}({}): {ts_ret} {{\n    this._assertLive();\n    try {{ return {call_expr}; }} catch (e) {{ return {lift}(e); }}\n  }}\n",
                    params.join(", ")
                ));
            } else {
                out.push_str(&format!(
                    "  {async_kw}{exported}({}): {ts_ret} {{\n    this._assertLive();\n    try {{ {call_expr}; }} catch (e) {{ {lift}(e); }}\n  }}\n",
                    params.join(", ")
                ));
            }
        } else if m.return_type.is_some() {
            out.push_str(&format!(
                "  {async_kw}{exported}({}): {ts_ret} {{\n    this._assertLive();\n    return {call_expr};\n  }}\n",
                params.join(", ")
            ));
        } else {
            out.push_str(&format!(
                "  {async_kw}{exported}({}): {ts_ret} {{\n    this._assertLive();\n    {call_expr};\n  }}\n",
                params.join(", ")
            ));
        }
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

fn render_function(f: &UdlFunction, cfg: &config::JsBindingsConfig) -> String {
    let mut out = String::new();

    let exported = cfg
        .rename
        .get(&f.name)
        .cloned()
        .unwrap_or_else(|| camel_case(&f.name));

    let params: Vec<String> = f.args.iter().map(render_param).collect();

    let ts_ret = if f.is_async {
        format!(
            "Promise<{}>",
            f.return_type
                .as_ref()
                .map(ts_type_str)
                .unwrap_or_else(|| "void".to_string())
        )
    } else {
        f.return_type
            .as_ref()
            .map(ts_type_str)
            .unwrap_or_else(|| "void".to_string())
    };

    let call_args: Vec<String> = f.args.iter().map(|a| camel_case(&a.name)).collect();
    // wasm-pack exports top-level functions under their original snake_case Rust names;
    // object methods are camelCase in the JS glue. Do not apply camel_case here.
    let raw_call = format!("__bg.{}({})", f.name, call_args.join(", "));
    let call_expr = lift_return(&raw_call, f.return_type.as_ref(), f.is_async);

    let async_kw = if f.is_async { "async " } else { "" };

    out.push_str(&render_jsdoc(f.docstring.as_deref(), "  "));
    if let Some(throws) = &f.throws_type {
        let lift = format!("_lift{}", type_name(throws));
        if f.return_type.is_some() {
            out.push_str(&format!(
                "  export {async_kw}function {exported}({}): {ts_ret} {{\n    try {{ return {call_expr}; }} catch (e) {{ return {lift}(e); }}\n  }}\n",
                params.join(", ")
            ));
        } else {
            out.push_str(&format!(
                "  export {async_kw}function {exported}({}): {ts_ret} {{\n    try {{ {call_expr}; }} catch (e) {{ {lift}(e); }}\n  }}\n",
                params.join(", ")
            ));
        }
    } else if f.return_type.is_some() {
        out.push_str(&format!(
            "  export {async_kw}function {exported}({}): {ts_ret} {{ return {call_expr}; }}\n",
            params.join(", ")
        ));
    } else {
        out.push_str(&format!(
            "  export {async_kw}function {exported}({}): {ts_ret} {{ {call_expr}; }}\n",
            params.join(", ")
        ));
    }

    out
}

// ---------------------------------------------------------------------------
// External type import collection
// ---------------------------------------------------------------------------

/// Collect all external types referenced in `metadata` and map them to TypeScript
/// import statements.  Returns a `BTreeMap<import_path, sorted_type_names>` so
/// that callers get deterministic output without any extra sorting step.
///
/// "External" means: a named type whose `module_path` does not begin with
/// `LOCAL_CRATE`.  That string matches the literal we pass to
/// `ComponentInterface::from_webidl(…, "crate_name")`, so it is the module
/// prefix of every type that is defined in the current UDL file.
/// The crate name sentinel used when parsing UDL via `ComponentInterface::from_webidl`.
/// All local types will have a `module_path` whose first `::` segment equals this value.
/// External types declared with `[External="crate_name"]` will differ.
const LOCAL_CRATE_SENTINEL: &str = "crate_name";

fn collect_external_imports(
    metadata: &UdlMetadata,
    external_packages: &HashMap<String, String>,
) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let mut map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    macro_rules! visit {
        ($t:expr) => {
            visit_type_for_external($t, external_packages, &mut map)?
        };
    }

    for f in &metadata.functions {
        for a in &f.args {
            visit!(&a.type_);
        }
        if let Some(r) = &f.return_type {
            visit!(r);
        }
        if let Some(t) = &f.throws_type {
            visit!(t);
        }
    }
    for r in &metadata.records {
        for f in &r.fields {
            visit!(&f.type_);
        }
    }
    for e in &metadata.errors {
        for v in &e.variants {
            for f in &v.fields {
                visit!(&f.type_);
            }
        }
        for m in &e.methods {
            for a in &m.args {
                visit!(&a.type_);
            }
            if let Some(r) = &m.return_type {
                visit!(r);
            }
            if let Some(t) = &m.throws_type {
                visit!(t);
            }
        }
    }
    for e in &metadata.enums {
        for v in &e.variants {
            for f in &v.fields {
                visit!(&f.type_);
            }
        }
        for m in &e.methods {
            for a in &m.args {
                visit!(&a.type_);
            }
            if let Some(r) = &m.return_type {
                visit!(r);
            }
            if let Some(t) = &m.throws_type {
                visit!(t);
            }
        }
    }
    for o in &metadata.objects {
        for c in &o.constructors {
            for a in &c.args {
                visit!(&a.type_);
            }
            if let Some(t) = &c.throws_type {
                visit!(t);
            }
        }
        for m in &o.methods {
            for a in &m.args {
                visit!(&a.type_);
            }
            if let Some(r) = &m.return_type {
                visit!(r);
            }
            if let Some(t) = &m.throws_type {
                visit!(t);
            }
        }
    }
    for cb in &metadata.callback_interfaces {
        for m in &cb.methods {
            for a in &m.args {
                visit!(&a.type_);
            }
            if let Some(r) = &m.return_type {
                visit!(r);
            }
        }
    }
    // Custom types carry their own module_path, so an external `[Custom]` typedef
    // must also be imported from the appropriate package.
    for ct in &metadata.custom_types {
        let crate_name = ct.module_path.split("::").next().unwrap_or("");
        if crate_name != LOCAL_CRATE_SENTINEL && !crate_name.is_empty() {
            match external_packages.get(crate_name) {
                Some(import_path) => {
                    map.entry(import_path.clone())
                        .or_default()
                        .insert(ct.name.clone());
                }
                None => {
                    anyhow::bail!(
                        "external custom type `{}` (crate `{}`) has no entry in \
                         [bindings.js] external_packages — add `{} = \"./path.js\"` \
                         to uniffi.toml",
                        ct.name,
                        crate_name,
                        crate_name,
                    );
                }
            }
        }
    }

    Ok(map)
}

fn visit_type_for_external(
    t: &Type,
    external_packages: &HashMap<String, String>,
    imports: &mut BTreeMap<String, BTreeSet<String>>,
) -> Result<()> {
    match t {
        Type::Optional { inner_type } => {
            visit_type_for_external(inner_type, external_packages, imports)
        }
        Type::Sequence { inner_type } => {
            visit_type_for_external(inner_type, external_packages, imports)
        }
        Type::Map {
            key_type,
            value_type,
        } => {
            visit_type_for_external(key_type, external_packages, imports)?;
            visit_type_for_external(value_type, external_packages, imports)
        }
        _ => {
            let crate_name = match t {
                Type::Object { module_path, .. }
                | Type::Record { module_path, .. }
                | Type::Enum { module_path, .. }
                | Type::CallbackInterface { module_path, .. }
                | Type::Custom { module_path, .. } => module_path.split("::").next(),
                _ => None,
            };
            let type_name = match t {
                Type::Object { name, .. }
                | Type::Record { name, .. }
                | Type::Enum { name, .. }
                | Type::CallbackInterface { name, .. }
                | Type::Custom { name, .. } => Some(name.as_str()),
                _ => None,
            };
            if let (Some(crate_name), Some(type_name)) = (crate_name, type_name) {
                if crate_name != LOCAL_CRATE_SENTINEL {
                    match external_packages.get(crate_name) {
                        Some(import_path) => {
                            imports
                                .entry(import_path.clone())
                                .or_default()
                                .insert(type_name.to_string());
                        }
                        None => {
                            anyhow::bail!(
                                "external type `{type_name}` (crate `{crate_name}`) has no \
                                 entry in [bindings.js] external_packages — add \
                                 `{crate_name} = \"./path.js\"` to uniffi.toml"
                            );
                        }
                    }
                }
            }
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Type helpers
// ---------------------------------------------------------------------------

fn ts_type_str(t: &Type) -> String {
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
        Type::Optional { inner_type } => format!("{} | null", ts_type_str(inner_type)),
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

fn type_name(t: &Type) -> String {
    match t {
        Type::Enum { name, .. }
        | Type::Record { name, .. }
        | Type::Object { name, .. }
        | Type::CallbackInterface { name, .. } => name.clone(),
        _ => ts_type_str(t),
    }
}

/// Return `true` if `module_path` belongs to the current (local) crate.
/// See `LOCAL_CRATE_SENTINEL` for why the sentinel value is `"crate_name"`.
fn is_local_module(module_path: &str) -> bool {
    module_path.split("::").next() == Some(LOCAL_CRATE_SENTINEL)
}

/// Wrap a raw WASM call expression with the appropriate lift for its return type.
/// Local object types are lifted via their static `_fromInner` factory so the caller
/// receives the TypeScript wrapper class rather than the raw wasm-bindgen instance.
/// External object types are returned as-is (their package owns the wrapping).
///
/// When `is_async` is true, `await` is placed **inside** the lift expression so that
/// `_fromInner` (or the IIFE / `.map()`) receives the resolved value, not a `Promise`.
fn lift_return(raw_call: &str, return_type: Option<&Type>, is_async: bool) -> String {
    let await_kw = if is_async { "await " } else { "" };
    match return_type {
        Some(Type::Object {
            name, module_path, ..
        }) => {
            if is_local_module(module_path) {
                format!("{name}._fromInner({await_kw}{raw_call})")
            } else {
                format!("{await_kw}{raw_call}")
            }
        }
        Some(Type::Optional { inner_type }) => match inner_type.as_ref() {
            Type::Object {
                name, module_path, ..
            } if is_local_module(module_path) => {
                // Evaluate the raw call once via an IIFE, then conditionally lift.
                if is_async {
                    format!("((__v) => __v == null ? null : {name}._fromInner(__v))({await_kw}{raw_call})")
                } else {
                    format!("((__v) => __v == null ? null : {name}._fromInner(__v))({raw_call})")
                }
            }
            _ => format!("{await_kw}{raw_call}"),
        },
        Some(Type::Sequence { inner_type }) => match inner_type.as_ref() {
            Type::Object {
                name, module_path, ..
            } if is_local_module(module_path) => {
                format!("({await_kw}{raw_call}).map((__v) => {name}._fromInner(__v))")
            }
            _ => format!("{await_kw}{raw_call}"),
        },
        // Map<_, Object> is not lifted element-wise: UniFFI does not commonly use
        // Map with Object values, and the marshaling shape depends heavily on how
        // wasm-bindgen serialises the map.  Add explicit handling if this becomes needed.
        _ => format!("{await_kw}{raw_call}"),
    }
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

/// Render an optional UDL docstring as a JSDoc block comment.
///
/// Returns an empty string when `docstring` is `None` or blank, so callers can
/// unconditionally prepend the result without introducing extra blank lines.
/// `indent` is prepended to every line (e.g. `""` for top-level, `"  "` for members).
fn render_jsdoc(docstring: Option<&str>, indent: &str) -> String {
    let raw = match docstring.map(str::trim) {
        Some(s) if !s.is_empty() => s,
        _ => return String::new(),
    };
    // Escape `*/` so it cannot prematurely close the JSDoc block.
    let lines: Vec<String> = raw
        .lines()
        .map(|l| l.trim().replace("*/", "*\\/"))
        .collect();
    // Use single-line format only when the whole comment fits on one line (≤80 chars).
    let single = &lines[0];
    let single_len = indent.len() + "/** ".len() + single.len() + " */".len();
    if lines.len() == 1 && single_len <= 80 {
        format!("{indent}/** {single} */\n")
    } else {
        let mut out = format!("{indent}/**\n");
        for line in &lines {
            if line.is_empty() {
                out.push_str(&format!("{indent} *\n"));
            } else {
                out.push_str(&format!("{indent} * {line}\n"));
            }
        }
        out.push_str(&format!("{indent} */\n"));
        out
    }
}

/// Render a function/method parameter as `name: Type`, `name: Type = default`,
/// or `name?: Type` (when the default is unspecified).
fn render_param(arg: &UdlArg) -> String {
    let ts_name = camel_case(&arg.name);
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

/// Render a UDL default value as a TypeScript literal expression.
fn render_default_value(dv: &DefaultValue) -> String {
    match dv {
        DefaultValue::Default => "undefined".to_string(),
        DefaultValue::Literal(lit) => render_literal(lit),
    }
}

fn render_literal(lit: &Literal) -> String {
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
        Literal::Some { inner } => render_default_value(inner),
    }
}

fn snake_case(input: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = input.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_ascii_uppercase() && i > 0 {
            let prev_upper = chars[i - 1].is_ascii_uppercase();
            let next_lower = chars.get(i + 1).is_some_and(|c| c.is_ascii_lowercase());
            // Insert underscore before: a lone uppercase after lowercase, OR
            // the last letter of an acronym run when the next char is lowercase.
            if !prev_upper || next_lower {
                out.push('_');
            }
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
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

    #[test]
    fn snake_case_handles_pascal() {
        assert_eq!(snake_case("Counter"), "counter");
        assert_eq!(snake_case("MyCounter"), "my_counter");
        assert_eq!(snake_case("MyHTTPClient"), "my_http_client");
        assert_eq!(snake_case("URLError"), "url_error");
        assert_eq!(snake_case("ABC"), "abc");
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
        // 76+ chars on the line itself should spill into block format to stay under 80
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
        // Escapes single quotes and backslashes
        assert_eq!(
            render_literal(&Literal::String("it's a \\path".into())),
            "'it\\'s a \\\\path'"
        );
    }

    #[test]
    fn render_literal_uint() {
        // Small uint → plain number
        assert_eq!(
            render_literal(&Literal::UInt(
                42,
                uniffi_bindgen::interface::Radix::Decimal,
                Type::UInt32
            )),
            "42"
        );
        // u64 → bigint suffix
        assert_eq!(
            render_literal(&Literal::UInt(
                100,
                uniffi_bindgen::interface::Radix::Decimal,
                Type::UInt64
            )),
            "100n"
        );
        // i64 → bigint suffix
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
        let arg = UdlArg {
            name: "user_name".into(),
            type_: Type::String,
            default: None,
        };
        assert_eq!(render_param(&arg), "userName: string");
    }

    #[test]
    fn render_param_literal_default() {
        let arg = UdlArg {
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
        let arg = UdlArg {
            name: "label".into(),
            type_: Type::String,
            default: Some(DefaultValue::Default),
        };
        assert_eq!(render_param(&arg), "label?: string");
    }

    // -----------------------------------------------------------------------
    // lift_return tests for Optional<Object> and Sequence<Object>
    // -----------------------------------------------------------------------

    #[test]
    fn lift_return_optional_object_sync() {
        let result = lift_return(
            "__bg.find_item()",
            Some(&Type::Optional {
                inner_type: Box::new(Type::Object {
                    name: "Item".into(),
                    module_path: "crate_name::mod".into(),
                    imp: uniffi_bindgen::interface::ObjectImpl::Struct,
                }),
            }),
            false,
        );
        assert_eq!(
            result,
            "((__v) => __v == null ? null : Item._fromInner(__v))(__bg.find_item())"
        );
    }

    #[test]
    fn lift_return_optional_object_async() {
        let result = lift_return(
            "__bg.find_item()",
            Some(&Type::Optional {
                inner_type: Box::new(Type::Object {
                    name: "Item".into(),
                    module_path: "crate_name::mod".into(),
                    imp: uniffi_bindgen::interface::ObjectImpl::Struct,
                }),
            }),
            true,
        );
        assert_eq!(
            result,
            "((__v) => __v == null ? null : Item._fromInner(__v))(await __bg.find_item())"
        );
    }

    #[test]
    fn lift_return_sequence_object_sync() {
        let result = lift_return(
            "__bg.list_items()",
            Some(&Type::Sequence {
                inner_type: Box::new(Type::Object {
                    name: "Item".into(),
                    module_path: "crate_name::mod".into(),
                    imp: uniffi_bindgen::interface::ObjectImpl::Struct,
                }),
            }),
            false,
        );
        assert_eq!(
            result,
            "(__bg.list_items()).map((__v) => Item._fromInner(__v))"
        );
    }

    #[test]
    fn lift_return_sequence_object_async() {
        let result = lift_return(
            "__bg.list_items()",
            Some(&Type::Sequence {
                inner_type: Box::new(Type::Object {
                    name: "Item".into(),
                    module_path: "crate_name::mod".into(),
                    imp: uniffi_bindgen::interface::ObjectImpl::Struct,
                }),
            }),
            true,
        );
        assert_eq!(
            result,
            "(await __bg.list_items()).map((__v) => Item._fromInner(__v))"
        );
    }

    #[test]
    fn lift_return_external_object_not_lifted() {
        let result = lift_return(
            "__bg.get_ext()",
            Some(&Type::Object {
                name: "ExtObj".into(),
                module_path: "other_crate::mod".into(),
                imp: uniffi_bindgen::interface::ObjectImpl::Struct,
            }),
            false,
        );
        assert_eq!(result, "__bg.get_ext()");
    }

    // -----------------------------------------------------------------------
    // render_enum_methods_on_class tests
    // -----------------------------------------------------------------------

    fn make_method(name: &str, return_type: Option<Type>, is_async: bool) -> UdlMethod {
        UdlMethod {
            name: name.into(),
            args: vec![],
            return_type,
            throws_type: None,
            is_async,
            docstring: None,
        }
    }

    #[test]
    fn render_enum_methods_on_class_instance_method() {
        let methods = vec![make_method("describe", Some(Type::String), false)];
        let cfg = config::JsBindingsConfig::default();
        let result = render_enum_methods_on_class(&methods, "StatusCode", &cfg);
        assert!(
            result.contains("describe(): string { return __bg.status_code_describe(this); }"),
            "got: {result}"
        );
    }

    #[test]
    fn render_enum_methods_on_class_async_method() {
        let methods = vec![make_method("process", None, true)];
        let cfg = config::JsBindingsConfig::default();
        let result = render_enum_methods_on_class(&methods, "Task", &cfg);
        assert!(
            result.contains("async process(): Promise<void>"),
            "got: {result}"
        );
    }

    // -----------------------------------------------------------------------
    // render_enum_type companion namespace tests
    // -----------------------------------------------------------------------

    #[test]
    fn render_enum_type_with_methods_has_companion_namespace() {
        let e = UdlEnum {
            name: "Shape".into(),
            variants: vec![UdlVariant {
                name: "Circle".into(),
                fields: vec![],
                docstring: None,
                discr: None,
            }],
            is_flat: true,
            docstring: None,
            methods: vec![make_method("area", Some(Type::Float64), false)],
        };
        let cfg = config::JsBindingsConfig::default();
        let result = render_enum_type(&e, &cfg);
        assert!(
            result.contains("export type Shape = 'Circle';"),
            "got: {result}"
        );
        assert!(result.contains("export namespace Shape {"), "got: {result}");
        assert!(
            result.contains(
                "export function area(self: Shape): number { return __bg.shape_area(self); }"
            ),
            "got: {result}"
        );
    }

    #[test]
    fn render_enum_type_without_methods_no_namespace() {
        let e = UdlEnum {
            name: "Dir".into(),
            variants: vec![UdlVariant {
                name: "Up".into(),
                fields: vec![],
                docstring: None,
                discr: None,
            }],
            is_flat: true,
            docstring: None,
            methods: vec![],
        };
        let cfg = config::JsBindingsConfig::default();
        let result = render_enum_type(&e, &cfg);
        assert!(!result.contains("namespace"), "got: {result}");
    }

    // -----------------------------------------------------------------------
    // pascal_case edge case
    // -----------------------------------------------------------------------

    #[test]
    fn pascal_case_empty_returns_fallback() {
        assert_eq!(pascal_case(""), "UniffiBindings");
    }

    // -----------------------------------------------------------------------
    // ts_type_str: Sequence<Optional<T>> parenthesization
    // -----------------------------------------------------------------------

    #[test]
    fn sequence_of_optional_parenthesized() {
        let t = Type::Sequence {
            inner_type: Box::new(Type::Optional {
                inner_type: Box::new(Type::String),
            }),
        };
        assert_eq!(ts_type_str(&t), "(string | null)[]");
    }

    #[test]
    fn sequence_of_plain_not_parenthesized() {
        let t = Type::Sequence {
            inner_type: Box::new(Type::String),
        };
        assert_eq!(ts_type_str(&t), "string[]");
    }

    // -----------------------------------------------------------------------
    // Generator error-path tests
    // -----------------------------------------------------------------------

    #[test]
    fn missing_external_package_errors_for_object() {
        let metadata = UdlMetadata {
            functions: vec![UdlFunction {
                name: "get_ext".into(),
                args: vec![],
                return_type: Some(Type::Object {
                    name: "ExtObj".into(),
                    module_path: "other_crate::mod".into(),
                    imp: uniffi_bindgen::interface::ObjectImpl::Struct,
                }),
                throws_type: None,
                is_async: false,
                docstring: None,
            }],
            ..Default::default()
        };
        let empty_packages = HashMap::new();
        let result = collect_external_imports(&metadata, &empty_packages);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("external_packages"),
            "error should mention external_packages, got: {msg}"
        );
        assert!(
            msg.contains("other_crate"),
            "error should mention the crate name, got: {msg}"
        );
    }

    #[test]
    fn missing_external_package_errors_for_custom_type() {
        let metadata = UdlMetadata {
            custom_types: vec![UdlCustomType {
                name: "RemoteUrl".into(),
                builtin: Type::String,
                module_path: "remote_crate::types".into(),
            }],
            ..Default::default()
        };
        let empty_packages = HashMap::new();
        let result = collect_external_imports(&metadata, &empty_packages);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("remote_crate"),
            "error should mention the crate name, got: {msg}"
        );
    }

    #[test]
    fn external_package_with_config_succeeds() {
        let metadata = UdlMetadata {
            functions: vec![UdlFunction {
                name: "get_ext".into(),
                args: vec![],
                return_type: Some(Type::Object {
                    name: "ExtObj".into(),
                    module_path: "other_crate::mod".into(),
                    imp: uniffi_bindgen::interface::ObjectImpl::Struct,
                }),
                throws_type: None,
                is_async: false,
                docstring: None,
            }],
            ..Default::default()
        };
        let mut packages = HashMap::new();
        packages.insert("other_crate".into(), "./other.js".into());
        let result = collect_external_imports(&metadata, &packages);
        assert!(result.is_ok());
        let imports = result.unwrap();
        assert!(imports.contains_key("./other.js"));
        assert!(imports["./other.js"].contains("ExtObj"));
    }

    #[test]
    fn local_types_not_treated_as_external() {
        let metadata = UdlMetadata {
            functions: vec![UdlFunction {
                name: "get_local".into(),
                args: vec![],
                return_type: Some(Type::Object {
                    name: "LocalObj".into(),
                    module_path: "crate_name::mod".into(),
                    imp: uniffi_bindgen::interface::ObjectImpl::Struct,
                }),
                throws_type: None,
                is_async: false,
                docstring: None,
            }],
            ..Default::default()
        };
        let empty_packages = HashMap::new();
        let result = collect_external_imports(&metadata, &empty_packages);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
