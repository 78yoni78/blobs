use raylib::prelude::*;

pub struct Window {
    handle: RaylibHandle,
    thread: RaylibThread,
}

pub type DrawingContext<'a> = RaylibDrawHandle<'a>;

pub use raylib::prelude::MouseButton;
pub use raylib::prelude::KeyboardKey;

pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub title: &'static str,
}

impl Window {
    pub fn new(WindowConfig { width, height, title }: &WindowConfig) -> Self {
        let (handle, thread) = raylib::init()
            .title(title)
            .size(*width as i32, *height as i32)
            .build();
        Self { handle, thread }
    }

    pub fn width(&self) -> u32 {
        self.handle.get_screen_width() as u32
    }

    pub fn height(&self) -> u32 {
        self.handle.get_screen_height() as u32
    }

    pub fn draw_loop<F>(&mut self, mut draw: F)
    where F: FnMut(DrawingContext) {
        while !self.handle.window_should_close() {
            draw(self.handle.begin_drawing(&self.thread));
        }
    }

    pub fn handle(&self) -> &RaylibHandle { &self.handle }
}

pub mod prelude {
    pub use super::{Window, DrawingContext, WindowConfig};
}