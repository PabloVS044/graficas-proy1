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

/// Velocidad de avance en **celdas por segundo**, no en píxeles: `block_size`
/// cambia con el nivel y con el tamaño de la ventana, así que una velocidad en
/// píxeles haría que el mismo juego se sintiera más rápido en los mapas grandes
/// y más lento al agrandar la ventana.
pub const MOVE_SPEED: f32 = 5.0;
const ROTATION_SPEED: f32 = PI;

#[derive(Default, Clone, Copy)]
pub struct Intent {
    pub forward: f32,
    pub strafe: f32,
    pub turn: f32,
    pub look_dx: f32,
    /// Acciones de este frame. Son flancos (se activan una vez por pulsación),
    /// no estados sostenidos, así que combinarlas es un OR.
    pub shoot: bool,
    pub reload: bool,
}

impl Intent {
    pub fn merge(self, other: Intent) -> Intent {
        Intent {
            forward: (self.forward + other.forward).clamp(-1.0, 1.0),
            strafe: (self.strafe + other.strafe).clamp(-1.0, 1.0),
            turn: (self.turn + other.turn).clamp(-1.0, 1.0),
            look_dx: self.look_dx + other.look_dx,
            shoot: self.shoot || other.shoot,
            reload: self.reload || other.reload,
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

    intent.shoot = window.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT)
        || window.is_key_pressed(KeyboardKey::KEY_LEFT_CONTROL);
    intent.reload = window.is_key_pressed(KeyboardKey::KEY_R);

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

    let speed = MOVE_SPEED * block_size as f32 * dt;
    let forward = intent.forward * speed;
    let strafe = intent.strafe * speed;

    let (sin_a, cos_a) = player.a.sin_cos();
    let dx = forward * cos_a - strafe * sin_a;
    let dy = forward * sin_a + strafe * cos_a;
    try_move(player, dx, dy, maze, block_size)
}

fn try_move(player: &mut Player, dx: f32, dy: f32, maze: &Maze, block_size: usize) -> bool {
    slide(&mut player.pos, dx, dy, maze, block_size, COLLISION_RADIUS)
}

/// Mueve `pos` un eje a la vez, para que chocar en diagonal siga dejando avanzar
/// sobre el eje libre. Devuelve si algún eje quedó bloqueado.
///
/// Lo usan el jugador y los enemigos con radios distintos: sin esto, la IA
/// necesitaría su propia copia de la colisión y las dos se irían separando.
pub fn slide(
    pos: &mut Vector2,
    dx: f32,
    dy: f32,
    maze: &Maze,
    block_size: usize,
    radius: f32,
) -> bool {
    let mut blocked = false;

    let next_x = pos.x + dx;
    if collides(next_x, pos.y, maze, block_size, radius) {
        blocked = true;
    } else {
        pos.x = next_x;
    }

    let next_y = pos.y + dy;
    if collides(pos.x, next_y, maze, block_size, radius) {
        blocked = true;
    } else {
        pos.y = next_y;
    }

    blocked
}

fn collides(x: f32, y: f32, maze: &Maze, block_size: usize, radius: f32) -> bool {
    let r = radius;
    [(-r, -r), (r, -r), (-r, r), (r, r)]
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
            ..Intent::default()
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
    fn las_acciones_se_combinan_con_or() {
        // Disparar desde el mando mientras el teclado no dispara tiene que
        // disparar igual: son flancos, no ejes.
        let teclado = Intent::default();
        let mando = Intent {
            shoot: true,
            ..Intent::default()
        };
        assert!(teclado.merge(mando).shoot);
        assert!(!teclado.merge(Intent::default()).shoot);
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
