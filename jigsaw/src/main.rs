// When compiling natively:
fn main() {
    let app = jigsaw::JigsawApp::example();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
