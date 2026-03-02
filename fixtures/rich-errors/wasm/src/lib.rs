use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Mirrors the [Error] interface NetworkError from the UDL.
/// Tags must be PascalCase (matching UDL variant names); field names are
/// camelCase to match what the generated TypeScript lift function reads.
#[derive(Serialize)]
#[serde(tag = "tag")]
enum NetworkError {
    NotFound { url: String },
    Timeout {
        url: String,
        #[serde(rename = "elapsedMs")]
        elapsed_ms: u32,
    },
    ServerError {
        #[serde(rename = "statusCode")]
        status_code: u16,
    },
    Unknown,
}

fn throw_network_error(err: NetworkError) -> JsValue {
    JsValue::from_str(&serde_json::to_string(&err).unwrap())
}

/// Returns "data for <url>", or throws a NetworkError.
///
/// Deterministic error rules for testing:
/// - url == "404" → NotFound
/// - url == "timeout" → Timeout
/// - url == "500" → ServerError
/// - url == "unknown" → Unknown
/// - otherwise → returns data string
#[wasm_bindgen]
pub fn fetch_data(url: &str) -> Result<String, JsValue> {
    match url {
        "404" => Err(throw_network_error(NetworkError::NotFound {
            url: url.to_string(),
        })),
        "timeout" => Err(throw_network_error(NetworkError::Timeout {
            url: url.to_string(),
            elapsed_ms: 5000,
        })),
        "500" => Err(throw_network_error(NetworkError::ServerError {
            status_code: 500,
        })),
        "unknown" => Err(throw_network_error(NetworkError::Unknown)),
        _ => Ok(format!("data for {url}")),
    }
}

/// Like fetch_data, but also accepts a timeout_ms parameter (unused in this
/// stub; included to exercise multi-argument [Throws] functions).
#[wasm_bindgen]
pub fn fetch_with_timeout(url: &str, _timeout_ms: u32) -> Result<String, JsValue> {
    fetch_data(url)
}

// Note: async wasm-bindgen functions require owned `String`, not `&str`,
// because the borrow cannot outlive the async suspension point.

/// Async version of fetch_data — exercises [Async, Throws=NetworkError].
#[wasm_bindgen]
pub async fn fetch_data_async(url: String) -> Result<String, JsValue> {
    fetch_data(&url)
}

/// Async version with a retry count parameter — exercises multi-argument
/// [Async, Throws=NetworkError].
#[wasm_bindgen]
pub async fn fetch_with_retry_async(url: String, _max_retries: u32) -> Result<String, JsValue> {
    fetch_data(&url)
}
