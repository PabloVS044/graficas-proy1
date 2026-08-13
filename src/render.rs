use raylib::prelude::*;

use crate::caster::cast_ray;
use crate::framebuffer::Framebuffer;
use crate::maze::{GOAL, Maze, SPAWN};
use crate::player::Player;

/// Rays drawn in the minimap. Enough to see the shape of the view cone without
/// hiding the map under a solid white fan.
const NUM_RAYS_MINIMAP: usize = 24;

/// Width of the minimap as a fraction of the screen. The maze keeps its aspect
/// ratio, so the height follows from this.
const MINIMAP_WIDTH_RATIO: f32 = 0.32;
/// Gap between the minimap and the edge of the screen, in pixels.
const MINIMAP_MARGIN: i32 = 12;
/// Thickness of the frame drawn around the minimap.
const MINIMAP_BORDER: i32 = 2;

/// Walls stop getting darker past this distance, in pixels.
const MAX_SHADE_DISTANCE: f32 = 500.0;
const MIN_SHADE: f32 = 0.30;

fn cell_color(cell: char) -> Color {
    match cell {
        '+' => Color::new(0x3A, 0x3F, 0x6B, 255), // corners
        '-' => Color::new(0x4A, 0x52, 0x8A, 255), // horizontal walls
        '|' => Color::new(0x33, 0x38, 0x5E, 255), // vertical walls
        GOAL => Color::new(0x5E, 0xD9, 0x8A, 255),
        SPAWN => Color::new(0x2A, 0x2D, 0x3E, 255),
        _ => Color::new(0x2A, 0x2D, 0x3E, 255), // floor
    }
}

fn shade(color: Color, factor: f32) -> Color {
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

/// Top-down view of the whole maze, the player and the rays it is casting,
/// shrunk into the top-right corner on top of the 3D view.
///
/// It is the same top-down render as before; the only difference is the
/// framebuffer transform, which scales world coordinates down into the corner.
/// `cast_ray` still works in world coordinates and doesn't know about any of this.
pub fn render_minimap(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    player: &Player,
    block_size: usize,
) {
    let world_width = maze.world_width(block_size);
    let world_height = maze.world_height(block_size);

    let scale = (framebuffer.width as f32 * MINIMAP_WIDTH_RATIO) / world_width;
    let map_width = (world_width * scale) as i32;
    let map_height = (world_height * scale) as i32;

    let origin_x = framebuffer.width - map_width - MINIMAP_MARGIN;
    let origin_y = MINIMAP_MARGIN;

    // Frame around the minimap, drawn untransformed in screen coordinates.
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

    // draw what the player sees
    for ray in 0..NUM_RAYS_MINIMAP {
        let current_ray = ray as f32 / NUM_RAYS_MINIMAP as f32; // current ray divided by total rays
        let a = player.a - (player.fov / 2.0) + (player.fov * current_ray);
        cast_ray(framebuffer, maze, player, a, block_size, true);
    }

    framebuffer.set_current_color(Color::new(0xFF, 0x6B, 0x6B, 255));
    framebuffer.circle(player.pos.x as i32, player.pos.y as i32, 2);

    framebuffer.reset_transform();
}

/// First-person view: one ray per screen column, each one drawn as a vertical
/// "stake" whose height is inversely proportional to the distance to the wall.
pub fn render_world(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    player: &Player,
    block_size: usize,
) {
    let num_rays = framebuffer.width as usize;
    let hw = framebuffer.width as f32 / 2.0; // precalculated half width
    let hh = framebuffer.height as f32 / 2.0; // precalculated half height

    // Distance from the optical center to the projection plane: the value that
    // makes a wall exactly fill the view when it spans the whole field of view.
    let distance_to_projection_plane = hw / (player.fov / 2.0).tan();

    // Ceiling and floor first; the stakes are drawn over them.
    framebuffer.set_current_color(Color::new(0x1B, 0x1E, 0x2B, 255));
    framebuffer.rect(0, 0, framebuffer.width, hh as i32);
    framebuffer.set_current_color(Color::new(0x2E, 0x24, 0x24, 255));
    framebuffer.rect(0, hh as i32, framebuffer.width, framebuffer.height - hh as i32);

    for i in 0..num_rays {
        let current_ray = i as f32 / num_rays as f32; // current ray divided by total rays
        let a = player.a - (player.fov / 2.0) + (player.fov * current_ray);
        let intersect = cast_ray(framebuffer, maze, player, a, block_size, false);

        // Perpendicular distance instead of the raw ray length: without this
        // correction the walls bulge outwards at the edges of the screen (fisheye).
        let distance_to_wall = (intersect.distance * (a - player.a).cos()).max(1.0);

        // Our equation for the stake height: a wall is one block tall, so its
        // projected height is (block_size / distance) * distance_to_projection_plane.
        let stake_height = (block_size as f32 / distance_to_wall) * distance_to_projection_plane;

        // Calculate the position to draw the stake
        let stake_top = hh - (stake_height / 2.0);
        let stake_bottom = hh + (stake_height / 2.0);

        let factor = (1.0 - distance_to_wall / MAX_SHADE_DISTANCE).clamp(MIN_SHADE, 1.0);
        framebuffer.set_current_color(shade(cell_color(intersect.impact), factor));
        framebuffer.vertical_line(i as i32, stake_top as i32, stake_bottom as i32);
    }
}
