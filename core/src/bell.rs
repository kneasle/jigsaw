//! A type-safe representation of a bell.

/// A lookup string of the bell names
const BELL_NAMES: &str = "1234567890ETABCDFGHJKLMNPQRSUVWXYZ";

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
    /// # fn test() -> Option<()> {
    /// use proj_core::Bell;
    ///
    /// // Converting a valid name to a `Bell` and back should be the identity function
    /// assert_eq!(Bell::from_name('1')?.name(), "1");
    /// assert_eq!(Bell::from_name('5')?.name(), "5");
    /// assert_eq!(Bell::from_name('0')?.name(), "0");
    /// assert_eq!(Bell::from_name('T')?.name(), "T");
    /// // Converting a lower-case letter should return `None`, even if the upper case
    /// // version is valid
    /// assert_eq!(Bell::from_name('e'), None);
    /// assert_eq!(Bell::from_name('t'), None);
    /// // Converting any old rubbish will return `None` (no shade on Ferris)
    /// assert_eq!(Bell::from_name('\r'), None);
    /// assert_eq!(Bell::from_name('🦀'), None);
    /// # Some(())
    /// # }
    /// # fn main() { test().unwrap() }
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
    /// # fn test() -> Option<()> {
    /// use proj_core::Bell;
    ///
    /// // The `Bell` with number '12' is the 12th and should be displayed as 'T'
    /// assert_eq!(Bell::from_number(12)?.name(), "T");
    /// // Trying to create a Bell with number `0` fails:
    /// assert_eq!(Bell::from_number(0), None);
    /// # Some(())
    /// # }
    /// # fn main() { test().unwrap() }
    /// ```
    pub fn from_number(number: usize) -> Option<Bell> {
        number.checked_sub(1).map(Bell::from_index)
    }

    /// A [`Bell`] representing the 'treble' on any stage.  Equivalent to
    /// `Bell::from_name('1').unwrap()`.
    ///
    /// # Example
    /// ```
    /// use proj_core::Bell;
    ///
    /// // `TREBLE` should be the bell with name '1'
    /// assert_eq!(Bell::from_name('1'), Some(Bell::TREBLE));
    /// // The `TREBLE` has index 0, and its number is 1
    /// assert_eq!(Bell::TREBLE.index(), 0);
    /// assert_eq!(Bell::TREBLE.number(), 1);
    /// // The treble should display as `"1"`
    /// assert_eq!(Bell::TREBLE.name(), "1");
    /// ```
    pub const TREBLE: Bell = Bell { index: 0 };

    /// Converts this `Bell` into the [`char`] that it should be displayed as.  If the `Bell` is
    /// too big to have a corresponding name, then [`None`] is returned.
    ///
    /// # Example
    /// ```
    /// # fn test() -> Option<()> {
    /// use proj_core::Bell;
    ///
    /// // A 'Bell' with index 0 is the treble, and therefore displays as `1`
    /// assert_eq!(Bell::from_index(0).to_char(), Some('1'));
    /// // The 11th should display as 'E'
    /// assert_eq!(Bell::from_number(11)?.to_char(), Some('E'));
    ///
    /// // Trying to display the 100th Bell fails:
    /// assert_eq!(Bell::from_number(100)?.to_char(), None);
    /// # Some(())
    /// # }
    /// # fn main() { test().unwrap() }
    /// ```
    pub fn to_char(&self) -> Option<char> {
        BELL_NAMES.as_bytes().get(self.index).map(|x| *x as char)
    }

    /// Returns the 0-indexed representation of this `Bell`.
    ///
    /// # Example
    /// ```
    /// # fn test() -> Option<()> {
    /// use proj_core::Bell;
    ///
    /// // Creating a `Bell` with `from_index` should return the same index passed to it
    /// assert_eq!(Bell::from_index(0).index(), 0);
    /// assert_eq!(Bell::from_index(12).index(), 12);
    ///
    /// assert_eq!(Bell::from_name('8')?.index(), 7);
    /// assert_eq!(Bell::from_name('0')?.index(), 9);
    /// assert_eq!(Bell::from_name('T')?.index(), 11);
    /// # Some(())
    /// # }
    /// # fn main() { test().unwrap() }
    /// ```
    #[inline]
    pub fn index(self) -> usize {
        self.index
    }

    /// Returns the 1-indexed representation of this `Bell`.
    ///
    /// # Example
    /// ```
    /// # fn test() -> Option<()> {
    /// use proj_core::Bell;
    ///
    /// assert_eq!(Bell::from_index(0).number(), 1);
    /// assert_eq!(Bell::from_name('0')?.number(), 10);
    /// // Using `from_number` should return the same number that was passed to it
    /// assert_eq!(Bell::from_number(4)?.number(), 4);
    /// assert_eq!(Bell::from_number(10)?.number(), 10);
    /// # Some(())
    /// # }
    /// # fn main() { test().unwrap() }
    /// ```
    #[inline]
    pub fn number(self) -> usize {
        self.index + 1
    }

    /// Converts this `Bell` into a [`String`] that it should be displayed as.  Unlike
    /// [`to_char`](Bell::to_char), this does not fail if the `Bell` is to big to have a name.
    /// Instead, it returns the 1-indexed ['number'](Bell::number) of the `Bell` in angle brackets.
    ///
    /// # Example
    /// ```
    /// # fn test() -> Option<()> {
    /// use proj_core::Bell;
    ///
    /// // Bells which are <= 9th should return their number as a `String`
    /// assert_eq!(Bell::from_number(1)?.name(), "1");
    /// assert_eq!(Bell::from_number(5)?.name(), "5");
    /// assert_eq!(Bell::from_number(9)?.name(), "9");
    /// // The 10th display as "0"
    /// assert_eq!(Bell::from_number(10)?.name(), "0");
    /// // Other bells display as their single-character names
    /// assert_eq!(Bell::from_number(12)?.name(), "T");
    /// assert_eq!(Bell::from_number(16)?.name(), "D");
    /// // Anything too big simply displays as '<{bell.number()}>'
    /// assert_eq!(Bell::from_number(100)?.name(), "<100>");
    /// # Some(())
    /// # }
    /// # fn main() { test().unwrap() }
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
