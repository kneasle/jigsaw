use crate::{derived_state::DerivedState, spec::Spec, view::View};
use wasm_bindgen::prelude::*;

/// The complete state of a partial composition.  The data-flow is:
/// - User makes some edit, which changes the [`Spec`]ification
/// - Once we have the new [`Spec`], we expand all the rows, and use these to rebuild the
///   `derived_state` so that the JS code doesn't recalculate this state every time the screen is
///   rendered.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct Comp {
    spec: Spec,
    view: View,
    derived_state: DerivedState,
}

impl Comp {
    pub fn from_spec(spec: Spec) -> Comp {
        Comp {
            derived_state: DerivedState::from_spec(&spec),
            view: View::default(),
            spec,
        }
    }
}

// Stuff required specifically for JS rendering
#[wasm_bindgen]
impl Comp {
    /// Create an example composition
    pub fn example() -> Comp {
        Self::from_spec(Spec::cyclic_max_eld())
    }

    /// Rebuild the cached state, as though the [`Spec`] had changed.
    pub fn rebuild_state(&mut self) {
        self.derived_state = DerivedState::from_spec(&self.spec);
    }

    /* Serialization/Deserialization */

    /// Return a JSON serialisation of the derived state
    pub fn ser_derived_state(&self) -> String {
        serde_json::to_string(&self.derived_state).unwrap()
    }

    /// Return a JSON serialisation of the current view settings
    pub fn ser_view(&self) -> String {
        serde_json::to_string(&self.view).unwrap()
    }

    pub fn set_view_from_json(&mut self, json: String) {
        self.view = serde_json::de::from_str(&json).unwrap();
    }

    // View Setters
    pub fn set_current_part(&mut self, new_part: usize) {
        self.view.current_part = new_part;
    }

    pub fn set_view_loc(&mut self, new_x: f32, new_y: f32) {
        self.view.view_x = new_x;
        self.view.view_y = new_y;
    }

    /* Comp-wide getters */

    pub fn stage(&self) -> usize {
        self.spec.stage.as_usize()
    }

    pub fn num_parts(&self) -> usize {
        self.spec.part_heads.len()
    }

    pub fn part_head_str(&self, i: usize) -> String {
        self.spec.part_heads[i].to_string()
    }

    /* Fragment getters */

    pub fn frag_x(&self, i: usize) -> f32 {
        self.spec.frags[i].x
    }

    pub fn frag_y(&self, i: usize) -> f32 {
        self.spec.frags[i].y
    }
}

#[cfg(test)]
mod tests {
    use super::Comp;

    #[test]
    fn test() {
        let _c = Comp::example();
    }
}
