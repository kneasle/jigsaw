use crate::{
    derived_state::DerivedState,
    spec::{Frag, Spec},
    view::View,
};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// The complete state of a partial composition.  The data-flow is:
/// - User makes some edit, which changes the [`Spec`]ification
/// - Once we have the new [`Spec`], we expand all the rows, and use these to rebuild the
///   `derived_state` so that the JS code doesn't recalculate this state every time the screen is
///   rendered.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct Comp {
    undo_history: Vec<Spec>,
    history_index: usize,
    view: View,
    derived_state: DerivedState,
}

impl Comp {
    fn from_spec(spec: Spec) -> Comp {
        Comp {
            derived_state: DerivedState::from_spec(&spec),
            view: View::default(),
            undo_history: vec![spec],
            history_index: 0,
        }
    }

    fn spec(&self) -> &Spec {
        &self.undo_history[self.history_index]
    }

    fn make_action(&mut self, action: impl Fn(&mut Spec)) {
        // Rollback the history so that `history_index` points to the last edit
        drop(self.undo_history.drain(self.history_index + 1..));
        // Perform the required action on a clone of the Spec being displayed
        let mut new_spec = self.undo_history[self.history_index].clone();
        action(&mut new_spec);
        // Add this modified Spec to the undo history, and make it the current one
        self.undo_history.push(new_spec);
        self.history_index += 1;
        // Rebuild the derived state, since the Spec has changed
        self.rebuild_state();
    }
}

// Stuff required specifically for JS rendering
#[wasm_bindgen]
impl Comp {
    /// Create an example composition
    pub fn example() -> Comp {
        Self::from_spec(Spec::cyclic_s8())
    }

    /// Rebuild the cached state, as though the [`Spec`] had changed.
    pub fn rebuild_state(&mut self) {
        self.derived_state = DerivedState::from_spec(self.spec());
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

    /* Undo/redo */

    pub fn undo(&mut self) {
        if self.history_index > 0 {
            self.history_index -= 1;
            self.rebuild_state();
        }
    }

    pub fn redo(&mut self) {
        if self.history_index < self.undo_history.len() - 1 {
            self.history_index += 1;
            self.rebuild_state();
        }
    }

    /* Actions */

    /// Add a new `Frag`ment to the composition.  For the time being, we always create the first
    /// lead of Plain Bob Major
    pub fn add_frag(&mut self) {
        self.make_action(|spec: &mut Spec| {
            spec.frags.push(Rc::new(Frag::one_lead_pb_maj(
                spec.frags.len() as f32 * 300.0,
                spec.frags.len() as f32 * 50.0,
            )));
        });
    }

    /// Move a given [`Frag`]ment to a new location, without changing any of the rows.  This
    /// doesn't clone any rows, because [`Frag`]s store the rows as `Rc<Vec<AnnotatedRow>>`, which
    /// simply increases the ref count on clone.
    pub fn move_frag(&mut self, frag_ind: usize, new_x: f32, new_y: f32) {
        self.make_action(|spec: &mut Spec| {
            let mut new_frag = spec.frags[frag_ind].as_ref().clone();
            new_frag.move_to(new_x, new_y);
            spec.frags[frag_ind] = Rc::new(new_frag);
        });
    }

    /// Deletes a [`Frag`]ment by index
    pub fn delete_frag(&mut self, frag_ind: usize) {
        self.make_action(|spec: &mut Spec| {
            spec.frags.remove(frag_ind);
        });
    }


    /// Splits a given [`Frag`]ment into two fragments, returning `""` on success and an error
    /// string on failure. `split_index` refers to the first row of the 2nd fragment (so row
    /// #`split_index` will also be the new leftover row of the 1st subfragment).
    pub fn split_frag(&mut self, frag_ind: usize, split_index: usize, new_y: f32) -> String {
        // Early return with an error message if any of the preconditions aren't met
        match self.spec().frags.get(frag_ind) {
            Some(f) => {
                if split_index == 0 || split_index >= f.len() {
                    return "Can't create 0-length fragment".to_owned();
                }
            }
            None => {
                return format!(
                    "Frag #{} doens't exist; there are only {} frags.",
                    frag_ind,
                    self.spec().frags.len(),
                );
            }
        }
        // Perform the split (this shouldn't be able to panic, since we checked the preconditions
        // upfront).
        self.make_action(|spec: &mut Spec| {
            let (f1, f2) = spec.frags[frag_ind].split(split_index, new_y);
            // Replace the 1st frag in-place, and append the 2nd (this stops fragments from jumping
            // to the top of the stack when split).
            spec.frags[frag_ind] = Rc::new(f1);
            spec.frags.push(Rc::new(f2));
        });
        // Return empty string for success
        "".to_owned()
    }

    /// Resets the composition to the example
    pub fn reset(&mut self) {
        self.make_action(|spec: &mut Spec| *spec = Spec::cyclic_s8());
    }

    /* View Setters */

    pub fn set_current_part(&mut self, new_part: usize) {
        self.view.current_part = new_part;
    }

    pub fn set_view_loc(&mut self, new_x: f32, new_y: f32) {
        self.view.view_x = new_x;
        self.view.view_y = new_y;
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
