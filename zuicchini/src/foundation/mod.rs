mod color;
mod image;
mod tga;

pub use color::Color;
pub use image::Image;
pub use tga::{load_tga, TgaError};
