use raylib::prelude::*;

/// A CPU-side pixel buffer. Everything the raycaster draws goes here, and the
/// whole buffer is uploaded to the GPU once per frame in `swap_buffers`.
///
/// Origin is top-left (x to the right, y down), matching the maze layout: row 0
/// of `maze.txt` is the top row on screen.
pub struct Framebuffer {
    pub width: i32,
    pub height: i32,
    color_buffer: Image,
    current_color: Color,
    background_color: Color,
    /// Applied to every draw call as `pixel = world * scale + offset`. This is
    /// what lets the minimap shrink the whole maze into a corner while the
    /// renderer (and `cast_ray`) keep working in plain world coordinates.
    scale: f32,
    offset_x: i32,
    offset_y: i32,
}

impl Framebuffer {
    pub fn new(width: i32, height: i32, background_color: Color) -> Self {
        let color_buffer = Image::gen_image_color(width, height, background_color);
        Framebuffer {
            width,
            height,
            color_buffer,
            current_color: Color::WHITE,
            background_color,
            scale: 1.0,
            offset_x: 0,
            offset_y: 0,
        }
    }

    pub fn set_transform(&mut self, scale: f32, offset_x: i32, offset_y: i32) {
        self.scale = scale;
        self.offset_x = offset_x;
        self.offset_y = offset_y;
    }

    pub fn reset_transform(&mut self) {
        self.set_transform(1.0, 0, 0);
    }

    fn map_x(&self, x: i32) -> i32 {
        (x as f32 * self.scale) as i32 + self.offset_x
    }

    fn map_y(&self, y: i32) -> i32 {
        (y as f32 * self.scale) as i32 + self.offset_y
    }

    pub fn clear(&mut self) {
        self.color_buffer.clear_background(self.background_color);
    }

    pub fn set_current_color(&mut self, color: Color) {
        self.current_color = color;
    }

    pub fn set_pixel(&mut self, x: i32, y: i32) {
        let (x, y) = (self.map_x(x), self.map_y(y));
        if x >= 0 && x < self.width && y >= 0 && y < self.height {
            self.color_buffer.draw_pixel(x, y, self.current_color);
        }
    }

    /// Filled rectangle in the current color. Used for maze cells in the 2D view
    /// and for the sky/floor bands in the 3D view.
    /// Maps both corners instead of scaling the size, so scaled-down cells tile
    /// exactly: no seams between them and no overlap.
    pub fn rect(&mut self, x: i32, y: i32, width: i32, height: i32) {
        let x0 = self.map_x(x);
        let y0 = self.map_y(y);
        let x1 = self.map_x(x + width);
        let y1 = self.map_y(y + height);
        self.color_buffer.draw_rectangle(
            x0,
            y0,
            (x1 - x0).max(1),
            (y1 - y0).max(1),
            self.current_color,
        );
    }

    /// One vertical span of pixels in the current color: this is a "stake", the
    /// single column of wall the 3D renderer draws per ray. Clamped to the
    /// buffer, so stakes taller than the screen are simply cut off.
    pub fn vertical_line(&mut self, x: i32, y_start: i32, y_end: i32) {
        let x = self.map_x(x);
        if x < 0 || x >= self.width {
            return;
        }
        let top = self.map_y(y_start).max(0);
        let bottom = self.map_y(y_end).min(self.height - 1);
        if top > bottom {
            return;
        }
        self.color_buffer
            .draw_rectangle(x, top, 1, bottom - top + 1, self.current_color);
    }

    /// The position is transformed but the radius is not: a marker has to stay
    /// visible even when the minimap shrinks the world around it.
    pub fn circle(&mut self, center_x: i32, center_y: i32, radius: i32) {
        self.color_buffer.draw_circle(
            self.map_x(center_x),
            self.map_y(center_y),
            radius,
            self.current_color,
        );
    }

    /// Upload the buffer and present it. `hud` runs inside the same drawing pass,
    /// so text drawn there lands on top of the framebuffer instead of clearing it.
    pub fn swap_buffers<F>(&self, window: &mut RaylibHandle, thread: &RaylibThread, hud: F)
    where
        F: FnOnce(&mut RaylibDrawHandle),
    {
        if let Ok(texture) = window.load_texture_from_image(thread, &self.color_buffer) {
            let mut d = window.begin_drawing(thread);
            d.clear_background(Color::BLACK);
            d.draw_texture(&texture, 0, 0, Color::WHITE);
            hud(&mut d);
        }
    }

    /// Exports the raw buffer, without the HUD drawn over it.
    #[allow(dead_code)]
    pub fn render_to_file(&self, file_path: &str) {
        self.color_buffer.export_image(file_path);
    }
}
