//! Code for rendering the canvas in the centre of the screen

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use bellframe::Bell;
use eframe::egui::{
    epaint::Galley, Color32, Pos2, Rect, Rgba, Sense, Shape, Stroke, TextStyle, Ui, Vec2, Widget,
};
use itertools::Itertools;

use jigsaw_comp::{
    full::{ExpandedRow, Fragment},
    FullState,
};
use jigsaw_utils::types::{FragIdx, PartIdx, RowSource};

use super::config::Config;

/// A [`Widget`] which renders the canvas-style view of the composition being edited
#[derive(Debug)]
pub(crate) struct Canvas<'a> {
    /// The [`FullState`] of the composition currently being viewed
    pub(crate) state: &'a FullState,
    /// Configuration & styling for the GUI
    pub(crate) config: &'a Config,
    /// Position of the camera
    pub(crate) camera_pos: Pos2,
    pub(crate) rows_to_highlight: HashSet<RowSource>,
    pub(crate) part_being_viewed: PartIdx,
}

impl<'a> Widget for Canvas<'a> {
    fn ui(self, ui: &mut Ui) -> eframe::egui::Response {
        let size = ui.available_size_before_wrap_finite();
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());

        let origin = rect.min - self.camera_pos.to_vec2();

        // Generate 'Galley's for every bell upfront, placing them in a lookup table when rendering
        let bell_name_galleys = self
            .state
            .stage
            .bells()
            .map(|bell| ui.fonts().layout_single_line(TextStyle::Body, bell.name()))
            .collect_vec();

        for (frag_idx, frag) in self.state.fragments.iter_enumerated() {
            self.draw_frag(ui, frag_idx, frag, origin, &bell_name_galleys);
        }

        response
    }
}

impl<'a> Canvas<'a> {
    /// Draw a [`Fragment`] to the display, returning the bounding [`Rect`] of this [`Fragment`]
    /// **in screen space**.
    fn draw_frag(
        &self,
        ui: &mut Ui,
        frag_index: FragIdx,
        frag: &Fragment,
        origin: Pos2, // Position of the origin in screen space
        bell_name_galleys: &[Arc<Galley>],
    ) -> Rect {
        // Create empty line paths for each bell which should be drawn as lines.  These will be
        // extended during row drawing, and then all rendered at the end.
        let mut lines: HashMap<_, _> = self
            .config
            .bell_lines
            .iter()
            .map(|(&bell, &(width, color))| (bell, (width, color, Vec::<Pos2>::new())))
            .collect();

        // The unpadded rectangle containing all the rows
        let frag_rows_bbox = Rect::from_min_size(
            origin + frag.position,
            Vec2::new(
                self.config.col_width * self.state.stage.num_bells() as f32,
                // TODO: This doesn't take row folding into account - once row folding is
                // implemented, this will become incorrect
                self.config.row_height * frag.expanded_rows.len() as f32,
            ),
        );
        // The bounding box of the fragment **after** padding has been added.  This is used for
        // detecting mouse input and is used to draw the backing rectangle
        let padded_bbox = frag_rows_bbox.expand2(self.config.frag_padding_vec());

        // Draw the background rect
        ui.painter().add(Shape::Rect {
            rect: padded_bbox,
            corner_radius: 0.0,
            fill: Color32::BLACK,
            stroke: Stroke::none(),
        });

        // Draw the rows
        for (row_index, exp_row) in frag.expanded_rows.iter_enumerated() {
            let row_source = RowSource {
                frag_index,
                row_index,
            };
            self.draw_row(
                ui,
                frag_rows_bbox.min,
                row_source,
                exp_row,
                bell_name_galleys,
                &mut lines,
            );
        }

        // Render lines, always in increasing order of bell (otherwise HashMap's non-determinism
        // makes the lines appear to flicker)
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

        // Return the bbox which should be used for collision detection
        padded_bbox
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_row(
        &self,
        ui: &mut Ui,
        screen_space_frag_pos: Pos2,
        source: RowSource,
        row: &ExpandedRow,
        bell_name_galleys: &[Arc<Galley>],
        lines: &mut HashMap<Bell, (f32, Color32, Vec<Pos2>)>,
    ) {
        // Opacity ranges from 0 to 1
        let mut opacity = 1.0;
        // If no rows are highlighted, then all rows are highlighted
        let is_highlighted =
            self.rows_to_highlight.is_empty() || self.rows_to_highlight.contains(&source);
        if !is_highlighted {
            opacity *= 0.5; // Fade out non-highlighted rows
        }
        if !row.is_proved {
            opacity *= 0.5; // Also fade out non-proved rows
        }

        let music_highlights = row.music_highlights.get(&self.part_being_viewed);
        for (col_idx, bell) in row.rows[self.part_being_viewed.index()]
            .bell_iter()
            .enumerate()
        {
            // Compute the screen-space rectangle covered by this bell
            let rect = Rect::from_min_size(
                screen_space_frag_pos
                    + Vec2::new(
                        col_idx as f32 * self.config.col_width,
                        source.row_index.index() as f32 * self.config.row_height,
                    ),
                self.config.bell_box_size(),
            );

            // Draw music highlight
            if music_highlights.map_or(false, |counts| counts[col_idx] > 0) {
                ui.painter().add(Shape::Rect {
                    rect,
                    corner_radius: 0.0,
                    fill: Color32::from_rgb(50, 100, 0),
                    stroke: Stroke::none(),
                });
            }
            // Draw text or add point to line
            if let Some((_, _, points)) = lines.get_mut(&bell) {
                // If this bell is part of a line, then add this location to the line path
                points.push(rect.center());
            } else {
                // If this bell isn't part of a line, then render it as text
                ui.painter().add(Shape::Text {
                    pos: rect.min
                        + Vec2::new(
                            self.config.col_width * self.config.text_pos_x,
                            self.config.row_height * self.config.text_pos_y,
                        ),
                    galley: bell_name_galleys[bell.index()].clone(),
                    color: Rgba::WHITE.multiply(opacity).into(),
                    fake_italics: false,
                });
            }
        }
    }
}
