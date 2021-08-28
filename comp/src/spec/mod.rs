use std::{
    cell::{Cell, Ref, RefCell},
    collections::HashSet,
    rc::Rc,
};

use bellframe::{RowBuf, Stage};
use emath::Vec2;
use index_vec::index_vec;
use itertools::Itertools;
use jigsaw_utils::types::{FragIdx, FragVec};

use self::part_heads::PartHeads;

mod expand;
pub mod part_heads;

pub(crate) use expand::expand;

/// The minimal but complete specification for a (partial) composition.  `CompSpec` is used for
/// undo history, and is designed to be a very compact representation which is cheap to clone and
/// modify.  Contrast this with [`FullState`](crate::full::FullState), which is computed from
/// `CompSpec` and is designed to be efficient to query and display to the user (and so contains a
/// large amount of redundant information).
#[derive(Debug, Clone)]
pub struct CompSpec {
    stage: Stage,
    part_heads: Rc<PartHeads>,
    methods: Vec<Rc<Method>>,
    calls: Vec<Rc<Call>>,
    fragments: FragVec<Rc<Fragment>>,
}

// This `impl` block is the entire public surface of `CompSpec`
impl CompSpec {
    /// Creates a [`CompSpec`] with a given [`Stage`] but no [`PartHeads`], [`Method`]s, [`Call`]s
    /// or [`Fragment`]s.
    #[allow(dead_code)]
    pub fn empty(stage: Stage) -> Self {
        CompSpec {
            stage,
            part_heads: Rc::new(PartHeads::one_part(stage)),
            methods: vec![],
            calls: vec![],
            fragments: index_vec![],
        }
    }

    /// Generates an example composition.
    pub fn example() -> Self {
        const STAGE: Stage = Stage::MAJOR;

        /// Create a new [`Method`] by parsing a string of place notation
        fn gen_method(shorthand: &str, name: &str, pn_str: &str) -> Rc<Method> {
            let method = Method::with_lead_end_ruleoff(
                bellframe::Method::from_place_not_string(String::new(), STAGE, pn_str).unwrap(),
                name.to_owned(),
                shorthand.to_string(),
            );
            Rc::new(method)
        }

        // The methods used in the composition
        let methods = vec![
            /* 0. */ gen_method("D", "Deva", "-58-14.58-58.36-14-58-36-18,18"),
            /* 1. */ gen_method("B", "Bristol", "-58-14.58-58.36.14-14.58-14-18,18"),
            /* 2. */ gen_method("E", "Lessness", "-38-14-56-16-12-58-14-58,12"),
            /* 3. */ gen_method("Y", "Yorkshire", "-38-14-58-16-12-38-14-78,12"),
            /* 4. */ gen_method("K", "York", "-38-14-12-38.14-14.38.14-14.38,12"),
            /* 5. */ gen_method("S", "Superlative", "-36-14-58-36-14-58-36-78,12"),
            /* 6. */ gen_method("W", "Cornwall", "-56-14-56-38-14-58-14-58,18"),
        ];

        // Touch is Deva, Yorkshire, York, Superlative, Lessness
        let chunks = [0usize, 3, 4, 5, 2]
            .iter()
            .map(|method_idx| {
                let method = methods[*method_idx].clone();
                let lead_len = method.inner.lead_len();
                // Add an entire lead of each method
                Rc::new(Chunk::Method {
                    method,
                    start_sub_lead_index: 0,
                    length: lead_len,
                })
            })
            .collect_vec();

        let fragment = Rc::new(Fragment {
            position: Vec2::new(200.0, 100.0),
            start_row: Rc::new(RowBuf::rounds(STAGE)),
            chunks,
            is_proved: true,
        });

        CompSpec {
            stage: STAGE,
            part_heads: Rc::new(
                PartHeads::parse("18234567", STAGE).unwrap(), /* PartHeads::one_part(STAGE) */
            ),
            methods,
            calls: vec![], // No calls for now
            fragments: index_vec![fragment],
        }
    }

    /////////////////////////
    // MODIFIERS & ACTIONS //
    /////////////////////////

    // All modifiers and actions will create steps in the undo history

    /// Overwrites the [`PartHeads`] of `self`.
    ///
    /// # Panics
    ///
    /// Panics if the [`Stage`]s of `self` and the new [`PartHeads`] don't match
    pub fn set_part_heads(&mut self, part_heads: PartHeads) {
        assert_eq!(self.stage, part_heads.stage());
        self.part_heads = Rc::new(part_heads);
    }

