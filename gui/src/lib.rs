//! Top-level code for Jigsaw's GUI

use canvas::{CanvasResponse, FragHover};
use eframe::{
    egui::{self, PointerButton, Pos2, Vec2},
    epi,
};

use jigsaw_comp::{
    full::FullState,
    spec::{self, part_heads::PartHeads, CompSpec},
    History,
};
use jigsaw_utils::indexed_vec::{FragIdx, PartIdx};

use self::config::Config;

mod canvas;
mod config;
mod side_panel;

// Imports only used for doc comments
#[allow(unused_imports)]
use bellframe::Row;

/// The top-level singleton for Jigsaw.  This isn't [`Clone`] because it is a singleton - at any
/// time, there should be at most one copy of it in existence.
#[derive(Debug)]
pub struct JigsawApp {
    config: Config,

    /// Undo history of anything which changes the [`Row`]s of the composition (methods, calls,
    /// fragments, part heads, etc.)
    history: History,
    /// The fully specified state, cached between frames and used to draw the GUI
    full_state: FullState,

    /* GUI state */
    /// The text currently in the part head UI box.  Whilst the user is typing, this can become
    /// invalid, and therefore must be able to diverge from `self.history`
    part_head_str: String,
    camera_pos: Pos2,
}

impl JigsawApp {
    /// Load an example composition
    pub fn example() -> Self {
        let spec = CompSpec::example();
        let full_state = FullState::new(&spec);
        let part_head_str = full_state.part_heads.spec_string();

        Self {
            config: Config::default(),

            history: History::new(spec),
            full_state,

            part_head_str,
            camera_pos: Pos2::ZERO,
        }
    }
}

impl epi::App for JigsawApp {
    fn name(&self) -> &str {
        "Jigsaw"
    }

    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        // To prevent bugs (and appease the borrow checker), Jigsaw's app is **immutable** during
        // both drawing and input gathering.  When the GUI wants to make changes to the app's state
        // without using interior mutability (e.g. because the user typed into the part head box,
        // or input a keyboard shortcut), then this change is represented as an `Action` and pushed
        // to a list of `actions` which will all be applied at the end of the frame.

        let mut actions = Vec::<Action>::new(); // These all take effect at the end of the frame

        let canvas_response = self.draw_gui(ctx, |a| actions.push(a));

        // PERF: Handling inputs **before** rendering the GUI would save a frame of latency
        self.handle_input(ctx, canvas_response, |action| actions.push(action));

        /* APPLY ALL ACTIONS */
        for action in actions {
            self.apply_action(action);
        }
    }

    fn max_size_points(&self) -> egui::Vec2 {
        // Increase the max size so that we can fill the page when running in a browser
        Vec2::new(5000.0, 3000.0)
    }
}

impl JigsawApp {
    //////////////
    // DRAW GUI //
    //////////////

    fn draw_gui(&self, ctx: &egui::CtxRef, push_action: impl FnMut(Action)) -> CanvasResponse {
        // Draw right-hand panel, and decide which rows should be highlighted
        let rows_to_highlight =
            side_panel::draw(ctx, &self.full_state, &self.part_head_str, push_action);
        // Draw the main canvas
        canvas::draw(
            ctx,
            &self.full_state,
            &self.config,
            self.camera_pos,
            rows_to_highlight,
            PartIdx::new(0), // Always display the first part until we can change this
        )
    }

    ////////////////////
    // INPUT HANDLING //
    ////////////////////

    /// Handle all input for this frame
    fn handle_input(
        &self,
        ctx: &egui::CtxRef,
        canvas_response: CanvasResponse,
        mut push_action: impl FnMut(Action),
    ) {
        // Keyboard events
        for evt in &ctx.input().events {
            if let egui::Event::Key {
                key,
                pressed,
                modifiers,
            } = *evt
            {
                if !ctx.wants_keyboard_input() && pressed {
                    if let Some(comp_action) =
                        self.handle_key_press(key, modifiers, canvas_response.frag_hover.as_ref())
                    {
                        push_action(Action::Comp(comp_action));
                    }
                }
            }
        }

        // Pan the canvas
        if canvas_response.inner.dragged_by(PointerButton::Middle) {
            push_action(Action::PanView(-canvas_response.inner.drag_delta()));
        }
    }

