//! Code for maintaining Jigsaw's GUI

use eframe::{
    egui::{self, Color32, Ui, Vec2},
    epi,
};

use crate::state::{
    spec::part_heads::{self, PartHeads},
    Music, State,
};

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
        egui::SidePanel::right("side_panel").show(ctx, |ui| {
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
            ui.collapsing(part_panel_title, |ui| self.draw_parts_panel(ui));

            // Methods panel
            let full_state = self.state.full();

            let method_panel_title = format!("Methods ({})", full_state.methods.len());
            ui.collapsing(method_panel_title, |ui| {
                // Add an entry per method
                for (i, method) in full_state.methods.iter().enumerate() {
                    ui.label(format!(
                        "(#{}, {}): {} - {}/{} rows",
                        i,
                        method.source.shorthand(),
                        method.source.name(),
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
        });
    }

    fn max_size_points(&self) -> egui::Vec2 {
        Vec2::new(5000.0, 3000.0)
    }
}

///////////////////////////
// GUI DRAWING FUNCTIONS //
///////////////////////////

impl JigsawApp {
    fn draw_parts_panel(&mut self, ui: &mut Ui) -> Option<PartHeads> {
        let full_state = self.state.full();

        // If the user has changed the part heads, then this will be set to
        // `Some(<the new part heads>)`
        let mut new_part_heads = None;

        // Part head input
        ui.text_edit_singleline(&mut self.part_head_str);
        match full_state.part_heads.try_reparse(&self.part_head_str) {
            Ok(part_heads::ReparseOk::DifferentRows(new_phs)) => new_part_heads = Some(new_phs),
            Ok(part_heads::ReparseOk::SameRows) => {} // No effect if the part heads haven't changed
            Err(e) => {
                // In the case of an error, create a new label for that error
                let err_label = egui::Label::new(e.to_string()).text_color(Color32::RED);
                ui.label(err_label);
            }
        }

        // Part list
        ui.separator();
        for r in full_state.part_heads.rows() {
            ui.label(r.to_string());
        }

        new_part_heads
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
