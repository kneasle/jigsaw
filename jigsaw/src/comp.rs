use crate::{
    derived_state::DerivedState,
    spec::{Frag, PartHeads, Spec},
    view::View,
};
use proj_core::{place_not::PnBlockParseError, PnBlock, Row};
use serde::Serialize;
use std::convert::TryFrom;
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
        /// A [`Spec`] that has already implemented the transposition.  This will be displayed to
        /// the user (without changing history) until the user presses 'Enter' at which point it
        /// will be 'committed' as the next stage in the edit history.
        spec: Spec,
    },
    /// The user is editing a [`MethodSpec`]
    EditingMethod {
        /// The index of the method which we are editing, or `None` if we are creating a new
        /// [`MethodSpec`]
        index: Option<usize>,
        /// The current state of the edit on-screen
        edit: MethodEdit,
    },
}

impl State {
    /// If this `State` can edit the [`Spec`] in real time, then return the currently edited
    /// [`Spec`].
    fn spec(&self) -> Option<&Spec> {
        match self {
            State::Transposing { spec, .. } => Some(spec),
            _ => None,
        }
    }
}

/// The state of a currently edited method.  Note that this can represent invalid states, and can't
/// always be converted back into a [`MethodSpec`]
#[derive(Serialize, Debug, Clone)]
pub struct MethodEdit {
    name: String,
    shorthand: String,
    // TODO: Add serde as a feature flag to `core`
    #[serde(serialize_with = "crate::ser_utils::ser_stage")]
    stage: Stage,
    place_not_string: String,
    #[serde(rename = "pn_parse_err")]
    #[serde(serialize_with = "crate::ser_utils::ser_pn_result")]
    parsed_pn_block: Result<PnBlock, PnBlockParseError>,
}

impl MethodEdit {
    /// Creates a `MethodEdit` for a newly created method (i.e. with all the fields blank)
    fn empty(stage: Stage) -> Self {
        Self::with_pn_string(String::new(), String::new(), stage, String::new())
    }

    /// Creates a `MethodEdit` with an existing place notation string
    pub fn with_pn_string(
        name: String,
        shorthand: String,
        stage: Stage,
        place_not_string: String,
    ) -> Self {
        MethodEdit {
            name,
            shorthand,
            parsed_pn_block: PnBlock::parse(&place_not_string, stage),
            place_not_string,
            stage,
        }
    }
}

/// The complete state of the composition editor, complete with undo history and UI/view state.
///
/// The general data-flow is:
/// - User generates some input (keypress, mouse click, etc.)
/// - JS reads this input and calls one of the `#[wasm_bindgen]` methods on `Comp`
/// - These call some `self.make_*action*` function which runs a given closure on the existing
///   [`Spec`]
///   - This also handles the undo history (i.e. doesn't overwrite old copies, and deallocates
///     future redo steps that are now unreachable).
///   - Because the [`Spec`] has changed, we rebuild the [`DerivedState`] from this new [`Spec`].
///     This is necessary because JS can't access the [`Spec`] directly.
/// - The following all happens during the call to the JS `on_comp_change()` method:
///   - After every edit, JS will call [`Comp::ser_derived_state`] which returns a JSON
///     serialisation of the current [`DerivedState`], which is parsed into a full-blown JS object
///     and the global `derived_state` variable is overwritten with this new value.
///   - The HUD UI (sidebar, etc.) are all updated to this new value
///   - A repaint is requested, so that the updated [`DerivedState`] gets fully rendered to the
///   screen.
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

    /// Gets the [`Spec`] that is currently viewed by this `Comp`.
    fn spec(&self) -> &Spec {
        self.state
            .spec()
            .unwrap_or(&self.undo_history[self.history_index])
    }

    /// Rebuild `self.derived_state` from `self.spec()`.  This should be called whenever
    /// `self.spec()` changes, but does not actually check whether or not any change has occurred -
    /// it will still do a full rebuild even if nothing has been changed.
    fn rebuild_state(&mut self) {
        self.derived_state = DerivedState::from_spec(self.spec());
        // Clamp the currently viewed part to within the range of possible parts in the composition
        // (because the number of parts might have changed by this edit)
        self.view.current_part = self.view.current_part.min(self.spec().num_parts() - 1);
    }

    /// Perform an action (some arbitrary function) on the current [`Spec`], maintaining the undo
    /// history and recalculating the [`DerivedState`].  This returns the value returned from the
    /// call of `action`.
    fn make_action<T>(&mut self, action: impl FnOnce(&mut Spec) -> T) -> T {
        // Perform the required action on a clone of the Spec being displayed
        let mut new_spec = self.undo_history[self.history_index].clone();
        let result = action(&mut new_spec);
        // Actually make that action present
        self.finish_action(new_spec);
        // 'bubble' the return value of the action out of this function too
        result
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
        let new_spec = action(self.spec())?;
        self.finish_action(new_spec);
        Ok(())
    }

    /// Given a `new_spec` to append to the undo history, this actually mutates `self` to make the
    /// edit take effect.  This handles things like maintaining the undo history, rebuilding the
    /// state, and enforcing bounds checks.
    fn finish_action(&mut self, new_spec: Spec) {
        // Rollback the history so that `history_index` points to the last edit
        drop(self.undo_history.drain(self.history_index + 1..));
        // Add this modified Spec to the undo history, and make it the current one
        self.undo_history.push(new_spec);
        self.history_index += 1;
        // Rebuild the derived state, since the Spec has changed
        self.rebuild_state();
    }

    /// Perform an action (some arbitrary function) on a single [`Frag`] in the current [`Spec`],
    /// maintaining the undo history and recalculating the [`DerivedState`].
    fn make_action_frag(&mut self, frag_ind: usize, action: impl Fn(&mut Frag)) {
        self.make_action(|spec| spec.make_action_frag(frag_ind, action));
    }
}

