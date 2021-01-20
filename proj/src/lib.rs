use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn reverse(s: String) -> String {
    s.chars().rev().skip(1).collect()
}
