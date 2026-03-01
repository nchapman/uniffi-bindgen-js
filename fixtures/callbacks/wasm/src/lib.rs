use wasm_bindgen::prelude::*;

/// Declare the JS-side `Formatter` interface so Rust can call its methods.
///
/// wasm-bindgen generates TypeScript declarations that match this extern block,
/// and the generated `callbacks.ts` interface is what consumers implement.
#[wasm_bindgen]
extern "C" {
    pub type Formatter;

    /// `format(input: string): string`
    #[wasm_bindgen(method)]
    pub fn format(this: &Formatter, input: &str) -> String;

    /// `formatWithPrefix(prefix: string, input: string): string`
    ///
    /// UDL names are snake_case; we use `js_name` so wasm-bindgen calls the
    /// camelCase method that our generated TypeScript interface declares.
    #[wasm_bindgen(method, js_name = formatWithPrefix)]
    pub fn format_with_prefix(this: &Formatter, prefix: &str, input: &str) -> String;
}

/// Apply the formatter's `format` method to the given template string.
#[wasm_bindgen]
pub fn apply_formatter(fmt: &Formatter, template: &str) -> String {
    fmt.format(template)
}

/// Use the formatter's `formatWithPrefix` method to produce a greeting.
#[wasm_bindgen]
pub fn format_greeting(fmt: &Formatter, name: &str) -> String {
    fmt.format_with_prefix("Hello", name)
}
