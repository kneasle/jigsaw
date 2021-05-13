use proj_core::{Row, RowTrait};
use serde::Serialize;
use std::{collections::HashMap, hash::Hash, rc::Rc};

use super::{Frag, MethodSpec, Spec};

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

#[derive(Debug, Clone, Serialize)]
struct SerFrag {
    rows: Vec<usize>,
    is_muted: bool,
    x: f32,
    y: f32,
}

impl SerFrag {
    fn from_frag<'f>(frag: &'f Frag, row_interner: &mut Dedup<Row>) -> Self {
        SerFrag {
            // TODO: Range compress this - there'll be a **ton** of sequences in this data
            rows: row_interner.intern_iter(
                frag.block
                    .rows()
                    .map(|r| unsafe { frag.start_row.mul_unchecked(r) }),
            ),
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
    strs: Vec<&'a str>,
    frags: Vec<SerFrag>,
    methods: Vec<SerMethod>,
}

/// Serialize a sequence of [`Spec`]s, duplicating as little data as possible
pub fn ser_history(specs: &[Spec]) -> String {
    let mut string_interner = Dedup::<&str>::default();
    let mut row_interner = Dedup::<Row>::default();
    let mut frag_interner = AddrDedup::<SerFrag>::default();
    let mut method_interner = AddrDedup::<SerMethod>::default();

    let specs = specs
        .iter()
        .map(|s| SerSpec {
            stage: s.stage.as_usize(),
            frags: frag_interner.intern_iter_with(s.frags.iter().map(Rc::as_ref), &mut |f| {
                SerFrag::from_frag(f, &mut row_interner)
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
        strs: string_interner.into(),
        frags: frag_interner.into(),
        methods: method_interner.into(),
    })
    .unwrap()
}
