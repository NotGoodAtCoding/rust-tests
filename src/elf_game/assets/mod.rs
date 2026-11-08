#[path = "../../../assets/palette.rs"]
pub mod palette;

#[path = "../../../assets/objects.rs"]
pub mod objects;

pub mod materials;

pub use palette::*;
pub use objects::*;
pub use materials::{WorkshopMaterials, AssetsPlugin};
