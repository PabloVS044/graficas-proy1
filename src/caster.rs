use raylib::color::Color;

use crate::framebuffer::Framebuffer;
use crate::maze::Maze;
use crate::player::Player;

#[derive(Clone, Copy, PartialEq)]
pub enum Side {
    Vertical,
    Horizontal,
}

pub struct Intersect {
    pub distance: f32,
    pub impact: char,
    pub tx: f32,
    pub side: Side,
}

const STEP: f32 = 1.0;

pub fn cast_ray(
    framebuffer: &mut Framebuffer,
    maze: &Maze,
    player: &Player,
    a: f32,
    block_size: usize,
    draw_line: bool,
) -> Intersect {
    let mut d = 0.0;

    framebuffer.set_current_color(Color::WHITESMOKE);

    let cos_a = a.cos();
    let sin_a = a.sin();

    let max_distance = maze
        .world_width(block_size)
        .hypot(maze.world_height(block_size));

    loop {
        let x = player.pos.x + d * cos_a;
        let y = player.pos.y + d * sin_a;

        let i = x.max(0.0) as usize / block_size;
        let j = y.max(0.0) as usize / block_size;

        if x < 0.0 || y < 0.0 || maze.is_wall(i, j) {
            return refine_hit(player, cos_a, sin_a, i, j, d, maze.cell(i, j), block_size);
        }

        if d > max_distance {
            return Intersect {
                distance: max_distance,
                impact: '+',
                tx: 0.0,
                side: Side::Vertical,
            };
        }

        if draw_line {
            framebuffer.set_pixel(x as i32, y as i32);
        }

        d += STEP;
    }
}

fn refine_hit(
    player: &Player,
    cos_a: f32,
    sin_a: f32,
    i: usize,
    j: usize,
    marched: f32,
    impact: char,
    block_size: usize,
) -> Intersect {
    let bs = block_size as f32;
    let cell_x = i as f32 * bs;
    let cell_y = j as f32 * bs;

    let t_x = if cos_a > 0.0 {
        (cell_x - player.pos.x) / cos_a
    } else if cos_a < 0.0 {
        (cell_x + bs - player.pos.x) / cos_a
    } else {
        f32::NEG_INFINITY
    };
    let t_y = if sin_a > 0.0 {
        (cell_y - player.pos.y) / sin_a
    } else if sin_a < 0.0 {
        (cell_y + bs - player.pos.y) / sin_a
    } else {
        f32::NEG_INFINITY
    };

    let side = if t_x > t_y {
        Side::Vertical
    } else {
        Side::Horizontal
    };

    let t = t_x.max(t_y).clamp(0.0, marched.max(0.0));

    let hit_x = player.pos.x + t * cos_a;
    let hit_y = player.pos.y + t * sin_a;

    let mut tx = match side {
        Side::Vertical => (hit_y - cell_y) / bs,
        Side::Horizontal => (hit_x - cell_x) / bs,
    };

    let flip = match side {
        Side::Vertical => cos_a < 0.0,
        Side::Horizontal => sin_a > 0.0,
    };
    if flip {
        tx = 1.0 - tx;
    }

    Intersect {
        distance: t,
        impact,
        tx: tx.clamp(0.0, 0.999_9),
        side,
    }
}
