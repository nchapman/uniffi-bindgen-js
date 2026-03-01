//! Manual UniFFI ABI implementation for the simple_fns fixture.
//!
//! Implements the canonical UniFFI C ABI so that the generated TypeScript
//! bindings can call these functions through koffi.  Strings are carried
//! as RustBuffer (capacity/len/data pointer) and errors are reported via
//! the *mut RustCallStatus out-parameter.

#![allow(clippy::missing_safety_doc)]

use std::mem::MaybeUninit;

// ---------------------------------------------------------------------------
// ABI types (mirror of uniffi_core layout)
// ---------------------------------------------------------------------------

/// `RustBuffer` carries opaque byte blobs between Rust and the host language.
/// For strings the bytes are valid UTF-8.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RustBuffer {
    pub capacity: u64,
    pub len: u64,
    pub data: *mut u8,
}

/// `RustCallStatus` is the error-channel out-parameter every UniFFI call
/// receives.  The host writes the code; on error it also fills `error_buf`.
#[repr(C)]
pub struct RustCallStatus {
    pub code: i8,
    pub error_buf: MaybeUninit<RustBuffer>,
}

const CALL_SUCCESS: i8 = 0;
#[allow(dead_code)] // used in Phase 2+ (error handling)
const CALL_UNEXPECTED_ERROR: i8 = 2;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn rust_buffer_from_string(s: String) -> RustBuffer {
    let mut bytes = s.into_bytes();
    bytes.shrink_to_fit();
    let capacity = bytes.capacity() as u64;
    let len = bytes.len() as u64;
    let data = bytes.as_mut_ptr();
    std::mem::forget(bytes);
    RustBuffer { capacity, len, data }
}

fn string_from_rust_buffer(buf: RustBuffer) -> String {
    if buf.data.is_null() || buf.len == 0 {
        return String::new();
    }
    let bytes = unsafe { std::slice::from_raw_parts(buf.data, buf.len as usize) };
    String::from_utf8_lossy(bytes).into_owned()
}

// ---------------------------------------------------------------------------
// Exported UniFFI ABI functions
// ---------------------------------------------------------------------------

/// `greet(name: String) -> String`
///
/// Returns `"hello, {name}"`.  Demonstrates the canonical RustBuffer string
/// ABI that the generated TypeScript uses.
///
/// # Safety
/// `call_status` must be a valid non-null pointer to a `RustCallStatus`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uniffi_simple_fns_fn_func_greet(
    name: RustBuffer,
    call_status: *mut RustCallStatus,
) -> RustBuffer {
    let name_str = string_from_rust_buffer(name);

    // Free the caller-allocated input buffer.
    if !name.data.is_null() && name.capacity > 0 {
        drop(Vec::from_raw_parts(
            name.data,
            name.len as usize,
            name.capacity as usize,
        ));
    }

    let result = format!("hello, {name_str}");
    let out = rust_buffer_from_string(result);

    (*call_status).code = CALL_SUCCESS;
    out
}

/// Free a `RustBuffer` that was returned by this library.
///
/// The host language must call this after lifting the value to avoid leaking
/// Rust-owned memory.
///
/// # Safety
/// `buf.data` must have been allocated by this library (or be null).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ffi_simple_fns_rustbuffer_free(buf: RustBuffer) {
    if !buf.data.is_null() && buf.capacity > 0 {
        drop(Vec::from_raw_parts(
            buf.data,
            buf.len as usize,
            buf.capacity as usize,
        ));
    }
}
