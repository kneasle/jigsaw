use crate::derived_state::DerivedState;
use crate::spec::Spec;
use wasm_bindgen::prelude::*;

fn clone_or_empty(string: &Option<String>) -> String {
    match string {
        Some(x) => x.clone(),
        None => "".to_owned(),
    }
}

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

    /// Rebuild the cached state, as though the [`Spec`] had changed.
    fn rebuild_state(&mut self) {
        self.derived_state = DerivedState::from_spec(&self.spec);
    }
}

// Stuff required specifically for JS rendering
#[wasm_bindgen]
impl Comp {
    /// Create an example composition
    pub fn example() -> Comp {
        Self::from_spec(Spec::example())
    }

    // Comp-wide getters
    pub fn stage(&self) -> usize {
        self.spec.stage.as_usize()
    }

    pub fn num_frags(&self) -> usize {
        self.spec.frags.len()
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

    pub fn frag_len(&self, i: usize) -> usize {
        self.spec.frags[i].len()
    }

    // Row getters
    pub fn method_str(&self, f: usize, r: usize) -> String {
        clone_or_empty(&self.spec.frags[f].rows[r].method_str)
    }

    pub fn call_str(&self, f: usize, r: usize) -> String {
        clone_or_empty(&self.spec.frags[f].rows[r].call_str)
    }

    pub fn is_ruleoff(&self, f: usize, r: usize) -> bool {
        self.spec.frags[f].rows[r].is_lead_end
    }

    pub fn bell_index(&self, p: usize, f: usize, r: usize, b: usize) -> usize {
        self.derived_state.annot_frags[f].exp_rows[r].expanded_rows[p][b].index()
    }

    pub fn music_highlights(&self, f: usize, r: usize) -> Vec<usize> {
        self.derived_state.annot_frags[f].exp_rows[r]
            .music_highlights
            .clone()
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
