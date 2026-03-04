// ---------------------------------------------------------------------------
// WASM metadata extraction
// ---------------------------------------------------------------------------
//
// Extracts UNIFFI_META_* metadata from compiled .wasm files.
//
// When Rust compiles a UniFFI crate to wasm32-unknown-unknown, each
// `UNIFFI_META_*` symbol becomes an exported i32 global whose value is a
// linear-memory address pointing into a data segment.  The bytes at that
// address are the same self-describing binary format used by native builds
// (ELF/Mach-O/PE), so we can feed them directly to `uniffi_meta::read_metadata`.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{bail, Context, Result};
use wasmparser::{Export, ExternalKind, GlobalType, Operator, Parser, Payload, ValType};

/// Extract all `UNIFFI_META_*` metadata items from a compiled `.wasm` file.
///
/// Returns the same `Vec<uniffi_meta::Metadata>` that upstream's
/// `macro_metadata::extract_from_bytes` produces for native libraries.
pub fn extract_from_wasm(path: &Path) -> Result<Vec<uniffi_meta::Metadata>> {
    let wasm_bytes =
        std::fs::read(path).with_context(|| format!("failed to read WASM: {}", path.display()))?;
    extract_from_wasm_bytes(&wasm_bytes)
}

fn extract_from_wasm_bytes(wasm_bytes: &[u8]) -> Result<Vec<uniffi_meta::Metadata>> {
    // Phase 1: Parse the WASM module to collect globals, exports, and data segments.
    let mut imported_global_count: u32 = 0;
    let mut globals: Vec<i32> = Vec::new(); // locally-defined globals only
    let mut meta_exports: BTreeMap<String, u32> = BTreeMap::new(); // name → global_index
    let mut data_segments: Vec<(u32, Vec<u8>)> = Vec::new(); // (base_addr, bytes)

    for payload in Parser::new(0).parse_all(wasm_bytes) {
        let payload = payload.context("failed to parse WASM payload")?;
        match payload {
            Payload::ImportSection(reader) => {
                for import in reader {
                    let import = import.context("failed to parse WASM import")?;
                    if matches!(import.ty, wasmparser::TypeRef::Global(_)) {
                        imported_global_count += 1;
                    }
                }
            }
            Payload::GlobalSection(reader) => {
                for global in reader {
                    let global = global.context("failed to parse WASM global")?;
                    let value = eval_i32_const_expr(&global.ty, &global.init_expr)?;
                    globals.push(value);
                }
            }
            Payload::ExportSection(reader) => {
                for export in reader {
                    let Export { name, kind, index } =
                        export.context("failed to parse WASM export")?;
                    if kind == ExternalKind::Global && is_uniffi_meta_symbol(name) {
                        meta_exports.insert(name.to_string(), index);
                    }
                }
            }
            Payload::DataSection(reader) => {
                for data in reader {
                    let data = data.context("failed to parse WASM data segment")?;
                    if let wasmparser::DataKind::Active {
                        memory_index: 0,
                        offset_expr,
                    } = &data.kind
                    {
                        let base = eval_i32_init_expr(offset_expr)?;
                        data_segments.push((base as u32, data.data.to_vec()));
                    }
                }
            }
            _ => {}
        }
    }

    if meta_exports.is_empty() {
        bail!("no UNIFFI_META_* exports found in WASM file — is this a UniFFI crate?");
    }

    // Phase 2: For each UNIFFI_META export, look up the address and read metadata.
    // Export indices are in the global index space (imported globals first, then local).
    let mut metadata_items = Vec::new();
    for (name, global_index) in &meta_exports {
        let local_index = (*global_index)
            .checked_sub(imported_global_count)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "global index {global_index} refers to an imported global \
                     (not a local data pointer) for export '{name}'"
                )
            })?;
        let addr = *globals.get(local_index as usize).ok_or_else(|| {
            anyhow::anyhow!("global index {global_index} out of range for export '{name}'")
        })? as u32;

        // `read_metadata` reads only the bytes it needs (self-delimiting format).
        // Returning the full tail slice is safe; trailing bytes are ignored.
        let bytes = read_from_data_segments(&data_segments, addr).with_context(|| {
            format!("failed to read metadata bytes for '{name}' at address {addr:#x}")
        })?;

        let item = uniffi_meta::read_metadata(bytes)
            .with_context(|| format!("failed to parse metadata for '{name}'"))?;
        metadata_items.push(item);
    }

    Ok(metadata_items)
}

/// Evaluate a constant expression that should produce an i32 value.
fn eval_i32_const_expr(ty: &GlobalType, init_expr: &wasmparser::ConstExpr) -> Result<i32> {
    if ty.content_type != ValType::I32 {
        bail!("expected i32 global, got {:?}", ty.content_type);
    }
    eval_i32_init_expr(init_expr)
}

/// Evaluate a const expression (init_expr / offset_expr) to get an i32.
///
/// The expected shape is `i32.const <value>; end`. We reject anything else.
fn eval_i32_init_expr(expr: &wasmparser::ConstExpr) -> Result<i32> {
    let mut reader = expr.get_operators_reader();
    match reader.read()? {
        Operator::I32Const { value } => Ok(value),
        other => bail!("expected I32Const in const expr, got: {other:?}"),
    }
}

fn is_uniffi_meta_symbol(name: &str) -> bool {
    let name = name.strip_prefix('_').unwrap_or(name);
    name.starts_with("UNIFFI_META")
}

/// Find the data segment containing `addr` and return a slice from that address onward.
fn read_from_data_segments(segments: &[(u32, Vec<u8>)], addr: u32) -> Result<&[u8]> {
    for (base, data) in segments {
        let end = base + data.len() as u32;
        if addr >= *base && addr < end {
            let offset = (addr - base) as usize;
            return Ok(&data[offset..]);
        }
    }
    bail!("address {addr:#x} not found in any active data segment");
}
