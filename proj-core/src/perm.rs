//! A representation of generic permutations (i.e. a [`Perm`] doesn't just have to permute
//! [`Row`]s).

use crate::Stage;

// Imports that are only used by doc comments (meaning rustc will generate a warning if not
// suppressed)
#[allow(unused_imports)]
use crate::Row;

/// An error created when a [`Perm`] was used to permute something with the wrong length
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct IncompatibleStages {
    /// The length of the slice that was attempted to be permuted
    pub length: usize,
    /// The stage of the [`Perm`] that was doing the permuting
    pub perm_stage: Stage,
}

impl IncompatibleStages {
    /// Constructs a new [`IncompatibleStages`] from its constituent parts
    ///
    /// # Example
    /// ```
    /// use proj_core::{Perm, Stage, perm::IncompatibleStages};
    ///
    /// assert_eq!(
    ///     format!("{}", IncompatibleStages::new(4, Stage::MAJOR)),
    ///     "A Perm of stage 8 can't permute a slice of len 4"
    /// );
    /// ```
    pub fn new(length: usize, perm_stage: Stage) -> Self {
        IncompatibleStages { length, perm_stage }
    }
}

impl std::fmt::Display for IncompatibleStages {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A Perm of stage {} can't permute a slice of len {}",
            self.perm_stage.as_usize(),
            self.length
        )
    }
}

impl std::error::Error for IncompatibleStages {}

/// A representation of a permutation.  A permutation can be thought of as a function that takes
/// any sequence and returns the same sequence with the elements in a different order.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Perm {
    /// A [`Vec`] representing the underlying permutation as a sequence of indices.
    vec: Vec<usize>,
}

impl Perm {
    /// Returns the identity [`Perm`] on a given [`Stage`].
    pub fn id(stage: Stage) -> Perm {
        Perm {
            vec: (0..stage.as_usize()).collect(),
        }
    }

    /// Returns the [`Stage`] of this permutation.
    #[inline]
    pub fn stage(&self) -> Stage {
        Stage::from(self.vec.len())
    }

    /// Permute a slice of any datatype according to the permutation represented by this [`Perm`],
    /// and returns a [`Vec`] of `Clone`d versions of those items.
    pub fn permute<T>(&self, slice: &[T]) -> Result<Vec<T>, IncompatibleStages>
    where
        T: Clone,
    {
        // Generate an error if the slice has the wrong length
        if slice.len() != self.stage().as_usize() {
            return Err(IncompatibleStages {
                length: slice.len(),
                perm_stage: self.stage(),
            });
        }
        // If the lengths are the same, we can perform the permutation using some iterator magic
        Ok(self.vec.iter().map(|i| slice[*i].clone()).collect())
    }
}

impl From<&[usize]> for Perm {
    fn from(slice: &[usize]) -> Self {
        Perm {
            vec: slice.iter().copied().collect(),
        }
    }
}

impl From<Vec<usize>> for Perm {
    fn from(vec: Vec<usize>) -> Self {
        Perm { vec }
    }
}

impl std::ops::Mul<Perm> for Perm {
    type Output = Result<Perm, IncompatibleStages>;

    /// Multiply two [`Perm`]s together (consuming both inputs).  This has the effect of using the
    /// RHS to permute the LHS, which is opposite to the mathematical matrix-based representation
    /// of permutations.
    ///
    /// # Example
    /// ```
    /// use proj_core::Perm;
    ///
    /// // Calculate the Perm that represents permuting by '2134' and then '4321'
    /// assert_eq!(
    ///     Perm::from(vec![0, 1, 2, 3]) * Perm::from(vec![3, 2, 1, 0]),
    ///     Ok(Perm::from(vec![3, 2, 1, 0]))
    /// );
    /// ```
    fn mul(self, rhs: Perm) -> Self::Output {
        &self * &rhs
    }
}

impl std::ops::Mul<&'_ Perm> for &'_ Perm {
    type Output = Result<Perm, IncompatibleStages>;

    /// Multiply two borrowed [`Perm`]s together.  This has the effect of using the RHS to permute
    /// the LHS, which is opposite to the mathematical matrix-based representation of permutations.
    ///
    /// # Example
    /// ```
    /// use proj_core::Perm;
    ///
    /// // Calculate the Perm that represents permuting by '2134' and then '4321'
    /// assert_eq!(
    ///     &Perm::from(vec![0, 1, 2, 3]) * &Perm::from(vec![3, 2, 1, 0]),
    ///     Ok(Perm::from(vec![3, 2, 1, 0]))
    /// );
    /// ```
    fn mul(self, rhs: &'_ Perm) -> Self::Output {
        Ok(Perm::from(rhs.permute(&self.vec)?))
    }
}
