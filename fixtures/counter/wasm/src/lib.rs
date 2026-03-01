use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Counter {
    value: i64,
}

#[wasm_bindgen]
impl Counter {
    #[wasm_bindgen(constructor)]
    pub fn new(start: i64) -> Counter {
        Counter { value: start }
    }

    pub fn increment(&mut self) {
        self.value += 1;
    }

    pub fn decrement(&mut self) {
        self.value -= 1;
    }

    pub fn get(&self) -> i64 {
        self.value
    }

    pub fn reset_to(&mut self, value: i64) {
        self.value = value;
    }
}
