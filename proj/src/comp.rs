use crate::{
    derived_state::DerivedState,
    spec::{Frag, Spec},
    view::View,
};
use proj_core::Row;
use wasm_bindgen::prelude::*;

// Imports used solely for doc comments
#[allow(unused_imports)]
use proj_core::Stage;

/// A enum of what states the [`Comp`] editor can be in.  Implementing the UI as a state machine
/// enforces the constraint that the user can only be performing one action at once.  This prevents
/// the user causing undefined behaviour by doing things like splitting/deleting a [`Frag`] whilst
/// dragging it, or changing the part heads whilst doing a transposition.
#[derive(Debug, Clone)]
pub enum State {
    /// The UI is idle, and the user is not actively performing an action
    Idle,
    /// The user is dragging the [`Frag`] at a given index.  In this `State` the `x`, `y` values of
    /// that particular [`Frag`] are allowed to get out of sync in the JS code to avoid unnecessary
    /// serialisation and undo steps.
    Dragging(usize),
    /// The user is transposing the [`Frag`] at a given index.
    Transposing {
        frag_ind: usize,
        row_ind: usize,
        /// The inverse of the part head visible when transposing started.  This is the [`Row`]
        /// that is used as a transposition to 'undo' the effect of the part head (so that the user
        /// can edit the [`Row`] they see on-screen, despite the fact that the underlying [`Row`]
        /// being edited is different to what they see.
        inv_part_head: Row,
    },
}

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
    state: State,
}

impl Comp {
    fn from_spec(spec: Spec) -> Comp {
        Comp {
            derived_state: DerivedState::from_spec(&spec),
            view: View::default(),
            undo_history: vec![spec],
            history_index: 0,
            state: State::Idle,
        }
    }

    fn spec(&self) -> &Spec {
        &self.undo_history[self.history_index]
    }

    /// Perform an fallible action (some arbitrary function which returns a `Result<Spec>`) on the
    /// current [`Spec`].  If this returns `Ok` then follow through with the edit - maintaining the
    /// undo history, recalculating the [`DerivedState`] and returning `Ok(())`.  If the action
    /// returns `Err` then that error is also returned from `make_fallible_action`.
    fn make_fallible_action<E>(
        &mut self,
        action: impl FnOnce(&Spec) -> Result<Spec, E>,
    ) -> Result<(), E> {
        // Make sure that we try the action **before** deleting the redo history -- if the user
        // performs an action which fails, we want them to not lose their redo history
        let new_spec = action(&mut self.undo_history[self.history_index])?;
        // Rollback the history so that `history_index` points to the last edit
        drop(self.undo_history.drain(self.history_index + 1..));
        // Add this modified Spec to the undo history, and make it the current one
        self.undo_history.push(new_spec);
        self.history_index += 1;
        // Rebuild the derived state, since the Spec has changed
        self.rebuild_state();
        Ok(())
    }

    /// Perform an action (some arbitrary function) on the current [`Spec`], maintaining the undo
    /// history and recalculating the [`DerivedState`].  This returns the value returned from the
    /// call of `action`.
    fn make_action<T>(&mut self, action: impl FnOnce(&mut Spec) -> T) -> T {
        // Rollback the history so that `history_index` points to the last edit
        drop(self.undo_history.drain(self.history_index + 1..));
        // Perform the required action on a clone of the Spec being displayed
        let mut new_spec = self.undo_history[self.history_index].clone();
        let result = action(&mut new_spec);
        // Add this modified Spec to the undo history, and make it the current one
        self.undo_history.push(new_spec);
        self.history_index += 1;
        // Rebuild the derived state, since the Spec has changed
        self.rebuild_state();
        result
    }

    /// Perform an action (some arbitrary function) on a single [`Frag`] in the current [`Spec`],
    /// maintaining the undo history and recalculating the [`DerivedState`].
    fn make_action_frag(&mut self, frag_ind: usize, action: impl Fn(&mut Frag)) {
        self.make_action(|spec| spec.make_action_frag(frag_ind, action));
    }
}

