use raylib::prelude::*;
use std::f32::consts::PI;

use crate::maze::Maze;

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

const MOVE_SPEED: f32 = 120.0;
const ROTATION_SPEED: f32 = PI;

#[derive(Default, Clone, Copy)]
pub struct Intent {
    pub forward: f32,
    pub strafe: f32,
    pub turn: f32,
    pub look_dx: f32,
}

impl Intent {
    pub fn merge(self, other: Intent) -> Intent {
        Intent {
            forward: (self.forward + other.forward).clamp(-1.0, 1.0),
            strafe: (self.strafe + other.strafe).clamp(-1.0, 1.0),
            turn: (self.turn + other.turn).clamp(-1.0, 1.0),
            look_dx: self.look_dx + other.look_dx,
        }
    }
}

pub const MOUSE_SENSITIVITY: f32 = 0.0018;

const MAX_MOUSE_DELTA: f32 = 200.0;

const COLLISION_RADIUS: f32 = 5.0;

pub fn look_delta(mouse_dx: f32) -> f32 {
    mouse_dx.clamp(-MAX_MOUSE_DELTA, MAX_MOUSE_DELTA) * MOUSE_SENSITIVITY
}

pub fn wrap_angle(a: f32) -> f32 {
    a.rem_euclid(2.0 * PI)
}

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

    if window.is_key_down(KeyboardKey::KEY_W) || window.is_key_down(KeyboardKey::KEY_UP) {
        intent.forward += 1.0;
    }
    if window.is_key_down(KeyboardKey::KEY_S) || window.is_key_down(KeyboardKey::KEY_DOWN) {
        intent.forward -= 1.0;
    }

    if window.is_key_down(KeyboardKey::KEY_D) {
        intent.strafe += 1.0;
    }
    if window.is_key_down(KeyboardKey::KEY_A) {
        intent.strafe -= 1.0;
    }

    intent
}

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

    let (sin_a, cos_a) = player.a.sin_cos();
    let dx = forward * cos_a - strafe * sin_a;
    let dy = forward * sin_a + strafe * cos_a;
    try_move(player, dx, dy, maze, block_size)
}

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
