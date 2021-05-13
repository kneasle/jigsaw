use std::cmp::Ordering;

use crate::{Bell, InvalidRowError, RowTrait, Stage};

/// A single `Row` of [`Bell`]s.
///
/// This can be viewed as a permutation of [rounds](Row::rounds) on a given [`Stage`].
///
/// A `Row` must always be valid according to
/// [the Framework](https://cccbr.github.io/method_ringing_framework/fundamentals.html) - i.e., it
/// must contain every [`Bell`] up to its [`Stage`] once and precisely once.  This is only checked
/// in the constructors and then used as assumed knowledge to avoid further checks.  This is
/// similar to how [`&str`](str) and [`String`] are required to be valid UTF-8.
///
/// # Example
/// ```
/// use proj_core::{Bell, Row, RowTrait, Stage, InvalidRowError};
///
/// // Create rounds on 8 bells.  Rounds is always valid on any `Stage`
/// let rounds_on_8 = Row::rounds(Stage::MAJOR);
/// assert_eq!(rounds_on_8.stage(), Stage::MAJOR);
/// assert_eq!(rounds_on_8.to_string(), "12345678");
///
/// // Parse a generic (valid) change from a string.  Note how invalid
/// // `char`s are skipped.  This could fail if the resulting `Row` is
/// // invalid, so we use ? to propogate that error out of the current
/// // function.
/// let queens = Row::parse("13579 | 24680")?;
/// assert_eq!(queens.stage(), Stage::ROYAL);
/// assert_eq!(queens.to_string(), "1357924680");
///
/// // If we try to parse an invalid `Row`, we get an error.  This means
/// // that we can assume that all `Row`s satisfy the Framework's definition
/// assert_eq!(
///     Row::parse("112345"),
///     Err(InvalidRowError::DuplicateBell(Bell::from_name('1').unwrap()))
/// );
/// #
/// # Ok::<(), InvalidRowError>(())
/// ```
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Row {
    /// The [`Bell`]s in the order that they would be rung.  Because of the 'valid row' invariant,
    /// this can't contain duplicate [`Bell`]s or any [`Bell`]s with number greater than the
    /// [`Stage`] of this [`Row`].
    bells: Vec<Bell>,
}

impl Row {
    /// Creates a `Row` from a [`Vec`] of [`Bell`]s, checking that the the resulting `Row` is valid.
    ///
    /// # Example
    /// ```
    /// use proj_core::{Bell, InvalidRowError, Row, RowTrait};
    ///
    /// // Converting a `Row` from a valid `Vec` of `Bell`s is fine
    /// assert_eq!(
    ///     Row::from_vec(vec![
    ///         Bell::from_name('4').unwrap(),
    ///         Bell::from_name('2').unwrap(),
    ///         Bell::from_name('1').unwrap(),
    ///         Bell::from_name('3').unwrap(),
    ///     ])?.to_string(),
    ///     "4213"
    /// );
    /// // Converting a `Row` from an invalid `Vec` of `Bell`s is not so fine
    /// assert_eq!(
    ///     Row::from_vec(vec![
    ///         Bell::from_name('4').unwrap(),
    ///         Bell::from_name('2').unwrap(),
    ///         Bell::from_name('1').unwrap(),
    ///         Bell::from_name('4').unwrap(),
    ///     ]),
    ///     Err(InvalidRowError::DuplicateBell(Bell::from_name('4').unwrap()))
    /// );
    /// # Ok::<(), InvalidRowError>(())
    /// ```
    pub fn from_vec(bells: Vec<Bell>) -> Result<Row, InvalidRowError> {
        // This unsafety is OK because the resulting row is never used for anything other than a
        // validity check
        unsafe { Self::from_vec_unchecked(bells) }.check_validity()
    }

    /// Creates a `Row` from a [`Vec`] of [`Bell`]s, **without** checking that the the resulting
    /// `Row` is valid.  Only use this if you're certain that the input is valid, since performing
    /// invalid operations on `Row`s is undefined behaviour.
    ///
    /// # Safety
    ///
    /// This function is safe if `bells` corresponds to a valid `Row` according to the CC's
    /// Framework.  This means that each [`Bell`] is unique, and has [`index`](Bell::index) smaller
    /// than the `bells.len()`.
    ///
    /// # Example
    /// ```
    /// use proj_core::{Bell, InvalidRowError, Row, RowTrait};
    ///
    /// # fn test() -> Option<()> {
    /// // Converting a `Row` from a valid `Vec` of `Bell`s is fine, but still unsafe
    /// assert_eq!(
    ///     unsafe {
    ///         Row::from_vec_unchecked(vec![
    ///             Bell::from_name('4')?,
    ///             Bell::from_name('2')?,
    ///             Bell::from_name('1')?,
    ///             Bell::from_name('3')?,
    ///         ])
    ///     }.to_string(),
    ///     "4213"
    /// );
    /// // Converting a `Row` from an invalid `Vec` of `Bell`s compiles and runs,
    /// // but silently creates an invalid `Row`
    /// assert_eq!(
    ///     unsafe {
    ///         Row::from_vec_unchecked(vec![
    ///             Bell::from_name('4')?,
    ///             Bell::from_name('2')?,
    ///             Bell::from_name('1')?,
    ///             Bell::from_name('4')?,
    ///         ])
    ///     }.to_string(),
    ///     "4214"
    /// );
    /// # Some(())
    /// # }
    /// # fn main() { test().unwrap() }
    /// ```
    #[inline]
    pub unsafe fn from_vec_unchecked(bells: Vec<Bell>) -> Row {
        Row { bells }
    }

