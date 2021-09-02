//! Code for maintaining and navigating an undo history.

use std::{collections::VecDeque, iter};

use super::spec::CompSpec;

/// An undo history of the composition being edited by Jigsaw.
#[derive(Debug, Clone)]
pub struct History {
    /// The sequence of [`CompSpec`]s representing the most recent undo history.  This is ordered
    /// chronologically with the most recent edit at the end.
    history: VecDeque<CompSpec>,
    /// The index within `history` of the [`CompSpec`] being currently displayed.  Redo and undo
    /// corresponds to incrementing/decrementing this pointer, respectively.
    current_undo_index: usize,
}

impl History {
    /// Creates a new [`History`] containing only one [`CompSpec`]
    pub(crate) fn new(spec: CompSpec) -> Self {
        Self {
            history: iter::once(spec).collect(),
            current_undo_index: 0,
        }
    }

    /// Moves one step backwards in the undo history.  Returns `false` if we are already on the
    /// oldest undo step.
    pub fn undo(&mut self) -> bool {
        if self.current_undo_index == 0 {
            false
        } else {
            self.current_undo_index -= 1;
            true
        }
    }

    /// Moves one step forwards in the undo history.  Returns `false` if we are already on the
    /// most recent undo step.
    pub fn redo(&mut self) -> bool {
        if self.current_undo_index == self.history.len() - 1 {
            false
        } else {
            self.current_undo_index += 1;
            true
        }
    }

    /// Apply a closure to modify current [`CompSpec`], thus creating a new step in the undo
    /// history
    pub fn apply_edit<O, E>(
        &mut self,
        edit: impl FnOnce(&mut CompSpec) -> Result<O, E>,
    ) -> Result<O, E> {
        // Apply the edit to a clone of the current spec
        let mut new_spec = self.comp_spec().clone();
        let edit_value = edit(&mut new_spec)?;
        // Add this new spec to the undo history
        self.append_history(new_spec);
        // Bubble the result
        Ok(edit_value)
    }

    /// Apply a closure to modify current [`CompSpec`], thus creating a new step in the undo
    /// history
    pub fn apply_infallible_edit<R>(&mut self, edit: impl FnOnce(&mut CompSpec) -> R) -> R {
        // Apply the edit to a clone of the current spec
        let mut new_spec = self.comp_spec().to_owned();
        let result = edit(&mut new_spec);
        // Add this new spec to the undo history
        self.append_history(new_spec);
        result // bubble the result
    }

    /// Add a new [`CompSpec`] to the undo history, after the [`CompSpec`] currently being viewed.
    fn append_history(&mut self, new_spec: CompSpec) {
        // Before making the edit, remove any undo history that happens **after** the current edit
        // (i.e. edits which could be redone).  This will be **replaced** by the new change
        self.history.drain(self.current_undo_index + 1..);
        // Add the new entry, and update the pointer to point to it
        self.history.push_back(new_spec);
        self.current_undo_index += 1;
        // Sanity check that `self.current_undo_index` should point to the last snapshot in the
        // history.  This should be guaranteed because we `drain` everything else
        assert_eq!(self.current_undo_index, self.history.len() - 1);
        // TODO: Possibly drop old history if the chain gets too long
    }

    pub(crate) fn comp_spec(&self) -> &CompSpec {
        &self.history[self.current_undo_index]
    }
}
