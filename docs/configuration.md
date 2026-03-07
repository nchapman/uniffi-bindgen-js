# Configuration

`uniffi-bindgen-js` reads `[bindings.js]` from `uniffi.toml`.

## Supported Keys

- `module_name`: overrides generated TypeScript namespace name (default: PascalCase of the crate/UDL namespace). **Recommended** when your crate name includes a suffix like `_uniffi` or `_ffi` — without this, a crate named `html2markdown_uniffi` produces a namespace called `Html2markdownUniffi` instead of the cleaner `Html2Markdown`.
- `rename`: map of UDL API identifiers to TypeScript public API names.
- `exclude`: list of UDL API identifiers to omit from generated TypeScript public API surface.
- `external_packages`: map of external UniFFI crate names to JS import paths used for generated external-type references.

## `rename` and `exclude` Identifier Format

- Top-level function: `function_name`
- Object/interface class name: `ObjectName`
- Object constructor/method: `ObjectName.member_name`
- Type name: `TypeName`
- Type method: `TypeName.method_name`

Examples:
- `add_numbers`
- `Counter`
- `Counter.current_value`

## Example

```toml
[bindings.js]
module_name = "MyBindings"
rename = { add_numbers = "sumValues", "Counter.current_value" = "getValue" }
exclude = ["internal_helper", "Counter.hidden_value"]
external_packages = { other_crate = "./other_bindings.js" }
```
