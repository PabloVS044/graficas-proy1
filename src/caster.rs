use raylib::color::Color;

use crate::framebuffer::Framebuffer;
use crate::maze::Maze;
use crate::player::Player;

/// What a ray found: how far it travelled and which wall char it hit.
pub struct Intersect {
    pub distance: f32,
    pub impact: char,
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
            return Intersect {
                distance: d,
                impact: maze.cell(i, j),
            };
        }

        // Safety net: a maze with a hole in its border would loop forever.
        if d > max_distance {
            return Intersect {
                distance: max_distance,
                impact: '+',
            };
        }

        if draw_line {
            framebuffer.set_pixel(x as i32, y as i32);
        }

        d += STEP;
    }
}
