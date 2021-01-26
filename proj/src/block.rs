use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Annotation {
    LeadEnd,
    LeadHead(String),
    Call(String),
    CourseHead,
}
