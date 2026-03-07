uniffi::setup_scaffolding!();

// Custom type: Url (String-backed, converted to URL in JS)
pub struct Url(String);

uniffi::custom_type!(Url, String, {
    try_lift: |s| Ok(Url(s)),
    lower: |url| url.0,
});

// Custom type: Handle (opaque i64 wrapper)
pub struct Handle(i64);

uniffi::custom_type!(Handle, i64, {
    try_lift: |v| Ok(Handle(v)),
    lower: |h| h.0,
});

#[uniffi::export]
pub fn normalize_url(url: Url) -> Url {
    let mut s = url.0.to_lowercase();
    if s.ends_with('/') && s.len() > 1 {
        s.pop();
    }
    Url(s)
}

#[uniffi::export]
pub fn make_handle(seed: i64) -> Handle {
    Handle(seed * 42)
}

#[uniffi::export]
pub fn handles_equal(a: Handle, b: Handle) -> bool {
    a.0 == b.0
}
