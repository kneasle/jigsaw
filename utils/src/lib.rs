pub mod serialisation;

// Imports used for doc comments
#[allow(unused_imports)]
use bellframe::Row;

/// The position of a [`Row`] within the source composition (i.e. before parts are expanded).  This
/// does not specify which part a [`Row`] occurs in - if you want this behaviour, then use
/// [`RowLocation`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RowSource {
    pub frag_index: usize,
    pub row_index: usize,
}

/// The position of a [`Row`] within the expanded/`full` composition - i.e. the same as
/// [`RowSource`], but also specifying the part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RowLocation {
    pub frag_index: usize,
    pub row_index: usize,
    pub part_index: usize,
}

impl RowLocation {
    pub fn as_source(&self) -> RowSource {
        RowSource {
            frag_index: self.frag_index,
            row_index: self.row_index,
        }
    }
}
