use proj_core::{Bell, Row};
use wasm_bindgen::prelude::*;

fn clone_or_empty(string: &Option<String>) -> String {
    match string {
        Some(x) => x.clone(),
        None => "".to_owned(),
    }
}

#[wasm_bindgen]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct AnnotatedRow {
    is_lead_end: bool,
    method_str: Option<String>,
    call_str: Option<String>,
    row: Row,
}

#[wasm_bindgen]
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

    /// Returns this [`Row`] without any annotations
    pub fn row(&self) -> Row {
        self.row.clone()
    }

    /// Returns the [`Bell`]s that make up this row, as [`Vec`] of 0-indexed integers
    pub fn bells_indices(&self) -> Vec<usize> {
        self.row.slice().iter().copied().map(Bell::index).collect()
    }

    /// Returns a [`String`] that should be used to signal this `Row` as the start of a 'lead'
    pub fn method_str(&self) -> String {
        clone_or_empty(&self.method_str)
    }

    /// Returns a [`String`] that should annotate this row as a call
    pub fn call_str(&self) -> String {
        clone_or_empty(&self.call_str)
    }

    /// Returns `true` if this `AnnotatedRow` should have a line rendered underneath it
    pub fn is_ruleoff(&self) -> bool {
        self.is_lead_end
    }
}

#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct Frag {
    rows: Vec<AnnotatedRow>,
}

#[wasm_bindgen]
impl Frag {
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

    /// Generates an example composition (in this case, it's https://complib.org/composition/75822)
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
            "Cooktown Orchid",
            "Superlative",
            "Cornwall",
            "Bristol",
        ];
        for i in 0..rows.len() / 32 {
            rows[i * 32].method_str = Some(method_names[i].to_owned());
            rows[i * 32 + 31].is_lead_end = true;
        }
        // Calls
        rows[31].call_str = Some("s".to_owned());
        rows[63].call_str = Some("s".to_owned());
        rows[223].call_str = Some("s".to_owned());
        rows[255].call_str = Some("s".to_owned());
        // Create the fragment and return
        Frag { rows }
    }
}

#[wasm_bindgen]
pub fn reverse(s: String) -> String {
    s.chars().rev().skip(1).collect()
}
