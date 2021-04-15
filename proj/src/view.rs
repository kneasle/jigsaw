use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SectionFolds {
    #[serde(default)]
    pub methods: bool,
}

impl SectionFolds {
    /// Toggle the folding of the a given section by name, returning `false` if no such section
    /// exists.
    // TODO: Generate this with a macro
    #[must_use]
    pub fn toggle(&mut self, name: &str) -> bool {
        let value = match name {
            "methods" => &mut self.methods,
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
