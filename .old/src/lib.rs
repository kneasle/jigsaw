// It's fine to allow links to private items, because this crate isn't meant for public consumption
// - it's only ever going to be imported by the JS part of this project.
#![allow(private_intra_doc_links)]

use vector2d::Vector2D;

pub mod derived_state;
pub mod jigsaw;
pub mod ser_utils;
pub mod spec;
pub mod spec2;
pub mod view;

/// Type alias for a 2D vector of [`f32`]s.
pub type V2 = Vector2D<f32>;
