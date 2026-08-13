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

/// Speeds are per second, not per frame, and get multiplied by the frame time.
/// With per-frame speeds the player walks slower whenever the framerate drops,
/// which is exactly what happens on a slower machine.
const MOVE_SPEED: f32 = 120.0; // px/s   (the old 2.0 px/frame at 60 fps)
const ROTATION_SPEED: f32 = PI; // rad/s  (the old PI/60 per frame at 60 fps)

/// What the player is asking for this frame, whatever device asked for it.
///
/// Every axis is in `[-1, 1]` and gets scaled by the speeds and the frame time,
/// except `look_dx`: a mouse delta is a movement that already happened, not a
/// speed, so it is applied as is (see `look_delta`).
#[derive(Default, Clone, Copy)]
pub struct Intent {
    pub forward: f32,
    pub strafe: f32,
    pub turn: f32,
    pub look_dx: f32,
}

impl Intent {
    /// Adds two intents, keeping each axis inside `[-1, 1]`.
    ///
    /// The clamp is what stops pushing the stick and holding W at the same time
    /// from moving at double speed.
    pub fn merge(self, other: Intent) -> Intent {
        Intent {
            forward: (self.forward + other.forward).clamp(-1.0, 1.0),
            strafe: (self.strafe + other.strafe).clamp(-1.0, 1.0),
            turn: (self.turn + other.turn).clamp(-1.0, 1.0),
            look_dx: self.look_dx + other.look_dx,
        }
    }
}

/// How much the view turns per pixel of mouse movement.
pub const MOUSE_SENSITIVITY: f32 = 0.0018; // rad/px

/// Mouse movement bigger than this in a single frame is clamped. Capturing the
/// cursor, coming back from an alt-tab or dragging the window all make GLFW
/// report one huge delta, and without the clamp the camera snaps around.
const MAX_MOUSE_DELTA: f32 = 200.0; // px

/// Half-width of the player's collision box, in pixels.
const COLLISION_RADIUS: f32 = 5.0;

/// How much the view turns for a horizontal mouse movement of `mouse_dx` pixels.
///
/// Note this is *not* scaled by the frame time: a mouse delta is already a
/// displacement, not a speed, so multiplying it by `dt` would make the same hand
/// movement turn a different amount depending on the framerate.
pub fn look_delta(mouse_dx: f32) -> f32 {
    mouse_dx.clamp(-MAX_MOUSE_DELTA, MAX_MOUSE_DELTA) * MOUSE_SENSITIVITY
}

/// Keeps the angle in `[0, 2PI)` so it never drifts into huge values.
pub fn wrap_angle(a: f32) -> f32 {
    a.rem_euclid(2.0 * PI)
}

/// What the keyboard is asking for. `mouse_dx` is how many pixels the mouse moved
/// horizontally this frame (0 when the cursor is not captured): the caller
/// measures it, because keeping the pointer inside the window needs
/// `&mut RaylibHandle`.
pub fn keyboard_intent(window: &RaylibHandle, mouse_dx: f32) -> Intent {
    let mut intent = Intent {
        look_dx: mouse_dx,
        ..Intent::default()
    };

    if window.is_key_down(KeyboardKey::KEY_RIGHT) {
        intent.turn += 1.0;
    }
    if window.is_key_down(KeyboardKey::KEY_LEFT) {
        intent.turn -= 1.0;
    }

    // Forward/back along the view direction.
    if window.is_key_down(KeyboardKey::KEY_W) || window.is_key_down(KeyboardKey::KEY_UP) {
        intent.forward += 1.0;
    }
    if window.is_key_down(KeyboardKey::KEY_S) || window.is_key_down(KeyboardKey::KEY_DOWN) {
        intent.forward -= 1.0;
    }

    // Strafe: sideways, without turning.
    if window.is_key_down(KeyboardKey::KEY_D) {
        intent.strafe += 1.0;
    }
    if window.is_key_down(KeyboardKey::KEY_A) {
        intent.strafe -= 1.0;
    }

    intent
}

