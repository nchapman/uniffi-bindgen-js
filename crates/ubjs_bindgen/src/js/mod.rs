use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use uniffi_bindgen::interface::{AsType, ComponentInterface, Type};

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

    let content = render_ts(&module_name, &namespace, &metadata, &cfg);
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
}

#[derive(Debug)]
struct UdlArg {
    name: String,
    type_: Type,
}

/// A variant field (used in rich error variants and data enum variants).
#[derive(Debug)]
struct UdlField {
    name: String,
    type_: Type,
}

/// One variant of an enum or error type.
#[derive(Debug)]
struct UdlVariant {
    name: String,
    /// Empty for flat variants (no associated data).
    fields: Vec<UdlField>,
}

/// A [Error] enum — generates a TypeScript error class.
#[derive(Debug)]
struct UdlError {
    name: String,
    variants: Vec<UdlVariant>,
    is_flat: bool,
}

/// A plain enum or [Enum] interface — generates a TypeScript union type.
#[derive(Debug)]
struct UdlEnum {
    name: String,
    variants: Vec<UdlVariant>,
    /// true ↔ all variants are unit variants (no fields); serialises as a string.
    is_flat: bool,
}

/// A `dictionary` declaration — generates a TypeScript interface.
#[derive(Debug)]
struct UdlRecord {
    name: String,
    fields: Vec<UdlField>,
}

/// A constructor of an `interface` object.
#[derive(Debug)]
struct UdlConstructor {
    /// Exported name in JS.  Usually "new".
    name: String,
    args: Vec<UdlArg>,
    throws_type: Option<Type>,
}

/// A method on an `interface` object.
#[derive(Debug)]
struct UdlMethod {
    name: String,
    args: Vec<UdlArg>,
    return_type: Option<Type>,
    throws_type: Option<Type>,
    is_async: bool,
}

/// An `interface` declaration — generates a TypeScript class.
#[derive(Debug)]
struct UdlObject {
    name: String,
    constructors: Vec<UdlConstructor>,
    methods: Vec<UdlMethod>,
}

/// A `[Custom]` typedef — generates a TypeScript type alias.
#[derive(Debug)]
struct UdlCustomType {
    /// The custom type name (e.g. `Url`).
    name: String,
    /// The underlying builtin type (e.g. `Type::String`).
    builtin: Type,
}

#[derive(Debug, Default)]
struct UdlMetadata {
    namespace: String,
    functions: Vec<UdlFunction>,
    errors: Vec<UdlError>,
    enums: Vec<UdlEnum>,
    records: Vec<UdlRecord>,
    objects: Vec<UdlObject>,
    custom_types: Vec<UdlCustomType>,
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
                })
                .collect(),
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
                        })
                        .collect(),
                    throws_type: c.throws_type().cloned(),
                })
                .collect(),
            methods: o
                .methods()
                .iter()
                .map(|m| UdlMethod {
                    name: m.name().to_string(),
                    args: m
                        .arguments()
                        .into_iter()
                        .map(|a| UdlArg {
                            name: a.name().to_string(),
                            type_: a.as_type(),
                        })
                        .collect(),
                    return_type: m.return_type().cloned(),
                    throws_type: m.throws_type().cloned(),
                    is_async: m.is_async(),
                })
                .collect(),
        })
        .collect();

    // Collect all [Custom] typedefs from the type universe, sorted by name for
    // deterministic output (iter_local_types order is not guaranteed by uniffi-bindgen).
    let mut seen_custom: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut custom_types: Vec<UdlCustomType> = Vec::new();
    for t in ci.iter_local_types() {
        if let Type::Custom { name, builtin, .. } = t {
            if seen_custom.insert(name.clone()) {
                custom_types.push(UdlCustomType {
                    name: name.clone(),
                    builtin: *builtin.clone(),
                });
            }
        }
    }
    custom_types.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(UdlMetadata {
        namespace: ci.namespace().to_string(),
        functions,
        errors,
        enums,
        records,
        objects,
        custom_types,
    })
}

