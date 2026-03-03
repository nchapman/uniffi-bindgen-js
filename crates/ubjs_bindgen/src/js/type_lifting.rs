// ---------------------------------------------------------------------------
// Object lifting: wrapping raw WASM values in TypeScript wrapper classes
// ---------------------------------------------------------------------------

use uniffi_bindgen::interface::Type;

/// The result of lifting a WASM call expression.
///
/// `preamble` is an optional setup statement (e.g. `const __v = expr;`) that must
/// be emitted before the main `expr`. This avoids inline IIFEs for Optional object
/// lifting, producing cleaner multi-line output.
pub(super) struct LiftedCall {
    pub expr: String,
    pub preamble: Option<String>,
}

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
///
/// For `Optional` types that need lifting, this still produces an IIFE when called
/// with a non-variable expression (e.g. inside `.map()` callbacks). The top-level
/// `lift_return` decomposes Optional into preamble + ternary instead.
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
/// `_fromInner` (or `.map()`) receives the resolved value, not a `Promise`.
///
/// For `Optional<Object>` returns, the IIFE is decomposed into a preamble + expression
/// so that callers can emit cleaner multi-line code instead of an inline IIFE.
pub(super) fn lift_return(
    raw_call: &str,
    return_type: Option<&Type>,
    is_async: bool,
    local_crate: &str,
) -> LiftedCall {
    let await_kw = if is_async { "await " } else { "" };

    match return_type {
        // Optional<Object> — decompose into preamble + ternary (no IIFE)
        Some(Type::Optional { inner_type }) if needs_object_lifting(inner_type, local_crate) => {
            let inner = lift_expr("__v", inner_type, local_crate);
            let preamble = format!("const __v = {await_kw}{raw_call};");
            let expr = format!("__v == null ? null : {inner}");
            LiftedCall {
                expr,
                preamble: Some(preamble),
            }
        }
        // Other types that need object lifting (direct Object, Sequence<Object>, Map<_,Object>)
        Some(t) if needs_object_lifting(t, local_crate) => {
            let awaited = format!("{await_kw}{raw_call}");
            LiftedCall {
                expr: lift_expr(&awaited, t, local_crate),
                preamble: None,
            }
        }
        // Optional without object lifting — coalesce undefined→null
        Some(t @ Type::Optional { .. }) if !needs_object_lifting(t, local_crate) => {
            let base = format!("{await_kw}{raw_call}");
            let expr = if is_async {
                format!("({base}) ?? null")
            } else {
                format!("{base} ?? null")
            };
            LiftedCall {
                expr,
                preamble: None,
            }
        }
        // No lifting needed
        _ => LiftedCall {
            expr: format!("{await_kw}{raw_call}"),
            preamble: None,
        },
    }
}
