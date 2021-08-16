// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let app = jigsaw::JigsawApp::example();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
