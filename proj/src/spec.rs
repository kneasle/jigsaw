use crate::derived_state::ExpandedRow;
use proj_core::{Row, Stage};
use serde::Serialize;
use std::rc::Rc;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct AnnotatedRow {
    pub(crate) is_lead_end: bool,
    pub(crate) method_str: Option<MethodName>,
    pub(crate) call_str: Option<String>,
    pub(crate) row: Row,
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

/// A convenient data structure of the long and short method names
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize)]
pub struct MethodName {
    name: String,
    shorthand: String,
}

/// A single unexpanded fragment of a composition
#[derive(Clone, Debug)]
pub struct Frag {
    /// Note that this [`Vec`] stores all the rows that should be displayed in this fragment,
    /// including the leftover row (which has to be displayed, but won't be used for proving).
    rows: Rc<Vec<AnnotatedRow>>,
    pub x: f32,
    pub y: f32,
}

impl Frag {
    /// Generates an example fragment (in this case, it's https://complib.org/composition/75822)
    pub fn cyclic_s8() -> Frag {
        let mut rows: Vec<_> = include_str!("cyclic-s8")
            .lines()
            .map(|x| Row::parse(x).unwrap())
            .map(AnnotatedRow::unannotated)
            .collect();
        /* ANNOTATIONS */
        // Method names and LE ruleoffs
        let method_names = [
            ("Deva", "V"),
            ("Bristol", "B"),
            ("Lessness", "E"),
            ("Yorkshire", "Y"),
            ("York", "K"),
            ("Superlative", "S"),
            ("Cornwall", "W"),
            ("Bristol", "B"),
        ];
        for i in 0..rows.len() / 32 {
            let (method_name, method_short) = method_names[i];
            rows[i * 32].method_str = Some(MethodName {
                name: method_name.to_owned(),
                shorthand: method_short.to_owned(),
            });
            rows[i * 32 + 31].is_lead_end = true;
        }
        // Calls
        rows[31].call_str = Some("sB".to_owned());
        rows[63].call_str = Some("sB".to_owned());
        rows[223].call_str = Some("sH".to_owned());
        rows[255].call_str = Some("sH".to_owned());
        // Create the fragment and return
        Frag {
            rows: Rc::new(rows),
            x: -100.0,
            y: -200.0,
        }
    }

    pub fn cyclic_max_eld() -> Frag {
        // Parse row and method locations
        let mut rows: Vec<_> = include_str!("cyclic-max-eld")
            .lines()
            .map(|x| Row::parse(x).unwrap())
            .map(AnnotatedRow::unannotated)
            .collect();

        // Add method names & ruleoffs to the appropriate rows
        let methods = [
            ("Mount Mackenzie Alliance", "Mm", 36),
            ("Baluan Alliance", "B", 36),
            ("Ganges Alliance", "Ga", 40),
            ("Cauldron Dome Little Delight", "Ca", 32),
            ("Europa Little Treble Place", "Eu", 16),
            ("Diamond Head Alliance", "D2", 44),
            ("Callisto Little Alliance", "Ca", 36),
            ("Darwin Little Alliance", "D", 44),
            ("Hallasan Alliance", "Ha", 32),
            ("Asaph Hall Surprise", "As", 48),
            ("Alcedo Alliance", "A", 40),
            ("Kilauea Differential", "Li", 14),
        ];
        let mut method_start = 0;
        for (full, short, length) in &methods {
            rows[method_start].method_str = Some(MethodName {
                name: full.to_string(),
                shorthand: short.to_string(),
            });
            rows[method_start + length - 1].is_lead_end = true;
            method_start += length;
        }
        assert_eq!(method_start + 1, rows.len());

        // Add calls
        rows[403].call_str = Some("-".to_owned());
        rows[417].call_str = Some("-".to_owned());

        Frag {
            rows: Rc::new(rows),
            x: -100.0,
            y: -200.0,
        }
    }

    /// Get the row at a given index
    #[inline]
    pub fn get_row(&self, index: usize) -> AnnotatedRow {
        self.rows[index].clone()
    }

    /// The number of rows in this fragment that should be proven
    #[inline]
    pub fn len(&self) -> usize {
        self.rows.len() - 1
    }
}

/// The _specfication_ for a composition.  This is what the user edits, and it is used to derive
/// the fully expanded set of rows and their origins.
#[derive(Debug, Clone)]
pub struct Spec {
    pub(crate) frags: Vec<Rc<Frag>>,
    pub(crate) part_heads: Rc<Vec<Row>>,
    pub(crate) stage: Stage,
}

impl Spec {
    /// Creates an example Spec
    pub fn cyclic_s8() -> Spec {
        // Generate all the cyclic part heads, and make sure that we start with rounds
        let mut part_heads = Row::parse("81234567").unwrap().closure();
        let rounds = part_heads.pop().unwrap();
        part_heads.insert(0, rounds);
        // Create a Spec and return
        Self::single_frag(Frag::cyclic_s8(), part_heads, Stage::MAJOR)
    }

    pub fn cyclic_max_eld() -> Spec {
        // Generate all the cyclic part heads, and make sure that we start with rounds
        let mut part_heads = Row::parse("T1234567890E").unwrap().closure();
        let rounds = part_heads.pop().unwrap();
        part_heads.insert(0, rounds);
        // Create a Spec and return
        Self::single_frag(Frag::cyclic_max_eld(), part_heads, Stage::MAXIMUS)
    }

    pub fn single_frag(frag: Frag, part_heads: Vec<Row>, stage: Stage) -> Spec {
        // Check that all the stages are the same
        for annot_r in frag.rows.iter() {
            assert_eq!(annot_r.row.stage(), stage);
        }
        for p in &part_heads {
            assert_eq!(p.stage(), stage);
        }
        Spec {
            frags: vec![Rc::new(frag)],
            part_heads: Rc::new(part_heads),
            stage,
        }
    }

    /// Gets the number of [`Row`]s that should be proved in the expanded version of this comp,
    /// without expanding anything.
    pub fn len(&self) -> usize {
        self.part_heads.len() * self.part_len()
    }

    /// Gets the number of [`Row`]s that are generated in one part of this composition
    pub fn part_len(&self) -> usize {
        self.frags.iter().map(|f| f.len()).sum::<usize>()
    }

    /// Generates all the rows generated by this `Spec`, storing them in the following
    /// datastructure:
    pub fn gen_rows(&self) -> Vec<(Vec<ExpandedRow>, ExpandedRow)> {
        self.frags
            .iter()
            .map(|f| {
                let len = f.rows.len() - 1;
                (
                    f.rows[..len]
                        .iter()
                        .map(|r| ExpandedRow::new(r, &self.part_heads, false))
                        .collect(),
                    ExpandedRow::new(&f.rows[len], &self.part_heads, true),
                )
            })
            .collect()
    }
}
