//! Code for maintaining Jigsaw's GUI

use eframe::{
    egui::{self, Vec2},
    epi,
};

use crate::{history::History, spec::CompSpec};

/// The top-level singleton for Jigsaw.  This isn't [`Clone`] because it is a singleton - at any
/// time, there should be at most one copy of it in existence.
#[derive(Debug)]
pub struct JigsawApp {
    /// The sequence of [`CompSpec`]s making up the undo/redo history
    history: History,
}

impl JigsawApp {
    /// The state that Jigsaw will be in the first time the user starts up.
    pub fn example() -> Self {
        // For the time being, just create an empty composition of Major
        Self::with_spec(CompSpec::example())
    }

    /// Creates a [`Jigsaw`] struct displaying a single [`CompSpec`], with no other undo history.
    pub(crate) fn with_spec(spec: CompSpec) -> Self {
        Self {
            history: History::new(spec),
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

            ui.collapsing("Calls", |ui| {
                ui.label("14 LE -");
                ui.label("1234 LE s");
            });
        });
    }

    fn max_size_points(&self) -> egui::Vec2 {
        Vec2::new(5000.0, 3000.0)
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
