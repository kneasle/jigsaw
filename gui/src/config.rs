use std::collections::HashMap;

use bellframe::{Bell, Stage};
use eframe::egui::{Color32, Vec2};

/// Configuration settings for Jigsaw's GUI
#[derive(Debug, Clone)]
pub struct Config {
    /* Display */
    pub(crate) col_width: f32,  // points
    pub(crate) row_height: f32, // points

    pub(crate) ruleoff_line_width: f32, // points

    pub(crate) text_pos_x: f32, // multiple of `col_width`
    pub(crate) text_pos_y: f32, // multiple of `row_height`

    pub(crate) frag_padding_x: f32, // multiple of `col_width`
    pub(crate) frag_padding_y: f32, // multiple of `row_height`

    /// Widths are multiples of `self.col_width`
    pub(crate) bell_lines: HashMap<Bell, (f32, Color32)>,

    /* User interaction */
    /// When splitting a fragment at a rule-off, the cursor must be less than this many rows away
    /// from the nearest rule-off.
    pub(crate) ruleoff_snap_distance: f32, // rows
    /// When a fragment is split, how far away is the 2nd fragment?
    pub(crate) split_height: f32, // multiples of `row_height`
}

impl Config {
    pub(crate) fn bell_box_size(&self) -> Vec2 {
        Vec2::new(self.col_width, self.row_height)
    }

    /// Returns the [`Vec2`] representing the size of the padding round a fragment, in (virtual)
    /// pixels.
    pub(crate) fn frag_padding_vec(&self) -> Vec2 {
        Vec2::new(
            self.col_width * self.frag_padding_x,
            self.row_height * self.frag_padding_y,
        )
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            col_width: 10.0,
            row_height: 16.0,

            ruleoff_line_width: 1.0,

            text_pos_x: 0.125,
            text_pos_y: 0.05,

            frag_padding_x: 0.5,
            frag_padding_y: 0.3,

            ruleoff_snap_distance: 3.0, // rows
            split_height: 2.0,

            bell_lines: {
                let mut map = HashMap::new();
                map.insert(Bell::TREBLE, (0.1, Color32::RED));
                map.insert(Bell::tenor(Stage::MAJOR), (0.2, Color32::LIGHT_BLUE));
                map
            },
        }
    }
}
