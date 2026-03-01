use wasm_bindgen::prelude::*;

/// Returns a normalised URL (lowercased, trailing slash removed).
/// The UDL custom type `Url` is `typedef string`; the WASM boundary sees a plain string.
#[wasm_bindgen]
pub fn normalize_url(url: &str) -> String {
    url.to_lowercase().trim_end_matches('/').to_string()
}

/// Derives a handle value from a seed.
/// The UDL custom type `Handle` is `typedef i64`; the WASM boundary sees a plain i64.
#[wasm_bindgen]
pub fn make_handle(seed: i64) -> i64 {
    seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
}

/// Returns true if two handles are equal.
#[wasm_bindgen]
pub fn handles_equal(a: i64, b: i64) -> bool {
    a == b
}