// Stuff required specifically for JS
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

    /// Attempt to parse a [`String`] into a [`Row`] of the correct [`Stage`] for this `Comp`.
    /// This returns `""` on success, and `"{error message}"` on failure.
    pub fn row_parse_err(&self, row_str: String) -> String {
        match Row::parse_with_stage(&row_str, self.spec().stage()) {
            Err(e) => format!("{}", e),
            Ok(_row) => "".to_owned(),
        }
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

    /* Idle State */

    /// Returns `true` if the editor is in [`State::Idle`]
    pub fn is_state_idle(&self) -> bool {
        match self.state {
            State::Idle => true,
            _ => false,
        }
    }

    /* Dragging State */

    /// Returns `true` if the editor is in [`State::Dragging`]
    pub fn is_state_dragging(&self) -> bool {
        match self.state {
            State::Dragging(_) => true,
            _ => false,
        }
    }

    /// Returns the index of the [`Frag`] being dragged, `panic!`ing if the UI is not in
    /// [`State::Dragging`].
    pub fn frag_being_dragged(&self) -> usize {
        if let State::Dragging(index) = self.state {
            index
        } else {
            unreachable!();
        }
    }

    /// Moves the UI into [`State::Dragging`], `panic!`ing if we start in any state other than
    /// [`State::Idle`]
    pub fn start_dragging(&mut self, frag_ind: usize) {
        assert!(self.is_state_idle());
        self.state = State::Dragging(frag_ind);
    }

    /// Called to exit [`State::Dragging`].  This moves the [`Frag`] the user was dragging to the
    /// provided coords (as a new undo step), and returns to [`State::Idle`].  This `panic!`s if
    /// called from any state other than [`State::Dragging`].
    pub fn finish_dragging(&mut self, new_x: f32, new_y: f32) {
        if let State::Dragging(frag_ind) = self.state {
            // Move the fragment we were dragging
            self.make_action_frag(frag_ind, |f| f.move_to(new_x, new_y));
            // Return to idle state (to release the UI)
            self.state = State::Idle;
        } else {
            unreachable!();
        }
    }

    /* Transposing State */

    /// Returns `true` if the editor is in [`State::Transposing`]
    pub fn is_state_transposing(&self) -> bool {
        match self.state {
            State::Transposing { .. } => true,
            _ => false,
        }
    }

    /// Moves the editor into [`State::Transposing`] the [`Frag`] at `frag_ind`.  This returns the
    /// string representation of the first [`Row`] of that [`Frag`], to initialise the
    /// transposition input box.  This `panic!`s if called from any state other than
    /// [`State::Idle`].
    pub fn start_transposing(&mut self, frag_ind: usize, row_ind: usize) -> String {
        assert!(self.is_state_idle());
        let part_ind = self.view.current_part;
        self.state = State::Transposing {
            frag_ind,
            row_ind,
            inv_part_head: !self.derived_state.get_part_head(part_ind).unwrap(),
        };
        self.derived_state
            .get_row(part_ind, frag_ind, row_ind)
            .unwrap()
            .to_string()
    }

    /// Called to exit [`State::Transposing`], saving the changes.  If `row_str` parses to a valid
    /// [`Row`] then this performs the desired transposition and returns the editor to
    /// [`State::Idle`] (returning `true`), otherwise no change occurs and this returns `false`.
    /// This `panic!`s if called from any state other than [`State::Transposing`].
    pub fn finish_transposing(&mut self, row_str: String) -> bool {
        // Switch the state to `State::Idle`, whilst also matching over the (moved) old state
        match std::mem::replace(&mut self.state, State::Idle) {
            State::Transposing {
                frag_ind,
                row_ind,
                inv_part_head,
            } => {
                let parsed_row = Row::parse_with_stage(&row_str, self.spec().stage());
                if let Ok(unpermuted_target_row) = &parsed_row {
                    let target_row = &inv_part_head * unpermuted_target_row;
                    self.make_action_frag(frag_ind, |f: &mut Frag| {
                        *f = f.transpose_row_to(row_ind, &target_row).unwrap();
                    });
                }
                parsed_row.is_ok()
            }
            _ => unreachable!(),
        }
    }

    /// Called to exit [`State::Transposing`], without saving the changes.  This `panic!`s if
    /// called from any state other than [`State::Transposing`].
    pub fn exit_transposing(&mut self) {
        assert!(self.is_state_transposing());
        self.state = State::Idle;
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

    /// See [`Spec::add_frag`] for docs
    pub fn add_frag(&mut self, x: f32, y: f32, add_course: bool) -> usize {
        // `self.make_action` bubbles through the return value from `Spec::add_frag`, which will
        // make sure we return the index of the newly added Frag
        self.make_action(|spec| spec.add_frag(x, y, add_course))
    }

    /// Deletes a [`Frag`]ment by index.
    pub fn delete_frag(&mut self, frag_ind: usize) {
        self.make_action(|spec| spec.delete_frag(frag_ind));
    }

    /// See [`Spec::join_frags`] for docs.
    pub fn join_frags(&mut self, frag_1_ind: usize, frag_2_ind: usize) {
        self.make_action(|spec| spec.join_frags(frag_1_ind, frag_2_ind));
    }

    /// Splits a given [`Frag`]ment into two fragments, returning `""` on success and an error
    /// string on failure. `split_index` refers to the first row of the 2nd fragment (so row
    /// #`split_index` will also be the new leftover row of the 1st subfragment).
    pub fn split_frag(&mut self, frag_ind: usize, split_index: usize, new_y: f32) -> String {
        self.make_fallible_action(|spec| spec.split_frag(frag_ind, split_index, new_y))
            .err()
            .map_or(String::new(), |e| e.to_string())
    }

    /// Toggle whether or not a given [`Frag`] is muted
    pub fn toggle_frag_mute(&mut self, frag_ind: usize) {
        self.make_action_frag(frag_ind, Frag::toggle_mute);
    }

    /// [`Frag`] soloing ala FL Studio; this has two cases:
    /// 1. `frag_ind` is the only unmuted [`Frag`], in which case we unmute everything
    /// 2. `frag_ind` isn't the only unmuted [`Frag`], in which case we mute everything except it
    pub fn toggle_frag_solo(&mut self, frag_ind: usize) {
        self.make_action(|spec: &mut Spec| spec.solo_single_frag(frag_ind));
    }

    /// Resets the composition to the example
    pub fn reset(&mut self) {
        self.make_action(|spec: &mut Spec| *spec = Spec::cyclic_s8());
    }

    /* View Setters */

    /// Moves the view's camera to a given location
    pub fn set_view_coords(&mut self, new_cam_x: f32, new_cam_y: f32) {
        self.view.view_x = new_cam_x;
        self.view.view_y = new_cam_y;
    }

    pub fn set_current_part(&mut self, new_part: usize) {
        self.view.current_part = new_part;
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