/// Functions exported to JavaScript.  These functions are the _only_ way that Rust and JavaScript
/// can interact directly.
#[wasm_bindgen]
impl Comp {
    /// Create an example composition
    pub fn example() -> Comp {
        console_error_panic_hook::set_once();
        Self::from_spec(Spec::example())
    }

    /// Attempt to parse a new part head specification [`String`].  If it successfully parses then
    /// update the part head list as a new edit (returning `""`), otherwise return a [`String`]
    /// summarising the issue with the parsing.
    pub fn parse_part_head_spec(&mut self, s: String) -> String {
        match PartHeads::parse(&s, self.spec().stage()) {
            // If it parsed correctly, then we update the part heads and return ""
            Ok(part_heads) => {
                // Only make an undo step if the part heads have actually changed
                if self.spec().part_heads() != &part_heads {
                    self.make_action(|spec: &mut Spec| spec.set_part_heads(part_heads));
                }
                "".to_owned()
            }
            // If the parsing failed, then we return the error message
            Err(e) => format!("{}", e),
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
        matches!(self.state, State::Idle)
    }

    /* Dragging State */

    /// Returns `true` if the editor is in [`State::Dragging`]
    pub fn is_state_dragging(&self) -> bool {
        matches!(self.state, State::Dragging(_))
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
        matches!(self.state, State::Transposing { .. })
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
            // We only store the inverse of the currently viewed part head, because we'll need it
            // in order to make the right transposition
            inv_part_head: !self.derived_state.get_part_head(part_ind).unwrap(),
            spec: self.spec().clone(),
        };
        // Return the String representation of the currently visible Row at the specified location
        self.derived_state
            .get_row(part_ind, frag_ind, row_ind)
            .unwrap()
            .to_string()
    }

    /// Attempt to parse a [`String`] into a [`Row`] of the correct [`Stage`] for this `Comp`, to
    /// be used in [`State::Transposing`].  There are two possible outcomes:
    /// - **The string corresponds to a valid [`Row`]**: This parsed [`Row`] is used to modify
    ///   the temporary [`Spec`] contained with in the [`State::Transposing`] enum.  The
    ///   [`DerivedState`] is updated and `""` is returned.
    /// - **The string creates a parse error**:  No modification is made, and a [`String`]
    ///   representing the error is returned.
    /// This `panic!`s if called from any state other than [`State::Transposing`].
    pub fn try_parse_transpose_row(&mut self, row_str: String) -> String {
        let parsed_row = Row::parse_with_stage(&row_str, self.spec().stage());
        match &mut self.state {
            State::Transposing {
                spec,
                inv_part_head,
                frag_ind,
                row_ind,
            } => match parsed_row {
                Err(e) => format!("{}", e),
                Ok(unpermuted_target_row) => {
                    let target_row = &*inv_part_head * &unpermuted_target_row;
                    spec.get_frag_mut(*frag_ind)
                        .unwrap()
                        .transpose_row_to(*row_ind, &target_row)
                        .unwrap();
                    self.rebuild_state();
                    "".to_owned()
                }
            },
            _ => unreachable!(),
        }
    }

