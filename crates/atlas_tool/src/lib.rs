//! Builds the glyph atlas offline from the fonts in `assets/fonts`: a
//! signed-distance-field image and a glyph table, committed and loaded by
//! the program. Not part of the program; run by hand when the fonts or the
//! glyph list change.

pub mod atlas;
pub mod distance_field;
pub mod glyph_list;
pub mod outline;
pub mod pgm;
