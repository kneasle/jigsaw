index_vec::define_index_type! { pub struct FragIdx = usize; }
index_vec::define_index_type! { pub struct RowIdx = usize; }
index_vec::define_index_type! { pub struct PartIdx = usize; }
index_vec::define_index_type! { pub struct MethodIdx = usize; }
index_vec::define_index_type! { pub struct ChunkIdx = usize; }

pub type FragVec<T> = index_vec::IndexVec<FragIdx, T>;
pub type RowVec<T> = index_vec::IndexVec<RowIdx, T>;
pub type PartVec<T> = index_vec::IndexVec<PartIdx, T>;
pub type MethodVec<T> = index_vec::IndexVec<MethodIdx, T>;
pub type ChunkVec<T> = index_vec::IndexVec<ChunkIdx, T>;

pub type FragSlice<T> = index_vec::IndexSlice<FragIdx, [T]>;
pub type RowSlice<T> = index_vec::IndexSlice<RowIdx, [T]>;
pub type PartSlice<T> = index_vec::IndexSlice<PartIdx, [T]>;
pub type MethodSlice<T> = index_vec::IndexSlice<MethodIdx, [T]>;
pub type ChunkSlice<T> = index_vec::IndexSlice<ChunkIdx, T>;