    /// Handle a keyboard key being pressed down
    #[must_use]
    fn handle_key_press(
        &self,
        key: egui::Key,
        modifiers: egui::Modifiers,
        frag_hover: Option<&FragHover>,
    ) -> Option<CompAction> {
        use egui::Key::*;

        // z with any set of modifiers is undo
        if key == Z && !modifiers.shift {
            return Some(CompAction::UndoRedo(HistoryDirection::Undo));
        }
        // Z, y or Y with any set of modifiers is redo
        if (key == Z && modifiers.shift) || key == Y {
            return Some(CompAction::UndoRedo(HistoryDirection::Redo));
        }

        // Actions which apply to a fragment under the cursor
        if let Some(frag_hover) = frag_hover {
            let action = match (key, modifiers.shift) {
                // d or D to delete the fragment under the cursor
                (D, _) => Some(CompAction::DeleteFragment(frag_hover.frag_idx)),
                // x to split the fragment at the nearest rule-off
                (X, false) => self.split_fragment(frag_hover, FragSplitLocation::NearestRuleoff),
                // X to split the hovered fragment at the cursor
                (X, true) => self.split_fragment(frag_hover, FragSplitLocation::NearestRow),
                // s to mute/unmute the fragment under the cursor
                (S, false) => Some(CompAction::MuteFragment(frag_hover.frag_idx)),
                // S to solo the fragment under the cursor
                (S, true) => Some(CompAction::SoloFragment(frag_hover.frag_idx)),

                // All other key presses are ignored
                _ => None,
            };
            // Return if this keyboard shortcut corresponds to an action (this is basically the
            // reverse of the `?` sigil).
            if let Some(action) = action {
                return Some(action);
            }
        }

        None
    }

    fn split_fragment(
        &self,
        frag_hover: &FragHover,
        location: FragSplitLocation,
    ) -> Option<CompAction> {
        let fragment = &self.full_state.fragments[frag_hover.frag_idx];

        // Decide which index to split the fragment
        let split_index = match location {
            FragSplitLocation::NearestRuleoff => fragment
                // Snap to the nearest rule-off ...
                .nearest_ruleoff_to(frag_hover.row_idx_float)
                // ... unless it's too far away ...
                .filter(|(_idx, dist)| *dist < self.config.ruleoff_snap_distance)
                // ... remove the distance
                .map(|(idx, _dist)| idx.index() as isize)?,
            FragSplitLocation::NearestRow => frag_hover.nearest_row_boundary(),
        };
        // Compute the position of the new fragment
        let pos_of_new_frag = fragment.position
            + Vec2::DOWN * self.config.row_height * (split_index as f32 + self.config.split_height);
        Some(CompAction::SplitFragment {
            frag_idx: frag_hover.frag_idx,
            split_index,
            pos_of_new_frag,
        })
    }

    ///////////////////
    // APPLY ACTIONS //
    ///////////////////

    fn apply_action(&mut self, action: Action) {
        match action {
            Action::PanView(delta) => self.camera_pos += delta,
            Action::SetPartHeadString(new_part_head_str) => self.part_head_str = new_part_head_str,
            Action::Comp(comp_action) => {
                if let Err(e) = self.apply_comp_action(comp_action) {
                    println!("EDIT ERROR: {:?}", e);
                }
            }
        }
    }

