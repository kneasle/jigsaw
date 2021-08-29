// Imports used for doc comments
#[allow(unused_imports)]
use bellframe::Row;

/// The position of a [`Row`] within the source composition (i.e. before parts are expanded).  This
/// does not specify which part a [`Row`] occurs in - if you want this behaviour, then use
/// [`RowLocation`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RowSource {
    pub frag_index: FragIdx,
    pub row_index: RowIdx,
}

/// The position of a [`Row`] within the expanded/`full` composition - i.e. the same as
/// [`RowSource`], but also specifying the part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RowLocation {
    pub frag_index: FragIdx,
    pub row_index: RowIdx,
    pub part_index: PartIdx,
}

impl RowLocation {
    pub fn as_source(&self) -> RowSource {
        RowSource {
            frag_index: self.frag_index,
            row_index: self.row_index,
        }
    }
}

///////////////////
// TYPED VECTORS //
///////////////////

index_vec::define_index_type! { pub struct FragIdx = usize; }
index_vec::define_index_type! { pub struct RowIdx = usize; }
index_vec::define_index_type! { pub struct PartIdx = usize; }
index_vec::define_index_type! { pub struct MethodIdx = usize; }

pub type FragVec<T> = index_vec::IndexVec<FragIdx, T>;
pub type RowVec<T> = index_vec::IndexVec<RowIdx, T>;
pub type PartVec<T> = index_vec::IndexVec<PartIdx, T>;
pub type MethodVec<T> = index_vec::IndexVec<MethodIdx, T>;

pub type FragSlice<T> = index_vec::IndexSlice<FragIdx, [T]>;
pub type RowSlice<T> = index_vec::IndexSlice<RowIdx, [T]>;
pub type PartSlice<T> = index_vec::IndexSlice<PartIdx, [T]>;
pub type MethodSlice<T> = index_vec::IndexSlice<MethodIdx, [T]>;
