uniffi::setup_scaffolding!();

#[uniffi::export]
pub fn hello(name: String) -> String {
    format!("hello, {name}")
}

#[uniffi::export]
pub fn farewell(name: String) -> String {
    format!("goodbye, {name}")
}

#[uniffi::export]
pub fn version() -> String {
    "1.0.0".to_string()
}

#[derive(uniffi::Record)]
pub struct InternalState {
    pub counter: u32,
    pub active: bool,
}
