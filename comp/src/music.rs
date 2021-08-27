//! Representation of musical [`Row`]s

use bellframe::{music::Regex, Stage};
use itertools::Itertools;

// Imports only used for doc comments
#[allow(unused_imports)]
use bellframe::Row;

/// A tree-like structure which recursively combines groups of musical [`Row`]s
#[derive(Debug, Clone)]
pub enum Music {
    /// An optionally named group of musical [`Row`]s, specified by a single [`Regex`] over
    /// [`Row`]s.  This cannot have any sub-groups.
    Regex(Option<String>, Regex),
    /// A named group of sub-groups of musical [`Row`]s
    Group(String, Vec<Music>),
}

impl Music {
    /// Creates a [`Music`] group for
    pub fn runs_front_and_back(stage: Stage, len: usize) -> Music {
        let name = format!("{}-bell runs", len);
        let sub_classes = vec![
            Self::group_from_regexes("front", Regex::runs_front(stage, len)),
            Self::group_from_regexes("back", Regex::runs_back(stage, len)),
        ];
        Music::Group(name, sub_classes)
    }

    /// Create a [`Music::Group`] containing one unnamed group per [`Regex`] yielded by `regexes`.
    pub fn group_from_regexes(name: &str, regexes: impl IntoIterator<Item = Regex>) -> Self {
        let sub_groups = regexes
            .into_iter()
            .map(|r| Music::Regex(None, r))
            .collect_vec();
        Self::Group(name.to_owned(), sub_groups)
    }
}
