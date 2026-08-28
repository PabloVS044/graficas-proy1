use raylib::prelude::*;
use std::f32::consts::PI;

use crate::framebuffer::Framebuffer;
use crate::player::Player;
use crate::render::{shade, shade_factor};
use crate::textures::TextureManager;

/// Impactos que aguanta un enemigo antes de morir.
pub const ENEMY_HEALTH: i32 = 3;
/// Cuánto dura el parpadeo blanco al recibir un disparo, en segundos.
pub const HIT_FLASH_TIME: f32 = 0.12;

pub struct Enemy {
    pub pos: Vector2,
    pub kind: char,
    pub health: i32,
    pub hit_flash: f32,
}

impl Enemy {
    pub fn new(pos: Vector2, kind: char) -> Self {
        Enemy {
            pos,
            kind,
            health: ENEMY_HEALTH,
            hit_flash: 0.0,
        }
    }

    pub fn alive(&self) -> bool {
        self.health > 0
    }

    /// Le pega un tiro. Devuelve `true` si con este murió.
    pub fn hit(&mut self) -> bool {
        if !self.alive() {
            return false;
        }
        self.health -= 1;
        self.hit_flash = HIT_FLASH_TIME;
        !self.alive()
    }
}

const NEAR_CLIP: f32 = 4.0;
const FOV_MARGIN: f32 = 0.4;

const SPRITE_SCALE: f32 = 0.85;

/// Dónde y de qué tamaño cae un sprite en la pantalla.
///
/// La usan el dibujo *y* la puntería del disparo: compartir el cálculo es lo que
/// hace que apuntar al centro y pegarle sea literalmente lo mismo que se ve.
pub struct Projection {
    pub start_x: f32,
    pub start_y: f32,
    pub width: f32,
    pub height: f32,
    pub distance: f32,
}

impl Projection {
    /// Si la columna `x` de la pantalla cae dentro del sprite.
    pub fn covers_column(&self, x: f32) -> bool {
        x >= self.start_x && x <= self.start_x + self.width
    }
}

/// Proyecta un punto del mundo a la pantalla. `aspect` es ancho/alto de la
/// imagen: el sprite no se fuerza a cuadrado porque deformaría al personaje.
///
/// Devuelve `None` si el sprite queda fuera del campo de visión o demasiado
/// cerca de la cámara.
pub fn project(
    player: &Player,
    pos: Vector2,
    screen_width: f32,
    screen_height: f32,
    block_size: usize,
    aspect: f32,
) -> Option<Projection> {
    let distance = (pos.x - player.pos.x).hypot(pos.y - player.pos.y);
    if distance < NEAR_CLIP {
        return None;
    }

    let sprite_a = (pos.y - player.pos.y).atan2(pos.x - player.pos.x);
    let diff = (sprite_a - player.a + PI).rem_euclid(2.0 * PI) - PI;
    if diff.abs() > player.fov / 2.0 + FOV_MARGIN {
        return None;
    }

    let hw = screen_width / 2.0;
    let hh = screen_height / 2.0;
    let distance_to_projection_plane = hw / (player.fov / 2.0).tan();

    let block_height = (block_size as f32 / distance) * distance_to_projection_plane;
    let height = block_height * SPRITE_SCALE;
    let width = height * aspect;

    let screen_x = hw + diff.tan() * distance_to_projection_plane;

    Some(Projection {
        start_x: screen_x - width / 2.0,
        // Apoyado en el piso: la base del bloque cae en `hh + block_height / 2`.
        start_y: hh + block_height / 2.0 - height,
        width,
        height,
        distance,
    })
}

/// Algo que se dibuja como billboard: un enemigo o la salida.
pub struct DrawItem {
    pub pos: Vector2,
    pub kind: char,
    /// Color con el que se tiñe. `None` deja la textura tal cual.
    pub tint: Option<Color>,
}

