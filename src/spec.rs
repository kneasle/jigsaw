use std::{
    cell::{Cell, Ref, RefCell},
    ops::Deref,
    rc::Rc,
};

use bellframe::{AnnotBlock, AnnotRow, Row, Stage};

use crate::{part_heads::PartHeads, V2};

/// The minimal but complete specification for a (partial) composition.  `CompSpec` is used for
/// undo history, and is designed to be a very compact representation which is cheap to clone and
/// modify.  Contrast this with [`FullComp`], which is computed from `CompSpec` and is designed to
/// be efficient to query and display to the user.
#[derive(Debug, Clone)]
pub(crate) struct CompSpec {
    stage: Stage,
    part_heads: Rc<PartHeads>,
    methods: Vec<Rc<Method>>,
    calls: Vec<Rc<Call>>,
    fragments: Vec<Rc<Fragment>>,
}

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
            fragments: vec![],
        }
    }

    /// Generates an example composition.
    pub fn example() -> Self {
        const STAGE: Stage = Stage::MAJOR;

        /// Create a new [`Method`] by parsing a string of place notation
        fn gen_method(shorthand: &str, name: &str, pn_str: &str) -> Rc<Method> {
            let method = Method::new(
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

        let fragment = {
            let mut block = AnnotBlock::<RowData>::empty(STAGE);
            // Touch is Deva, Yorkshire, York, Superlative, Lessness
            for &meth_idx in &[0usize, 3, 4, 5, 2] {
                let method_rc = methods[meth_idx].clone();
                block
                    .extend(
                        method_rc
                            .clone()
                            .inner
                            .first_lead()
                            .gen_annots_from_indices(|sub_lead_index| RowData {
                                method: method_rc.clone(),
                                sub_lead_index,
                                call: None,
                                fold: None,
                            }),
                    )
                    .unwrap();
            }

            Rc::new(Fragment {
                position: V2::new(100.0, 200.0),
                block,
                is_proved: true,
            })
        };

        CompSpec {
            stage: STAGE,
            part_heads: Rc::new(PartHeads::one_part(STAGE)),
            methods,
            calls: vec![], // No calls for now
            fragments: vec![fragment],
        }
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn part_heads(&self) -> &PartHeads {
        &self.part_heads
    }

    pub fn part_heads_rc(&self) -> Rc<PartHeads> {
        self.part_heads.clone()
    }

    /// An iterator over the [`Method`]s in this [`CompSpec`].
    pub fn methods(&self) -> impl Iterator<Item = &Method> {
        self.methods.iter().map(Deref::deref)
    }

    pub fn method_rcs(&self) -> &[Rc<Method>] {
        &self.methods
    }

    /// An iterator over the [`Call`]s in this [`CompSpec`].
    pub fn calls(&self) -> impl Iterator<Item = &Call> {
        self.calls.iter().map(Deref::deref)
    }

    /// An iterator over the [`Call`]s in this [`CompSpec`].
    pub fn fragments(&self) -> impl Iterator<Item = &Fragment> {
        self.fragments.iter().map(Deref::deref)
    }
}

/// A single `Fragment` of composition.
#[derive(Debug, Clone)]
pub(crate) struct Fragment {
    /// The on-screen location of the top-left corner of the top row this `Frag`
    position: V2,
    /// The `Block` of annotated [`Row`]s making up this `Fragment`
    block: AnnotBlock<RowData>,
    /// Set to `false` if this `Fragment` is visible but 'muted' - i.e. visually greyed out and not
    /// included in the proving, ATW calculations, statistics, etc.
    is_proved: bool,
}

impl Fragment {
    pub fn position(&self) -> V2 {
        self.position
    }

    pub fn is_proved(&self) -> bool {
        self.is_proved
    }

    /// Get a reference to the fragment's rows.
    pub fn annot_rows(&self) -> impl Iterator<Item = AnnotRow<RowData>> {
        self.block.annot_rows()
    }

    /// Gets the leftover row of this [`Fragment`]
    pub fn leftover_row(&self) -> &Row {
        self.block.leftover_row()
    }

    /// Gets the number of non-leftover [`Row`]s in this [`Fragment`] in one part of the
    /// composition.
    pub fn len(&self) -> usize {
        self.block.len()
    }
}

/// The meta-data associated with each (non-leftover) [`Row`] in the composition.
#[derive(Debug, Clone)]
pub(crate) struct RowData {
    /// A reference to the [`Method`] that generated this [`Row`]
    method: Rc<Method>,
    /// The index within the method's lead that this [`Row`] belongs to.  This is used for many
    /// purposes - such as ATW checking, inserting ruleoffs, determining valid call locations, etc.
    sub_lead_index: usize,
    /// This is `Some(c)` if an instance of that call **starts** on this [`Row`].
    ///
    /// **NOTE**: This is the opposite way round to how lead locations are defined (a call at a
    /// lead location will _finish_ at that location).  For example, the 0th row of a lead is
    /// usually referred to as `"LE"` for 'Lead End' and all lead end calls (including those in
    /// Grandsire) will finish at the 0th row.  However, we have to do it this way round because we
    /// might have a call at the end of a `Fragment`, in which case we would have to attach data to
    /// the leftover row (which [`AnnotBlock`] doesn't allow):
    /// ```text
    ///            ...
    ///          31425678
    ///          13246587    call = Some(<bob>)
    ///          --------
    ///  (LE) -H 12345678  <- leftover row; can't be assigned a `Call` but **can** be rendered
    ///                       with text
    /// ```
    /// Note, however, that `FullComp` allows the leftover row to be given annotations, so we can
    /// display the `-H` to the user in the place they expect.
    call: Option<Rc<Call>>,
    /// If `self.fold.is_some()`, then this [`Row`] corresponds to a fold-point in the composition.
    fold: Option<Rc<Fold>>,
}

impl RowData {
    /// The [`Method`] that owns this [`Row`]
    pub(crate) fn method(&self) -> &Method {
        &self.method
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
    /// even have to be unique (since the rows store their corresponding method through an [`Rc`]).
    shorthand: RefCell<String>,
}

impl Method {
    fn new(inner: bellframe::Method, name: String, shorthand: String) -> Self {
        Self {
            inner,
            name: RefCell::new(name),
            shorthand: RefCell::new(shorthand),
        }
    }

    /// Get a reference to the method's name.
    pub fn shorthand(&self) -> Ref<String> {
        self.shorthand.borrow()
    }

    /// Get a reference to the method's name.
    pub fn name(&self) -> Ref<String> {
        self.name.borrow()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Call {}

/// A point where the composition can be folded.  Composition folding is not part of the undo
/// history and therefore relies on interior mutability.
#[derive(Debug, Clone)]
pub(crate) struct Fold {
    is_open: Cell<bool>,
}
