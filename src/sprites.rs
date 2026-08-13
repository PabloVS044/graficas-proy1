use raylib::prelude::*;
use std::f32::consts::PI;

use crate::framebuffer::Framebuffer;
use crate::player::Player;
use crate::render::{shade, shade_factor};
use crate::textures::TextureManager;

/// A billboard standing in the maze: a flat image that always faces the player
/// and scales with distance. `kind` is the char that picks its texture.
pub struct Enemy {
    pub pos: Vector2,
    pub kind: char,
}

/// Sprites this close to the camera are skipped: the projected size explodes and
/// the player is standing inside them anyway.
const NEAR_CLIP: f32 = 4.0;
/// Extra angle beyond half the FOV that still counts as visible, so a sprite
/// entering from the side slides in instead of popping into existence.
const FOV_MARGIN: f32 = 0.4;

/// Draws every enemy over the already rendered world.
///
/// `zbuffer` holds the perpendicular distance to the wall on each column, which
/// is what decides per pixel whether the sprite is in front of the wall or
/// hidden behind it.
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

    // Far to near, so a closer sprite paints over the one behind it.
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

        // 1. Angle from the player to the sprite. atan2 keeps the quadrant that
        //    a plain atan(y/x) would throw away.
        let sprite_a = (enemy.pos.y - player.pos.y).atan2(enemy.pos.x - player.pos.x);

        // 2. Angular difference normalized to [-PI, PI], so a sprite straddling
        //    the 0/2PI wrap doesn't look like it is 359 degrees away.
        let diff = (sprite_a - player.a + PI).rem_euclid(2.0 * PI) - PI;

        // 3. Outside the field of view: nothing to draw.
        if diff.abs() > player.fov / 2.0 + FOV_MARGIN {
            continue;
        }

        // 4/5. Size and horizontal position on screen. Both come from the same
        //      projection plane the walls use, so the sprite sits exactly on the
        //      column of wall it is standing in front of.
        let sprite_size = (block_size as f32 / distance) * distance_to_projection_plane;
        let screen_x = hw + diff.tan() * distance_to_projection_plane;

        let start_x = screen_x - sprite_size / 2.0;
        let start_y = hh - sprite_size / 2.0;

        let Some((tex_width, tex_height)) = texture_manager.size(enemy.kind) else {
            continue;
        };

        // Signed clamping: a sprite entering from the left has a negative
        // start_x, and the texture coordinate still has to be measured from
        // there, not from the first visible column.
        let first_x = (start_x.floor() as i32).max(0);
        let last_x = ((start_x + sprite_size).ceil() as i32).min(framebuffer.width);
        let first_y = (start_y.floor() as i32).max(0);
        let last_y = ((start_y + sprite_size).ceil() as i32).min(framebuffer.height);

        let factor = shade_factor(distance);

        for x in first_x..last_x {
            // 6. Depth test: a wall closer than the sprite hides this column.
            if zbuffer[x as usize] < distance {
                continue;
            }

            let tx = ((x as f32 - start_x) / sprite_size * tex_width as f32) as u32;

            for y in first_y..last_y {
                let ty = ((y as f32 - start_y) / sprite_size * tex_height as f32) as u32;

                // `None` is a transparent pixel (alpha or magenta key): the wall
                // behind the sprite stays visible there.
                if let Some(color) = texture_manager.get_pixel(enemy.kind, tx, ty) {
                    framebuffer.set_pixel_color(x, y, shade(color, factor));
                }
            }
        }
    }
}