/// Dibuja los sprites sobre el mundo ya renderizado, de lejos a cerca.
pub fn render_sprites(
    framebuffer: &mut Framebuffer,
    player: &Player,
    items: &[DrawItem],
    texture_manager: &TextureManager,
    zbuffer: &[f32],
    block_size: usize,
) {
    let screen_w = framebuffer.width as f32;
    let screen_h = framebuffer.height as f32;

    let mut order: Vec<(usize, f32)> = items
        .iter()
        .enumerate()
        .map(|(idx, it)| {
            (
                idx,
                (it.pos.x - player.pos.x).hypot(it.pos.y - player.pos.y),
            )
        })
        .collect();
    order.sort_by(|a, b| b.1.total_cmp(&a.1));

    for (idx, _) in order {
        let item = &items[idx];
        let size = texture_manager.size(item.kind);
        let aspect = size.map_or(1.0, |(w, h)| w as f32 / h as f32);

        let Some(p) = project(player, item.pos, screen_w, screen_h, block_size, aspect) else {
            continue;
        };

        let first_x = (p.start_x.floor() as i32).max(0);
        let last_x = ((p.start_x + p.width).ceil() as i32).min(framebuffer.width);
        let first_y = (p.start_y.floor() as i32).max(0);
        let last_y = ((p.start_y + p.height).ceil() as i32).min(framebuffer.height);

        let factor = shade_factor(p.distance);

        for x in first_x..last_x {
            if zbuffer[x as usize] < p.distance {
                continue;
            }

            for y in first_y..last_y {
                let color = match size {
                    Some((tex_w, tex_h)) => {
                        let tx = ((x as f32 - p.start_x) / p.width * tex_w as f32) as u32;
                        let ty = ((y as f32 - p.start_y) / p.height * tex_h as f32) as u32;
                        texture_manager.get_pixel(item.kind, tx, ty)
                    }
                    // Sin textura: un rombo liso, para que el nivel siga siendo
                    // jugable aunque falte el archivo.
                    None => diamond_pixel(&p, x as f32, y as f32),
                };

                if let Some(color) = color {
                    let color = match item.tint {
                        Some(t) => mix(color, t),
                        None => color,
                    };
                    framebuffer.set_pixel_color(x, y, shade(color, factor));
                }
            }
        }
    }
}

/// Silueta de rombo, el sustituto cuando un sprite no tiene textura.
fn diamond_pixel(p: &Projection, x: f32, y: f32) -> Option<Color> {
    let u = (x - p.start_x) / p.width - 0.5;
    let v = (y - p.start_y) / p.height - 0.5;
    (u.abs() + v.abs() <= 0.5).then_some(Color::new(0xF2, 0xE6, 0x6B, 255))
}

/// Mezcla a mitad de camino, que es como se aplica el parpadeo del impacto y el
/// tinte rojo de la salida bloqueada.
fn mix(color: Color, tint: Color) -> Color {
    Color::new(
        ((color.r as u16 + tint.r as u16) / 2) as u8,
        ((color.g as u16 + tint.g as u16) / 2) as u8,
        ((color.b as u16 + tint.b as u16) / 2) as u8,
        255,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tres_impactos_matan() {
        let mut e = Enemy::new(Vector2::new(0.0, 0.0), 'e');
        assert!(e.alive());
        assert!(!e.hit(), "el primer tiro no debería matar");
        assert!(!e.hit());
        assert!(e.hit(), "el tercero sí");
        assert!(!e.alive());
    }

    #[test]
    fn un_muerto_no_revive_ni_vuelve_a_morir() {
        let mut e = Enemy::new(Vector2::new(0.0, 0.0), 'e');
        for _ in 0..ENEMY_HEALTH {
            e.hit();
        }
        assert!(!e.hit(), "no puede volver a morir");
        assert!(!e.alive());
        assert_eq!(e.health, 0, "la vida no baja de cero");
    }

    #[test]
    fn el_impacto_prende_el_parpadeo() {
        let mut e = Enemy::new(Vector2::new(0.0, 0.0), 'e');
        assert_eq!(e.hit_flash, 0.0);
        e.hit();
        assert_eq!(e.hit_flash, HIT_FLASH_TIME);
    }
}
