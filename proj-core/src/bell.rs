//! A type-safe representation of a bell.

/// A lookup string of the bell names
const BELL_NAMES: &'static str = "1234567890ETABCDFGHJKLMNPQRSUVWXYZ";

/// A `Bell` representing the 'treble' on any stage.  Equivalent to `Bell::from_name('1')`.
///
/// # Example
/// ```
/// use proj_core::{Bell, bell::TREBLE};
///
/// // `TREBLE` should be the bell with name '1'
/// assert_eq!(Bell::from_name('1'), Some(TREBLE));
/// // The `TREBLE` has index 0, and its number is 1
/// assert_eq!(TREBLE.index(), 0);
/// assert_eq!(TREBLE.number(), 1);
/// // The treble should display as `"1"`
/// assert_eq!(TREBLE.name(), "1");
/// ```
pub const TREBLE: Bell = Bell { index: 0 };

/// A type-safe representation of a 'bell', which adds things like conversions to and from
/// commonly-used bell names.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Bell {
    /// A zero-indexed number representing the `Bell`.  I.e the treble is always
    /// `Bell { index: 0 }`, and the 12th is `Bell { index: 11 }` but would be
    /// [`Display`](std::fmt::Display)ed as `T`.
    index: usize,
}

impl Bell {
    /// Creates a `Bell` from a [`char`] containing a bell name (e.g. `'4'` or `'T'`).  If the name
    /// is not valid, then this fails and returns [`None`].  Note that lower case [`char`]s are not
    /// considered valid bell names.
    ///
    /// # Example
    /// ```
    /// use proj_core::Bell;
    ///
    /// // Converting a valid name to a `Bell` and back should be the identity function
    /// assert_eq!(Bell::from_name('1').unwrap().name(), "1");
    /// assert_eq!(Bell::from_name('5').unwrap().name(), "5");
    /// assert_eq!(Bell::from_name('0').unwrap().name(), "0");
    /// assert_eq!(Bell::from_name('T').unwrap().name(), "T");
    /// // Converting a lower-case letter should return `None`, even if the upper case
    /// // version is valid
    /// assert_eq!(Bell::from_name('e'), None);
    /// assert_eq!(Bell::from_name('t'), None);
    /// // Converting any old rubbish will return `None` (no shade on Ferris)
    /// assert_eq!(Bell::from_name('\r'), None);
    /// assert_eq!(Bell::from_name('🦀'), None);
    /// ```
    pub fn from_name(c: char) -> Option<Bell> {
        BELL_NAMES
            .chars()
            .position(|x| x == c)
            .map(Bell::from_index)
    }

    /// Creates a `Bell` from a 0-indexed integer.
    ///
    /// # Example
    /// ```
    /// use proj_core::Bell;
    ///
    /// // A 'Bell' with index 0 is the treble
    /// assert_eq!(Bell::from_index(0).name(), "1");
    /// // A 'Bell' with index 11 is the '12' or 'T'
    /// assert_eq!(Bell::from_index(11).name(), "T");
    /// ```
    #[inline]
    pub fn from_index(index: usize) -> Bell {
        Bell { index }
    }

    /// Creates a `Bell` from a 1-indexed integer.  This could fail if `number` is `0`, so in that
    /// case [`None`] is returned.
    ///
    /// # Example
    /// ```
    /// use proj_core::Bell;
    ///
    /// // The `Bell` with number '12' is the 12th and should be displayed as 'T'
    /// assert_eq!(Bell::from_number(12).unwrap().name(), "T");
    /// // Trying to create a Bell with number `0` fails:
    /// assert_eq!(Bell::from_number(0), None);
    /// ```
    pub fn from_number(number: usize) -> Option<Bell> {
        number.checked_sub(1).map(Bell::from_index)
    }

    /// Returns the 0-indexed representation of this `Bell`.
    ///
    /// # Example
    /// ```
    /// use proj_core::Bell;
    ///
    /// // Creating a `Bell` with `from_index` should return the same index passed to it
    /// assert_eq!(Bell::from_index(0).index(), 0);
    /// assert_eq!(Bell::from_index(12).index(), 12);
    /// ```
    #[inline]
    pub fn index(self) -> usize {
        self.index
    }

    /// Returns the 1-indexed representation of this `Bell`.
    ///
    /// # Example
    /// ```
    /// use proj_core::Bell;
    ///
    /// assert_eq!(Bell::from_index(0).number(), 1);
    /// // Using `from_number` should return the same number that was passed to it
    /// assert_eq!(Bell::from_number(4).unwrap().number(), 4);
    /// assert_eq!(Bell::from_number(10).unwrap().number(), 10);
    /// ```
    #[inline]
    pub fn number(self) -> usize {
        self.index + 1
    }

    /// Converts this `Bell` into the [`char`] that it should be displayed as.  If the `Bell` is
    /// too big to have a corresponding name, then [`None`] is returned.
    ///
    /// # Example
    /// ```
    /// use proj_core::Bell;
    ///
    /// // A 'Bell' with index 0 is the treble, and therefore displays as `1`
    /// assert_eq!(Bell::from_index(0).to_char(), Some('1'));
    /// // The 11th should display as 'E'
    /// assert_eq!(Bell::from_number(11).unwrap().to_char(), Some('E'));
    ///
    /// // Trying to display the 100th Bell fails:
    /// assert_eq!(Bell::from_number(100).unwrap().to_char(), None);
    /// ```
    pub fn to_char(&self) -> Option<char> {
        BELL_NAMES.as_bytes().get(self.index).map(|x| *x as char)
    }

    /// Converts this `Bell` into a [`String`] that it should be displayed as.  Unlike
    /// [`to_char`](Bell::to_char), this does not fail if the `Bell` is to big to have a name.
    /// Instead, it returns the 1-indexed ['number'](Bell::number) of the `Bell` in angle brackets.
    ///
    /// # Example
    /// ```
    /// use proj_core::Bell;
    ///
    /// // Bells which are <= 9th should return their number as a `String`
    /// assert_eq!(Bell::from_number(1).unwrap().name(), "1");
    /// assert_eq!(Bell::from_number(5).unwrap().name(), "5");
    /// assert_eq!(Bell::from_number(9).unwrap().name(), "9");
    /// // The 10th display as "0"
    /// assert_eq!(Bell::from_number(10).unwrap().name(), "0");
    /// // Other bells display as their single-character names
    /// assert_eq!(Bell::from_number(12).unwrap().name(), "T");
    /// assert_eq!(Bell::from_number(16).unwrap().name(), "D");
    /// // Anything too big simply display as '<{bell.number()}>'
    /// assert_eq!(Bell::from_number(100).unwrap().name(), "<100>");
    /// ```
    pub fn name(&self) -> String {
        match self.to_char() {
            None => format!("<{}>", self.number()),
            Some(c) => c.to_string(),
        }
    }
}

impl std::fmt::Display for Bell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
