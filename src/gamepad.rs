use raylib::prelude::*;

use crate::player::Intent;

pub const PAD: i32 = 0;

const DEADZONE: f32 = 0.18;

pub fn deadzone(value: f32) -> f32 {
    let magnitude = value.abs();
    if magnitude < DEADZONE {
        return 0.0;
    }
    let scaled = (magnitude - DEADZONE) / (1.0 - DEADZONE);
    scaled.min(1.0) * value.signum()
}

pub fn intent(window: &RaylibHandle) -> Intent {
    if !window.is_gamepad_available(PAD) {
        return Intent::default();
    }

    let mut forward =
        -deadzone(window.get_gamepad_axis_movement(PAD, GamepadAxis::GAMEPAD_AXIS_LEFT_Y));
    let mut strafe =
        deadzone(window.get_gamepad_axis_movement(PAD, GamepadAxis::GAMEPAD_AXIS_LEFT_X));
    let turn = deadzone(window.get_gamepad_axis_movement(PAD, GamepadAxis::GAMEPAD_AXIS_RIGHT_X));

    if window.is_gamepad_button_down(PAD, GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_UP) {
        forward += 1.0;
    }
    if window.is_gamepad_button_down(PAD, GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_DOWN) {
        forward -= 1.0;
    }
    if window.is_gamepad_button_down(PAD, GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_RIGHT) {
        strafe += 1.0;
    }
    if window.is_gamepad_button_down(PAD, GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_LEFT) {
        strafe -= 1.0;
    }

    Intent {
        forward: forward.clamp(-1.0, 1.0),
        strafe: strafe.clamp(-1.0, 1.0),
        turn,
        look_dx: 0.0,
        // Gatillo derecho dispara, X recarga.
        shoot: window.is_gamepad_button_pressed(PAD, GamepadButton::GAMEPAD_BUTTON_RIGHT_TRIGGER_2),
        reload: window
            .is_gamepad_button_pressed(PAD, GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_LEFT),
    }
}

pub fn menu_axis(window: &RaylibHandle) -> i32 {
    if !window.is_gamepad_available(PAD) {
        return 0;
    }

    if window.is_gamepad_button_down(PAD, GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_UP) {
        return -1;
    }
    if window.is_gamepad_button_down(PAD, GamepadButton::GAMEPAD_BUTTON_LEFT_FACE_DOWN) {
        return 1;
    }

    let stick = deadzone(window.get_gamepad_axis_movement(PAD, GamepadAxis::GAMEPAD_AXIS_LEFT_Y));
    if stick.abs() < 0.5 {
        0
    } else {
        stick.signum() as i32
    }
}

pub fn confirm_pressed(window: &RaylibHandle) -> bool {
    window.is_gamepad_available(PAD)
        && (window.is_gamepad_button_pressed(PAD, GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_DOWN)
            || window.is_gamepad_button_pressed(PAD, GamepadButton::GAMEPAD_BUTTON_MIDDLE_RIGHT))
}

pub fn name(window: &RaylibHandle) -> Option<String> {
    if !window.is_gamepad_available(PAD) {
        return None;
    }
    window.get_gamepad_name(PAD)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resting_drift_is_ignored() {
        assert_eq!(deadzone(0.0), 0.0);
        assert_eq!(deadzone(DEADZONE * 0.99), 0.0);
        assert_eq!(deadzone(-DEADZONE * 0.99), 0.0);
    }

    #[test]
    fn there_is_no_jump_at_the_edge_of_the_dead_zone() {
        let just_past = deadzone(DEADZONE + 0.001);
        assert!(just_past > 0.0 && just_past < 0.01, "{just_past}");
    }

    #[test]
    fn full_tilt_saturates_at_one() {
        assert!((deadzone(1.0) - 1.0).abs() < f32::EPSILON);
        assert!((deadzone(-1.0) + 1.0).abs() < f32::EPSILON);
        assert_eq!(deadzone(1.4), 1.0);
    }

    #[test]
    fn the_sign_survives() {
        assert!(deadzone(0.5) > 0.0);
        assert!(deadzone(-0.5) < 0.0);
        assert_eq!(deadzone(0.5), -deadzone(-0.5));
    }
}
