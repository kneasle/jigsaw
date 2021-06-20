use std::{collections::HashMap, hash::Hash, ops::Range, rc::Rc};

use bellframe::Row;
use serde::Serialize;

use super::{Annot, CallRef, Frag, MethodRef, MethodSpec, Spec};

/// The most number of undo steps which are saved as history
const MAX_SAVED_UNDO_STEPS: usize = 50;

/// A newtype representing locations of items in memory
type Addr = usize;

/// A generic symbol interner which stores its values in a [`Vec`] and returns indices into this
#[derive(Debug, Clone)]
struct Dedup<T: Hash> {
    map: HashMap<T, usize>,
    next_ind: usize,
}

impl<T: Hash> Dedup<T> {
    fn intern(&mut self, v: T) -> usize
    where
        T: Eq,
    {
        if let Some(ind) = self.map.get(&v) {
            *ind
        } else {
            let i = self.next_ind;
            self.map.insert(v, i);
            self.next_ind += 1;
            i
        }
    }

    fn intern_iter(&mut self, vs: impl IntoIterator<Item = T>) -> Vec<usize>
    where
        T: Eq,
    {
        vs.into_iter().map(|v| self.intern(v)).collect()
    }
}

impl<T: Hash> Default for Dedup<T> {
    fn default() -> Self {
        Dedup {
            map: HashMap::new(),
            next_ind: 0,
        }
    }
}

impl<T: Hash + Clone> Into<Vec<T>> for Dedup<T> {
    fn into(self) -> Vec<T> {
        // PERF: It would be safe to use `MaybeUninit` here
        let mut opt_res: Vec<Option<T>> = vec![None; self.map.len()];
        for (t, i) in self.map {
            opt_res[i] = Some(t);
        }
        opt_res.into_iter().map(Option::unwrap).collect()
    }
}

/// A generic symbol interner which stores its values in a [`Vec`] and returns indices into this
#[derive(Debug, Clone)]
struct AddrDedup<T> {
    map: HashMap<Addr, (T, usize)>,
    next_ind: usize,
}

