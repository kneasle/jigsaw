use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct View {
    pub(crate) current_part: usize,
    pub(crate) view_x: f32,
    pub(crate) view_y: f32,
}
