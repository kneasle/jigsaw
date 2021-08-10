use std::rc::Rc;

use bellframe::AnnotBlock;

use crate::V2;

/// The minimal but complete specification for a (partial) composition.  `CompSpec` is used for
/// undo history, and is designed to be a very compact representation which is cheap to clone and
/// modify.  Contrast this with [`FullComp`], which is computed from `CompSpec` and is designed to
/// be efficient to query and display to the user.
#[derive(Debug, Clone)]
pub struct CompSpec {
    methods: Vec<Rc<Method>>,
    calls: Vec<Rc<Call>>,
    fragments: Vec<Rc<Fragment>>,
}

/// A single `Fragment` of composition.
#[derive(Debug, Clone)]
struct Fragment {
    /// The on-screen location of the top-left corner of the top row this `Frag`
    position: V2,
    /// The `Block` of annotated [`Row`]s making up this `Fragment`
    rows: AnnotBlock<RowData>,
}

/// The meta-data associated with each (non-leftover) [`Row`] in the composition
#[derive(Debug, Clone)]
struct RowData {
    method: Rc<Method>,
    sub_lead_index: usize,
    /// This is `Some(c)` if an instance of that call **starts** on this [`Row`].
    ///
    /// **NOTE**: This is the opposite way round to how lead locations are defined (a call at a
    /// lead location will _finish_ at that location).  For example, the 0th row of a lead is
    /// usually referred to as `"LE"` for 'Lead End' and all lead end calls (including those in
    /// Grandsire) will finish at the 0th row.  However, we have to do it this way round because we
    /// might have a call at the end of a `Fragment`, in which case we would have to annotate the
    /// leftover row (which [`AnnotBlock`] doesn't allow):
    ///
    /// ```text
    ///          ...
    ///        31425678
    ///        13246587    call = Some(<bob>)
    ///        --------
    ///   (LE) 12345678  <- leftover row; can't be given a `Call`
    /// ```
    call: Option<Rc<Call>>,
    /// If `self.fold.is_some()`, then this [`Row`] corresponds to a fold-point in the composition.
    fold: Option<Rc<Fold>>,
}

/// The data required to define a [`Method`] that's used somewhere in the composition
#[derive(Debug, Clone)]
struct Method {}

#[derive(Debug, Clone)]
struct Call {}

/// A point where the composition can be folded.  Composition folding is not part of the undo
/// history and therefore relies on interior mutability.
#[derive(Debug, Clone)]
struct Fold {}