    /// Called to exit [`State::Transposing`], saving the changes.  If `row_str` parses to a valid
    /// [`Row`] then this commits the desired transposition and returns the editor to
    /// [`State::Idle`] (returning `true`), otherwise no change occurs and this returns `false`.
    /// This `panic!`s if called from any state other than [`State::Transposing`].
    pub fn finish_transposing(&mut self, row_str: String) -> bool {
        // Early return false if the
        if Row::parse_with_stage(&row_str, self.spec().stage()).is_err() {
            return false;
        }
        // Switch the state to `State::Idle`, whilst also matching over the (moved) old state
        match std::mem::replace(&mut self.state, State::Idle) {
            State::Transposing { spec, .. } => {
                // We are already displaying the resulting `Spec` to the user, so we don't need to
                // perform the modification again, we just consume the value from `self.state` and
                // finish the action
                self.finish_action(spec);
            }
            _ => unreachable!(),
        }
        true
    }

    /// Called to exit [`State::Transposing`], **without** saving the changes.  This `panic!`s if
    /// called from any state other than [`State::Transposing`].
    pub fn exit_transposing(&mut self) {
        assert!(self.is_state_transposing());
        self.state = State::Idle;
        // `State::Transposing` modifies its own `Spec`, so we have to rebuild the state when we
        // are exiting transposing mode in order to revert the state of the display
        self.rebuild_state();
    }

    /* Method Editing */

    /// Returns `true` if the editor is in [`State::EditingMethod`]
    pub fn is_state_editing_method(&self) -> bool {
        matches!(self.state, State::EditingMethod { .. })
    }

    /// Starts editing the [`MethodSpec`] at a given index
    pub fn start_editing_method(&mut self, index: usize) {
        assert!(self.is_state_idle());
        self.state = State::EditingMethod {
            index: Some(index),
            edit: self.spec().get_method_spec(index).unwrap().to_edit(),
        };
    }

    /// Starts editing a new [`MethodSpec`], which will get added at the end
    pub fn start_editing_new_method(&mut self) {
        assert!(self.is_state_idle());
        self.state = State::EditingMethod {
            index: None,
            edit: MethodEdit::empty(self.spec().stage()),
        };
    }

    /// Return all the information required for JS to update the method edit box, serialised as
    /// JSON
    pub fn method_edit_state(&self) -> String {
        match &self.state {
            State::EditingMethod { edit, .. } => serde_json::to_string(edit).unwrap(),
            _ => unreachable!(),
        }
    }

    /// Sets both the name and shorthand of the method being edited
    pub fn set_method_names(&mut self, new_name: String, new_shorthand: String) {
        match &mut self.state {
            State::EditingMethod { edit, .. } => {
                edit.name = new_name;
                edit.shorthand = new_shorthand;
            }
            _ => unreachable!(),
        }
    }

    /// Sets the place notatation string in the method edit box, and reparses to generate a new
    /// error.  Called whenever the user types into the method box
    pub fn set_method_pn(&mut self, new_pn: String) {
        match &mut self.state {
            State::EditingMethod { edit, .. } => {
                edit.parsed_pn_block = PnBlock::parse(&new_pn, edit.stage);
                edit.place_not_string = new_pn;
            }
            _ => unreachable!(),
        }
    }

    /// Exit method editing mode, without commiting any of the changes
    pub fn exit_method_edit(&mut self) {
        assert!(self.is_state_editing_method());
        self.state = State::Idle;
    }

