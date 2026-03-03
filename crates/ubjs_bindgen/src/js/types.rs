// ---------------------------------------------------------------------------
// Internal data types extracted from UDL
// ---------------------------------------------------------------------------

use uniffi_bindgen::interface::{DefaultValue, Literal, Type};

/// The crate name sentinel used when parsing UDL via `ComponentInterface::from_webidl`.
/// All local types will have a `module_path` whose first `::` segment equals this value.
/// External types declared with `[External="crate_name"]` will differ.
pub(super) const LOCAL_CRATE_SENTINEL: &str = "crate_name";

#[derive(Debug)]
pub(super) struct UdlFunction {
    pub name: String,
    pub args: Vec<UdlArg>,
    pub return_type: Option<Type>,
    pub throws_type: Option<Type>,
    pub is_async: bool,
    pub docstring: Option<String>,
}

#[derive(Debug)]
pub(super) struct UdlArg {
    pub name: String,
    pub type_: Type,
    pub default: Option<DefaultValue>,
}

/// A variant field (used in rich error variants and data enum variants),
/// or a record field (used in dictionary declarations).
#[derive(Debug)]
pub(super) struct UdlField {
    pub name: String,
    pub type_: Type,
    pub docstring: Option<String>,
    pub default: Option<DefaultValue>,
}

/// One variant of an enum or error type.
#[derive(Debug)]
pub(super) struct UdlVariant {
    pub name: String,
    /// Empty for flat variants (no associated data).
    pub fields: Vec<UdlField>,
    pub docstring: Option<String>,
    /// Explicit discriminant value (e.g. `= 10`), if declared.
    pub discr: Option<Literal>,
}

/// A [Error] enum — generates a TypeScript error class.
#[derive(Debug)]
pub(super) struct UdlError {
    pub name: String,
    pub variants: Vec<UdlVariant>,
    pub is_flat: bool,
    pub is_non_exhaustive: bool,
    pub docstring: Option<String>,
    /// Methods declared on the error enum.
    pub methods: Vec<UdlMethod>,
    /// Constructors declared on the error enum (proc-macro only).
    pub constructors: Vec<UdlConstructor>,
}

/// A plain enum or [Enum] interface — generates a TypeScript union type.
#[derive(Debug)]
pub(super) struct UdlEnum {
    pub name: String,
    pub variants: Vec<UdlVariant>,
    /// true ↔ all variants are unit variants (no fields); serialises as a string.
    pub is_flat: bool,
    pub is_non_exhaustive: bool,
    pub docstring: Option<String>,
    /// Methods declared on the enum (from `impl` blocks).
    pub methods: Vec<UdlMethod>,
    /// Constructors declared on the enum (proc-macro only).
    pub constructors: Vec<UdlConstructor>,
    /// Synthesised trait methods.
    pub traits: SynthesisedTraits,
}

/// Synthesised trait methods (Display, Eq, Hash).
#[derive(Debug, Default)]
pub(super) struct SynthesisedTraits {
    /// Method name for Display::fmt (produces `toString()`).
    pub display: Option<String>,
    /// Method name for PartialEq::eq (produces `equals(other)`).
    pub eq: Option<String>,
    /// Method name for Hash::hash (produces `hashCode()`).
    pub hash: Option<String>,
}

/// A `dictionary` declaration — generates a TypeScript interface.
#[derive(Debug)]
pub(super) struct UdlRecord {
    pub name: String,
    pub fields: Vec<UdlField>,
    pub docstring: Option<String>,
    /// Methods declared on the record (from `impl` blocks, proc-macro only).
    pub methods: Vec<UdlMethod>,
    /// Constructors declared on the record (proc-macro only).
    pub constructors: Vec<UdlConstructor>,
    /// Synthesised trait methods.
    pub traits: SynthesisedTraits,
}

/// A constructor of an `interface` object.
#[derive(Debug)]
pub(super) struct UdlConstructor {
    /// Exported name in JS.  Usually "new".
    pub name: String,
    pub args: Vec<UdlArg>,
    pub throws_type: Option<Type>,
    pub is_async: bool,
    pub docstring: Option<String>,
}

