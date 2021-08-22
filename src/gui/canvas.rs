//! Code for rendering the canvas in the centre of the screen

use eframe::egui::{Color32, Sense, Shape, TextStyle, Vec2, Widget};
use itertools::Itertools;

use crate::state::FullState;

/// A [`Widget`] which renders the canvas-style view of the composition being edited
#[derive(Debug, Clone)]
pub(super) struct Canvas<'s> {
    state: &'s FullState,
}

impl<'s> Canvas<'s> {
    pub(super) fn new(state: &'s FullState) -> Self {
        Self { state }
    }
}

impl<'s> Widget for Canvas<'s> {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let size = ui.available_size_before_wrap_finite();
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());

        // Generate 'Galley's for every bell upfront, placing them in a lookup table when rendering
        let galleys = self
            .state
            .stage
            .bells()
            .map(|bell| ui.fonts().layout_single_line(TextStyle::Body, bell.name()))
            .collect_vec();

        for frag in &self.state.fragments {
            for (row_idx, exp_row) in frag.expanded_rows.iter().enumerate() {
                for (col_idx, bell) in exp_row.rows[0].bell_iter().enumerate() {
                    ui.painter().add(Shape::Text {
                        pos: rect.min
                            + frag.position
                            + Vec2::new(col_idx as f32 * 10.0, row_idx as f32 * 16.0),
                        galley: galleys[bell.index()].clone(),
                        color: Color32::WHITE,
                        fake_italics: false,
                    });
                }
            }
        }

        response
    }
}