    /// Exit method editing mode, commiting the new method to the composition if valid.  This
    /// returns `false` if no change occured
    pub fn finish_editing_method(&mut self) -> bool {
        match std::mem::replace(&mut self.state, State::Idle) {
            State::EditingMethod { edit, index } => {
                // Extract the place notation block of the new method, and return false if it
                // doesn't exist
                let pn_block = match edit.parsed_pn_block {
                    Ok(p) => p,
                    Err(_) => return false,
                };
                // Move all the values _outside_ the closure, so that the borrow checker
                // understands that this is acceptable
                let name = edit.name;
                let shorthand = edit.shorthand;
                let place_not_string = edit.place_not_string;
                // Perform the action
                self.make_action(|spec| {
                    spec.edit_method(index, name, shorthand, pn_block, place_not_string)
                });
            }
            _ => unreachable!(),
        }
        true
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

    /// See [`Spec::extend_frag_end`] for docs
    pub fn extend_frag(&mut self, frag_ind: usize, method_ind: usize, add_course: bool) {
        // `self.make_action` bubbles through the return value from `Spec::add_frag`, which will
        // make sure we return the index of the newly added Frag
        self.make_action(|spec| spec.extend_frag_end(frag_ind, method_ind, add_course));
    }

    /// See [`Spec::add_frag`] for docs
    pub fn add_frag(&mut self, x: f32, y: f32, method_ind: usize, add_course: bool) -> usize {
        // `self.make_action` bubbles through the return value from `Spec::add_frag`, which will
        // make sure we return the index of the newly added Frag
        self.make_action(|spec| spec.add_frag(x, y, method_ind, add_course))
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

    /// Replace the call at the end of a composition.  Calls are referenced by their index, and any
    /// negative number will correspond to removing a call.  See [`Spec::set_call`] for more docs.
    pub fn set_call(&mut self, frag_ind: usize, row_ind: usize, call_ind: isize) -> String {
        self.make_fallible_action(|spec| {
            spec.set_call(frag_ind, row_ind, usize::try_from(call_ind).ok())
        })
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

    /// Toggles the lead folding at a given **on screen** row index.  This doesn't update the undo
    /// history.
    pub fn toggle_lead_fold(&mut self, frag_ind: usize, on_screen_row_ind: usize) {
        // Figure out which source row the on-screen row actually corresponds to
        let foldable_row = self
            .derived_state
            .last_lead_foldable_row(frag_ind, on_screen_row_ind);
        self.spec().toggle_lead_fold(
            frag_ind,
            self.derived_state.source_row_ind(frag_ind, foldable_row),
        );
        self.rebuild_state();
    }

    /// Remove a method from the list, if it doesn't appear in the composition
    pub fn remove_method(&mut self, method_ind: usize) -> String {
        match self.derived_state.is_method_used(method_ind) {
            Some(false) => {
                // Only perform the action if the method exists but isn't rung
                self.make_action(|spec| spec.remove_method(method_ind));
                ""
            }
            Some(true) => "Can't remove a method that's used in the composition.",
            None => "Method index out of range",
        }
        .to_owned()
    }

    /// Change the shorthand name of a method
    pub fn set_method_shorthand(&mut self, method_ind: usize, new_name: String) {
        self.spec().set_method_shorthand(method_ind, new_name);
        self.rebuild_state();
    }

    /// Change the full name of a method (without causing an undo history
    pub fn set_method_name(&mut self, method_ind: usize, new_name: String) {
        self.spec().set_method_name(method_ind, new_name);
        self.rebuild_state();
    }

    /// Resets the composition to the example
    pub fn reset(&mut self) {
        // We directly finish the action because we are fully overwriting it, and  calling
        // `self.make_action` would likely clone then immediately drop the current Spec
        self.finish_action(Spec::example());
    }

    /* View Setters */

    /// Moves the view's camera to a given location
    pub fn set_view_coords(&mut self, new_cam_x: f32, new_cam_y: f32) {
        self.view.view_x = new_cam_x;
        self.view.view_y = new_cam_y;
    }

    /// Sets the current part being viewed
    pub fn set_current_part(&mut self, new_part: usize) {
        self.view.current_part = new_part;
    }

    /// Toggles the foldedness of the method section, returning `false` if no section with that
    /// name exists.
    pub fn toggle_section_fold(&mut self, section_name: String) -> bool {
        self.view.section_folds.toggle(&section_name)
    }

    /// Toggles the foldedness of a specific method panel
    pub fn toggle_method_fold(&mut self, method_ind: usize) {
        let cell = self.spec().method_panel_cell(method_ind).unwrap();
        let v = cell.get();
        cell.set(!v);
    }

    /// Returns whether or not a given method info panel is open
    // TODO/PERF: Turn `View` into something similar to `DerivedState`, which aggregates its data
    // from some internal view structure and a `Spec`.  For now, though, the performance is
    // adequate.
    pub fn is_method_panel_open(&mut self, method_ind: usize) -> bool {
        self.spec().method_panel_cell(method_ind).unwrap().get()
    }
}

#[cfg(test)]
mod tests {
    use super::Comp;

    #[test]
    fn example_doesnt_crash() {
        let c = Comp::example();
        c.ser_derived_state();
        c.ser_view();
    }

    #[test]
    fn replace_call() {
        let mut c = Comp::example();
        c.set_call(0, 31, -1);
    }
}