/// A method on an `interface` object.
#[derive(Debug)]
pub(super) struct UdlMethod {
    pub name: String,
    pub args: Vec<UdlArg>,
    pub return_type: Option<Type>,
    pub throws_type: Option<Type>,
    pub is_async: bool,
    pub docstring: Option<String>,
}

/// An `interface` declaration — generates a TypeScript class.
#[derive(Debug)]
pub(super) struct UdlObject {
    pub name: String,
    pub constructors: Vec<UdlConstructor>,
    pub methods: Vec<UdlMethod>,
    pub docstring: Option<String>,
    /// True when this object is used as a `[Throws=...]` error type.
    pub is_error: bool,
}

/// A `[Custom]` typedef — generates a TypeScript type alias.
#[derive(Debug)]
pub(super) struct UdlCustomType {
    /// The custom type name (e.g. `Url`).
    pub name: String,
    /// The underlying builtin type (e.g. `Type::String`).
    pub builtin: Type,
    /// The `module_path` from the source `Type::Custom` — used to detect external custom types.
    pub module_path: String,
}

/// A method on a `callback interface` — generates a method signature in a TS interface.
///
/// `throws_type` is intentionally omitted: TypeScript interfaces have no `throws`
/// annotation syntax, so there is nothing to emit. Errors flow outward from the JS
/// implementor into Rust; the TypeScript interface only describes the return type.
///
/// `is_async` IS expressible in UDL (`[Async]` on a callback method). The generator
/// emits `Promise<T>` for the method return type, which is the correct TypeScript
/// contract. Wasm fixture crates must use `wasm_bindgen_futures` and return a
/// `js_sys::Promise` to back async callback methods at runtime.
#[derive(Debug)]
pub(super) struct UdlCallbackMethod {
    pub name: String,
    pub args: Vec<UdlArg>,
    pub return_type: Option<Type>,
    pub is_async: bool,
    pub docstring: Option<String>,
}

/// A `callback interface` declaration — generates a TypeScript interface.
#[derive(Debug)]
pub(super) struct UdlCallbackInterface {
    pub name: String,
    pub methods: Vec<UdlCallbackMethod>,
    pub docstring: Option<String>,
}

/// A compile-time checksum for a single FFI function/method/constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct UdlApiChecksum {
    /// The FFI symbol name (e.g. `uniffi_crate_name_checksum_func_greet`).
    pub symbol: String,
    /// The expected u16 checksum value.
    pub expected: u16,
}

#[derive(Debug)]
pub(super) struct UdlMetadata {
    pub namespace: String,
    pub namespace_docstring: Option<String>,
    /// The module_path prefix for types local to this crate.
    /// For UDL mode this is `LOCAL_CRATE_SENTINEL`; for library mode it is the actual crate name.
    pub local_crate: String,
    /// The UniFFI contract version expected by the generated bindings.
    pub uniffi_contract_version: Option<u32>,
    /// The FFI symbol for querying the scaffolding contract version at runtime.
    pub ffi_uniffi_contract_version_symbol: Option<String>,
    /// Compile-time API checksums for each function/method/constructor.
    pub api_checksums: Vec<UdlApiChecksum>,
    pub functions: Vec<UdlFunction>,
    pub errors: Vec<UdlError>,
    pub enums: Vec<UdlEnum>,
    pub records: Vec<UdlRecord>,
    pub objects: Vec<UdlObject>,
    pub custom_types: Vec<UdlCustomType>,
    pub callback_interfaces: Vec<UdlCallbackInterface>,
}

impl Default for UdlMetadata {
    fn default() -> Self {
        Self {
            namespace: String::new(),
            namespace_docstring: None,
            local_crate: LOCAL_CRATE_SENTINEL.to_string(),
            uniffi_contract_version: None,
            ffi_uniffi_contract_version_symbol: None,
            api_checksums: Vec::new(),
            functions: Vec::new(),
            errors: Vec::new(),
            enums: Vec::new(),
            records: Vec::new(),
            objects: Vec::new(),
            custom_types: Vec::new(),
            callback_interfaces: Vec::new(),
        }
    }
}
