mod gui;
mod ser_utils;
mod state;
mod utils;

/* Web start-up code */

// Export `gui::Jigsaw::example()` out of the library.  We're really unlikely to ever use this, but
// exporting it will prevent the compiler from flagging everything as 'dead_code' when we aren't
// building with `wasm32`.
pub use gui::JigsawApp;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Start Jigsaw's GUI in a given canvas window
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<(), eframe::wasm_bindgen::JsValue> {
    let app = gui::JigsawApp::example();
    eframe::start_web(canvas_id, Box::new(app))
}
