//! Code for maintaining Jigsaw's GUI

use eframe::{
    egui::{self, Color32, Ui, Vec2},
    epi,
};

use crate::state::{self, full, spec::part_heads, FullState, State};

mod canvas;

/// The top-level singleton for Jigsaw.  This isn't [`Clone`] because it is a singleton - at any
/// time, there should be at most one copy of it in existence.
#[derive(Debug)]
pub struct JigsawApp {
    state: State,
    /// The text currently in the part head UI box.  This may not parse to a valid sequence of
    /// [`Row`]s, and therefore is allowed to diverge from `self.history`
    part_head_str: String,
}

impl JigsawApp {
    /// Load an example composition
    pub fn example() -> Self {
        let state = State::example();
        Self {
            part_head_str: state.full().part_heads.spec_string(),
            state,
        }
    }

    fn full_state(&self) -> &FullState {
        self.state.full()
    }
}

impl epi::App for JigsawApp {
    fn name(&self) -> &str {
        "Jigsaw"
    }

    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        // Handle input
        for evt in &ctx.input().events {
            match evt {
                &egui::Event::Key {
                    key,
                    pressed,
                    modifiers,
                } => {
                    if !ctx.wants_keyboard_input() {
                        self.handle_key_input(key, pressed, modifiers);
                    }
                }
                _ => {}
            }
        }

        // Draw right-hand panel
        egui::SidePanel::right("side_panel").show(ctx, |ui| self.draw_side_panel(ui));
        // Draw main canvas
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(canvas::Canvas::new(self.full_state()));
        });
    }

    fn max_size_points(&self) -> egui::Vec2 {
        Vec2::new(5000.0, 3000.0)
    }
}

/////////////////////////////////
// GUI DRAWING/INPUT FUNCTIONS //
/////////////////////////////////

impl JigsawApp {
    /// Handle a keyboard input signal that **isn't** captured by [`egui`] itself
    fn handle_key_input(&mut self, key: egui::Key, pressed: bool, modifiers: egui::Modifiers) {
        use egui::Key::*;

        if pressed {
            // z with any set of modifiers is undo
            if key == Z && !modifiers.shift {
                self.state.undo();
                // Update the part head box, since we have potentially changed the part heads.  If
                // we don't do this, then the code will notice that the contents of the part head
                // box is different to the current part heads, and promptly creates a new undo step
                // to change them.
                self.part_head_str = self.full_state().part_heads.spec_string();
            }
            // Z, y or Y with any set of modifiers is redo
            if (key == Z && modifiers.shift) || key == Y {
                self.state.redo();
                // Update the part head box, since we have potentially changed the part heads.  If
                // we don't do this, then the code will notice that the contents of the part head
                // box is different to the current part heads, and promptly creates a new undo step
                // to change them.
                self.part_head_str = self.full_state().part_heads.spec_string();
            }
        }
    }

    fn draw_side_panel(&mut self, ui: &mut Ui) {
        const PANEL_SPACE: f32 = 5.0; // points

        ui.heading("Jigsaw");

        // General info
        let full_state = self.full_state();
        let part_len = full_state.stats.part_len;
        let num_parts = full_state.part_heads.len();
        ui.label(format!(
            "{} rows * {} parts = {} rows",
            part_len,
            num_parts,
            part_len * num_parts
        ));

        ui.add_space(PANEL_SPACE);

        // Create a scrollable panel for the rest of the dropdowns
        egui::ScrollArea::auto_sized().show(ui, |panels_ui| {
            // Parts panel
            let part_panel_title = format!("Parts ({})", self.full_state().part_heads.len());
            let r = egui::CollapsingHeader::new(part_panel_title)
                .id_source("Parts")
                .show(panels_ui, |ui| self.draw_parts_panel(ui));
            // Add space only when the panel is open
            if r.body_response.is_some() {
                panels_ui.add_space(PANEL_SPACE);
            }

            // Methods panel
            let method_panel_title = format!("Methods ({})", self.full_state().methods.len());
            let r = egui::CollapsingHeader::new(method_panel_title)
                .id_source("Methods")
                .show(panels_ui, |ui| self.draw_method_panel(ui));
            // Add space only when the panel is open
            if r.body_response.is_some() {
                panels_ui.add_space(PANEL_SPACE);
            }

            // Calls panel
            let r = panels_ui.collapsing("Calls", |ui| {
                ui.label("14 LE -");
                ui.label("1234 LE s");
            });
            // Add space only when the panel is open
            if r.body_response.is_some() {
                panels_ui.add_space(PANEL_SPACE);
            }

            // Music panel
            let music = &self.full_state().music;
            let label = format!("Music ({}/{})", music.total_count(), music.max_count());
            egui::CollapsingHeader::new(label)
                .id_source("Music")
                .show(panels_ui, |ui| {
                    draw_music_ui(music.groups(), ui);
                });
        });
    }

