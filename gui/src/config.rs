use std::collections::HashMap;

use bellframe::{Bell, Stage};
use eframe::egui::Color32;

/// Configuration settings for Jigsaw's GUI
#[derive(Debug, Clone)]
pub struct Config {
    pub(super) col_width: f32,
    pub(super) row_height: f32,

    pub(super) text_pos_x: f32, // multiple of `col_width`
    pub(super) text_pos_y: f32, // multiple of `row_height`

    /// Widths are multiples of `self.col_width`
    pub(super) bell_lines: HashMap<Bell, (f32, Color32)>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            col_width: 10.0,
            row_height: 16.0,

            text_pos_x: 0.125,
            text_pos_y: 0.05,

            bell_lines: {
                let mut map = HashMap::new();
                map.insert(Bell::TREBLE, (0.1, Color32::RED));
                map.insert(Bell::tenor(Stage::MAJOR), (0.2, Color32::LIGHT_BLUE));
                map
            },
        }
    }
}
