use crate::Perm;

/// A `Block` is a generalisation of [`Perm`], where instead of taking a [`Row`] and mapping that
/// to a single [`Row`], we map that [`Row`] to **multiple** [`Row`]s.
///
/// A few properties hold:
/// - A [`Perm`] is just a special case of a [`Block`] of length `1`.
/// - Blocks are closed under concatenation
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Block {
    /// The [`Perm`]s making up this `Block`.
    ///
    /// A few important implementation details to note:
    /// 1. All [`Perm`]s in `Block::perms` are permuting _the starting [`Row`]_, not each other.
    /// 2. There is an implicit [identity `Perm`](Perm::id) at the start of every `Block`, but this
    ///    is not stored in `Block::perms`.
    /// 3. The last [`Perm`] in `BlocK::perms` is 'left-over' - i.e. it shouldn't be used for truth
    ///    checking, and is used to generate the starting [`Row`] for the next `Block` in a chain
    ///
    /// As an example, let's take the `Block` representing one lead of
    /// [Bastow Little Bob Minor](https://rsw.me.uk/blueline/methods/view/Bastow_Little_Bob_Minor).
    /// In order to be as unambiguous as possible, I'm going to be permuting `abcdef`.  The rows we
    /// would want to truth check are
    /// ```text
    /// abcdef
    /// badcfe
    /// bacdef
    /// abdcfe
    /// ```
    /// and the 'left-over' [`Perm`] should be `adbfce`.  However, the `abcdef` at the start is an
    /// arbitrary choice of the input list, so we don't include it in the representation (following
    /// point 2.), and instead we store the 'left-over' [`Perm`] on the end of the [`Vec`]
    /// (following point 3.).  Therefore, this `Block` would be stored in memory as the following
    /// slice (note that [`Perm`]s are 0-indexed):
    /// ```ignore
    /// [
    ///     Perm::from(&[1, 0, 3, 2, 5, 4]),
    ///     Perm::from(&[1, 0, 2, 3, 4, 5]),
    ///     Perm::from(&[0, 1, 3, 2, 5, 4]),
    ///     Perm::from(&[0, 3, 1, 5, 2, 4]),
    /// ]
    /// ```
    perms: Vec<Perm>,
}
