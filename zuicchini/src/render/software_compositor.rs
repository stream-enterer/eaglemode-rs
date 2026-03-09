use crate::foundation::{Color, Image};
use crate::panel::{PanelTree, View};

use super::Painter;

pub struct SoftwareCompositor {
    framebuffer: Image,
}

impl SoftwareCompositor {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            framebuffer: Image::new(width, height, 4),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.framebuffer = Image::new(width, height, 4);
    }

    pub fn render(&mut self, tree: &mut PanelTree, view: &View) {
        self.framebuffer.fill(Color::BLACK);
        let mut painter = Painter::new(&mut self.framebuffer);
        view.paint(tree, &mut painter);
    }

    pub fn framebuffer(&self) -> &Image {
        &self.framebuffer
    }
}
