// ---------------------------------------------------------------------------
// Object lifting: wrapping raw WASM values in TypeScript wrapper classes
// ---------------------------------------------------------------------------

use uniffi_bindgen::interface::Type;

/// Return `true` if `module_path` belongs to the current (local) crate.
pub(super) fn is_local_module(module_path: &str, local_crate: &str) -> bool {
    module_path.split("::").next() == Some(local_crate)
}

/// Return `true` if the type tree contains any local `Type::Object` that needs lifting.
pub(super) fn needs_object_lifting(t: &Type, local_crate: &str) -> bool {
    match t {
        Type::Object { module_path, .. } => is_local_module(module_path, local_crate),
        Type::Optional { inner_type } => needs_object_lifting(inner_type, local_crate),
        Type::Sequence { inner_type } => needs_object_lifting(inner_type, local_crate),
        Type::Map { value_type, .. } => needs_object_lifting(value_type, local_crate),
        _ => false,
    }
}

/// Build a lifting expression for a variable `var` given its type.
/// This is the inner recursive worker — it does NOT handle `await`.
pub(super) fn lift_expr(var: &str, t: &Type, local_crate: &str) -> String {
    match t {
        Type::Object {
            name, module_path, ..
        } if is_local_module(module_path, local_crate) => {
            format!("{name}._fromInner({var})")
        }
        Type::Optional { inner_type } if needs_object_lifting(inner_type, local_crate) => {
            let inner = lift_expr("__v", inner_type, local_crate);
            format!("((__v) => __v == null ? null : {inner})({var})")
        }
        // Note: Sequence<Int64> and Sequence<UInt64> are BigInt64Array / BigUint64Array
        // at the wasm boundary; they don't need lifting and fall through to the identity arm.
        Type::Sequence { inner_type } if needs_object_lifting(inner_type, local_crate) => {
            let inner = lift_expr("__v", inner_type, local_crate);
            format!("({var}).map((__v) => {inner})")
        }
        Type::Map { value_type, .. } if needs_object_lifting(value_type, local_crate) => {
            let inner = lift_expr("__v", value_type, local_crate);
            format!("new Map([...{var}].map(([__k, __v]) => [__k, {inner}]))")
        }
        _ => var.to_string(),
    }
}

/// Wrap a raw WASM call expression with the appropriate lift for its return type.
/// Local object types are lifted via their static `_fromInner` factory so the caller
/// receives the TypeScript wrapper class rather than the raw wasm-bindgen instance.
/// External object types are returned as-is (their package owns the wrapping).
///
/// When `is_async` is true, `await` is placed **inside** the lift expression so that
/// `_fromInner` (or the IIFE / `.map()`) receives the resolved value, not a `Promise`.
pub(super) fn lift_return(
    raw_call: &str,
    return_type: Option<&Type>,
    is_async: bool,
    local_crate: &str,
) -> String {
    let await_kw = if is_async { "await " } else { "" };
    let base = match return_type {
        Some(t) if needs_object_lifting(t, local_crate) => {
            let awaited = format!("{await_kw}{raw_call}");
            lift_expr(&awaited, t, local_crate)
        }
        _ => format!("{await_kw}{raw_call}"),
    };
    // wasm-bindgen returns `undefined` for Option::None, but our public API uses `T | null`.
    // Coalesce to keep the type contract consistent. Skip when object lifting already
    // handles it (the IIFE uses `== null` which catches both null and undefined).
    match return_type {
        Some(Type::Optional { .. }) if !needs_object_lifting(return_type.unwrap(), local_crate) => {
            if is_async {
                // Parenthesize so `??` binds to the resolved value, not the Promise.
                format!("({base}) ?? null")
            } else {
                format!("{base} ?? null")
            }
        }
        _ => base,
    }
}