    /// Deletes the [`Fragment`] with a given [`FragIdx`]
    ///
    /// # Panics
    ///
    /// Panics if no [`Fragment`] has the given [`FragIdx`]
    pub fn delete_fragment(&mut self, frag_idx: FragIdx) {
        self.fragments.remove(frag_idx);
    }
}

/// A single `Fragment` of composition.
#[derive(Debug, Clone)]
pub(crate) struct Fragment {
    /// The on-screen location of the top-left corner of the top row this `Frag`
    position: Vec2,
    start_row: Rc<RowBuf>,
    /// A sequence of [`Chunk`]s that make up this `Fragment`
    chunks: Vec<Rc<Chunk>>,
    /// Set to `false` if this `Fragment` is visible but 'muted' - i.e. visually greyed out and not
    /// included in the proving, ATW calculations, statistics, etc.
    is_proved: bool,
}

impl Fragment {
    pub fn position(&self) -> Vec2 {
        self.position
    }

    /// Gets the number of non-leftover [`Row`]s in this [`Fragment`] in one part of the
    /// composition.
    pub fn len(&self) -> usize {
        self.chunks.iter().map(|c| c.len()).sum()
    }
}

/// A `Chunk` of a [`Fragment`], consisting of either a contiguous segment of a [`Method`] or a
/// [`Call`] rung all the way through
#[derive(Debug, Clone)]
enum Chunk {
    Method {
        method: Rc<Method>,
        start_sub_lead_index: usize,
        length: usize,
    },
    Call {
        call: Rc<Call>,
        method: Rc<Method>,
        start_sub_lead_index: usize,
    },
}

impl Chunk {
    /// Return the number of [`Row`]s generated by this [`Chunk`]
    fn len(&self) -> usize {
        match self {
            Chunk::Method { length, .. } => *length,
            Chunk::Call { call, .. } => call.inner.len(),
        }
    }

    /// Gets the [`Method`] to which these rows are assigned
    fn method(&self) -> &Method {
        match self {
            Chunk::Method { method, .. } => method,
            Chunk::Call { method, .. } => method,
        }
    }

    /// Gets the sub-lead index of the first [`Row`] in this `Chunk`
    fn start_sub_lead_index(&self) -> usize {
        match self {
            Chunk::Method {
                start_sub_lead_index,
                ..
            } => *start_sub_lead_index,
            Chunk::Call {
                start_sub_lead_index,
                ..
            } => *start_sub_lead_index,
        }
    }
}

/// The data required to define a [`Method`] that's used somewhere in the composition.  This is a
/// wrapper around [`bellframe::Method`] adding extra data like method shorthand names.
#[derive(Debug, Clone)]
pub(crate) struct Method {
    inner: bellframe::Method,
    /// The name (not title) of this `Method`.  For example, the method who's title is `"Bristol
    /// Surprise Major"` would have name `"Bristol"`.
    name: RefCell<String>,
    /// A short string which denotes this Method.  There are no restrictions on this - they do not
    /// even have to be unique or non-empty (since the rows store their corresponding method
    /// through an [`Rc`]).  For example, `B` is often used as a shorthand for `"Bristol Surprise
    /// Major"`.
    shorthand: RefCell<String>,
    /// Which locations in the lead should have lines drawn **below** them
    ruleoffs: HashSet<usize>, // TODO: Use a bitmask
}

impl Method {
    fn with_lead_end_ruleoff(inner: bellframe::Method, name: String, shorthand: String) -> Self {
        let lead_len = inner.lead_len();
        Self::new(
            inner,
            name,
            shorthand,
            std::iter::once(lead_len - 1).collect(),
        )
    }

    fn new(
        inner: bellframe::Method,
        name: String,
        shorthand: String,
        ruleoffs: HashSet<usize>,
    ) -> Self {
        Self {
            inner,
            name: RefCell::new(name),
            shorthand: RefCell::new(shorthand),
            ruleoffs,
        }
    }

    pub fn shorthand(&self) -> Ref<String> {
        self.shorthand.borrow()
    }

    pub fn name(&self) -> Ref<String> {
        self.name.borrow()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Call {
    inner: bellframe::Call,
}

/// A point where the composition can be folded.  Composition folding is not part of the undo
/// history and therefore relies on interior mutability.
#[derive(Debug, Clone)]
pub(crate) struct Fold {
    is_open: Cell<bool>,
}
