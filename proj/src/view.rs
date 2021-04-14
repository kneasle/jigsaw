use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PanelFolds {
    #[serde(default)]
    pub method: bool,
}

/// State that is saved per-composition, but shouldn't be tracked in the undo history.  This
/// includes the view state (e.g. where the camera is, which part the user's looking at) and
/// the state of the UI (e.g. which side-bar panels are collapsed).
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct View {
    pub current_part: usize,
    pub view_x: f32,
    pub view_y: f32,
    #[serde(default)]
    #[serde(flatten)]
    pub panel_folds: PanelFolds,
}