impl<T> AddrDedup<T> {
    fn intern<'q, Q>(&mut self, v: &'q Q) -> usize
    where
        T: From<&'q Q>,
    {
        self.intern_with(v, &mut T::from)
    }

    fn intern_with<'q, Q>(&mut self, v: &'q Q, f: &mut impl FnMut(&'q Q) -> T) -> usize {
        let addr = v as *const Q as usize;
        if let Some(ind) = self.map.get(&addr) {
            ind.1
        } else {
            let i = self.next_ind;
            self.map.insert(addr, (f(v), i));
            self.next_ind += 1;
            i
        }
    }

    fn intern_iter<'q, Q: 'q>(&mut self, vs: impl IntoIterator<Item = &'q Q>) -> Vec<usize>
    where
        T: From<&'q Q>,
    {
        vs.into_iter().map(|v| self.intern(v)).collect()
    }

    fn intern_iter_with<'q, Q: 'q>(
        &mut self,
        vs: impl IntoIterator<Item = &'q Q>,
        f: &mut impl FnMut(&'q Q) -> T,
    ) -> Vec<usize> {
        vs.into_iter().map(|v| self.intern_with(v, f)).collect()
    }
}

impl<T> Default for AddrDedup<T> {
    fn default() -> Self {
        AddrDedup {
            map: HashMap::new(),
            next_ind: 0,
        }
    }
}

impl<T: Clone> Into<Vec<T>> for AddrDedup<T> {
    fn into(self) -> Vec<T> {
        // PERF: It would be safe to use `MaybeUninit` here
        let mut opt_res: Vec<Option<T>> = vec![None; self.map.len()];
        for (_, (t, i)) in self.map {
            opt_res[i] = Some(t);
        }
        opt_res.into_iter().map(Option::unwrap).collect()
    }
}

/// An interned version of a [`MethodSpec`]
#[derive(Debug, Clone, Serialize)]
struct SerMethod {
    name: String,
    shorthand: String,
    place_not_string: usize,
    is_panel_open: bool,
}

impl SerMethod {
    fn from_meth<'s>(m: &'s MethodSpec, interner: &mut Dedup<&'s str>) -> Self {
        SerMethod {
            name: m.name.borrow().clone(),
            shorthand: m.shorthand.borrow().clone(),
            place_not_string: interner.intern(&m.place_not_string),
            is_panel_open: m.is_panel_open.get(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize)]
struct SerAnnot {
    #[serde(default = "crate::ser_utils::get_false")]
    #[serde(skip_serializing_if = "crate::ser_utils::is_false")]
    is_lead_end: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    method: Option<MethodRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    call: Option<CallRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    is_fold_open: Option<bool>,
}

impl From<&Annot> for SerAnnot {
    fn from(a: &Annot) -> Self {
        Self {
            is_lead_end: a.is_lead_end,
            method: a.method,
            call: a.call,
            is_fold_open: a.fold.as_ref().map(|f| f.is_open.get()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct SerFrag {
    row_ranges: Vec<Range<usize>>,
    annot_ranges: Vec<Range<usize>>,
    is_muted: bool,
    x: f32,
    y: f32,
}

impl SerFrag {
    fn from_frag<'f>(
        frag: &'f Frag,
        row_interner: &mut Dedup<Row>,
        annot_interner: &mut Dedup<SerAnnot>,
    ) -> Self {
        // Intern all the rows
        let row_indices = row_interner.intern_iter(
            frag.block
                .rows()
                .map(|r| unsafe { frag.start_row.mul_unchecked(r) }),
        );
        // Intern the annotations
        let annot_indices =
            annot_interner.intern_iter(frag.block.annots().map(|a| SerAnnot::from(a)));

        SerFrag {
            row_ranges: range_compress(row_indices),
            annot_ranges: range_compress(annot_indices),
            is_muted: frag.is_muted,
            x: frag.x,
            y: frag.y,
        }
    }
}

/// An serialised version of a [`Spec`]
#[derive(Debug, Clone, Serialize)]
struct SerSpec {
    frags: Vec<usize>,
    part_head_str: usize,
    methods: Vec<usize>,
    stage: usize,
}

/// A fully serialised version of an undo history
#[derive(Debug, Clone, Serialize)]
struct SerHistory<'a> {
    specs: Vec<SerSpec>,
    #[serde(serialize_with = "crate::ser_utils::ser_rows")]
    rows: Vec<Row>,
    annots: Vec<SerAnnot>,
    strings: Vec<&'a str>,
    frags: Vec<SerFrag>,
    methods: Vec<SerMethod>,
}

/// Serialize a sequence of [`Spec`]s, duplicating as little data as possible
pub fn ser_history(specs: &[Spec]) -> String {
    let mut string_interner = Dedup::<&str>::default();
    let mut row_interner = Dedup::<Row>::default();
    let mut annot_interner = Dedup::<SerAnnot>::default();
    let mut frag_interner = AddrDedup::<SerFrag>::default();
    let mut method_interner = AddrDedup::<SerMethod>::default();

    let specs = specs
        .iter()
        // Serialize only the last undo steps
        .rev()
        .take(MAX_SAVED_UNDO_STEPS)
        .rev()
        // Serialize each Spec
        .map(|s| SerSpec {
            stage: s.stage.as_usize(),
            frags: frag_interner.intern_iter_with(s.frags.iter().map(Rc::as_ref), &mut |f| {
                SerFrag::from_frag(f, &mut row_interner, &mut annot_interner)
            }),
            part_head_str: string_interner.intern(s.part_heads.spec_string()),
            methods: method_interner.intern_iter_with(s.methods.iter().map(Rc::as_ref), &mut |m| {
                SerMethod::from_meth(m, &mut string_interner)
            }),
        })
        .collect::<Vec<_>>();

    serde_json::to_string(&SerHistory {
        specs,
        rows: row_interner.into(),
        annots: annot_interner.into(),
        strings: string_interner.into(),
        frags: frag_interner.into(),
        methods: method_interner.into(),
    })
    .unwrap()
}

fn range_compress(mut indices: Vec<usize>) -> Vec<Range<usize>> {
    // Push a `0` to the end so that the end is no longer a special case
    indices.push(0);
    // Turn this sequence of ints into a sequence of ranges (i.e. `[1, 2, 3, 4, 10, 11, 12]`
    // would compress into `[1..=4, 10..=12]`
    let mut ranges = Vec::new();
    let mut current_range_start: Option<(usize, usize)> = None;
    for (i, r) in indices.into_iter().enumerate() {
        if let Some((start_index, start_row)) = current_range_start {
            let rows_since_last_start = i - start_index;
            // If we reach a point where the row indices stop increasing 1 by 1, push the last
            // range and start again
            if r != start_row + rows_since_last_start {
                ranges.push(start_row..start_row + rows_since_last_start);
                current_range_start = Some((i, r));
            }
        } else {
            // If `current_range_start` is None, then we must be at the start.  Therefore, set
            current_range_start = Some((i, r));
        }
    }
    ranges
}