fn parse_enums(ci: &ComponentInterface) -> (Vec<UdlError>, Vec<UdlEnum>) {
    let mut errors = Vec::new();
    let mut enums = Vec::new();

    for e in ci.enum_definitions() {
        let variants = e
            .variants()
            .iter()
            .map(|v| UdlVariant {
                name: v.name().to_string(),
                fields: v
                    .fields()
                    .iter()
                    .map(|f| UdlField {
                        name: f.name().to_string(),
                        type_: f.as_type(),
                    })
                    .collect(),
            })
            .collect();

        if ci.is_name_used_as_error(e.name()) {
            errors.push(UdlError {
                name: e.name().to_string(),
                variants,
                is_flat: e.is_flat(),
            });
        } else {
            enums.push(UdlEnum {
                name: e.name().to_string(),
                variants,
                is_flat: e.is_flat(),
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
) -> String {
    let mut out = String::new();

    // Header
    out.push_str("// Generated by uniffi-bindgen-js. DO NOT EDIT.\n");
    out.push_str(&format!(
        "import __init, * as __bg from './{namespace}_bg.js';\n"
    ));
    out.push_str("export { __init as init };\n");

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
            out.push_str(&render_error_class(e));
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
            out.push_str(&render_enum_type(e));
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
    let all_throws: Vec<String> = {
        let mut names: Vec<String> = Vec::new();
        for f in &metadata.functions {
            if let Some(t) = &f.throws_type {
                let n = type_name(t);
                if !names.contains(&n) {
                    names.push(n);
                }
            }
        }
        for o in &metadata.objects {
            for m in &o.methods {
                if let Some(t) = &m.throws_type {
                    let n = type_name(t);
                    if !names.contains(&n) {
                        names.push(n);
                    }
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
        out.push_str(&format!("\nexport namespace {module_name} {{\n"));
        for f in &visible_fns {
            out.push_str(&render_function(f, cfg));
        }
        out.push_str("}\n");
    }

    out
}

// ---------------------------------------------------------------------------
// Error class generation
// ---------------------------------------------------------------------------

fn render_error_class(e: &UdlError) -> String {
    let mut out = String::new();
    let name = &e.name;

    if e.is_flat {
        // Flat error: single `tag` string property, no variant fields
        let tag_union: Vec<String> = e.variants.iter().map(|v| format!("'{}'", v.name)).collect();

        out.push_str(&format!("export class {name} extends Error {{\n"));
        out.push_str(&format!("  readonly tag: {};\n", tag_union.join(" | ")));
        out.push_str(&format!(
            "  constructor(tag: {}) {{\n",
            tag_union.join(" | ")
        ));
        out.push_str("    super(tag);\n");
        out.push_str(&format!("    this.name = '{name}';\n"));
        out.push_str("    this.tag = tag;\n");
        out.push_str("  }\n");
        for v in &e.variants {
            out.push_str(&format!(
                "  static {}(): {name} {{ return new {name}('{}'); }}\n",
                v.name, v.name
            ));
        }
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
                    "  | {{ tag: '{}'; {} }}{sep}\n",
                    v.name,
                    fields.join("; ")
                ));
            }
        }
        out.push_str(&format!("export class {name} extends Error {{\n"));
        out.push_str(&format!(
            "  constructor(public readonly variant: {variant_type}) {{\n"
        ));
        out.push_str("    super(variant.tag);\n");
        out.push_str(&format!("    this.name = '{name}';\n"));
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
            out.push_str(&format!(
                "  static {}({}): {name} {{ return new {name}({variant_obj}); }}\n",
                v.name,
                params.join(", ")
            ));
        }
        out.push_str("}\n");
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
    out.push_str(&format!("export interface {} {{\n", r.name));
    for f in &r.fields {
        let ts_name = camel_case(&f.name);
        let ts_type = ts_type_str(&f.type_);
        out.push_str(&format!("  {ts_name}: {ts_type};\n"));
    }
    out.push_str("}\n");
    out
}

// ---------------------------------------------------------------------------
// Enum type generation
// ---------------------------------------------------------------------------

fn render_enum_type(e: &UdlEnum) -> String {
    let mut out = String::new();
    if e.is_flat {
        // Flat enum → TypeScript union of string literals
        let variants: Vec<String> = e.variants.iter().map(|v| format!("'{}'", v.name)).collect();
        out.push_str(&format!(
            "export type {} = {};\n",
            e.name,
            variants.join(" | ")
        ));
    } else {
        // Data enum → discriminated union
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
                    "  | {{ tag: '{}'; {} }}{sep}\n",
                    v.name,
                    fields.join("; ")
                ));
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Object class generation
// ---------------------------------------------------------------------------

fn render_object_class(o: &UdlObject, _namespace: &str, cfg: &config::JsBindingsConfig) -> String {
    let mut out = String::new();
    let name = &o.name;
    let bg_name = snake_case(name); // wasm-bindgen uses the Rust struct name

    out.push_str(&format!("export class {name} {{\n"));
    out.push_str(&format!("  private readonly _inner: __bg.{name};\n"));

    // Constructors — the primary constructor wraps the wasm-bindgen `new` call.
    // Named constructors become static factory methods.
    let primary_ctor = o.constructors.iter().find(|c| c.name == "new");
    let named_ctors: Vec<&UdlConstructor> =
        o.constructors.iter().filter(|c| c.name != "new").collect();

    // Private base constructor — always present for internal use
    out.push_str(&format!("  private constructor(inner: __bg.{name}) {{\n"));
    out.push_str("    this._inner = inner;\n");
    out.push_str("  }\n");

    if let Some(ctor) = primary_ctor {
        let params: Vec<String> = ctor
            .args
            .iter()
            .map(|a| format!("{}: {}", camel_case(&a.name), ts_type_str(&a.type_)))
            .collect();
        let args: Vec<String> = ctor.args.iter().map(|a| camel_case(&a.name)).collect();
        let body = format!("new __bg.{name}({})", args.join(", "));
        if let Some(throws) = &ctor.throws_type {
            let lift = format!("_lift{}", type_name(throws));
            out.push_str(&format!(
                "  static new({}): {name} {{\n    try {{ return new {name}({body}); }} catch (e) {{ return {lift}(e); }}\n  }}\n",
                params.join(", ")
            ));
        } else {
            out.push_str(&format!(
                "  static new({}): {name} {{ return new {name}({body}); }}\n",
                params.join(", ")
            ));
        }
    }

    for ctor in named_ctors {
        let exported = cfg
            .rename
            .get(&format!("{}.{}", name, ctor.name))
            .cloned()
            .unwrap_or_else(|| camel_case(&ctor.name));
        let params: Vec<String> = ctor
            .args
            .iter()
            .map(|a| format!("{}: {}", camel_case(&a.name), ts_type_str(&a.type_)))
            .collect();
        let args: Vec<String> = ctor.args.iter().map(|a| camel_case(&a.name)).collect();
        let ctor_fn = format!("{bg_name}_{}", ctor.name);
        let body = format!("{ctor_fn}({})", args.join(", "));
        if let Some(throws) = &ctor.throws_type {
            let lift = format!("_lift{}", type_name(throws));
            out.push_str(&format!(
                "  static {exported}({}): {name} {{\n    try {{ return new {name}(__bg.{body}); }} catch (e) {{ return {lift}(e); }}\n  }}\n",
                params.join(", ")
            ));
        } else {
            out.push_str(&format!(
                "  static {exported}({}): {name} {{ return new {name}(__bg.{body}); }}\n",
                params.join(", ")
            ));
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
            .cloned()
            .unwrap_or_else(|| camel_case(&m.name));
        let params: Vec<String> = m
            .args
            .iter()
            .map(|a| format!("{}: {}", camel_case(&a.name), ts_type_str(&a.type_)))
            .collect();
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
        // wasm-bindgen converts Rust method names to camelCase in its JS glue.
        let raw_call = format!(
            "this._inner.{}({})",
            camel_case(&m.name),
            call_args.join(", ")
        );
        let call_expr = if m.is_async {
            format!("await {raw_call}")
        } else {
            raw_call
        };

        let async_kw = if m.is_async { "async " } else { "" };

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

    // free() — wasm-bindgen generates this on all object classes
    out.push_str("  free(): void { this._inner.free(); }\n");

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

    let params: Vec<String> = f
        .args
        .iter()
        .map(|a| format!("{}: {}", camel_case(&a.name), ts_type_str(&a.type_)))
        .collect();

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
    let raw_call = format!("__bg.{}({})", f.name, call_args.join(", "));
    let call_expr = if f.is_async {
        format!("await {raw_call}")
    } else {
        raw_call
    };

    let async_kw = if f.is_async { "async " } else { "" };

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
        Type::Optional { inner_type } => format!("{} | null", ts_type_str(inner_type)),
        Type::Sequence { inner_type } => format!("{}[]", ts_type_str(inner_type)),
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
        _ => "unknown".to_string(),
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

fn snake_case(input: &str) -> String {
    let mut out = String::new();
    for (i, ch) in input.chars().enumerate() {
        if ch.is_ascii_uppercase() && i > 0 {
            out.push('_');
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
    }
}
