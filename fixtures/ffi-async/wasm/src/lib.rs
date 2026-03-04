#[uniffi::export]
pub async fn async_add(a: u32, b: u32) -> u32 {
    a + b
}

#[uniffi::export]
pub async fn async_greet(name: String) -> String {
    format!("Hello, {}!", name)
}

#[uniffi::export]
pub async fn async_noop() {}

uniffi::setup_scaffolding!();
