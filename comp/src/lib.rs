#![allow(private_intra_doc_links)] // We're not exporting a public API, so internal docs are OK

mod expanded_frag;
pub mod full;
mod history;
mod music;
pub mod spec;

use bellframe::{music::Regex, Stage};
use history::History;

use spec::CompSpec;

pub use full::FullState;
pub use music::Music;

// Imports only used for doc comments
#[allow(unused_imports)]
use bellframe::Row;

/// The internal composition 'model' of Jigsaw
#[derive(Debug)]
pub struct State {
    /// Undo history of anything which changes the [`Row`]s of the composition (methods, calls,
    /// fragments, part heads, etc.)
    history: History,
    /// The types of music that Jigsaw cares about
    music_groups: Vec<Music>,
    /// The fully specified state, cached between frames and used to draw the GUI
    full_state: FullState,
}

impl State {
    //////////////////
    // CONSTRUCTORS //
    //////////////////

    /// The state that Jigsaw will be in the first time the user starts up.
    pub fn example() -> Self {
        // For the time being, just create an empty composition of Major
        Self::new(
            CompSpec::example(),
            vec![
                Music::Group(
                    "56s/65s".to_owned(),
                    vec![
                        Music::Regex(Some("65s".to_owned()), Regex::parse("*6578")),
                        Music::Regex(Some("56s".to_owned()), Regex::parse("*5678")),
                    ],
                ),
                Music::runs_front_and_back(Stage::MAJOR, 4),
                Music::runs_front_and_back(Stage::MAJOR, 5),
                Music::runs_front_and_back(Stage::MAJOR, 6),
                Music::runs_front_and_back(Stage::MAJOR, 7),
                Music::Regex(Some("Queens".to_owned()), Regex::parse("13572468")),
            ],
        )
    }

    /// Creates a [`Jigsaw`] struct displaying a single [`CompSpec`], with no other undo history.
    pub(crate) fn new(spec: CompSpec, music_groups: Vec<Music>) -> Self {
        let full_state = FullState::new(&spec, &music_groups);
        let history = History::new(spec);
        Self {
            history,
            music_groups,
            full_state,
        }
    }

    ///////////////
    // MODIFIERS //
    ///////////////

    /// Move one step back through the undo history, returning `false` if we're already at the
    /// oldest undo step
    pub fn undo(&mut self) -> bool {
        let was_undo_possible = self.history.undo();
        if was_undo_possible {
            self.rebuild_full_state();
        }
        was_undo_possible
    }

    /// Move one step forward through the undo history, returning `false` if we're already at the
    /// most recent undo step
    pub fn redo(&mut self) -> bool {
        let was_redo_possible = self.history.redo();
        if was_redo_possible {
            self.rebuild_full_state();
        }
        was_redo_possible
    }

    /// Apply a closure to modify current [`CompSpec`], thus creating a new step in the undo
    /// history.  If the closure returns `Err(e)` then no changes are made
    pub fn apply_edit<O, E>(
        &mut self,
        edit: impl FnOnce(&mut CompSpec) -> Result<O, E>,
    ) -> Result<O, E> {
        let edit_value = self.history.apply_edit(edit)?;
        self.rebuild_full_state(); // Make sure that `self.full_state` reflects the edit
        Ok(edit_value) // Bubble the result
    }

    /// Apply a closure to modify current [`CompSpec`], thus creating a new step in the undo
    /// history
    pub fn apply_infallible_edit<R>(&mut self, edit: impl FnOnce(&mut CompSpec) -> R) -> R {
        let result = self.history.apply_infallible_edit(edit);
        self.rebuild_full_state(); // Make sure that `self.full_state` reflects the edit
        result // Bubble the result
    }

    /// Update `self.full_state` so that it is up-to-date with any changes to `self`
    pub fn rebuild_full_state(&mut self) {
        self.full_state
            .update(self.history.comp_spec(), &self.music_groups);
    }

    /////////////
    // GETTERS //
    /////////////

    /// Gets the fully expanded state of the composition
    pub fn full(&self) -> &FullState {
        &self.full_state
    }

    pub fn music_groups(&self) -> &[Music] {
        self.music_groups.as_slice()
    }
}
