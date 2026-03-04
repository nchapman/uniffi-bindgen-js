// ---------------------------------------------------------------------------
// FFI metadata: function naming, element counts, type mapping
// ---------------------------------------------------------------------------
//
// This module provides helpers for:
// 1. Computing FFI buffer element counts for each type
// 2. Constructing FFI export names (uniffi_ffibuffer_*, ffi_*_rustbuffer_*, etc.)
// 3. Generating inline TypeScript code to lower/lift values for FFI calls

use uniffi_bindgen::interface::Type;

use super::naming::{camel_case, safe_js_identifier};

// ---------------------------------------------------------------------------
// Element counts
// ---------------------------------------------------------------------------

/// Number of FfiBufferElements required for a given UniFFI type.
pub(super) fn element_count(t: &Type) -> usize {
    match t {
        // Primitives: 1 element each
        Type::Int8
        | Type::UInt8
        | Type::Int16
        | Type::UInt16
        | Type::Int32
        | Type::UInt32
        | Type::Int64
        | Type::UInt64
        | Type::Float32
        | Type::Float64
        | Type::Boolean => 1,
        // Object handle: u64 = 1 element
        Type::Object { .. } => 1,
        // Callback interfaces are passed as u64 handles (like objects): 1 element
        Type::CallbackInterface { .. } => 1,
        // All compound types pass through RustBuffer: 3 elements
        Type::String
        | Type::Bytes
        | Type::Duration
        | Type::Timestamp
        | Type::Optional { .. }
        | Type::Sequence { .. }
        | Type::Map { .. }
        | Type::Record { .. }
        | Type::Enum { .. }
        | Type::Custom { .. } => 3,
    }
}

/// RustCallStatus always takes 4 elements: code(1) + error_buf RustBuffer(3).
pub(super) const CALL_STATUS_ELEMENTS: usize = 4;

/// Whether a type is passed via RustBuffer (compound/serialized) vs direct element.
pub(super) fn is_rust_buffer_type(t: &Type) -> bool {
    element_count(t) == 3
}

// ---------------------------------------------------------------------------
// FFI function name construction
// ---------------------------------------------------------------------------

/// FFI buffer function name for a top-level function.
pub(super) fn ffibuf_fn_func(namespace: &str, fn_name: &str) -> String {
    format!("uniffi_ffibuffer_{namespace}_fn_func_{fn_name}")
}

/// FFI buffer function name for a constructor.
///
/// Note: UniFFI uses `.to_ascii_lowercase()` for object names in FFI exports,
/// NOT snake_case. `AsyncCounter` → `asynccounter`, not `async_counter`.
pub(super) fn ffibuf_fn_constructor(namespace: &str, obj_name: &str, ctor_name: &str) -> String {
    let obj_lower = obj_name.to_ascii_lowercase();
    format!("uniffi_ffibuffer_{namespace}_fn_constructor_{obj_lower}_{ctor_name}")
}

/// FFI buffer function name for a method.
pub(super) fn ffibuf_fn_method(namespace: &str, obj_name: &str, method_name: &str) -> String {
    let obj_lower = obj_name.to_ascii_lowercase();
    format!("uniffi_ffibuffer_{namespace}_fn_method_{obj_lower}_{method_name}")
}

/// Regular (non-FFI-buffer) function name for object free.
pub(super) fn fn_free(namespace: &str, obj_name: &str) -> String {
    let obj_lower = obj_name.to_ascii_lowercase();
    format!("uniffi_{namespace}_fn_free_{obj_lower}")
}

/// Regular (non-FFI-buffer) function name for object clone.
///
/// Cloning a handle is required before every method call because the FFI
/// scaffolding *consumes* handles: `try_lift(handle)` calls `Arc::from_raw`
/// without incrementing the reference count.  Without a preceding clone the
/// very first method call would decrement the ref-count to 0 and destroy the
/// underlying Rust object.
pub(super) fn fn_clone(namespace: &str, obj_name: &str) -> String {
    let obj_lower = obj_name.to_ascii_lowercase();
    format!("uniffi_{namespace}_fn_clone_{obj_lower}")
}

/// Function name for callback interface VTable initialization.
pub(super) fn fn_init_callback_vtable(namespace: &str, cb_name: &str) -> String {
    let cb_lower = cb_name.to_ascii_lowercase();
    format!("uniffi_{namespace}_fn_init_callback_vtable_{cb_lower}")
}

// ---------------------------------------------------------------------------
// Async / RustFuture function names
// ---------------------------------------------------------------------------

/// The FFI type suffix for `rust_future_*` functions, determined by the return type.
///
/// Objects (handles) use "u64" since handles are `u64` at the FFI level.
/// All compound types (String, Bytes, records, enums, etc.) use "rust_buffer".
/// Void return uses "void".
pub(super) fn rust_future_type_suffix(return_type: Option<&Type>) -> &'static str {
    match return_type {
        None => "void",
        Some(t) => match t {
            Type::Int8 => "i8",
            Type::UInt8 => "u8",
            Type::Int16 => "i16",
            Type::UInt16 => "u16",
            Type::Int32 => "i32",
            Type::UInt32 => "u32",
            Type::Int64 => "i64",
            Type::UInt64 => "u64",
            Type::Float32 => "f32",
            Type::Float64 => "f64",
            Type::Boolean => "i8",
            Type::Object { .. } | Type::CallbackInterface { .. } => "u64",
            Type::String
            | Type::Bytes
            | Type::Duration
            | Type::Timestamp
            | Type::Optional { .. }
            | Type::Sequence { .. }
            | Type::Map { .. }
            | Type::Record { .. }
            | Type::Enum { .. }
            | Type::Custom { .. } => "rust_buffer",
        },
    }
}

pub(super) fn rust_future_poll(namespace: &str, suffix: &str) -> String {
    format!("ffi_{namespace}_rust_future_poll_{suffix}")
}

pub(super) fn rust_future_complete(namespace: &str, suffix: &str) -> String {
    format!("ffi_{namespace}_rust_future_complete_{suffix}")
}

