//! Shared text support built on the Parley text stack.

mod catalog;
mod store;
mod text_system;

pub use catalog::SystemFonts;
pub use fontique::Blob as FontDataBlob;
pub use store::{
    BitmapFallbackGlyphRasterizer, ColorGlyphKind, FontSynthesis, FontVariation, GlyphRasterizer,
    RasterFace, SwashGlyphRasterizer,
};
pub use text_system::ParleyTextSystem;

pub(crate) use store::FontStore;
