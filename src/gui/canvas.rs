//! Code for rendering the canvas in the centre of the screen

use std::{collections::HashMap, sync::Arc};

use eframe::egui::{
    epaint::Galley, Color32, Pos2, Sense, Shape, Stroke, TextStyle, Ui, Vec2, Widget,
};
use itertools::Itertools;

use crate::state::{full::Fragment, FullState};

use super::config::Config;

/// A [`Widget`] which renders the canvas-style view of the composition being edited
#[derive(Debug)]
pub(super) struct Canvas<'a> {
    /// The [`FullState`] of the composition currently being viewed
    pub(super) state: &'a FullState,
    /// Configuration & styling for the GUI
    pub(super) config: &'a Config,
    /// Position of the camera
    pub(super) camera_pos: Pos2,
}

impl<'a> Widget for Canvas<'a> {
    fn ui(self, ui: &mut Ui) -> eframe::egui::Response {
        let size = ui.available_size_before_wrap_finite();
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());

        let origin = rect.min - self.camera_pos;

        // Generate 'Galley's for every bell upfront, placing them in a lookup table when rendering
        let bell_name_galleys = self
            .state
            .stage
            .bells()
            .map(|bell| ui.fonts().layout_single_line(TextStyle::Body, bell.name()))
            .collect_vec();

        for frag in &self.state.fragments {
            self.draw_frag(ui, frag, origin, &bell_name_galleys);
        }

        response
    }
}

impl<'a> Canvas<'a> {
    /// Draw a [`Fragment`] to the display
    fn draw_frag(
        &self,
        ui: &mut Ui,
        frag: &Fragment,
        origin: Vec2, // Position of the origin in screen space
        bell_name_galleys: &[Arc<Galley>],
    ) {
        // Which bells' paths are currently being drawn
        let mut lines: HashMap<_, _> = self
            .config
            .bell_lines
            .iter()
            .map(|(&bell, &(width, color))| (bell, (width, color, Vec::<Pos2>::new())))
            .collect();

        // Render rows
        for (row_idx, exp_row) in frag.expanded_rows.iter().enumerate() {
            for (col_idx, bell) in exp_row.rows[0].bell_iter().enumerate() {
                let top_left_coord = origin
                    + frag.position
                    + Vec2::new(
                        col_idx as f32 * self.config.col_width,
                        row_idx as f32 * self.config.row_height,
                    );
                let top_left_coord = Pos2::new(top_left_coord.x, top_left_coord.y);

                if let Some((_, _, points)) = lines.get_mut(&bell) {
                    // If this bell is part of a line, then add this location to the line path
                    points.push(
                        top_left_coord
                            + Vec2::new(self.config.col_width, self.config.row_height) / 2.0,
                    );
                } else {
                    // If this bell isn't part of a line, then render it as text
                    ui.painter().add(Shape::Text {
                        pos: top_left_coord
                            + Vec2::new(
                                self.config.col_width * self.config.text_pos_x,
                                self.config.row_height * self.config.text_pos_y,
                            ),
                        galley: bell_name_galleys[bell.index()].clone(),
                        color: Color32::WHITE,
                        fake_italics: false,
                    });
                }
            }
        }

        // Render lines, always in increasing order (otherwise the non-determinism makes the
        // lines appear to flicker)
        let mut lines = lines.into_iter().collect_vec();
        lines.sort_by_key(|(k, _)| *k);
        for (_bell, (width, color, points)) in lines {
            ui.painter().add(Shape::Path {
                points,
                closed: false,
                fill: Color32::TRANSPARENT,
                stroke: Stroke {
                    width: width * self.config.col_width,
                    color,
                },
            });
        }
    }
}
