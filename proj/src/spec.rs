use crate::derived_state::RowOrigin;
use proj_core::{Row, Stage};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct AnnotatedRow {
    pub is_lead_end: bool,
    pub method_str: Option<String>,
    pub call_str: Option<String>,
    pub row: Row,
}

impl AnnotatedRow {
    /// Creates an [`AnnotatedRow`] representing a given [`Row`] with no annotations
    pub fn unannotated(row: Row) -> AnnotatedRow {
        AnnotatedRow {
            is_lead_end: false,
            method_str: None,
            call_str: None,
            row,
        }
    }
}

/// A single unexpanded fragment of a composition
#[derive(Clone, Debug)]
pub struct Frag {
    pub(crate) rows: Vec<AnnotatedRow>,
    pub x: f32,
    pub y: f32,
}

impl Frag {
    /// Generates an example fragment (in this case, it's https://complib.org/composition/75822)
    pub fn example() -> Frag {
        let mut rows: Vec<_> = include_str!("example-comp")
            .lines()
            .map(|x| Row::parse(x).unwrap())
            .map(AnnotatedRow::unannotated)
            .collect();
        /* ANNOTATIONS */
        // Method names and LE ruleoffs
        let method_names = [
            "Deva",
            "Bristol",
            "Lessness",
            "Yorkshire",
            "York",
            "Superlative",
            "Cornwall",
            "Bristol",
        ];
        for i in 0..rows.len() / 32 {
            rows[i * 32].method_str = Some(method_names[i].to_owned());
            rows[i * 32 + 31].is_lead_end = true;
        }
        // Calls
        rows[31].call_str = Some("sB".to_owned());
        rows[63].call_str = Some("sB".to_owned());
        rows[223].call_str = Some("sH".to_owned());
        rows[255].call_str = Some("sH".to_owned());
        // Create the fragment and return
        Frag {
            rows,
            x: -100.0,
            y: -200.0,
        }
    }

    /// Get the row at a given index
    #[inline]
    pub fn get_row(&self, index: usize) -> AnnotatedRow {
        self.rows[index].clone()
    }

    /// The number of rows in this fragment
    #[inline]
    pub fn len(&self) -> usize {
        self.rows.len()
    }
}

/// The _specfication_ for a composition.  This is what the user edits, and it is used to derive
/// the fully expanded set of rows and their origins.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct Spec {
    pub(crate) frags: Vec<Frag>,
    pub(crate) part_heads: Vec<Row>,
    pub(crate) stage: Stage,
}

impl Spec {
    /// Creates an example Spec
    pub fn example() -> Spec {
        // Generate all the cyclic part heads, and make sure that we start with rounds
        let mut part_heads = Row::parse("18234567").unwrap().closure();
        let rounds = part_heads.pop().unwrap();
        part_heads.insert(0, rounds);
        // Create a Spec and return
        Spec {
            frags: vec![Frag::example()],
            part_heads,
            stage: Stage::MAJOR,
        }
    }

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
                        part_head * &annot_row.row,
                    ));
                }
            }
        }
    }
}
