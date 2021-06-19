use crate::ser_utils::get_true;
use serde::{Deserialize, Serialize};

macro_rules! define_section_folds {
    ( $( $n: ident ),* ) => {
        /// A data structure which stores the foldedness of every sidebar element
        #[derive(Serialize, Deserialize, Debug, Clone)]
        pub struct SectionFolds {
            // Generate all the fields with annotations
            $(
                #[serde(default = "get_true")]
                pub $n: bool
            ),*
        }

        // All section folds should default to open
        impl Default for SectionFolds {
            fn default() -> Self {
                SectionFolds {
                    $( $n: true, )*
                }
            }
        }

        impl SectionFolds {
            /// Toggle the folding of the a given section by name, returning `false` if no such
            /// section exists.
            #[must_use]
            pub fn toggle(&mut self, name: &str) -> bool {
                let value = match name {
                    // Map each stringified identifier to a mutable reference to that field
                    $( stringify!($n) => &mut self.$n, )*
                    // Anything that isn't a given ident will return false
                    _ => return false,
                };
                *value = !*value;
                true
            }
        }
    };
}

define_section_folds!(general, keys, partheads, methods, calls, music);

/// State that is saved per-composition, but shouldn't be tracked in the undo history.  This
/// includes the view state (e.g. where the camera is, which part the user's looking at) and
/// the state of the UI (e.g. which side-bar sections are collapsed).
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct View {
    pub current_part: usize,
    pub view_x: f32,
    pub view_y: f32,
    #[serde(default)]
    pub section_folds: SectionFolds,
}
