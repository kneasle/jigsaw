//! A type-safe representation of a bell.

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Bell(usize);
