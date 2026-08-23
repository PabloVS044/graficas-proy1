use raylib::prelude::*;

use crate::caster::{Side, cast_ray};
use crate::framebuffer::Framebuffer;
use crate::maze::{ENEMY, GOAL, Maze, SPAWN};
use crate::player::Player;
use crate::sprites::Enemy;
use crate::textures::TextureManager;

const NUM_RAYS_MINIMAP: usize = 24;

const MINIMAP_WIDTH_RATIO: f32 = 0.32;
const MINIMAP_MARGIN: i32 = 12;
const MINIMAP_BORDER: i32 = 2;

const CEILING_COLOR: Color = Color::new(0x3E, 0x9E, 0xAA, 255);
const FLOOR_COLOR: Color = Color::new(0xBF, 0xB6, 0x93, 255);

const MAX_SHADE_DISTANCE: f32 = 500.0;
const MIN_SHADE: f32 = 0.30;

pub fn shade_factor(distance: f32) -> f32 {
    (1.0 - distance / MAX_SHADE_DISTANCE).clamp(MIN_SHADE, 1.0)
}

fn cell_color(cell: char) -> Color {
    match cell {
        '+' => Color::new(0x3A, 0x3F, 0x6B, 255),
        '-' => Color::new(0x4A, 0x52, 0x8A, 255),
        '|' => Color::new(0x33, 0x38, 0x5E, 255),
        GOAL => Color::new(0x5E, 0xD9, 0x8A, 255),
        SPAWN | ENEMY => Color::new(0x2A, 0x2D, 0x3E, 255),
        _ => Color::new(0x2A, 0x2D, 0x3E, 255),
    }
}

pub fn shade(color: Color, factor: f32) -> Color {
    Color::new(
        (color.r as f32 * factor) as u8,
        (color.g as f32 * factor) as u8,
        (color.b as f32 * factor) as u8,
        255,
    )
}

fn draw_cell(framebuffer: &mut Framebuffer, xo: usize, yo: usize, block_size: usize, cell: char) {
    framebuffer.set_current_color(cell_color(cell));
    framebuffer.rect(xo as i32, yo as i32, block_size as i32, block_size as i32);
}

pub fn render_minimap(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    player: &Player,
    enemies: &[Enemy],
    block_size: usize,
) {
    let world_width = maze.world_width(block_size);
    let world_height = maze.world_height(block_size);

    let scale = (framebuffer.width as f32 * MINIMAP_WIDTH_RATIO) / world_width;
    let map_width = (world_width * scale) as i32;
    let map_height = (world_height * scale) as i32;

    let origin_x = framebuffer.width - map_width - MINIMAP_MARGIN;
    let origin_y = MINIMAP_MARGIN;

    framebuffer.reset_transform();
    framebuffer.set_current_color(Color::new(0x0D, 0x0F, 0x18, 255));
    framebuffer.rect(
        origin_x - MINIMAP_BORDER,
        origin_y - MINIMAP_BORDER,
        map_width + MINIMAP_BORDER * 2,
        map_height + MINIMAP_BORDER * 2,
    );

    framebuffer.set_transform(scale, origin_x, origin_y);

    for j in 0..maze.height {
        for i in 0..maze.width {
            let xo = i * block_size;
            let yo = j * block_size;
            draw_cell(framebuffer, xo, yo, block_size, maze.cell(i, j));
        }
    }

    for ray in 0..NUM_RAYS_MINIMAP {
        let current_ray = ray as f32 / NUM_RAYS_MINIMAP as f32;
        let a = player.a - (player.fov / 2.0) + (player.fov * current_ray);
        cast_ray(framebuffer, maze, player, a, block_size, true);
    }

    framebuffer.set_current_color(Color::new(0xE8, 0xC0, 0x50, 255));
    for enemy in enemies {
        framebuffer.circle(enemy.pos.x as i32, enemy.pos.y as i32, 2);
    }

    framebuffer.set_current_color(Color::new(0xFF, 0x6B, 0x6B, 255));
    framebuffer.circle(player.pos.x as i32, player.pos.y as i32, 2);

    framebuffer.reset_transform();
}

pub fn render_world(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    player: &Player,
    texture_manager: &TextureManager,
    block_size: usize,
) -> Vec<f32> {
    let num_rays = framebuffer.width as usize;
    let mut zbuffer = vec![f32::INFINITY; num_rays];
    let hw = framebuffer.width as f32 / 2.0;
    let hh = framebuffer.height as f32 / 2.0;

    let distance_to_projection_plane = hw / (player.fov / 2.0).tan();

    framebuffer.set_current_color(CEILING_COLOR);
    framebuffer.rect(0, 0, framebuffer.width, hh as i32);
    framebuffer.set_current_color(FLOOR_COLOR);
    framebuffer.rect(
        0,
        hh as i32,
        framebuffer.width,
        framebuffer.height - hh as i32,
    );

    for i in 0..num_rays {
        let current_ray = i as f32 / num_rays as f32;
        let a = player.a - (player.fov / 2.0) + (player.fov * current_ray);
        let intersect = cast_ray(framebuffer, maze, player, a, block_size, false);

        let distance_to_wall = (intersect.distance * (a - player.a).cos()).max(1.0);

        let stake_height = (block_size as f32 / distance_to_wall) * distance_to_projection_plane;

        // Calculate the position to draw the stake
        let stake_top = hh - (stake_height / 2.0);
        let stake_bottom = hh + (stake_height / 2.0);

        zbuffer[i] = distance_to_wall;

        let factor = shade_factor(distance_to_wall)
            * match intersect.side {
                Side::Horizontal => 0.75,
                Side::Vertical => 1.0,
            };

        match texture_manager.size(intersect.impact) {
            Some((tex_width, tex_height)) => {
                let tx = (intersect.tx * tex_width as f32) as u32;

                let first = stake_top.max(0.0) as i32;
                let last = (stake_bottom.min(framebuffer.height as f32 - 1.0)) as i32;

                for y in first..=last {
                    let ty = ((y as f32 - stake_top) / (stake_bottom - stake_top)
                        * tex_height as f32) as u32;
                    if let Some(color) = texture_manager.get_pixel(intersect.impact, tx, ty) {
                        framebuffer.set_pixel_color(i as i32, y, shade(color, factor));
                    }
                }
            }
            None => {
                framebuffer.set_current_color(shade(cell_color(intersect.impact), factor));
                framebuffer.vertical_line(i as i32, stake_top as i32, stake_bottom as i32);
            }
        }
    }

    zbuffer
}
