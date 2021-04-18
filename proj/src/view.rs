use crate::ser_utils::get_true;
use serde::{Deserialize, Serialize};

// TODO: Generate this whole struct with a macro (using `stringify!`)
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SectionFolds {
    #[serde(default = "get_true")]
    pub general: bool,
    #[serde(default = "get_true")]
    pub methods: bool,
    #[serde(default = "get_true")]
    pub calls: bool,
    #[serde(default = "get_true")]
    pub music: bool,
}

impl SectionFolds {
    /// Toggle the folding of the a given section by name, returning `false` if no such section
    /// exists.
    #[must_use]
    pub fn toggle(&mut self, name: &str) -> bool {
        let value = match name {
            "general" => &mut self.general,
            "methods" => &mut self.methods,
            "calls" => &mut self.calls,
            "music" => &mut self.music,
            _ => return false,
        };
        *value = !*value;
        true
    }
}

/// State that is saved per-composition, but shouldn't be tracked in the undo history.  This
/// includes the view state (e.g. where the camera is, which part the user's looking at) and
/// the state of the UI (e.g. which side-bar sections are collapsed).
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct View {
    pub current_part: usize,
    pub view_x: f32,
    pub view_y: f32,
    #[serde(default)]
    pub section_folds: SectionFolds,
}
