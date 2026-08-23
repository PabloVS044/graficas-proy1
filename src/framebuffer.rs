use raylib::prelude::*;

pub struct Framebuffer {
    pub width: i32,
    pub height: i32,
    pixels: Vec<u8>,
    color_buffer: Image,
    current_color: Color,
    background_color: Color,
    scale: f32,
    offset_x: i32,
    offset_y: i32,
}

impl Framebuffer {
    pub fn new(width: i32, height: i32, background_color: Color) -> Self {
        let color_buffer = Image::gen_image_color(width, height, background_color);
        let pixels = vec![0u8; (width * height * 4) as usize];
        let mut framebuffer = Framebuffer {
            width,
            height,
            pixels,
            color_buffer,
            current_color: Color::WHITE,
            background_color,
            scale: 1.0,
            offset_x: 0,
            offset_y: 0,
        };
        framebuffer.clear();
        framebuffer
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
        let bg = [
            self.background_color.r,
            self.background_color.g,
            self.background_color.b,
            self.background_color.a,
        ];
        for pixel in self.pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&bg);
        }
    }

    pub fn set_current_color(&mut self, color: Color) {
        self.current_color = color;
    }

    pub fn set_pixel(&mut self, x: i32, y: i32) {
        let (x, y) = (self.map_x(x), self.map_y(y));
        self.set_pixel_color(x, y, self.current_color);
    }

    pub fn set_pixel_color(&mut self, x: i32, y: i32, color: Color) {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            return;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        self.pixels[idx] = color.r;
        self.pixels[idx + 1] = color.g;
        self.pixels[idx + 2] = color.b;
        self.pixels[idx + 3] = color.a;
    }

    pub fn rect(&mut self, x: i32, y: i32, width: i32, height: i32) {
        let x0 = self.map_x(x);
        let y0 = self.map_y(y);
        let x1 = x0 + (self.map_x(x + width) - x0).max(1);
        let y1 = y0 + (self.map_y(y + height) - y0).max(1);
        self.fill_rect(x0, y0, x1, y1, self.current_color);
    }

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
        self.fill_rect(x, top, x + 1, bottom + 1, self.current_color);
    }

    pub fn circle(&mut self, center_x: i32, center_y: i32, radius: i32) {
        let cx = self.map_x(center_x);
        let cy = self.map_y(center_y);
        let color = self.current_color;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy <= radius * radius {
                    self.set_pixel_color(cx + dx, cy + dy, color);
                }
            }
        }
    }

    fn fill_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let x0 = x0.max(0);
        let y0 = y0.max(0);
        let x1 = x1.min(self.width);
        let y1 = y1.min(self.height);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let rgba = [color.r, color.g, color.b, color.a];
        for y in y0..y1 {
            let row = (y * self.width) as usize * 4;
            for x in x0..x1 {
                let idx = row + x as usize * 4;
                self.pixels[idx..idx + 4].copy_from_slice(&rgba);
            }
        }
    }

    pub fn swap_buffers<F>(&mut self, window: &mut RaylibHandle, thread: &RaylibThread, hud: F)
    where
        F: FnOnce(&mut RaylibDrawHandle),
    {
        self.upload();
        if let Ok(texture) = window.load_texture_from_image(thread, &self.color_buffer) {
            let mut d = window.begin_drawing(thread);
            d.clear_background(Color::BLACK);
            d.draw_texture(&texture, 0, 0, Color::WHITE);
            hud(&mut d);
        }
    }

    fn upload(&mut self) {
        debug_assert_eq!(
            self.color_buffer.format(),
            PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8A8
        );
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.pixels.as_ptr(),
                self.color_buffer.data() as *mut u8,
                self.pixels.len(),
            );
        }
    }

    #[allow(dead_code)]
    pub fn render_to_file(&mut self, file_path: &str) {
        self.upload();
        self.color_buffer.export_image(file_path);
    }
}
