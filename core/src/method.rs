use crate::{AnnotBlock, PnBlock};

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

    /// How many [`Row`]s are in a single lead of this `Method`?
    #[inline]
    pub fn lead_len(&self) -> usize {
        self.first_lead.len()
    }

    /// Gets the name of this `Method`
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets or clears the label at a given index, panicking if the index is out of range
    pub fn set_label(&mut self, index: usize, label: Option<String>) {
        *self.first_lead.get_annot_mut(index).unwrap() = label;
    }

    /// Returns the label at a given index, panicking if the index is out of range
    pub fn get_label(&mut self, index: usize) -> Option<&str> {
        if let Some(s) = self.first_lead.get_annot(index).unwrap() {
            Some(s.as_str())
        } else {
            None
        }
    }
}
