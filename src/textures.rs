use raylib::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Color used as a transparency key for sprites drawn without an alpha channel.
pub const TRANSPARENT_COLOR: Color = Color::new(152, 0, 136, 255);

/// Which image file each maze character is painted with, in order of preference:
/// the first candidate that exists wins. The three wall chars are only different
/// cell types in the file — they are all the same block on screen — so a single
/// `assets/wall.png` textures all of them, and dropping a specific file next to
/// it overrides just that one.
///
/// A character with no candidate on disk falls back to the flat colors of
/// `render::cell_color`, so the game still runs with an empty `assets/`.
const TEXTURE_FILES: [(char, &[&str]); 5] = [
    // Las esquinas llevan la textura secundaria: son los postes entre tramos de
    // muro, y darles otra imagen es lo que hace que se lea dónde termina una
    // pared y empieza la otra.
    (
        '+',
        &[
            "assets/wall_corner.png",
            "assets/pared.png",
            "assets/wall.png",
        ],
    ),
    // Los muros largos, con la textura principal.
    (
        '-',
        &["assets/wall_h.png", "assets/enemigo.png", "assets/wall.png"],
    ),
    (
        '|',
        &["assets/wall_v.png", "assets/enemigo.png", "assets/wall.png"],
    ),
    ('g', &["assets/goal.png", "assets/meta.png"]),
    // El enemigo se dibuja con su fondo, como un cuadro: es una decisión, no un
    // olvido. `enemy.png` queda como override por si algún día se quiere una
    // versión recortada con alpha.
    ('e', &["assets/enemy.png", "assets/pared2.png"]),
];

/// One texture already decoded to RGBA in a plain `Vec`, so sampling a pixel is
/// an array index instead of an FFI call into raylib.
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

/// Keeps every texture in memory and hands out individual pixels, which is the
/// only thing a raycaster needs from an image.
///
/// Chars point at an index instead of owning a `Texture`, so several chars
/// sharing one file (the three wall chars on a single `wall.png`) load and store
/// that image once, and the per-pixel lookup stays a single hash on a `char`.
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
                    // Converting once at load time (instead of reading the raw
                    // image data on every sample) keeps the per-pixel path free
                    // of unsafe code and independent of the file's pixel format.
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

    /// Size of the texture bound to `ch`, needed to scale `tx`/`ty` from the
    /// [0,1) range into pixels.
    pub fn size(&self, ch: char) -> Option<(u32, u32)> {
        self.texture(ch).map(|t| (t.width, t.height))
    }

    /// The pixel at `(tx, ty)`, or `None` when there is no texture for `ch` or
    /// the pixel is transparent (alpha or magenta key). `None` means "don't
    /// paint": walls fall back to a flat color, sprites let the wall show through.
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

/// Look for a file next to the working directory first, then fall back to the
/// project root so `cargo run` works from anywhere.
pub fn asset_path(relative: &str) -> PathBuf {
    let local = PathBuf::from(relative);
    if local.exists() {
        return local;
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}
