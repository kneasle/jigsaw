//! A representation of a stage, with human-friendly `const`s and display names.

/// A newtype over [`usize`] that represents a stage.
///
/// To create a new `Stage`, you can either create it directly by using `Stage::from(usize)` or use
/// a constant for the human name for each `Stage`:
/// ```
/// use proj_core::Stage;
///
/// // Converting from numbers is the same as using the constants
/// assert_eq!(Stage::SINGLES, Stage::from(3));
/// assert_eq!(Stage::MAJOR, Stage::from(8));
/// assert_eq!(Stage::CINQUES, Stage::from(11));
/// assert_eq!(Stage::SIXTEEN, Stage::from(16));
/// // We can use `Stage::from` to generate `Stage`s that don't have names
/// assert_eq!(Stage::from(2).as_usize(), 2);
/// assert_eq!(Stage::from(100).as_usize(), 100);
/// ```
///
/// `Stage`s with names will also be [`Display`](std::fmt::Display)ed as their names:
/// ```
/// # use proj_core::Stage;
/// #
/// assert_eq!(&format!("{}", Stage::MAXIMUS), "Maximus");
/// assert_eq!(&format!("{}", Stage::from(9)), "Caters");
/// ```
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Stage(usize);

impl Stage {
    /// Returns this `Stage` as a [`usize`].
    ///
    /// # Example
    /// ```
    /// use proj_core::Stage;
    ///
    /// assert_eq!(Stage::DOUBLES.as_usize(), 5);
    /// assert_eq!(Stage::MAXIMUS.as_usize(), 12);
    /// ```
    #[inline]
    pub fn as_usize(self) -> usize {
        self.0
    }
}

/// User-friendly constants for commonly used `Stage`s.
///
/// # Example
/// ```
/// use proj_core::Stage;
///
/// assert_eq!(Stage::MINIMUS, Stage::from(4));
/// assert_eq!(Stage::MINOR, Stage::from(6));
/// assert_eq!(Stage::TRIPLES, Stage::from(7));
/// assert_eq!(Stage::FOURTEEN, Stage::from(14));
/// assert_eq!(Stage::SEXTUPLES, Stage::from(15));
/// ```
impl Stage {
    /// A `Stage` with `3` working bells
    pub const SINGLES: Stage = Stage(3);

    /// A `Stage` with `4` working bells
    pub const MINIMUS: Stage = Stage(4);

    /// A `Stage` with `5` working bells
    pub const DOUBLES: Stage = Stage(5);

    /// A `Stage` with `6` working bells
    pub const MINOR: Stage = Stage(6);

    /// A `Stage` with `7` working bells
    pub const TRIPLES: Stage = Stage(7);

    /// A `Stage` with `8` working bells
    pub const MAJOR: Stage = Stage(8);

    /// A `Stage` with `9` working bells
    pub const CATERS: Stage = Stage(9);

    /// A `Stage` with `10` working bells
    pub const ROYAL: Stage = Stage(10);

    /// A `Stage` with `11` working bells
    pub const CINQUES: Stage = Stage(11);

    /// A `Stage` with `12` working bells
    pub const MAXIMUS: Stage = Stage(12);

    /// A `Stage` with `13` working bells
    pub const SEPTUPLES: Stage = Stage(13);

    /// A `Stage` with `14` working bells
    pub const FOURTEEN: Stage = Stage(14);

    /// A `Stage` with `15` working bells
    pub const SEXTUPLES: Stage = Stage(15);

    /// A `Stage` with `16` working bells
    pub const SIXTEEN: Stage = Stage(16);
}

impl std::fmt::Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<usize> for Stage {
    fn from(n: usize) -> Self {
        Stage(n)
    }
}