    /* Once GATs are possible, we will be able to make this part of RowTrait */

    /// Returns an iterator over the [`Bell`]s in this [`Row`]
    #[inline]
    pub fn bell_iter(&self) -> std::iter::Cloned<std::slice::Iter<'_, Bell>> {
        self.slice().iter().cloned()
    }

    /// Returns an immutable reference to the underlying slice of [`Bell`]s that makes up this
    /// `Row`.
    ///
    /// # Example
    /// ```
    /// use proj_core::{Bell, Row, RowTrait};
    ///
    /// let tittums = Row::parse("15263748")?;
    /// assert_eq!(tittums.slice()[3], Bell::from_name('6').unwrap());
    /// # Ok::<(), proj_core::InvalidRowError>(())
    /// ```
    #[inline]
    pub fn slice(&self) -> &[Bell] {
        self.bells.as_slice()
    }

    /// Concatenates the names of the [`Bell`]s in this `Row` to the end of a [`String`].  Using
    /// `row.to_string()` will behave the same as this but will return an newly allocated
    /// [`String`].
    ///
    /// # Example
    /// ```
    /// use proj_core::{Row, RowTrait};
    ///
    /// let waterfall = Row::parse("6543217890")?;
    /// let mut string = "Waterfall is: ".to_owned();
    /// waterfall.push_to_string(&mut string);
    /// assert_eq!(string, "Waterfall is: 6543217890");
    /// # Ok::<(), proj_core::InvalidRowError>(())
    /// ```
    pub fn push_to_string(&self, string: &mut String) {
        for b in &self.bells {
            string.push_str(&b.name());
        }
    }

    /// A very collision-resistant hash function.  It is guarunteed to be perfectly
    /// collision-resistant on the following [`Stage`]s:
    /// - 16-bit machines: Up to 6 bells
    /// - 32-bit machines: Up to 9 bells
    /// - 64-bit machines: Up to 16 bells
    ///
    /// This hashing algorithm works by reading the row as a number using the stage as a base, thus
    /// guarunteeing that (ignoring overflow), two [`Row`]s will only be hashed to the same value
    /// if they are in fact the same.  This is ludicrously inefficient in terms of hash density,
    /// but it is fast and perfect and in most cases will suffice.
    pub fn fast_hash(&self) -> usize {
        let mut accum = 0;
        let mut multiplier = 1;
        for b in self.bells.iter() {
            accum += b.index() * multiplier;
            multiplier *= self.stage().as_usize();
        }
        accum
    }
}

impl std::fmt::Debug for Row {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Row({})", self.to_string())
    }
}

impl std::fmt::Display for Row {
    /// Returns a [`String`] representing this `Row`.
    ///
    /// # Example
    /// ```
    /// use proj_core::{Row, RowTrait, Stage};
    ///
    /// assert_eq!(Row::rounds(Stage::MAJOR).to_string(), "12345678");
    /// assert_eq!(Row::parse("146235")?.to_string(), "146235");
    /// # Ok::<(), proj_core::InvalidRowError>(())
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::with_capacity(self.stage().as_usize());
        self.push_to_string(&mut s);
        write!(f, "{}", s)
    }
}

impl std::ops::Index<usize> for Row {
    type Output = Bell;

    fn index(&self, index: usize) -> &Bell {
        &self.slice()[index]
    }
}

impl std::ops::Mul for Row {
    type Output = Row;

    /// See [`&Row * &Row`](<&Row as std::ops::Mul>::mul) for docs.
    fn mul(self, rhs: Row) -> Self::Output {
        // Delegate to the borrowed version
        &self * &rhs
    }
}

impl std::ops::Mul for &Row {
    type Output = Row;

    /// Uses the RHS to permute the LHS without consuming either argument.
    ///
    /// # Example
    /// ```
    /// use proj_core::{Row, RowTrait};
    ///
    /// // Multiplying two Rows of the same Stage just returns a new Row
    /// assert_eq!(
    ///     &Row::parse("13425678")? * &Row::parse("43217568")?,
    ///     Row::parse("24317568")?
    /// );
    /// # Ok::<(), proj_core::InvalidRowError>(())
    /// ```
    ///
    /// ```should_panic
    /// use proj_core::{Row, RowTrait};
    ///
    /// // Multiplying two Rows of different Stages panics rather than
    /// // producing undefined behaviour
    /// let _unrow = &Row::parse("13425678")? * &Row::parse("4321")?;
    /// # Ok::<(), proj_core::InvalidRowError>(())
    /// ```
    fn mul(self, rhs: &Row) -> Self::Output {
        assert_eq!(self.stage(), rhs.stage());
        // This unsafety is OK because the product of two valid Rows of the same Stage is always
        // valid (because groups are closed under their binary operation).
        unsafe { self.mul_unchecked(rhs) }
    }
}

