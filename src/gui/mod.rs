//! Code for maintaining Jigsaw's GUI

use eframe::{
    egui::{self, Color32, Ui, Vec2},
    epi,
};

use crate::state::{spec::part_heads, Music, State};

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

        egui::SidePanel::right("side_panel").show(ctx, |ui| self.draw_side_panel(ui));
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
                self.part_head_str = self.state.full().part_heads.spec_string();
            }
            // Z, y or Y with any set of modifiers is redo
            if (key == Z && modifiers.shift) || key == Y {
                self.state.redo();
                // Update the part head box, since we have potentially changed the part heads.  If
                // we don't do this, then the code will notice that the contents of the part head
                // box is different to the current part heads, and promptly creates a new undo step
                // to change them.
                self.part_head_str = self.state.full().part_heads.spec_string();
            }
        }
    }

    fn draw_side_panel(&mut self, ui: &mut Ui) {
        ui.heading("Jigsaw");

        {
            // General info
            let full_state = self.state.full();

            let part_len = full_state.stats.part_len;
            let num_parts = full_state.part_heads.len();
            ui.label(format!(
                "{} rows * {} parts = {} rows",
                part_len,
                num_parts,
                part_len * num_parts
            ));
        }

        // Parts panel
        let part_panel_title = format!("Parts ({})", self.state.full().part_heads.len());
        egui::CollapsingHeader::new(part_panel_title)
            .id_source("Parts")
            .show(ui, |ui| self.draw_parts_panel(ui));

        // Methods panel
        let full_state = self.state.full();

        let method_panel_title = format!("Methods ({})", full_state.methods.len());
        ui.collapsing(method_panel_title, |ui| {
            // Add an entry per method
            for (i, method) in full_state.methods.iter().enumerate() {
                ui.label(format!(
                    "(#{}, {}): {} - {}/{} rows",
                    i,
                    method.shorthand(),
                    method.name(),
                    method.num_proved_rows,
                    method.num_rows,
                ));
            }
        });

        // Calls panel
        ui.collapsing("Calls", |ui| {
            ui.label("14 LE -");
            ui.label("1234 LE s");
        });

        // Music panel
        ui.collapsing("Music", |ui| {
            for c in self.state.music_groups() {
                draw_music_ui(c, ui);
            }
        });
    }

    fn draw_parts_panel(&mut self, ui: &mut Ui) {
        // Part head input
        ui.text_edit_singleline(&mut self.part_head_str);
        match self
            .state
            .full()
            .part_heads
            .try_reparse(&self.part_head_str)
        {
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

        // Part list
        ui.separator();
        for r in self.state.full().part_heads.rows() {
            ui.label(r.to_string());
        }
    }
}

/// Recursively creates the GUI for a music class
fn draw_music_ui(music: &Music, ui: &mut Ui) {
    match music {
        Music::Regex(name, r) => {
            match name {
                Some(name) => ui.label(name),
                None => ui.label(format!("{}", r)),
            };
        }
        Music::Group(name, sub_groups) => {
            ui.collapsing(name, |ui| {
                for m in sub_groups {
                    draw_music_ui(m, ui);
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::JigsawApp;

    /// Just test that [`Jigsaw::example`] doesn't panic
    #[test]
    fn example() {
        let j = JigsawApp::example();
        dbg!(j.state.full());
    }
}
