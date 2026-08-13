use raylib::prelude::*;
use std::f32::consts::PI;

use crate::maze::Maze;

/// The player is also the camera: `pos` is the optical center, `a` is the
/// direction it looks at (radians, growing clockwise on screen because y grows
/// down) and `fov` is how wide the view cone is.
pub struct Player {
    pub pos: Vector2,
    pub a: f32,
    pub fov: f32,
}

impl Player {
    pub fn new(pos: Vector2) -> Self {
        Player {
            pos,
            a: PI / 3.0,
            fov: PI / 3.0,
        }
    }
}

const MOVE_SPEED: f32 = 2.0;
const ROTATION_SPEED: f32 = PI / 60.0;
/// Half-width of the player's collision box, in pixels.
const COLLISION_RADIUS: f32 = 5.0;

/// Left/Right turn the view, Up/Down walk along the direction of view.
pub fn process_events(player: &mut Player, window: &RaylibHandle, maze: &Maze, block_size: usize) {
    if window.is_key_down(KeyboardKey::KEY_LEFT) {
        player.a -= ROTATION_SPEED;
    }
    if window.is_key_down(KeyboardKey::KEY_RIGHT) {
        player.a += ROTATION_SPEED;
    }
    // Keep the angle in [0, 2PI) so it never drifts into huge values.
    player.a = player.a.rem_euclid(2.0 * PI);

    let mut step = 0.0;
    if window.is_key_down(KeyboardKey::KEY_UP) {
        step += MOVE_SPEED;
    }
    if window.is_key_down(KeyboardKey::KEY_DOWN) {
        step -= MOVE_SPEED;
    }

    if step != 0.0 {
        // Same trigonometry as the ray: cos on x, sin on y.
        let dx = step * player.a.cos();
        let dy = step * player.a.sin();
        try_move(player, dx, dy, maze, block_size);
    }
}

/// Move one axis at a time so that sliding along a wall keeps working instead of
/// blocking the whole movement.
fn try_move(player: &mut Player, dx: f32, dy: f32, maze: &Maze, block_size: usize) {
    let next_x = player.pos.x + dx;
    if !collides(next_x, player.pos.y, maze, block_size) {
        player.pos.x = next_x;
    }

    let next_y = player.pos.y + dy;
    if !collides(player.pos.x, next_y, maze, block_size) {
        player.pos.y = next_y;
    }
}

/// The player is a small box, not a point: test its four corners so it can't
/// clip into a wall corner.
fn collides(x: f32, y: f32, maze: &Maze, block_size: usize) -> bool {
    const R: f32 = COLLISION_RADIUS;
    [(-R, -R), (R, -R), (-R, R), (R, R)]
        .iter()
        .any(|(ox, oy)| maze.is_wall_at_pixel(x + ox, y + oy, block_size))
}