impl std::ops::Not for Row {
    type Output = Row;

    /// See [`!&Row`](<&Row as std::ops::Not>::not) for docs.
    #[inline]
    fn not(self) -> Self::Output {
        // Delegate to the borrowed version
        !&self
    }
}

impl std::ops::Not for &Row {
    type Output = Row;

    /// Find the inverse of a [`Row`].  If `X` is the input [`Row`], and `Y = !X`, then
    /// `XY = YX = I` where `I` is the identity on the same stage as `X` (i.e. rounds).  This
    /// operation cannot fail, since valid [`Row`]s are guaruteed to have an inverse.
    ///
    /// # Example
    /// ```
    /// use proj_core::{Row, RowTrait, Stage};
    ///
    /// // The inverse of Queens is Tittums
    /// assert_eq!(!Row::parse("135246")?, Row::parse("142536")?);
    /// // Backrounds is self-inverse
    /// assert_eq!(!Row::backrounds(Stage::MAJOR), Row::backrounds(Stage::MAJOR));
    /// // `1324` inverts to `1423`
    /// assert_eq!(!Row::parse("1342")?, Row::parse("1423")?);
    /// #
    /// # Ok::<(), proj_core::InvalidRowError>(())
    /// ```
    fn not(self) -> Self::Output {
        self.inv()
    }
}

impl RowTrait for Row {
    unsafe fn from_iter_unchecked(iter: impl Iterator<Item = Bell>) -> Self {
        Self::from_vec_unchecked(iter.collect())
    }

    #[inline]
    fn stage(&self) -> Stage {
        self.bells.len().into()
    }

    unsafe fn mul_unchecked(&self, rhs: &Self) -> Self {
        // We bypass the validity check because if two Rows are valid, then so is their product.
        // However, this function is also unsafe because permuting two rows of different Stages
        // causes undefined behaviour
        Row::from_vec_unchecked(rhs.bell_iter().map(|b| self[b.index()]).collect())
    }

    unsafe fn mul_into_unchecked(&self, rhs: &Self, out: &mut Self) {
        // We bypass the validity check because if two Rows are valid, then so is their product.
        // However, this function is also unsafe because permuting two rows of different Stages
        // causes undefined behaviour
        out.bells.clear();
        out.bells.extend(rhs.bell_iter().map(|b| self[b.index()]));
    }

    fn inv(&self) -> Self {
        let mut inv_bells = vec![Bell::TREBLE; self.stage().as_usize()];
        for (i, b) in self.bells.iter().enumerate() {
            inv_bells[b.index()] = Bell::from_index(i);
        }
        // This unsafety is OK because Rows form a group and by the closure of groups under
        // inversion, if `self` is in the group of permutations, then so is `!self`.
        unsafe { Row::from_vec_unchecked(inv_bells) }
    }

    fn inv_into(&self, out: &mut Self) {
        // Make sure that `out` has the right stage
        match out.stage().cmp(&self.stage()) {
            Ordering::Less => {
                out.bells.extend(
                    std::iter::repeat(Bell::TREBLE).take(self.bells.len() - out.bells.len()),
                );
            }
            Ordering::Greater => {
                out.bells.drain(self.bells.len()..);
            }
            Ordering::Equal => {}
        }
        debug_assert_eq!(out.stage(), self.stage());
        // Now perform the inversion
        for (i, b) in self.bell_iter().enumerate() {
            out.bells[b.index()] = Bell::from_index(i);
        }
    }

    fn extend_to_stage(&mut self, stage: Stage) {
        self.bells
            .extend((self.bells.len()..stage.as_usize()).map(Bell::from_index));
    }

    /// Checks the validity of a potential `Row`, returning it if valid and returning an
    /// [`InvalidRowError`] otherwise (consuming the potential `Row` so it can't be used).
    fn check_validity(self) -> Result<Self, InvalidRowError> {
        super::check_validity(self.stage(), self.bell_iter())?;
        Ok(self)
    }

    /// Checks the validity of a potential `Row`, extending it to the given [`Stage`] if valid and
    /// returning an [`InvalidRowError`] otherwise (consuming the potential `Row` so it can't be
    /// used).  This will provide nicer errors than [`Row::check_validity`] since this has extra
    /// information about the desired [`Stage`] of the potential `Row`.
    fn check_validity_with_stage(mut self, stage: Stage) -> Result<Self, InvalidRowError> {
        super::check_validity_with_stage(stage, self.bell_iter())?;
        // If no errors were generated so far, then extend the row and return
        self.extend_to_stage(stage);
        Ok(self)
    }

    #[inline]
    fn place_of(&self, bell: Bell) -> Option<usize> {
        self.bells.iter().position(|b| *b == bell)
    }

    #[inline]
    fn swap(&mut self, a: usize, b: usize) {
        self.bells.swap(a, b);
    }

    fn is_rounds(&self) -> bool {
        self.bell_iter().enumerate().all(|(i, b)| b.index() == i)
    }
}
