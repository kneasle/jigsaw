//! Code for expanding a [`CompSpec`] into a [`FullComp`] that represents the same data.

use super::FullComp;
use crate::spec::CompSpec;

/// Convert a [`CompSpec`] to a [`FullComp`] which represents the same composition.  [`FullComp`]
/// explicitly specifies all the information that is implied by a [`CompSpec`], so this function
/// essentially computes that extra information.
pub fn expand(spec: &CompSpec) -> FullComp {
    todo!()
}
