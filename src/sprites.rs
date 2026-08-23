use raylib::prelude::*;
use std::f32::consts::PI;

use crate::framebuffer::Framebuffer;
use crate::player::Player;
use crate::render::{shade, shade_factor};
use crate::textures::TextureManager;

pub struct Enemy {
    pub pos: Vector2,
    pub kind: char,
}

const NEAR_CLIP: f32 = 4.0;
const FOV_MARGIN: f32 = 0.4;

const SPRITE_SCALE: f32 = 0.55;

pub fn render_enemies(
    framebuffer: &mut Framebuffer,
    player: &Player,
    enemies: &[Enemy],
    texture_manager: &TextureManager,
    zbuffer: &[f32],
    block_size: usize,
) {
    let hw = framebuffer.width as f32 / 2.0;
    let hh = framebuffer.height as f32 / 2.0;
    let distance_to_projection_plane = hw / (player.fov / 2.0).tan();

    let mut order: Vec<(usize, f32)> = enemies
        .iter()
        .enumerate()
        .map(|(idx, e)| (idx, (e.pos.x - player.pos.x).hypot(e.pos.y - player.pos.y)))
        .collect();
    order.sort_by(|a, b| b.1.total_cmp(&a.1));

    for (idx, distance) in order {
        let enemy = &enemies[idx];

        if distance < NEAR_CLIP {
            continue;
        }

        let sprite_a = (enemy.pos.y - player.pos.y).atan2(enemy.pos.x - player.pos.x);

        let diff = (sprite_a - player.a + PI).rem_euclid(2.0 * PI) - PI;

        if diff.abs() > player.fov / 2.0 + FOV_MARGIN {
            continue;
        }

        let block_height = (block_size as f32 / distance) * distance_to_projection_plane;
        let sprite_size = block_height * SPRITE_SCALE;
        let screen_x = hw + diff.tan() * distance_to_projection_plane;

        let start_x = screen_x - sprite_size / 2.0;
        let start_y = hh - sprite_size / 2.0;

        let Some((tex_width, tex_height)) = texture_manager.size(enemy.kind) else {
            continue;
        };

        let first_x = (start_x.floor() as i32).max(0);
        let last_x = ((start_x + sprite_size).ceil() as i32).min(framebuffer.width);
        let first_y = (start_y.floor() as i32).max(0);
        let last_y = ((start_y + sprite_size).ceil() as i32).min(framebuffer.height);

        let factor = shade_factor(distance);

        for x in first_x..last_x {
            if zbuffer[x as usize] < distance {
                continue;
            }

            let tx = ((x as f32 - start_x) / sprite_size * tex_width as f32) as u32;

            for y in first_y..last_y {
                let ty = ((y as f32 - start_y) / sprite_size * tex_height as f32) as u32;


                if let Some(color) = texture_manager.get_pixel(enemy.kind, tx, ty) {
                    framebuffer.set_pixel_color(x, y, shade(color, factor));
                }
            }
        }
    }
}
