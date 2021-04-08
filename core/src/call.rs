use crate::{method::LABEL_LEAD_END, Block, PnBlock, Row};

pub const NOTATION_BOB: char = '-';
pub const NOTATION_SINGLE: char = 's';

#[derive(Debug, Clone)]
pub struct Call {
    notation: char,
    loc: String,
    replaces: usize,
    block: Block,
}

impl Call {
    /// Creates a new `Call` from its parts
    #[inline]
    pub fn new(notation: char, loc: String, replaces: usize, block: Block) -> Self {
        Call {
            notation,
            loc,
            replaces,
            block,
        }
    }

    /// Creates a call with notation `'-'`, which covers only the lead end
    pub fn le_bob(pn_block: PnBlock) -> Self {
        Self::new(
            NOTATION_BOB,
            String::from(LABEL_LEAD_END),
            pn_block.len(),
            pn_block.to_block(),
        )
    }

    /// Creates a call with notation `'s'`, which covers only the lead end
    pub fn le_single(pn_block: PnBlock) -> Self {
        Self::new(
            NOTATION_SINGLE,
            String::from(LABEL_LEAD_END),
            pn_block.len(),
            pn_block.to_block(),
        )
    }

    /// Gets the [`char`] that represents this `Call`.
    #[inline]
    pub fn notation(&self) -> char {
        self.notation
    }

    /// Returns a [`Row`] representing the overall transposition of this `Call` (i.e. the
    /// permutation which maps the [`Row`] where this `Call` starts to the [`Row`] where it ends).
    /// This is useful for generating things such as calling positions.
    #[inline]
    pub fn transposition(&self) -> &Row {
        self.block.leftover_row()
    }
}
