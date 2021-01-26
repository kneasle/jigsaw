use proj_core::{Perm, Row};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

pub mod block;

type BlockID = String;
type FragID = usize;

#[derive(Copy, Clone, Debug)]
pub struct RowID {
    part: usize,
    frag: FragID,
    sub_frag_index: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Block {
    Nested(Vec<BlockID>),
    Atom(proj_core::Block),
}

/// A single 'fragment' of a composition - i.e. a single not-necessarily-round [`Block`], along with
/// the [`Row`] that the [`Block`] should be used to permute.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Frag {
    block: BlockID,
    start_row: Row,
}

#[derive(Debug, Clone)]
pub struct Comp {
    /// The [`Frag`]s that make up the comp
    frags: Vec<Frag>,
    /// A mapping between the block IDs and the underlying [`Block`]s
    blocks: HashMap<BlockID, Block>,
    /// The part heads of the composition
    part_heads: Vec<Perm>,
}

impl Comp {
    pub fn rows(&self) -> Vec<(RowID, Row)> {
        let mut rows = Vec::new();
        for p in &self.part_heads {
            for (fi, f) in self.frags.iter().enumerate() {}
        }
        rows
    }
}

#[wasm_bindgen]
pub fn reverse(s: String) -> String {
    s.chars().rev().skip(1).collect()
}
