use raylib::color::Color;

use crate::framebuffer::Framebuffer;
use crate::maze::Maze;
use crate::player::Player;

/// Which face of the block the ray came in through: a face on a vertical plane
/// (constant x) or on a horizontal one (constant y). It decides which coordinate
/// of the hit point runs along the wall, and therefore which one gives `tx`.
#[derive(Clone, Copy, PartialEq)]
pub enum Side {
    Vertical,
    Horizontal,
}

/// What a ray found: how far it travelled, which wall char it hit, where along
/// the face of that wall it landed (`tx`, in [0,1)) and through which face.
pub struct Intersect {
    pub distance: f32,
    pub impact: char,
    pub tx: f32,
    pub side: Side,
}

/// How much the ray advances per iteration, in pixels. Smaller is more precise
/// and slower; 1 pixel is exact enough that walls never look ragged.
const STEP: f32 = 1.0;

/// March a ray from the player in direction `a` until it hits a wall.
///
/// `draw_line` paints the ray into the framebuffer as it advances; only the 2D
/// view wants that, the 3D view just needs the distance back.
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

    // cos/sin are constant along the ray, so compute them once instead of per step.
    let cos_a = a.cos();
    let sin_a = a.sin();

    let max_distance = maze.world_width(block_size).hypot(maze.world_height(block_size));

    loop {
        let x = player.pos.x + d * cos_a;
        let y = player.pos.y + d * sin_a;

        // convert pixels to a position in the maze
        let i = x.max(0.0) as usize / block_size;
        let j = y.max(0.0) as usize / block_size;

        // if the current cell is not walkable we have hit a wall and we stop
        if x < 0.0 || y < 0.0 || maze.is_wall(i, j) {
            return refine_hit(player, cos_a, sin_a, i, j, d, maze.cell(i, j), block_size);
        }

        // Safety net: a maze with a hole in its border would loop forever.
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

/// Turn the 1-pixel-accurate hit of the marching loop into the exact point where
/// the ray enters the wall cell.
///
/// The marching step is fine to find *which* cell is solid, but `tx` taken from
/// the sampled point is off by up to one pixel — about 4% of the texture width
/// with 24-pixel blocks — and that makes the texture wobble as the player walks.
/// Solving the crossing with the two entry planes of the cell removes the error:
/// the ray enters the cell at the *later* of the two crossings.
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

    // Distance at which the ray crosses each entry plane of the cell. A ray
    // parallel to an axis never crosses that one, hence the -infinity.
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

    // Clamped to the marched distance: if the player is standing inside a wall
    // the entry plane is behind them and `t` comes out negative.
    let t = t_x.max(t_y).clamp(0.0, marched.max(0.0));

    let hit_x = player.pos.x + t * cos_a;
    let hit_y = player.pos.y + t * sin_a;

    let mut tx = match side {
        Side::Vertical => (hit_y - cell_y) / bs,
        Side::Horizontal => (hit_x - cell_x) / bs,
    };

    // Mirror the coordinate on the two faces the ray reaches from behind, so the
    // texture is not flipped between opposite sides of the same block.
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
