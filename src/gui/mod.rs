//! Code for maintaining Jigsaw's GUI

use bellframe::{music::Regex, Stage};
use eframe::{
    egui::{self, Ui, Vec2},
    epi,
};

use crate::{history::History, music::Music, spec::CompSpec};

/// The top-level singleton for Jigsaw.  This isn't [`Clone`] because it is a singleton - at any
/// time, there should be at most one copy of it in existence.
#[derive(Debug)]
pub struct JigsawApp {
    /// A sequence of [`CompSpec`]s making up the undo/redo history
    history: History,
    /// The top-level classes of patterns which are considered musical
    music_classes: Vec<Music>,
}

impl JigsawApp {
    /// The state that Jigsaw will be in the first time the user starts up.
    pub fn example() -> Self {
        // For the time being, just create an empty composition of Major
        Self::new(
            CompSpec::example(),
            vec![
                Music::Group(
                    "56s/65s".to_owned(),
                    vec![
                        Music::Regex(Some("65s".to_owned()), Regex::parse("*6578")),
                        Music::Regex(Some("56s".to_owned()), Regex::parse("*5678")),
                    ],
                ),
                Music::runs_front_and_back(Stage::MAJOR, 4),
                Music::runs_front_and_back(Stage::MAJOR, 5),
                Music::runs_front_and_back(Stage::MAJOR, 6),
                Music::runs_front_and_back(Stage::MAJOR, 7),
            ],
        )
    }

    /// Creates a [`Jigsaw`] struct displaying a single [`CompSpec`], with no other undo history.
    pub(crate) fn new(spec: CompSpec, music_classes: Vec<Music>) -> Self {
        Self {
            history: History::new(spec),
            music_classes,
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

            let full_comp = self.history.full_comp();

            // General info
            let part_len = full_comp.stats.part_len;
            let num_parts = full_comp.part_heads.len();
            ui.label(format!(
                "{} rows * {} parts = {} rows",
                part_len,
                num_parts,
                part_len * num_parts
            ));

            // Parts panel
            let mut part_head_str = full_comp.part_heads.spec_string().to_owned();
            let part_panel_title = format!("Parts ({})", full_comp.part_heads.len());
            ui.collapsing(part_panel_title, |ui| {
                ui.text_edit_singleline(&mut part_head_str);
            });

            // Methods panel
            let method_panel_title = format!("Methods ({})", full_comp.methods.len());
            ui.collapsing(method_panel_title, |ui| {
                // Add an entry per method
                for (i, method) in full_comp.methods.iter().enumerate() {
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
                ui.label("Music, whoop whoop!");

                for c in &self.music_classes {
                    gen_music_ui(c, ui);
                }
            });
        });
    }

    fn max_size_points(&self) -> egui::Vec2 {
        Vec2::new(5000.0, 3000.0)
    }
}

/// Recursively creates the GUI for a music class
fn gen_music_ui(music: &Music, ui: &mut Ui) {
    match music {
        Music::Regex(_name, r) => {
            ui.label(format!("{}", r));
        }
        Music::Group(name, sub_groups) => {
            ui.collapsing(name, |ui| {
                for m in sub_groups {
                    gen_music_ui(m, ui);
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
        dbg!(j.history.full_comp());
    }
}