    fn apply_comp_action(&mut self, action: CompAction) -> Result<(), ActionError> {
        match action {
            /* UPDATES WHICH CHANGE THE COMPOSITION */
            CompAction::UndoRedo(direction) => {
                let was_successful = match direction {
                    HistoryDirection::Undo => self.history.undo(),
                    HistoryDirection::Redo => self.history.redo(),
                };
                if !was_successful {
                    // Abort with an error if the undo wasn't possible
                    return Err(ActionError::NoSteps(direction));
                }
                // Update the part head box, since we have potentially changed the part heads.  If
                // we don't do this, then the code will notice that the contents of the part head
                // box is different to the current part heads, and promptly creates a new undo step
                // to change them.
                //
                // TODO: Don't update the box if the user is part-way through editing it?
                self.part_head_str = self.full_state.part_heads.spec_string();
            }
            CompAction::SetPartHeads(new_part_heads) => {
                self.history
                    .apply_infallible_edit(|spec| spec.set_part_heads(new_part_heads));
            }
            CompAction::SoloFragment(frag_idx) => {
                self.history.apply_edit(|spec| spec.solo_frag(frag_idx))?
            }
            CompAction::MuteFragment(frag_idx) => self
                .history
                .apply_frag_edit(frag_idx, |frag| frag.toggle_mute())?,
            CompAction::DeleteFragment(frag_idx) => self
                .history
                .apply_edit(|spec| spec.delete_fragment(frag_idx))?,
            CompAction::SplitFragment {
                frag_idx,
                split_index,
                pos_of_new_frag,
            } => self
                .history
                .apply_edit(|spec| spec.split_fragment(frag_idx, split_index, pos_of_new_frag))?,
        }
        // If the edit succeeded, rebuild `self.full_state` so that the new changes are rendered
        self.full_state.update(&self.history.comp_spec());
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
enum FragSplitLocation {
    NearestRow,
    NearestRuleoff,
}

/*
/// The possible ways that the state of `JigsawApp` can be mutated.  These can be randomly
/// generated to test the app without the overhead of running a full GUI.
#[derive(Debug, Clone)]
pub(crate) enum Action {
    /// Update the 'Part Heads' box to some new value
    SetPartHeadString(String),
    /// Updates the [`PartHeads`] of the current [`CompSpec`]
    SetPartHeads(PartHeads),
    /// Undo or redo (which are similar enough to be handled as one case)
    UndoRedo(HistoryDirection),
    /// Delete a fragment
    DeleteFragment(FragIdx),
    /// Split a fragment at a given row
    SplitFragment {
        frag_idx: FragIdx,
        split_index: isize,
        pos_of_new_frag: Pos2,
    },
}
*/

/// The possible ways that the state of `JigsawApp` can be mutated.  These can be randomly
/// generated to test the app without the overhead of running a full GUI.
#[derive(Debug, Clone)]
pub(crate) enum Action {
    /// Pan the canvas view.  Note that this refers to the position of the 'camera', not the
    /// positions of the canvas (so increasing both axis corresponds to the fragments moving
    /// up and left).
    PanView(Vec2),
    /// Update the 'Part Heads' box to some new value
    SetPartHeadString(String),
    /// Make an edit to the composition
    Comp(CompAction),
}

/// Actions which modify the composition
#[derive(Debug, Clone)]
pub(crate) enum CompAction {
    /// Updates the [`PartHeads`] of the current [`CompSpec`]
    SetPartHeads(PartHeads),
    /// Undo or redo (which are similar enough to be handled as one case)
    UndoRedo(HistoryDirection),
    MuteFragment(FragIdx),
    SoloFragment(FragIdx),
    /// Delete a fragment
    DeleteFragment(FragIdx),
    /// Split a fragment at a given row
    SplitFragment {
        frag_idx: FragIdx,
        split_index: isize,
        pos_of_new_frag: Pos2,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum ActionError {
    /// The user tried to undo/redo when there were no steps in that direction
    NoSteps(HistoryDirection),
    /// There was an error whilst modifying the [`CompSpec`]
    EditError(spec::EditError),
}

/// Allow `?` to implicitly wrap [`spec::EditError`]s into [`ActionError`]s
impl From<spec::EditError> for ActionError {
    fn from(e: spec::EditError) -> Self {
        ActionError::EditError(e)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub(crate) enum HistoryDirection {
    Undo,
    Redo,
}
