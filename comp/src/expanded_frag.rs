//! Intermediate representation between 'expansion' (which converts a
//! [`CompSpec`] into a list of
//! [`ExpandedFrag`]s, where the expansion of each [`Fragment`](crate::spec::Fragment) is
//! independent) and 'annotation' (where the data from all [`ExpandedFrag`]s,
//! [`Music`](crate::music::Music) definitions, etc. are combined to add additional annotations
//! like falseness, frag links, etc).
//!
//! The main purpose of this intermediate step is to separate the full [`CompSpec`] ->
//! [`FullState`](crate::full::FullState) conversion from the implementation details of
//! [`CompSpec`] itself.  [`ExpandedFrag`] is intended as the public interface of [`CompSpec`]'s
//! fragment storage - so long as [`CompSpec`] can generate [`ExpandedFrag`]s, its internal
//! representation can be changed at any point.
//!
//! [`CompSpec`]: spec::CompSpec

use std::rc::Rc;

use bellframe::SameStageVec;
use emath::Pos2;
use jigsaw_utils::indexed_vec::{PartVec, RowVec};

use crate::spec::{self, part_heads::PartHeads};

#[derive(Debug, Clone)]
pub(crate) struct ExpandedFrag {
    pub position: Pos2,
    /// Each of these contains all the [`Row`]s of the source [`Fragment`] (including the leftover)
    /// for each part in the expanded composition
    pub rows_per_part: PartVec<SameStageVec>,
    /// Stores data for each [`Row`] which is independent of which part is being seen
    pub row_data: RowVec<RowData>,
    /// `false` if the source [`Fragment`] is muted
    pub is_proved: bool,
}

impl ExpandedFrag {
    pub(crate) fn from_single_part(
        rows_in_one_part: SameStageVec,
        row_data: RowVec<RowData>,
        is_proved: bool,
        position: Pos2,
        part_heads: &PartHeads,
    ) -> Self {
        assert_eq!(rows_in_one_part.len(), row_data.len());
        assert_eq!(rows_in_one_part.stage(), part_heads.stage());
        let rows_per_part = part_heads
            .rows()
            .iter()
            .map(|part_head| rows_in_one_part.pre_multiplied(part_head).unwrap())
            .collect();
        Self {
            position,
            rows_per_part,
            row_data,
            is_proved,
        }
    }

    /// The number of proved [`Row`]s in this [`ExpandedFrag`] in one part of the composition.
    pub(crate) fn len(&self) -> usize {
        if self.is_proved {
            self.row_data.len() - 1 // All rows except leftover are proved
        } else {
            0 // No rows are proved
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RowData {
    pub(crate) method_source: Option<(Rc<spec::Method>, usize)>,
    pub(crate) call_source: Option<(Rc<spec::Call>, usize)>,
    pub is_proved: bool,
}
