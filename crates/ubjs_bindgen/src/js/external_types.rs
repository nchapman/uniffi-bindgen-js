// ---------------------------------------------------------------------------
// External type import collection
// ---------------------------------------------------------------------------

use std::collections::{BTreeMap, BTreeSet, HashMap};

use anyhow::Result;
use uniffi_bindgen::interface::Type;

use super::types::UdlMetadata;

/// Collect all external types referenced in `metadata` and map them to TypeScript
/// import statements.  Returns a `BTreeMap<import_path, sorted_type_names>` so
/// that callers get deterministic output without any extra sorting step.
///
/// "External" means: a named type whose `module_path` does not begin with
/// `LOCAL_CRATE`.  That string matches the literal we pass to
/// `ComponentInterface::from_webidl(…, "crate_name")`, so it is the module
/// prefix of every type that is defined in the current UDL file.
pub(super) fn collect_external_imports(
    metadata: &UdlMetadata,
    external_packages: &HashMap<String, String>,
    local_crate: &str,
) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let mut map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    macro_rules! visit {
        ($t:expr) => {
            visit_type_for_external($t, external_packages, local_crate, &mut map)?
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
        for c in &e.constructors {
            for a in &c.args {
                visit!(&a.type_);
            }
            if let Some(t) = &c.throws_type {
                visit!(t);
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
        for c in &e.constructors {
            for a in &c.args {
                visit!(&a.type_);
            }
            if let Some(t) = &c.throws_type {
                visit!(t);
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
            // UdlCallbackMethod has no throws_type by design; see types.rs.
        }
    }
    // Custom types carry their own module_path, so an external `[Custom]` typedef
    // must also be imported from the appropriate package.
    for ct in &metadata.custom_types {
        let crate_name = ct.module_path.split("::").next().unwrap_or("");
        if crate_name != local_crate && !crate_name.is_empty() {
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
    local_crate: &str,
    imports: &mut BTreeMap<String, BTreeSet<String>>,
) -> Result<()> {
    match t {
        Type::Optional { inner_type } => {
            visit_type_for_external(inner_type, external_packages, local_crate, imports)
        }
        Type::Sequence { inner_type } => {
            visit_type_for_external(inner_type, external_packages, local_crate, imports)
        }
        Type::Map {
            key_type,
            value_type,
        } => {
            visit_type_for_external(key_type, external_packages, local_crate, imports)?;
            visit_type_for_external(value_type, external_packages, local_crate, imports)
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
                if crate_name != local_crate {
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
