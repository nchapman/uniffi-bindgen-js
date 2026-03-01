// ---------------------------------------------------------------------------
// UDL parsing via uniffi_bindgen ComponentInterface
// ---------------------------------------------------------------------------

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use camino::Utf8Path;
use uniffi_bindgen::interface::{AsType, ComponentInterface, Type};

use super::types::*;

pub(super) fn parse_udl_metadata(
    source: &Path,
    crate_name: Option<&str>,
    library_mode: bool,
) -> Result<UdlMetadata> {
    if source.extension().and_then(|e| e.to_str()) != Some("udl") {
        if !library_mode {
            anyhow::bail!(
                "source '{}' is not a UDL file; pass --library to extract metadata from a compiled library",
                source.display()
            );
        }
        // Library mode: extract metadata from a compiled cdylib.
        let source_str = source
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("library source path must be valid UTF-8"))?;
        let source_utf8 = Utf8Path::new(source_str);
        let cis = uniffi_bindgen::library_mode::find_cis(
            source_utf8,
            &uniffi_bindgen::EmptyCrateConfigSupplier,
        )
        .with_context(|| format!("failed to parse library metadata: {}", source.display()))?;
        let ci = if let Some(crate_name) = crate_name {
            cis.into_iter()
                .find(|ci| ci.crate_name() == crate_name)
                .ok_or_else(|| {
                    anyhow::anyhow!("crate '{crate_name}' not found in library metadata")
                })?
        } else {
            cis.into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("no UniFFI components found in library metadata"))?
        };
        let local_crate = ci.crate_name().to_string();
        return component_interface_to_metadata(ci, &local_crate);
    }

    let udl = fs::read_to_string(source)
        .with_context(|| format!("failed to read UDL: {}", source.display()))?;
    let ci = ComponentInterface::from_webidl(&udl, LOCAL_CRATE_SENTINEL)
        .with_context(|| format!("failed to parse UDL: {}", source.display()))?;
    component_interface_to_metadata(ci, LOCAL_CRATE_SENTINEL)
}

/// Convert a `ComponentInterface` into our internal `UdlMetadata`.
/// `local_crate` is the module-path prefix for types defined in this crate
/// (for UDL: `LOCAL_CRATE_SENTINEL`, for library mode: the actual crate name).
fn component_interface_to_metadata(
    ci: ComponentInterface,
    local_crate: &str,
) -> Result<UdlMetadata> {
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
        local_crate: local_crate.to_string(),
        functions,
        errors,
        enums,
        records,
        objects,
        custom_types,
        callback_interfaces,
    })
}

pub(super) fn parse_methods(methods: &[uniffi_bindgen::interface::Method]) -> Vec<UdlMethod> {
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

pub(super) fn parse_constructors(
    constructors: &[uniffi_bindgen::interface::Constructor],
) -> Vec<UdlConstructor> {
    constructors
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
        let constructors = parse_constructors(e.constructors());

        if ci.is_name_used_as_error(e.name()) {
            errors.push(UdlError {
                name: e.name().to_string(),
                variants,
                is_flat: e.is_flat(),
                is_non_exhaustive: e.is_non_exhaustive(),
                docstring: e.docstring().map(ToOwned::to_owned),
                methods,
                constructors,
            });
        } else {
            enums.push(UdlEnum {
                name: e.name().to_string(),
                variants,
                is_flat: e.is_flat(),
                is_non_exhaustive: e.is_non_exhaustive(),
                docstring: e.docstring().map(ToOwned::to_owned),
                methods,
                constructors,
            });
        }
    }

    (errors, enums)
}

/// Extract a namespace from the source file stem (fallback for non-UDL sources).
pub(super) fn namespace_from_source(source: &Path) -> Result<String> {
    source
        .file_stem()
        .and_then(|s| s.to_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow::anyhow!("source path must have a valid UTF-8 file stem"))
}
