mod full;
mod history;
mod music;
pub mod spec;

use bellframe::{music::Regex, Stage};
use full::FullState;
use history::History;

use spec::CompSpec;

pub use music::Music;

/// The internal composition 'model' of Jigsaw
#[derive(Debug, Clone)]
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
    pub(crate) fn new(spec: CompSpec, music_classes: Vec<Music>) -> Self {
        let full_state = FullState::from_spec(&spec);
        let history = History::new(spec);
        Self {
            full_state,
            history,
            music_groups: music_classes,
        }
    }

    /// Gets the fully expanded state of the composition
    pub(crate) fn full(&self) -> &FullState {
        &self.full_state
    }

    pub fn music_groups(&self) -> &[Music] {
        self.music_groups.as_slice()
    }
}
