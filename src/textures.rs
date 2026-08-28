use raylib::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const TRANSPARENT_COLOR: Color = Color::new(152, 0, 136, 255);

const TEXTURE_FILES: [(char, &[&str]); 5] = [
    ('+', &["assets/pared2.png", "assets/pared.png"]),
    ('-', &["assets/pared.png"]),
    ('|', &["assets/pared3.png", "assets/pared.png"]),
    // La salida se dibuja como sprite, así que conviene con canal alpha.
    (
        'g',
        &["assets/salida.png", "assets/goal.png", "assets/meta.png"],
    ),
    ('e', &["assets/enemigo.png"]),
];

struct Texture {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Texture {
    fn pixel(&self, tx: u32, ty: u32) -> Color {
        let x = tx.min(self.width - 1);
        let y = ty.min(self.height - 1);
        let idx = ((y * self.width + x) * 4) as usize;
        Color::new(
            self.pixels[idx],
            self.pixels[idx + 1],
            self.pixels[idx + 2],
            self.pixels[idx + 3],
        )
    }
}
pub struct TextureManager {
    textures: Vec<Texture>,
    by_char: HashMap<char, usize>,
}

impl TextureManager {
    pub fn new() -> Self {
        let mut textures: Vec<Texture> = Vec::new();
        let mut by_path: HashMap<String, usize> = HashMap::new();
        let mut by_char = HashMap::new();

        for (ch, candidates) in TEXTURE_FILES {
            let Some(path) = candidates.iter().find(|p| asset_path(p).exists()) else {
                eprintln!(
                    "aviso: no hay textura para '{ch}' (se buscó {candidates:?}); se usa color plano"
                );
                continue;
            };

            if let Some(&idx) = by_path.get(*path) {
                by_char.insert(ch, idx);
                continue;
            }

            let full_path = asset_path(path);
            match Image::load_image(full_path.to_str().unwrap_or(path)) {
                Ok(image) => {
                    let pixels = image.get_image_data_u8(false);
                    textures.push(Texture {
                        width: image.width() as u32,
                        height: image.height() as u32,
                        pixels,
                    });
                    let idx = textures.len() - 1;
                    by_path.insert((*path).to_string(), idx);
                    by_char.insert(ch, idx);
                }
                Err(e) => {
                    eprintln!("aviso: no se pudo cargar '{path}' ({e}); se usa color plano");
                }
            }
        }

        TextureManager { textures, by_char }
    }

    fn texture(&self, ch: char) -> Option<&Texture> {
        self.textures.get(*self.by_char.get(&ch)?)
    }

    pub fn size(&self, ch: char) -> Option<(u32, u32)> {
        self.texture(ch).map(|t| (t.width, t.height))
    }

    pub fn get_pixel(&self, ch: char, tx: u32, ty: u32) -> Option<Color> {
        let color = self.texture(ch)?.pixel(tx, ty);
        if color.a < 128 || is_transparent_key(color) {
            return None;
        }
        Some(color)
    }
}

fn is_transparent_key(color: Color) -> bool {
    color.r == TRANSPARENT_COLOR.r
        && color.g == TRANSPARENT_COLOR.g
        && color.b == TRANSPARENT_COLOR.b
}

pub fn asset_path(relative: &str) -> PathBuf {
    let local = PathBuf::from(relative);
    if local.exists() {
        return local;
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}
