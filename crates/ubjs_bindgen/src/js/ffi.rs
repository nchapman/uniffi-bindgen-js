// ---------------------------------------------------------------------------
// FFI metadata: function naming, element counts, type mapping
// ---------------------------------------------------------------------------
//
// This module provides helpers for:
// 1. Computing FFI buffer element counts for each type
// 2. Constructing FFI export names (uniffi_ffibuffer_*, ffi_*_rustbuffer_*, etc.)
// 3. Generating inline TypeScript code to lower/lift values for FFI calls

use uniffi_bindgen::interface::Type;

use super::naming::{camel_case, safe_js_identifier, snake_case};

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
        | Type::CallbackInterface { .. }
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
pub(super) fn ffibuf_fn_constructor(namespace: &str, obj_name: &str, ctor_name: &str) -> String {
    let obj_snake = snake_case(obj_name);
    format!("uniffi_ffibuffer_{namespace}_fn_constructor_{obj_snake}_{ctor_name}")
}

/// FFI buffer function name for a method.
pub(super) fn ffibuf_fn_method(namespace: &str, obj_name: &str, method_name: &str) -> String {
    let obj_snake = snake_case(obj_name);
    format!("uniffi_ffibuffer_{namespace}_fn_method_{obj_snake}_{method_name}")
}

/// Regular (non-FFI-buffer) function name for object free.
pub(super) fn fn_free(namespace: &str, obj_name: &str) -> String {
    let obj_snake = snake_case(obj_name);
    format!("uniffi_{namespace}_fn_free_{obj_snake}")
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
fn gen_top_level_lower(var: &str, t: &Type) -> String {
    match t {
        Type::String => format!("_rt.lowerString({var})"),
        Type::Bytes => format!("_rt.lowerBytes({var})"),
        Type::Custom { builtin, .. } => gen_top_level_lower(var, builtin),
        _ => {
            // Compound types: use UniFFI binary serialization
            let lower_body = gen_lower_expr(var, t);
            format!("_rt.lowerIntoBuffer((w) => {{ {lower_body}; }})")
        }
    }
}

/// Generate a TypeScript expression that lifts a top-level return from FFI buffer elements.
///
/// `offset_expr` points to the first RustBuffer element in the return buffer.
fn gen_top_level_lift(t: &Type, offset_expr: &str) -> String {
    match t {
        Type::String => {
            format!("_rt.liftString(_rt.readRustBufferElements({offset_expr}))")
        }
        Type::Bytes => {
            format!("_rt.liftBytes(_rt.readRustBufferElements({offset_expr}))")
        }
        Type::Custom { builtin, .. } => gen_top_level_lift(builtin, offset_expr),
        _ => {
            // Compound types: use UniFFI binary deserialization
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
fn gen_lower_expr(var: &str, t: &Type) -> String {
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
            let inner_lower = gen_lower_expr("_v", inner_type);
            format!("w.writeOptional({var}, (_w, _v) => {{ {inner_lower}; }})")
        }
        Type::Sequence { inner_type } => {
            let inner_lower = gen_lower_expr("_v", inner_type);
            format!("w.writeSequence({var}, (_w, _v) => {{ {inner_lower}; }})")
        }
        Type::Map {
            key_type,
            value_type,
        } => {
            let key_lower = gen_lower_expr("_k", key_type);
            let val_lower = gen_lower_expr("_v", value_type);
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
            gen_lower_expr(var, builtin)
        }
        Type::Object { .. } => {
            // Objects are handles, not serialized. This path shouldn't be reached
            // for FFI buffer element writes, but if an Object appears inside a
            // RustBuffer (e.g. Optional<Object>), it's lowered as a u64 handle.
            format!("w.writeU64({var}._handle)")
        }
        Type::CallbackInterface { .. } => {
            // Callback interfaces require vtable registration (Phase 4)
            "/* TODO: callback interface lowering */".to_string()
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
        Type::CallbackInterface { .. } => "/* TODO: callback interface lifting */".to_string(),
    }
}

/// Convert a JS value to its FFI element representation (for primitive types).
fn gen_to_ffi(var: &str, t: &Type) -> String {
    match t {
        Type::Int64 | Type::UInt64 => format!("BigInt({var})"),
        _ => var.to_string(),
    }
}

/// Convert an FFI element representation back to JS (for primitive types).
fn gen_from_ffi(raw: &str, _t: &Type) -> String {
    raw.to_string()
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
        Type::Object { .. } => "writeHandleElement",
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
        Type::Object { .. } => "readHandleElement",
        _ => unreachable!("compound types don't use direct element reads"),
    }
}

// ---------------------------------------------------------------------------
// Type-specific lower/lift helper generation (for records and enums)
// ---------------------------------------------------------------------------

use super::types::{UdlEnum, UdlError, UdlRecord};

/// Generate a `_lowerFoo(w, value)` helper function for a record type.
pub(super) fn gen_record_lower_fn(r: &UdlRecord) -> String {
    let name = &r.name;
    let mut out = format!("function _lower{name}(w: UniFFIWriter, value: {name}): void {{\n");
    for f in &r.fields {
        let ts_field = safe_js_identifier(&camel_case(&f.name));
        let lower = gen_lower_expr(&format!("value.{ts_field}"), &f.type_);
        // Every field must always be serialized — default values affect the TS
        // signature (making the param optional) but not the binary format.
        out.push_str(&format!("  {lower};\n"));
    }
    out.push_str("}\n");
    out
}

/// Generate a `_liftFoo(r)` helper function for a record type.
pub(super) fn gen_record_lift_fn(r: &UdlRecord) -> String {
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
pub(super) fn gen_flat_enum_lower_fn(e: &UdlEnum) -> String {
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
pub(super) fn gen_flat_enum_lift_fn(e: &UdlEnum) -> String {
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
pub(super) fn gen_data_enum_lower_fn(e: &UdlEnum) -> String {
    let name = &e.name;
    let mut out = format!("function _lower{name}(w: UniFFIWriter, value: {name}): void {{\n");
    for (i, v) in e.variants.iter().enumerate() {
        let tag = &v.name;
        out.push_str(&format!("  if (value.tag === '{tag}') {{\n"));
        out.push_str(&format!("    w.writeI32({});\n", i + 1));
        for f in &v.fields {
            let ts_field = safe_js_identifier(&camel_case(&f.name));
            let lower = gen_lower_expr(&format!("value.{ts_field}"), &f.type_);
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

pub(super) fn gen_data_enum_lift_fn(e: &UdlEnum) -> String {
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
pub(super) fn gen_flat_error_lift_fn(e: &UdlError) -> String {
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
pub(super) fn gen_rich_error_lift_fn(e: &UdlError) -> String {
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

// (FFI call generation uses Type directly, not our own UdlArg/UdlFunction)

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
            let lower = gen_top_level_lower(var, t);
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
