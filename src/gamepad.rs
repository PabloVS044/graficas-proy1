use raylib::prelude::*;

use crate::player::Intent;

/// The first connected controller. Nothing here supports two players, so this is
/// always the pad the game listens to.
pub const PAD: i32 = 0;

/// Stick movement below this is ignored. Xbox sticks rest slightly off center,
/// and without a dead zone that drift turns the camera on its own forever.
const DEADZONE: f32 = 0.18;

/// Rescales `value` so that the dead zone is cut out *without* a jump.
///
/// Returning the raw value once it passes the threshold would make the stick go
/// from 0 to 0.18 the instant it crosses, which feels like a kick. Stretching
/// `[DEADZONE, 1]` back onto `[0, 1]` keeps it continuous.
pub fn deadzone(value: f32) -> f32 {
    let magnitude = value.abs();
    if magnitude < DEADZONE {
        return 0.0;
    }
    let scaled = (magnitude - DEADZONE) / (1.0 - DEADZONE);
    scaled.min(1.0) * value.signum()
}

/// What the controller is asking for this frame.
///
/// Everything is zero when no pad is connected, so unplugging it mid-game just
/// leaves the keyboard in charge instead of breaking anything.
pub fn intent(window: &RaylibHandle) -> Intent {
    if !window.is_gamepad_available(PAD) {
        return Intent::default();
    }

    // Sticks. The Y axis points down (up on the stick is negative), so forward
    // is the negated value.
    let mut forward = -deadzone(window.get_gamepad_axis_movement(PAD, GamepadAxis::GAMEPAD_AXIS_LEFT_Y));
    let mut strafe = deadzone(window.get_gamepad_axis_movement(PAD, GamepadAxis::GAMEPAD_AXIS_LEFT_X));
    let turn = deadzone(window.get_gamepad_axis_movement(PAD, GamepadAxis::GAMEPAD_AXIS_RIGHT_X));

    // D-pad, all or nothing, on the same axes as the left stick.
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
    }
}

/// A or Start: what confirms on the title screen.
pub fn confirm_pressed(window: &RaylibHandle) -> bool {
    window.is_gamepad_available(PAD)
        && (window.is_gamepad_button_pressed(PAD, GamepadButton::GAMEPAD_BUTTON_RIGHT_FACE_DOWN)
            || window.is_gamepad_button_pressed(PAD, GamepadButton::GAMEPAD_BUTTON_MIDDLE_RIGHT))
}

/// Name of the connected pad, to show it on screen.
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
        // Just past the threshold the output has to start from ~0, not from the
        // threshold itself: that jump is what feels like a kick in the hand.
        let just_past = deadzone(DEADZONE + 0.001);
        assert!(just_past > 0.0 && just_past < 0.01, "{just_past}");
    }

    #[test]
    fn full_tilt_saturates_at_one() {
        assert!((deadzone(1.0) - 1.0).abs() < f32::EPSILON);
        assert!((deadzone(-1.0) + 1.0).abs() < f32::EPSILON);
        // Drivers can overshoot slightly; that must not go past 1.
        assert_eq!(deadzone(1.4), 1.0);
    }

    #[test]
    fn the_sign_survives() {
        assert!(deadzone(0.5) > 0.0);
        assert!(deadzone(-0.5) < 0.0);
        assert_eq!(deadzone(0.5), -deadzone(-0.5));
    }
}