/// Turns and moves the camera. Returns whether the movement was blocked by a
/// wall on at least one axis.
pub fn apply_intent(
    player: &mut Player,
    intent: Intent,
    maze: &Maze,
    block_size: usize,
    dt: f32,
) -> bool {
    player.a += look_delta(intent.look_dx) + intent.turn * ROTATION_SPEED * dt;
    player.a = wrap_angle(player.a);

    if intent.forward == 0.0 && intent.strafe == 0.0 {
        return false;
    }

    let forward = intent.forward * MOVE_SPEED * dt;
    let strafe = intent.strafe * MOVE_SPEED * dt;

    // Same trigonometry as the ray: cos on x, sin on y. The strafe direction is
    // that vector rotated a quarter turn: (-sin, cos).
    let (sin_a, cos_a) = player.a.sin_cos();
    let dx = forward * cos_a - strafe * sin_a;
    let dy = forward * sin_a + strafe * cos_a;
    try_move(player, dx, dy, maze, block_size)
}

/// Move one axis at a time so that sliding along a wall keeps working instead of
/// blocking the whole movement. Returns whether either axis was blocked.
fn try_move(player: &mut Player, dx: f32, dy: f32, maze: &Maze, block_size: usize) -> bool {
    let mut blocked = false;

    let next_x = player.pos.x + dx;
    if collides(next_x, player.pos.y, maze, block_size) {
        blocked = true;
    } else {
        player.pos.x = next_x;
    }

    let next_y = player.pos.y + dy;
    if collides(player.pos.x, next_y, maze, block_size) {
        blocked = true;
    } else {
        player.pos.y = next_y;
    }

    blocked
}

/// The player is a small box, not a point: test its four corners so it can't
/// clip into a wall corner.
fn collides(x: f32, y: f32, maze: &Maze, block_size: usize) -> bool {
    const R: f32 = COLLISION_RADIUS;
    [(-R, -R), (R, -R), (-R, R), (R, R)]
        .iter()
        .any(|(ox, oy)| maze.is_wall_at_pixel(x + ox, y + oy, block_size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_right_turns_right() {
        // y grows down, so a growing angle is a clockwise turn on screen, which
        // is what moving the mouse to the right has to do.
        assert!(look_delta(10.0) > 0.0);
        assert!(look_delta(-10.0) < 0.0);
        assert_eq!(look_delta(0.0), 0.0);
    }

    #[test]
    fn look_delta_is_proportional_to_the_movement() {
        assert_eq!(look_delta(40.0), 40.0 * MOUSE_SENSITIVITY);
        assert!((look_delta(20.0) - 2.0 * look_delta(10.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn huge_jumps_are_clamped_symmetrically() {
        let capped = MAX_MOUSE_DELTA * MOUSE_SENSITIVITY;
        assert_eq!(look_delta(10_000.0), capped);
        assert_eq!(look_delta(-10_000.0), -capped);
        // Right at the limit nothing is lost yet.
        assert_eq!(look_delta(MAX_MOUSE_DELTA), capped);
    }

    #[test]
    fn merging_two_devices_does_not_double_the_speed() {
        let keyboard = Intent {
            forward: 1.0,
            strafe: 1.0,
            turn: 1.0,
            look_dx: 0.0,
        };
        let pad = keyboard;
        let both = keyboard.merge(pad);
        assert_eq!(both.forward, 1.0);
        assert_eq!(both.strafe, 1.0);
        assert_eq!(both.turn, 1.0);
    }

    #[test]
    fn opposite_inputs_cancel_out() {
        let forward = Intent {
            forward: 1.0,
            ..Intent::default()
        };
        let back = Intent {
            forward: -1.0,
            ..Intent::default()
        };
        assert_eq!(forward.merge(back).forward, 0.0);
    }

    #[test]
    fn mouse_movement_adds_up_instead_of_clamping() {
        // look_dx is a displacement in pixels, not a -1..1 axis: clamping it
        // would silently cap how fast the view can turn.
        let a = Intent {
            look_dx: 40.0,
            ..Intent::default()
        };
        let b = Intent {
            look_dx: 30.0,
            ..Intent::default()
        };
        assert_eq!(a.merge(b).look_dx, 70.0);
    }

    #[test]
    fn angles_stay_in_one_turn() {
        let two_pi = 2.0 * PI;
        for a in [-10.0, -PI, 0.0, PI, 7.0, 100.0] {
            let wrapped = wrap_angle(a);
            assert!((0.0..two_pi).contains(&wrapped), "{a} -> {wrapped}");
        }
        assert!((wrap_angle(-PI / 2.0) - (1.5 * PI)).abs() < 1e-5);
        assert!((wrap_angle(PI / 4.0) - PI / 4.0).abs() < 1e-6);
    }
}