pub(super) fn rust_future_free(namespace: &str, suffix: &str) -> String {
    format!("ffi_{namespace}_rust_future_free_{suffix}")
}

/// Whether a `rust_future_complete_*` function returns the result via retptr.
///
/// On wasm32, `complete_rust_buffer` uses retptr (the RustBuffer is too large to
/// return directly): `(retptr: i32, handle: i64, status: i32) -> void`.
/// All primitive types and void return directly or have no return.
pub(super) fn rust_future_complete_uses_retptr(suffix: &str) -> bool {
    suffix == "rust_buffer"
}

// ---------------------------------------------------------------------------
// Code generation: lower (JS → FFI buffer)
// ---------------------------------------------------------------------------

/// Generate a TypeScript expression to read a return value from the FFI return buffer.
///
/// `offset_expr` is the pointer to the first return element.
pub(super) fn gen_read_return(t: &Type, offset_expr: &str) -> String {
    if is_rust_buffer_type(t) {
        gen_top_level_lift(t, offset_expr)
    } else {
        let read_fn = element_read_fn(t);
        let raw = format!("_rt.{read_fn}({offset_expr})");
        gen_from_ffi(&raw, t)
    }
}

// ---------------------------------------------------------------------------
// Top-level lower/lift (FfiConverter::lower / FfiConverter::lift)
// ---------------------------------------------------------------------------
//
// These are for top-level FFI arguments/returns. String and Bytes use raw
// data in the RustBuffer (no length prefix). All other compound types use
// the inner UniFFI binary serialization format.

/// Generate a TypeScript expression that lowers a top-level argument to a RustBufferDescriptor.
fn gen_top_level_lower(var: &str, t: &Type, namespace: &str) -> String {
    match t {
        // String has a custom lower/lift in UniFFI that uses raw UTF-8 in
        // the RustBuffer (no length prefix). All other compound types use
        // the standard serialized format via lower_into_rust_buffer().
        Type::String => format!("_rt.lowerString({var})"),
        Type::Custom { builtin, .. } => gen_top_level_lower(var, builtin, namespace),
        _ => {
            let lower_body = gen_lower_expr(var, t, namespace);
            format!("_rt.lowerIntoBuffer((w) => {{ {lower_body}; }})")
        }
    }
}

