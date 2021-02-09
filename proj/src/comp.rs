use crate::Frag;
use proj_core::{Row, Stage};
use wasm_bindgen::prelude::*;

/// A small datatype that represents **where** a given row comes from in the composition.  This is
/// useful because the composition contains many fragments, and each row of this could expand into
/// multiple actual rows (one for each part).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct RowOrigin {
    /// The index of the part that this [`Row`] came from
    part: usize,
    /// The index of the fragment that this [`Row`] came from
    frag: usize,
    /// The index of the row within the fragment that this [`Row`] came from
    row: usize,
}

impl RowOrigin {
    /// Creates a new `RowOrigin` from it's parts
    pub fn new(part: usize, frag: usize, row: usize) -> RowOrigin {
        RowOrigin { part, frag, row }
    }
}

/// The _specfication_ for a composition.  This is what the user edits, and it is used to derive
/// the fully expanded set of rows and their origins.
#[derive(Debug, Clone)]
pub struct Spec {
    frags: Vec<Frag>,
    part_heads: Vec<Row>,
}

impl Spec {
    /// Gets the number of [`Row`]s that will appear in the expanded version of this comp, without
    /// expanding anything.
    pub fn len(&self) -> usize {
        self.part_heads.len() * self.frags.iter().map(|f| f.len()).sum::<usize>()
    }

    /// Write all the [`Row`]s that make up this `Spec` into a [`Vec`], reusing the storage where
    /// needed.
    pub fn gen_rows(&self, out_vec: &mut Vec<(RowOrigin, Row)>) {
        out_vec.clear();
        out_vec.reserve(self.len());
        for (part_index, part_head) in self.part_heads.iter().enumerate() {
            for (frag_index, frag) in self.frags.iter().enumerate() {
                for (row_index, annot_row) in frag.rows.iter().enumerate() {
                    out_vec.push((
                        RowOrigin::new(part_index, frag_index, row_index),
                        part_head * &annot_row.row(),
                    ));
                }
            }
        }
    }
}

/// The complete state of a partial composition.  The data-flow is:
/// - User makes some edit, which changes the [`Spec`]ification
/// - Because the [`Spec`] changed, `all_rows` is recalculated
/// - Once we have the new [`Spec`] and all the expanded rows, we use these to rebuild the
///   `derived_state` so that the JS code doesn't recalculate this state every time the screen is
///   rendered.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct Comp {
    spec: Spec,
    all_rows: Vec<(RowOrigin, Row)>,
    false_row_groups: Vec<Vec<RowOrigin>>,
}

impl Comp {
    pub fn from_spec(spec: Spec) -> Comp {
        let mut c = Comp {
            spec,
            all_rows: Vec::new(),
            false_row_groups: Vec::new(),
        };
        c.rebuild_state();
        c
    }
}

#[wasm_bindgen]
impl Comp {
    /// Rebuild the cached state, as though the [`Spec`] had changed.
    pub fn rebuild_state(&mut self) {
        // Rebuild the expanded rows
        self.spec.gen_rows(&mut self.all_rows);
        // Generate the truth of the composition, by sorting the rows and then generating the
        // groups of >=2 `RowOrigin`s which produced identical rows.  These groups will be used to
        // mark which source rows are duplicated
        self.all_rows.sort_by(|p1, p2| p1.1.cmp(&p2.1));
        self.false_row_groups.clear();
        let mut last_row: Option<&Row> = None;
        let mut current_group = Vec::new();
        for (origin, row) in &self.all_rows {
            current_group.push(*origin);
            if let Some(r) = last_row {
                if r != row {
                    if current_group.len() > 1 {
                        self.false_row_groups.push(current_group.clone());
                    }
                    current_group.clear();
                }
            }
            last_row = Some(row);
        }
        // Make sure that the last row group is added
        if current_group.len() > 1 {
            self.false_row_groups.push(current_group.clone());
        }
    }

    /// Create an example composition
    pub fn example() -> Comp {
        Self::from_spec(Spec {
            frags: vec![Frag::example()],
            part_heads: vec![Row::rounds(Stage::MAJOR), Row::backrounds(Stage::MAJOR)],
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        let c = super::Comp::example();
    }
}
