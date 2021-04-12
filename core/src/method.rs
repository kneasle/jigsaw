use crate::{block::AnnotRowIter, AnnotBlock, PnBlock, Stage};

// Imports used solely for doc comments
#[allow(unused_imports)]
use crate::{Block, Row};

/// A standard label name used for denoting the 'lead end' of a method
pub const LABEL_LEAD_END: &str = "LE";

/// The definition of a 'method' within Change Ringing.  This struct follows quite a loose
/// definition, which allows the `Method` struct to represent things that may not count as methods
/// (as determined by the Framework).  Essentially, a `Method` consists of a [`Block`] which is
/// intended to be rung as a repeating unit (usually a 'lead'), along with names for specific
/// locations within this [`Block`].  Calls can then be attached to these locations (by name), and
/// thus the single lead can be modified to determine the effect of calls in a general way.  This
/// follows how complib.org's composition input works.
#[derive(Debug, Clone)]
pub struct Method {
    name: String,
    first_lead: AnnotBlock<Option<String>>,
}

impl Method {
    /// Creates a new `Method` given its name and a [`Block`]
    pub fn new(name: String, first_lead: AnnotBlock<Option<String>>) -> Self {
        Method { name, first_lead }
    }

    /// Creates a new `Method` from some place notation, adding a lead end annotation.
    pub fn with_lead_end(name: String, block: &PnBlock) -> Self {
        let mut first_lead: AnnotBlock<Option<String>> = block.to_block();
        *first_lead.get_annot_mut(first_lead.len() - 1).unwrap() =
            Some(String::from(LABEL_LEAD_END));
        Method { name, first_lead }
    }

    /// Returns an `AnnotBlock` of the first lead of this `Method`
    #[inline]
    pub fn lead(&self) -> &AnnotBlock<Option<String>> {
        &self.first_lead
    }

    /// The overall transposing effect of one lead of this `Method`.
    #[inline]
    pub fn lead_head(&self) -> &Row {
        self.first_lead.leftover_row()
    }

    /// How many [`Row`]s are in a single lead of this `Method`?
    #[inline]
    pub fn lead_len(&self) -> usize {
        self.first_lead.len()
    }

    /// Gets the [`Stage`] of this `Method`
    #[inline]
    pub fn stage(&self) -> Stage {
        self.first_lead.stage()
    }

    /// Gets the name of this `Method`
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Generates a new [`CourseIter`] which generates an infinite course of this [`Method`],
    /// starting at a given `starting_row`.
    #[inline]
    pub fn course_iter(&self, starting_row: Row) -> CourseIter<'_> {
        CourseIter::new(self, starting_row)
    }

    /// Generates a new [`CourseIter`] which generates the plain course of this [`Method`] forever.
    #[inline]
    pub fn plain_course_iter(&self) -> CourseIter<'_> {
        CourseIter::new(self, Row::rounds(self.stage()))
    }

    /// Sets or clears the label at a given index, panicking if the index is out of range
    pub fn set_label(&mut self, index: usize, label: Option<String>) {
        *self.first_lead.get_annot_mut(index).unwrap() = label;
    }

    /// Returns the label at a given index, panicking if the index is out of range
    pub fn get_label(&self, index: usize) -> Option<&str> {
        if let Some(s) = self.first_lead.get_annot(index).unwrap() {
            Some(s.as_str())
        } else {
            None
        }
    }
}

/// Type alias used for brevity in [`CourseIter`]
type _InternalIter<'m> =
    std::iter::Peekable<std::iter::Enumerate<AnnotRowIter<'m, Option<String>>>>;

/// An iterator that generates repeating leads of a given [`Method`].  **This iterator never
/// returns.**
#[derive(Clone, Debug)]
pub struct CourseIter<'m> {
    method: &'m Method,
    current_iter: _InternalIter<'m>,
    // PERF: We could replace this with an accumulator to stop needless allocations
    current_lead_head: Row,
}

impl<'m> CourseIter<'m> {
    /// Creates a new `CourseIter` which generates a given [`Method`], beginning at some inital
    /// [`Row`].
    fn new(method: &'m Method, first_lead_head: Row) -> Self {
        CourseIter {
            method,
            current_lead_head: first_lead_head,
            current_iter: Self::get_iter(method),
        }
    }

    /// Gets a new [`_InternalIter`] from a [`Method`]
    fn get_iter(method: &'m Method) -> _InternalIter<'m> {
        method.first_lead.annot_rows().iter().enumerate().peekable()
    }
}

// PERF: We should implement more of the iterator methods like `skip`, which are used extensively
// but generate very bad code by default
impl<'m> Iterator for CourseIter<'m> {
    type Item = (usize, Option<&'m str>, Row);

    fn next(&mut self) -> Option<Self::Item> {
        // If the iterator is about to finish, then move on by a lead and create a new iterator
        if self.current_iter.peek().is_none() {
            self.current_iter = Self::get_iter(self.method);
            // This unsafety is OK because the rows all originate from the same `AnnotBlock`
            // which guarutees that its rows have the same stage
            self.current_lead_head = unsafe {
                self.current_lead_head
                    .mul_unchecked(&self.method.lead_head())
            };
        }
        // Now, generate the next item to return.  Unwrapping here is fine, because
        // `self.current_iter` must generate at least one Row (because methods can never have a
        // 0-length lead)
        let (sub_lead_index, annot_r) = self.current_iter.next().unwrap();
        Some((
            sub_lead_index,
            annot_r.annot().as_deref(),
            // This unsafety is OK because the rows all originate from the same `AnnotBlock` which
            // guarutees that its rows have the same stage
            unsafe { self.current_lead_head.mul_unchecked(annot_r.row()) },
        ))
    }
}