/// Generate a TypeScript expression that lifts a top-level return from FFI buffer elements.
///
/// `offset_expr` points to the first RustBuffer element in the return buffer.
fn gen_top_level_lift(t: &Type, offset_expr: &str) -> String {
    match t {
        // String has a custom lower/lift in UniFFI that uses raw UTF-8 in
        // the RustBuffer (no length prefix).
        Type::String => {
            format!("_rt.liftString(_rt.readRustBufferElements({offset_expr}))")
        }
        Type::Custom { builtin, .. } => gen_top_level_lift(builtin, offset_expr),
        _ => {
            let lift_body = gen_lift_expr("r", t);
            format!(
                "_rt.liftFromBuffer(_rt.readRustBufferElements({offset_expr}), (r) => {{ return {lift_body}; }})"
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Inner serialization (UniFFI binary format inside RustBuffer)
// ---------------------------------------------------------------------------

/// Generate TypeScript writer calls to serialize `var` into a UniFFIWriter `w`.
///
/// This is the core serializer. It generates inline code like:
///   `w.writeString(name)` for strings
///   `w.writeI32(1); w.writeString(v.field1); ...` for enums
///   etc.
fn gen_lower_expr(var: &str, t: &Type, namespace: &str) -> String {
    match t {
        Type::String => format!("w.writeString({var})"),
        Type::Bytes => format!("w.writeBytes({var})"),
        Type::Boolean => format!("w.writeBool({var})"),
        Type::Int8 => format!("w.writeI8({var})"),
        Type::UInt8 => format!("w.writeU8({var})"),
        Type::Int16 => format!("w.writeI16({var})"),
        Type::UInt16 => format!("w.writeU16({var})"),
        Type::Int32 => format!("w.writeI32({var})"),
        Type::UInt32 => format!("w.writeU32({var})"),
        Type::Int64 => format!("w.writeI64({var})"),
        Type::UInt64 => format!("w.writeU64({var})"),
        Type::Float32 => format!("w.writeF32({var})"),
        Type::Float64 => format!("w.writeF64({var})"),
        Type::Duration => format!("w.writeDuration({var})"),
        Type::Timestamp => format!("w.writeTimestamp({var})"),
        Type::Optional { inner_type } => {
            let inner_lower = gen_lower_expr("_v", inner_type, namespace);
            format!("w.writeOptional({var}, (_w, _v) => {{ {inner_lower}; }})")
        }
        Type::Sequence { inner_type } => {
            let inner_lower = gen_lower_expr("_v", inner_type, namespace);
            format!("w.writeSequence({var}, (_w, _v) => {{ {inner_lower}; }})")
        }
        Type::Map {
            key_type,
            value_type,
        } => {
            let key_lower = gen_lower_expr("_k", key_type, namespace);
            let val_lower = gen_lower_expr("_v", value_type, namespace);
            format!(
                "w.writeMap({var}, (_w, _k) => {{ {key_lower}; }}, (_w, _v) => {{ {val_lower}; }})"
            )
        }
        // Record, Enum, Custom, CallbackInterface — these need type-specific serialization
        // that will be generated as standalone helper functions. Use their name.
        Type::Record { name, .. } => {
            format!("_lower{name}(w, {var})")
        }
        Type::Enum { name, .. } => {
            format!("_lower{name}(w, {var})")
        }
        Type::Custom { builtin, .. } => {
            // Custom types: lower via the builtin type
            gen_lower_expr(var, builtin, namespace)
        }
        Type::Object { name, .. } => {
            // Objects inside compound types (Optional<Obj>, Sequence<Obj>, etc.)
            // must be cloned before lowering, matching Kotlin's FfiConverter.lower()
            // which always calls uniffiCloneHandle(). The scaffolding consumes handles
            // via Arc::from_raw, so without cloning, the JS-side object would be
            // left with a dangling/freed Rust allocation.
            let clone_fn = fn_clone(namespace, name);
            format!("w.writeU64(_rt.cloneObjectHandle('{clone_fn}', {var}._handle))")
        }
        Type::CallbackInterface { .. } => {
            // Callback interfaces are lowered as u64 handles via the handle map.
            // The VTable is already registered during module initialization.
            format!("w.writeU64(_rt.insertCallbackHandle({var}))")
        }
    }
}

/// Generate TypeScript reader calls to deserialize from a UniFFIReader `r`.
fn gen_lift_expr(reader_var: &str, t: &Type) -> String {
    match t {
        Type::String => format!("{reader_var}.readString()"),
        Type::Bytes => format!("{reader_var}.readBytes()"),
        Type::Boolean => format!("{reader_var}.readBool()"),
        Type::Int8 => format!("{reader_var}.readI8()"),
        Type::UInt8 => format!("{reader_var}.readU8()"),
        Type::Int16 => format!("{reader_var}.readI16()"),
        Type::UInt16 => format!("{reader_var}.readU16()"),
        Type::Int32 => format!("{reader_var}.readI32()"),
        Type::UInt32 => format!("{reader_var}.readU32()"),
        Type::Int64 => format!("{reader_var}.readI64()"),
        Type::UInt64 => format!("{reader_var}.readU64()"),
        Type::Float32 => format!("{reader_var}.readF32()"),
        Type::Float64 => format!("{reader_var}.readF64()"),
        Type::Duration => format!("{reader_var}.readDuration()"),
        Type::Timestamp => format!("{reader_var}.readTimestamp()"),
        Type::Optional { inner_type } => {
            let inner_lift = gen_lift_expr("_r", inner_type);
            format!("{reader_var}.readOptional((_r) => {inner_lift})")
        }
        Type::Sequence { inner_type } => {
            let inner_lift = gen_lift_expr("_r", inner_type);
            format!("{reader_var}.readSequence((_r) => {inner_lift})")
        }
        Type::Map {
            key_type,
            value_type,
        } => {
            let key_lift = gen_lift_expr("_r", key_type);
            let val_lift = gen_lift_expr("_r", value_type);
            format!("{reader_var}.readMap((_r) => {key_lift}, (_r) => {val_lift})")
        }
        Type::Record { name, .. } => {
            format!("_lift{name}({reader_var})")
        }
        Type::Enum { name, .. } => {
            format!("_lift{name}({reader_var})")
        }
        Type::Custom { builtin, .. } => {
            // Custom types: lift from the builtin, then apply custom lift if configured
            gen_lift_expr(reader_var, builtin)
        }
        Type::Object { name, .. } => {
            // Object inside a buffer = u64 handle → wrap in class
            format!("{name}._fromHandle({reader_var}.readU64())")
        }
        Type::CallbackInterface { .. } => {
            // Callback interfaces are handles — look up from handle map
            format!("_rt.getCallbackHandle({reader_var}.readU64())")
        }
    }
}

/// Convert a JS value to its FFI element representation (for primitive types).
fn gen_to_ffi(var: &str, t: &Type) -> String {
    match t {
        Type::Int64 | Type::UInt64 => format!("BigInt({var})"),
        // Callback interfaces are lowered to u64 handles via the handle map
        Type::CallbackInterface { .. } => format!("_rt.insertCallbackHandle({var})"),
        _ => var.to_string(),
    }
}

/// Convert an FFI element representation back to JS (for primitive types).
fn gen_from_ffi(raw: &str, _t: &Type) -> String {
    raw.to_string()
}

/// Convert a raw FFI value (from C ABI call, always a number) to JS.
/// Unlike gen_from_ffi, this handles Boolean→boolean conversion since
/// C ABI returns i32 (0/1) not JS boolean.
fn gen_from_ffi_raw(raw: &str, t: &Type) -> String {
    match t {
        Type::Boolean => format!("{raw} !== 0"),
        _ => raw.to_string(),
    }
}

/// TypeScript write function name on UniffiRuntime for a primitive type element.
fn element_write_fn(t: &Type) -> &'static str {
    match t {
        Type::Int8 => "writeI8Element",
        Type::UInt8 => "writeU8Element",
        Type::Int16 => "writeI16Element",
        Type::UInt16 => "writeU16Element",
        Type::Int32 => "writeI32Element",
        Type::UInt32 => "writeU32Element",
        Type::Int64 => "writeI64Element",
        Type::UInt64 => "writeU64Element",
        Type::Float32 => "writeF32Element",
        Type::Float64 => "writeF64Element",
        Type::Boolean => "writeBoolElement",
        Type::Object { .. } | Type::CallbackInterface { .. } => "writeHandleElement",
        _ => unreachable!("compound types don't use direct element writes"),
    }
}

/// TypeScript read function name on UniffiRuntime for a primitive type element.
fn element_read_fn(t: &Type) -> &'static str {
    match t {
        Type::Int8 => "readI8Element",
        Type::UInt8 => "readU8Element",
        Type::Int16 => "readI16Element",
        Type::UInt16 => "readU16Element",
        Type::Int32 => "readI32Element",
        Type::UInt32 => "readU32Element",
        Type::Int64 => "readI64Element",
        Type::UInt64 => "readU64Element",
        Type::Float32 => "readF32Element",
        Type::Float64 => "readF64Element",
        Type::Boolean => "readBoolElement",
        Type::Object { .. } | Type::CallbackInterface { .. } => "readHandleElement",
        _ => unreachable!("compound types don't use direct element reads"),
    }
}

// ---------------------------------------------------------------------------
// Type-specific lower/lift helper generation (for records and enums)
// ---------------------------------------------------------------------------

use super::types::{EnumDef, ErrorDef, RecordDef};

/// Generate a `_lowerFoo(w, value)` helper function for a record type.
pub(super) fn gen_record_lower_fn(r: &RecordDef, namespace: &str) -> String {
    let name = &r.name;
    let mut out = format!("function _lower{name}(w: UniFFIWriter, value: {name}): void {{\n");
    for f in &r.fields {
        let ts_field = safe_js_identifier(&camel_case(&f.name));
        let lower = gen_lower_expr(&format!("value.{ts_field}"), &f.type_, namespace);
        // Every field must always be serialized — default values affect the TS
        // signature (making the param optional) but not the binary format.
        out.push_str(&format!("  {lower};\n"));
    }
    out.push_str("}\n");
    out
}

/// Generate a `_liftFoo(r)` helper function for a record type.
pub(super) fn gen_record_lift_fn(r: &RecordDef) -> String {
    let name = &r.name;
    let mut out = format!("function _lift{name}(r: UniFFIReader): {name} {{\n  return {{\n");
    for f in &r.fields {
        let ts_field = safe_js_identifier(&camel_case(&f.name));
        let lift = gen_lift_expr("r", &f.type_);
        out.push_str(&format!("    {ts_field}: {lift},\n"));
    }
    out.push_str("  };\n}\n");
    out
}

/// Generate a `_lowerFoo(w, value)` helper for a flat enum (string literal union).
pub(super) fn gen_flat_enum_lower_fn(e: &EnumDef, _namespace: &str) -> String {
    let name = &e.name;
    let mut out = format!("function _lower{name}(w: UniFFIWriter, value: {name}): void {{\n");
    // Flat enums are serialized as i32 variant ordinal (1-based)
    for (i, v) in e.variants.iter().enumerate() {
        out.push_str(&format!(
            "  if (value === '{}') {{ w.writeI32({}); return; }}\n",
            v.name,
            i + 1
        ));
    }
    out.push_str(&format!(
        "  throw new Error(`Unknown {name} variant: ${{value}}`);\n"
    ));
    out.push_str("}\n");
    out
}

/// Generate a `_liftFoo(r)` helper for a flat enum.
pub(super) fn gen_flat_enum_lift_fn(e: &EnumDef) -> String {
    let name = &e.name;
    let mut out = format!("function _lift{name}(r: UniFFIReader): {name} {{\n");
    out.push_str("  const ordinal = r.readI32();\n");
    for (i, v) in e.variants.iter().enumerate() {
        out.push_str(&format!(
            "  if (ordinal === {}) return '{}';\n",
            i + 1,
            v.name
        ));
    }
    if e.is_non_exhaustive {
        out.push_str(&format!("  return `variant_${{ordinal}}` as {name};\n"));
    } else {
        out.push_str(&format!(
            "  throw new Error(`Unknown {name} ordinal: ${{ordinal}}`);\n"
        ));
    }
    out.push_str("}\n");
    out
}

/// Generate lower/lift helpers for a data enum (discriminated union).
pub(super) fn gen_data_enum_lower_fn(e: &EnumDef, namespace: &str) -> String {
    let name = &e.name;
    let mut out = format!("function _lower{name}(w: UniFFIWriter, value: {name}): void {{\n");
    for (i, v) in e.variants.iter().enumerate() {
        let tag = &v.name;
        out.push_str(&format!("  if (value.tag === '{tag}') {{\n"));
        out.push_str(&format!("    w.writeI32({});\n", i + 1));
        for f in &v.fields {
            let ts_field = safe_js_identifier(&camel_case(&f.name));
            let lower = gen_lower_expr(&format!("value.{ts_field}"), &f.type_, namespace);
            out.push_str(&format!("    {lower};\n"));
        }
        out.push_str("    return;\n  }\n");
    }
    out.push_str(&format!(
        "  throw new Error(`Unknown {name} variant: ${{(value as any).tag}}`);\n"
    ));
    out.push_str("}\n");
    out
}

pub(super) fn gen_data_enum_lift_fn(e: &EnumDef) -> String {
    let name = &e.name;
    let mut out = format!("function _lift{name}(r: UniFFIReader): {name} {{\n");
    out.push_str("  const ordinal = r.readI32();\n");
    for (i, v) in e.variants.iter().enumerate() {
        let tag = &v.name;
        out.push_str(&format!("  if (ordinal === {}) {{\n", i + 1));
        if v.fields.is_empty() {
            out.push_str(&format!("    return {{ tag: '{tag}' }};\n"));
        } else {
            let fields: Vec<String> = v
                .fields
                .iter()
                .map(|f| {
                    let ts_field = safe_js_identifier(&camel_case(&f.name));
                    let lift = gen_lift_expr("r", &f.type_);
                    format!("{ts_field}: {lift}")
                })
                .collect();
            out.push_str(&format!(
                "    return {{ tag: '{tag}', {} }};\n",
                fields.join(", ")
            ));
        }
        out.push_str("  }\n");
    }
    if e.is_non_exhaustive {
        out.push_str(&format!(
            "  return {{ tag: `variant_${{ordinal}}` }} as {name};\n"
        ));
    } else {
        out.push_str(&format!(
            "  throw new Error(`Unknown {name} ordinal: ${{ordinal}}`);\n"
        ));
    }
    out.push_str("}\n");
    out
}

/// Generate a lift function for a flat error type (from RustCallStatus error buffer).
pub(super) fn gen_flat_error_lift_fn(e: &ErrorDef) -> String {
    let name = &e.name;
    let mut out = format!("function _liftError{name}(rb: any): {name} {{\n");
    out.push_str("  return _rt.liftFromBuffer(rb, (r) => {\n");
    out.push_str("    const ordinal = r.readI32();\n");
    for (i, v) in e.variants.iter().enumerate() {
        out.push_str(&format!(
            "    if (ordinal === {}) return new {name}('{}');\n",
            i + 1,
            v.name
        ));
    }
    if e.is_non_exhaustive {
        out.push_str(&format!(
            "    return new {name}(`variant_${{ordinal}}` as any);\n"
        ));
    } else {
        out.push_str(&format!(
            "    throw new Error(`Unknown {name} ordinal: ${{ordinal}}`);\n"
        ));
    }
    out.push_str("  });\n");
    out.push_str("}\n");
    out
}

/// Generate a lift function for a rich error type.
pub(super) fn gen_rich_error_lift_fn(e: &ErrorDef) -> String {
    let name = &e.name;
    let variant_type = format!("{name}Variant");
    let mut out = format!("function _liftError{name}(rb: any): {name} {{\n");
    out.push_str("  return _rt.liftFromBuffer(rb, (r) => {\n");
    out.push_str("    const ordinal = r.readI32();\n");
    for (i, v) in e.variants.iter().enumerate() {
        let tag = &v.name;
        out.push_str(&format!("    if (ordinal === {}) {{\n", i + 1));
        if v.fields.is_empty() {
            out.push_str(&format!("      return new {name}({{ tag: '{tag}' }});\n"));
        } else {
            let fields: Vec<String> = v
                .fields
                .iter()
                .map(|f| {
                    let ts_field = safe_js_identifier(&camel_case(&f.name));
                    let lift = gen_lift_expr("r", &f.type_);
                    format!("{ts_field}: {lift}")
                })
                .collect();
            out.push_str(&format!(
                "      return new {name}({{ tag: '{tag}', {} }});\n",
                fields.join(", ")
            ));
        }
        out.push_str("    }\n");
    }
    if e.is_non_exhaustive {
        out.push_str(&format!(
            "    return new {name}({{ tag: `variant_${{ordinal}}` }} as {variant_type});\n"
        ));
    } else {
        out.push_str(&format!(
            "    throw new Error(`Unknown {name} ordinal: ${{ordinal}}`);\n"
        ));
    }
    out.push_str("  });\n");
    out.push_str("}\n");
    out
}

// ---------------------------------------------------------------------------
// FFI call generation
// ---------------------------------------------------------------------------

// (FFI call generation uses Type directly, not our own ArgDef/FnDef)

/// Generate a complete FFI call block for a function/method.
///
/// This produces a multi-line block that:
/// 1. Saves scratch position
/// 2. Lowers arguments to FFI buffer
/// 3. Allocates return buffer
/// 4. Calls the WASM export
/// 5. Checks call status
/// 6. Lifts return value
/// 7. Resets scratch
///
/// `ffi_name` is the full WASM export name.
/// `indent` is the base indentation (e.g. "    " for method bodies).
pub(super) fn gen_ffi_call(
    ffi_name: &str,
    namespace: &str,
    args: &[(&str, &Type)], // (js_var_name, type)
    return_type: Option<&Type>,
    throws_name: Option<&str>,
    indent: &str,
) -> String {
    let mut lines = Vec::new();

    // Calculate element counts
    let arg_elements: usize = args.iter().map(|(_, t)| element_count(t)).sum();
    let ret_elements = return_type.map_or(0, element_count);
    let total_ret_elements = ret_elements + CALL_STATUS_ELEMENTS;

    // Setup: lower RustBuffer args before allocating the element buffer.
    // String and Bytes use raw data (FfiConverter::lower), everything else uses
    // UniFFI binary serialization (lower_into_rust_buffer).
    let mut rb_setups = Vec::new();
    for (var, t) in args {
        if is_rust_buffer_type(t) {
            let rb_var = format!("_rb_{}", var.replace('.', "_"));
            let lower = gen_top_level_lower(var, t, namespace);
            rb_setups.push(format!("{indent}const {rb_var} = {lower};"));
        }
    }
    lines.extend(rb_setups);

    // Allocate arg buffer
    if arg_elements > 0 {
        lines.push(format!(
            "{indent}const _argPtr = _rt.scratchAlloc({} * 8);",
            arg_elements
        ));
    } else {
        // No arguments — pass a placeholder pointer (WASM reads 0 elements)
        lines.push(format!("{indent}const _argPtr = 0;"));
    }

    // Write args to buffer
    let mut offset = 0;
    for (var, t) in args {
        let offset_expr = if offset == 0 {
            "_argPtr".to_string()
        } else {
            format!("_argPtr + {}", offset * 8)
        };

        if is_rust_buffer_type(t) {
            let rb_var = format!("_rb_{}", var.replace('.', "_"));
            lines.push(format!(
                "{indent}_rt.writeRustBufferElements({offset_expr}, {rb_var});"
            ));
        } else {
            let write_fn = element_write_fn(t);
            let value = gen_to_ffi(var, t);
            lines.push(format!("{indent}_rt.{write_fn}({offset_expr}, {value});"));
        }
        offset += element_count(t);
    }

    // Allocate return buffer
    lines.push(format!(
        "{indent}const _retPtr = _rt.scratchAlloc({} * 8);",
        total_ret_elements
    ));

    // Call
    lines.push(format!("{indent}_rt.call('{ffi_name}', _argPtr, _retPtr);"));

    // Check call status
    let status_offset = if ret_elements > 0 {
        format!("_retPtr + {}", ret_elements * 8)
    } else {
        "_retPtr".to_string()
    };

    if let Some(err_name) = throws_name {
        lines.push(format!(
            "{indent}_rt.checkCallStatus({status_offset}, (rb) => _liftError{err_name}(rb));"
        ));
    } else {
        lines.push(format!("{indent}_rt.checkCallStatus({status_offset});"));
    }

    // Read return value
    if let Some(ret_type) = return_type {
        let result = gen_read_return(ret_type, "_retPtr");
        lines.push(format!("{indent}const _result = {result};"));
        lines.push(format!("{indent}_rt.scratchReset();"));
        lines.push(format!("{indent}return _result;"));
    } else {
        lines.push(format!("{indent}_rt.scratchReset();"));
    }

    lines.join("\n")
}

/// Generate an async FFI call body.
///
/// The async protocol is:
/// 1. Lower args and call FFI buffer function → get a RustFuture Handle
/// 2. Poll the future to readiness via `_rt.pollToReady()`
/// 3. Call `rust_future_complete_*` (C ABI) to extract the result
/// 4. Free the future via `rust_future_free_*` in a finally block
/// 5. Lift the return value
///
/// `ffi_name` is the FFI buffer function name (returns a Handle).
/// `namespace` is used to construct `ffi_{ns}_rust_future_*` names.
pub(super) fn gen_async_ffi_call(
    ffi_name: &str,
    namespace: &str,
    args: &[(&str, &Type)],
    return_type: Option<&Type>,
    throws_name: Option<&str>,
    indent: &str,
) -> String {
    let mut lines = Vec::new();
    let suffix = rust_future_type_suffix(return_type);
    let poll_fn = rust_future_poll(namespace, suffix);
    let complete_fn = rust_future_complete(namespace, suffix);
    let free_fn = rust_future_free(namespace, suffix);
    let uses_retptr = rust_future_complete_uses_retptr(suffix);

    // --- Step 1: Lower args + call FFI buffer to get future handle ---

    let arg_elements: usize = args.iter().map(|(_, t)| element_count(t)).sum();

    // Lower RustBuffer args
    for (var, t) in args {
        if is_rust_buffer_type(t) {
            let rb_var = format!("_rb_{}", var.replace('.', "_"));
            let lower = gen_top_level_lower(var, t, namespace);
            lines.push(format!("{indent}const {rb_var} = {lower};"));
        }
    }

    // Allocate arg buffer
    if arg_elements > 0 {
        lines.push(format!(
            "{indent}const _argPtr = _rt.scratchAlloc({} * 8);",
            arg_elements
        ));
    } else {
        lines.push(format!("{indent}const _argPtr = 0;"));
    }

    // Write args
    let mut offset = 0;
    for (var, t) in args {
        let offset_expr = if offset == 0 {
            "_argPtr".to_string()
        } else {
            format!("_argPtr + {}", offset * 8)
        };
        if is_rust_buffer_type(t) {
            let rb_var = format!("_rb_{}", var.replace('.', "_"));
            lines.push(format!(
                "{indent}_rt.writeRustBufferElements({offset_expr}, {rb_var});"
            ));
        } else {
            let write_fn = element_write_fn(t);
            let value = gen_to_ffi(var, t);
            lines.push(format!("{indent}_rt.{write_fn}({offset_expr}, {value});"));
        }
        offset += element_count(t);
    }

    // Return buffer: 1 element for the Handle
    lines.push(format!("{indent}const _retPtr = _rt.scratchAlloc(1 * 8);"));

    // Call FFI buffer function to create the future
    lines.push(format!("{indent}_rt.call('{ffi_name}', _argPtr, _retPtr);"));
    lines.push(format!(
        "{indent}const _futureHandle = _rt.readHandleElement(_retPtr);"
    ));
    // NOTE: Do NOT scratchReset() here. The await below may yield, and a
    // concurrent async call could reuse scratch in the meantime. Instead,
    // reset after resuming (the complete phase is fully synchronous).

    // --- Step 2-5: Poll, complete, free, lift ---
    lines.push(format!("{indent}try {{"));

    // Poll to readiness (may yield to the event loop)
    lines.push(format!(
        "{indent}  await _rt.pollToReady(_futureHandle, '{poll_fn}');"
    ));
    // Safe to reset scratch here: JS is single-threaded, and we have
    // exclusive synchronous control until the next await/return.
    lines.push(format!("{indent}  _rt.scratchReset();"));

    // Complete: call rust_future_complete_*
    if uses_retptr {
        // rust_buffer: (retptr: i32, handle: i64, status: i32) -> void
        lines.push(format!(
            "{indent}  const _rbRetPtr = _rt.scratchAlloc({RUST_BUFFER_STRUCT_SIZE});"
        ));
        lines.push(format!(
            "{indent}  const _statusPtr = _rt.scratchAlloc({RUST_CALL_STATUS_STRUCT_SIZE});"
        ));
        lines.push(format!(
            "{indent}  _rt._writeRustCallStatusStruct(_statusPtr);"
        ));
        lines.push(format!(
            "{indent}  (_rt.getExport('{complete_fn}') as any)(_rbRetPtr, _futureHandle, _statusPtr);"
        ));
    } else if suffix == "void" {
        // void: (handle: i64, status: i32) -> void
        lines.push(format!(
            "{indent}  const _statusPtr = _rt.scratchAlloc({RUST_CALL_STATUS_STRUCT_SIZE});"
        ));
        lines.push(format!(
            "{indent}  _rt._writeRustCallStatusStruct(_statusPtr);"
        ));
        lines.push(format!(
            "{indent}  (_rt.getExport('{complete_fn}') as any)(_futureHandle, _statusPtr);"
        ));
    } else {
        // Primitives: (handle: i64, status: i32) -> T
        lines.push(format!(
            "{indent}  const _statusPtr = _rt.scratchAlloc({RUST_CALL_STATUS_STRUCT_SIZE});"
        ));
        lines.push(format!(
            "{indent}  _rt._writeRustCallStatusStruct(_statusPtr);"
        ));
        lines.push(format!(
            "{indent}  const _result = (_rt.getExport('{complete_fn}') as any)(_futureHandle, _statusPtr);"
        ));
    }

    // Check status (error lifting)
    let status_check = if let Some(err_name) = throws_name {
        format!("{indent}  _rt.checkCallStatus(_statusPtr, (rb) => _liftError{err_name}(rb));",)
    } else {
        format!("{indent}  _rt.checkCallStatus(_statusPtr);")
    };

    // Wait — checkCallStatus works with FFI buffer elements. But _statusPtr points to a
    // RustCallStatus *C struct* (32 bytes), not FFI elements. They happen to have
    // the same layout: [code(i8/8 bytes), rb_cap(u64), rb_len(u64), rb_data(ptr)].
    // Actually no — the C struct has code at offset 0 (1 byte + 7 padding), then
    // RustBuffer at offset 8. The FFI elements have code at element[0] (i8 at first
    // byte) and RustBuffer at elements[1..3]. The byte layout is identical because
    // ELEMENT_SIZE = 8, so element[0] starts at offset 0 and element[1] at offset 8.
    // So yes, checkCallStatus can read the C struct directly!
    lines.push(status_check);

    // Read and return result
    if uses_retptr {
        // Read RustBuffer from the retptr C struct
        let rb = "_rt._readRustBufferStruct(_rbRetPtr)";
        if let Some(ret_type) = return_type {
            let lift = gen_top_level_lift_from_rb(ret_type, rb);
            lines.push(format!("{indent}  const _result = {lift};"));
            lines.push(format!("{indent}  _rt.scratchReset();"));
            lines.push(format!("{indent}  return _result;"));
        } else {
            lines.push(format!("{indent}  _rt.scratchReset();"));
        }
    } else if suffix == "void" {
        lines.push(format!("{indent}  _rt.scratchReset();"));
    } else {
        // Primitive or handle return (from C ABI, always a number/bigint)
        if let Some(ret_type) = return_type {
            let raw = gen_from_ffi_raw("_result", ret_type);
            lines.push(format!("{indent}  _rt.scratchReset();"));
            lines.push(format!("{indent}  return {raw};"));
        } else {
            lines.push(format!("{indent}  _rt.scratchReset();"));
        }
    }

    // Finally: free the future
    lines.push(format!("{indent}}} finally {{"));
    lines.push(format!(
        "{indent}  (_rt.getExport('{free_fn}') as any)(_futureHandle);"
    ));
    lines.push(format!("{indent}}}"));

    lines.join("\n")
}

/// Size constants used in generated code.
const RUST_BUFFER_STRUCT_SIZE: usize = 24;
const RUST_CALL_STATUS_STRUCT_SIZE: usize = 32;

/// Lift a value from a `RustBufferDescriptor` expression (already read from C struct).
/// Used for `rust_future_complete_rust_buffer` returns.
fn gen_top_level_lift_from_rb(t: &Type, rb_expr: &str) -> String {
    match t {
        Type::String => format!("_rt.liftString({rb_expr})"),
        Type::Custom { builtin, .. } => gen_top_level_lift_from_rb(builtin, rb_expr),
        _ => {
            let lift_body = gen_lift_expr("r", t);
            format!("_rt.liftFromBuffer({rb_expr}, (r) => {{ return {lift_body}; }})")
        }
    }
}

// ---------------------------------------------------------------------------
// Callback interface VTable generation
// ---------------------------------------------------------------------------

use super::types::CallbackInterfaceDef;

/// WASM type string for a UniFFI Type used in callback method signatures.
///
/// On wasm32, compound types passed through RustBuffer use pointers (i32).
/// Primitives and handles use their native WASM types.
fn wasm_type_str(t: &Type) -> &'static str {
    match t {
        Type::Int8
        | Type::UInt8
        | Type::Int16
        | Type::UInt16
        | Type::Int32
        | Type::UInt32
        | Type::Boolean => "i32",
        Type::Int64 | Type::UInt64 => "i64",
        Type::Float32 => "f32",
        Type::Float64 => "f64",
        Type::Object { .. } | Type::CallbackInterface { .. } => "i64",
        // All compound types are passed by pointer (i32) in callback VTable methods
        Type::String
        | Type::Bytes
        | Type::Duration
        | Type::Timestamp
        | Type::Optional { .. }
        | Type::Sequence { .. }
        | Type::Map { .. }
        | Type::Record { .. }
        | Type::Enum { .. }
        | Type::Custom { .. } => "i32",
    }
}

/// Whether a type is passed by pointer (as RustBuffer) in a callback VTable method.
fn is_callback_ptr_type(t: &Type) -> bool {
    matches!(
        t,
        Type::String
            | Type::Bytes
            | Type::Duration
            | Type::Timestamp
            | Type::Optional { .. }
            | Type::Sequence { .. }
            | Type::Map { .. }
            | Type::Record { .. }
            | Type::Enum { .. }
            | Type::Custom { .. }
    )
}

/// Generate the JS code that reads a callback method argument from a pointer in WASM memory.
///
/// For compound types, the argument is a pointer to a RustBuffer C struct in WASM memory.
/// We read the RustBuffer, then lift the value from it.
fn gen_callback_arg_lift(var: &str, t: &Type) -> String {
    match t {
        Type::String => {
            format!(
                "(() => {{ const _rb = _rt._readRustBufferStruct({var}); return _rt._readUtf8(_rb.dataPtr, _rb.len); }})()"
            )
        }
        _ if is_callback_ptr_type(t) => {
            let lift_body = gen_lift_expr("_r", t);
            format!(
                "_rt.liftFromBuffer(_rt._readRustBufferStruct({var}), (_r) => {{ return {lift_body}; }})"
            )
        }
        Type::Boolean => format!("{var} !== 0"),
        _ => var.to_string(),
    }
}

/// Generate the JS code that writes a callback method return value back to Rust.
///
/// For compound types, we need to lower the value into a RustBuffer and write it
/// to the output pointer as a RustBuffer C struct.
fn gen_callback_ret_lower(result_var: &str, out_ptr: &str, t: &Type, namespace: &str) -> String {
    match t {
        Type::String => {
            format!(
                "const _retRb = _rt.lowerString({result_var}); _rt._writeRustBufferStruct({out_ptr}, _retRb);"
            )
        }
        _ if is_callback_ptr_type(t) => {
            let lower_body = gen_lower_expr(result_var, t, namespace);
            format!(
                "const _retRb = _rt.lowerIntoBuffer((w) => {{ {lower_body}; }}); _rt._writeRustBufferStruct({out_ptr}, _retRb);"
            )
        }
        // Primitives: write directly to the output pointer
        Type::Boolean => {
            format!("_rt._dv().setInt8({out_ptr}, {result_var} ? 1 : 0);")
        }
        Type::Int8 => format!("_rt._dv().setInt8({out_ptr}, {result_var});"),
        Type::UInt8 => format!("_rt._dv().setUint8({out_ptr}, {result_var});"),
        Type::Int16 => format!("_rt._dv().setInt16({out_ptr}, {result_var}, true);"),
        Type::UInt16 => format!("_rt._dv().setUint16({out_ptr}, {result_var}, true);"),
        Type::Int32 => format!("_rt._dv().setInt32({out_ptr}, {result_var}, true);"),
        Type::UInt32 => format!("_rt._dv().setUint32({out_ptr}, {result_var}, true);"),
        Type::Int64 => format!("_rt._dv().setBigInt64({out_ptr}, {result_var}, true);"),
        Type::UInt64 => format!("_rt._dv().setBigUint64({out_ptr}, {result_var}, true);"),
        Type::Float32 => format!("_rt._dv().setFloat32({out_ptr}, {result_var}, true);"),
        Type::Float64 => format!("_rt._dv().setFloat64({out_ptr}, {result_var}, true);"),
        Type::Object { .. } | Type::CallbackInterface { .. } => {
            format!("_rt._dv().setBigUint64({out_ptr}, {result_var}, true);")
        }
        _ => "/* unsupported callback return type */".to_string(),
    }
}

/// Generate the VTable registration code for a callback interface.
///
/// This produces a block of TypeScript that:
/// 1. Creates trampoline functions for each VTable entry (uniffi_free, uniffi_clone, methods)
/// 2. Adds them to the WASM indirect function table
/// 3. Writes the VTable struct to persistent memory
/// 4. Calls the VTable init function
pub(super) fn gen_callback_vtable_registration(
    cb: &CallbackInterfaceDef,
    namespace: &str,
) -> String {
    let mut out = String::new();
    let cb_name = &cb.name;
    out.push_str(&format!(
        "// --- VTable for callback interface {cb_name} ---\n"
    ));

    // For each method, determine its WASM signature
    // VTable method signature: (handle: i64, ...args_as_ptrs..., out_return: i32, call_status: i32) -> void
    // For void-returning methods: (handle: i64, ...args_as_ptrs..., call_status: i32) -> void

    // Generate trampoline functions
    // uniffi_free: (handle: i64) -> void
    out.push_str(&format!(
        "_rt.registerCallbackVTable('{cb_name}', '{init_fn}', [\n",
        init_fn = fn_init_callback_vtable(namespace, cb_name),
    ));

    // Entry 0: uniffi_free
    out.push_str("  {\n");
    out.push_str("    params: ['i64'], results: [],\n");
    out.push_str("    fn: (handle: bigint) => { _rt.removeCallbackHandle(handle); },\n");
    out.push_str("  },\n");

    // Entry 1: uniffi_clone
    out.push_str("  {\n");
    out.push_str("    params: ['i64'], results: ['i64'],\n");
    out.push_str("    fn: (handle: bigint) => { return _rt.cloneCallbackHandle(handle); },\n");
    out.push_str("  },\n");

    // Entries 2+: methods
    for m in &cb.methods {
        let method_name = &m.name;
        let ts_method = safe_js_identifier(&camel_case(method_name));

        // Build WASM param types: i64 (handle) + per-arg types + out_return (i32, if non-void) + call_status (i32)
        let mut wasm_params = vec!["'i64'".to_string()]; // handle
        for arg in &m.args {
            wasm_params.push(format!("'{}'", wasm_type_str(&arg.type_)));
        }
        let has_return = m.return_type.is_some();
        if has_return {
            wasm_params.push("'i32'".to_string()); // out_return pointer
        }
        wasm_params.push("'i32'".to_string()); // call_status pointer

        out.push_str("  {\n");
        out.push_str(&format!(
            "    params: [{}], results: [],\n",
            wasm_params.join(", ")
        ));

        // Build trampoline function signature
        let mut param_names = vec!["_handle: bigint".to_string()];
        for (i, arg) in m.args.iter().enumerate() {
            let wt = wasm_type_str(&arg.type_);
            let ts_type = if wt == "i64" { "bigint" } else { "number" };
            param_names.push(format!("_arg{i}: {ts_type}"));
        }
        if has_return {
            param_names.push("_outPtr: number".to_string());
        }
        param_names.push("_statusPtr: number".to_string());

        out.push_str(&format!("    fn: ({}) => {{\n", param_names.join(", ")));
        // Save scratch offset — callbacks run DURING a WASM call, so the outer
        // call's scratch data must be preserved.
        out.push_str("      const _savedScratch = _rt.scratchSave();\n");
        out.push_str("      try {\n");
        out.push_str("        const _obj = _rt.getCallbackHandle(_handle) as any;\n");

        // Lift each argument
        let mut call_args = Vec::new();
        for (i, arg) in m.args.iter().enumerate() {
            let arg_var = format!("_arg{i}");
            let lifted = gen_callback_arg_lift(&arg_var, &arg.type_);
            let lifted_var = format!("_lifted{i}");
            out.push_str(&format!("        const {lifted_var} = {lifted};\n"));
            call_args.push(lifted_var);
        }

        // Call the JS method
        let call_expr = format!("_obj.{ts_method}({})", call_args.join(", "));
        if has_return {
            out.push_str(&format!("        const _result = {call_expr};\n"));
            let ret_type = m.return_type.as_ref().unwrap();
            let lower_code = gen_callback_ret_lower("_result", "_outPtr", ret_type, namespace);
            out.push_str(&format!("        {lower_code}\n"));
        } else {
            out.push_str(&format!("        {call_expr};\n"));
        }

        // Write success status
        out.push_str("        _rt._writeCallStatusSuccess(_statusPtr);\n");
        out.push_str("      } catch (_e) {\n");
        out.push_str("        _rt._writeCallStatusPanic(_statusPtr, _e);\n");
        out.push_str("      } finally {\n");
        out.push_str("        _rt.scratchRestore(_savedScratch);\n");
        out.push_str("      }\n");
        out.push_str("    },\n");
        out.push_str("  },\n");
    }

    out.push_str("]);\n");
    out
}
