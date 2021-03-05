use crate::derived_state::DerivedState;
use crate::spec::Spec;
use serde_json::to_string;
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
    derived_state: DerivedState,
}

impl Comp {
    pub fn from_spec(spec: Spec) -> Comp {
        Comp {
            derived_state: DerivedState::from_spec(&spec),
            spec,
        }
    }
}

// Stuff required specifically for JS rendering
#[wasm_bindgen]
impl Comp {
    /// Rebuild the cached state, as though the [`Spec`] had changed.
    pub fn rebuild_state(&mut self) {
        self.derived_state = DerivedState::from_spec(&self.spec);
    }

    /// Return a JSON serialisation of the derived state
    pub fn derived_state(&self) -> String {
        to_string(&self.derived_state).unwrap()
    }

    /// Create an example composition
    pub fn example() -> Comp {
        Self::from_spec(Spec::cyclic_qp())
    }

    // Comp-wide getters
    pub fn stage(&self) -> usize {
        self.spec.stage.as_usize()
    }

    pub fn num_parts(&self) -> usize {
        self.spec.part_heads.len()
    }

    pub fn part_head_str(&self, i: usize) -> String {
        self.spec.part_heads[i].to_string()
    }

    // Fragment getters
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
