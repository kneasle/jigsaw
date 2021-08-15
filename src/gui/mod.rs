//! Code for maintaining Jigsaw's GUI

use bellframe::Stage;
use eframe::{
    egui::{self, Vec2},
    epi,
};

use crate::{history::History, spec::CompSpec};

/// The top-level singleton for Jigsaw.  This isn't [`Clone`] because it is a singleton - at any
/// time, there should be at most one copy of it in existence.
#[derive(Debug)]
pub struct Jigsaw {
    history: History,
}

impl Jigsaw {
    /// The state that Jigsaw will be in the first time the user starts up.
    pub fn example() -> Self {
        // For the time being, just create an empty composition of Major
        Self::with_spec(CompSpec::empty(Stage::MAJOR))
    }

    /// Creates a [`Jigsaw`] struct displaying a single [`CompSpec`], with no other undo history.
    pub fn with_spec(spec: CompSpec) -> Self {
        Self {
            history: History::new(spec),
        }
    }
}

impl epi::App for Jigsaw {
    fn name(&self) -> &str {
        "Jigsaw"
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        egui::SidePanel::right("side_panel").show(ctx, |ui| {
            ui.heading("Jigsaw");
            ui.label("Hello, this is a suuuuper long string, which will hopefully cause some line wrapping.  If it doesn't, then no biggie.");
        });
    }

    fn max_size_points(&self) -> egui::Vec2 {
        Vec2::new(5000.0, 3000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Jigsaw;

    /// Just test that [`Jigsaw::example`] doesn't panic
    #[test]
    fn example() {
        let j = Jigsaw::example();
        dbg!(j.history.full_comp());
    }
}