    fn draw_parts_panel(&mut self, ui: &mut Ui) {
        // Part head input
        ui.text_edit_singleline(&mut self.part_head_str);

        // Parse the user's input
        let parse_result = self
            .full_state()
            .part_heads
            .try_reparse(&self.part_head_str);
        match parse_result {
            // If the part heads changed, then replace them as another undo step
            Ok(part_heads::ReparseOk::DifferentRows(new_phs)) => {
                self.state.apply_edit(|spec| spec.set_part_heads(new_phs))
            }
            // No effect if the part heads haven't changed
            Ok(part_heads::ReparseOk::SameRows) => {}
            // In the case of an error, create a new label for that error
            Err(e) => {
                let err_label = egui::Label::new(e.to_string()).text_color(Color32::RED);
                ui.label(err_label);
            }
        }

        // Add a warning if the parts don't form a group
        if !self.full_state().part_heads.is_group() {
            ui.label("Parts don't form a group!");
        }

        // Part list
        ui.separator();
        for r in self.full_state().part_heads.rows() {
            ui.label(r.to_string());
        }
    }

    fn draw_method_panel(&mut self, ui: &mut Ui) {
        for (i, method) in self.full_state().methods.iter().enumerate() {
            left_then_right(
                ui,
                // The main label sticks to the left
                |left_ui| {
                    left_ui.label(format!(
                        "(#{}, {}): {}",
                        i,
                        method.shorthand(),
                        method.name()
                    ))
                },
                |right_ui| {
                    if method.num_rows == 0 {
                        // Because we're in a right-to-left block, the buttons are added from right
                        // to left (which feels like the reverse order)
                        if right_ui.button("del").clicked() {
                            println!(
                                "Can't delete methods.  Even {}, good though it is!",
                                method.name()
                            );
                        }
                        if right_ui.button("edit").clicked() {
                            println!(
                                "Can't edit methods.  Even {}, good though it is!",
                                method.name()
                            );
                        }
                    } else {
                        // If the method is used, then display either 'x rows' or 'x/y rows',
                        // depending on whether or not all the method's rows are muted
                        let label_text = if method.num_proved_rows == method.num_rows {
                            format!("{} rows", method.num_rows,)
                        } else {
                            format!("{}/{} rows", method.num_proved_rows, method.num_rows,)
                        };
                        right_ui.label(label_text);
                    }
                },
            );
        }
    }
}

/// Recursively creates the GUI for a set of `MusicGroup`s
fn draw_music_ui(musics: &[state::full::MusicGroup], ui: &mut Ui) {
    for m in musics {
        draw_music_group_ui(m, ui);
    }
}

/// Recursively creates the GUI for a single `MusicGroup`
fn draw_music_group_ui(group: &state::full::MusicGroup, ui: &mut Ui) {
    match group {
        full::MusicGroup::Regex {
            name,
            count,
            max_count,
        } => {
            left_then_right(
                ui,
                |left_ui| left_ui.label(name),
                |right_ui| right_ui.label(format!("{}/{}", count, max_count)),
            );
        }

        full::MusicGroup::Group {
            name,
            count,
            max_count,
            sub_groups,
        } => {
            let label = format!("{} ({}/{})", name, count, max_count);
            egui::CollapsingHeader::new(label)
                .id_source(name)
                .show(ui, |sub_ui| draw_music_ui(&sub_groups, sub_ui));
        }
    }
}

/// Draw two pieces of GUI, one aligned left and one aligned right
fn left_then_right<L, R>(
    ui: &mut Ui,
    left: impl FnOnce(&mut Ui) -> L,
    right: impl FnOnce(&mut Ui) -> R,
) -> (L, R) {
    let response = ui.horizontal(|left_ui| {
        let left_res = left(left_ui);
        let right_res = left_ui.with_layout(egui::Layout::right_to_left(), right);
        (left_res, right_res.inner)
    });
    response.inner
}

#[cfg(test)]
mod tests {
    use super::JigsawApp;

    /// Just test that [`Jigsaw::example`] doesn't panic
    #[test]
    fn example() {
        let j = JigsawApp::example();
        dbg!(j.full_state());
    }
}
