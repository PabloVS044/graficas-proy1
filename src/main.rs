mod caster;
mod framebuffer;
mod maze;
mod player;
mod render;

use raylib::prelude::*;
use std::path::{Path, PathBuf};

use framebuffer::Framebuffer;
use maze::{GOAL, Maze, SPAWN};
use player::Player;

const WINDOW_WIDTH: i32 = 800;
const WINDOW_HEIGHT: i32 = 600;

fn main() {
    let maze = Maze::load(maze_path().to_str().unwrap());

    // World scale: one maze cell is this many pixels. The minimap scales it down
    // on its own, so this only has to be a comfortable size to walk around in.
    let block_size = (WINDOW_WIDTH as usize / maze.width)
        .min(WINDOW_HEIGHT as usize / maze.height)
        .max(8);

    let spawn = maze
        .find(SPAWN)
        .map(|(i, j)| maze.cell_center(i, j, block_size))
        .expect("maze has no 'p' spawn cell");
    let goal = maze.find(GOAL);

    let mut player = Player::new(spawn);

    let (mut window, thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title("Proyecto 1 - Raycaster")
        .build();
    window.set_target_fps(60);

    let mut framebuffer = Framebuffer::new(WINDOW_WIDTH, WINDOW_HEIGHT, Color::BLACK);
    let mut show_minimap = true;
    let mut won = false;

    // `cargo run -- --screenshot` draws one full frame, saves the window to disk
    // and exits, which is handy for checking the render without playing.
    if std::env::args().any(|arg| arg == "--screenshot") {
        framebuffer.clear();
        render::render_world(&mut framebuffer, &maze, &player, block_size);
        render::render_minimap(&mut framebuffer, &maze, &player, block_size);
        framebuffer.swap_buffers(&mut window, &thread, |d| draw_hud(d, 0, false));
        window.take_screenshot(&thread, "screenshot.png");
        return;
    }

    while !window.window_should_close() {
        // 1. show or hide the minimap
        if window.is_key_pressed(KeyboardKey::KEY_M) {
            show_minimap = !show_minimap;
        }

        // 2. move the player on user input
        player::process_events(&mut player, &window, &maze, block_size);

        if let Some((gi, gj)) = goal {
            if maze.cell_at_pixel(player.pos, block_size) == (gi, gj) {
                won = true;
            }
        }

        // 3. clear framebuffer
        framebuffer.clear();

        // 4. draw stuff: the 3D world first, then the minimap on top of it
        render::render_world(&mut framebuffer, &maze, &player, block_size);
        if show_minimap {
            render::render_minimap(&mut framebuffer, &maze, &player, block_size);
        }

        let fps = window.get_fps();

        framebuffer.swap_buffers(&mut window, &thread, |d| draw_hud(d, fps, won));
    }
}

/// Everything drawn on top of the raycast image, in window coordinates.
fn draw_hud(d: &mut RaylibDrawHandle, fps: u32, won: bool) {
    d.draw_text(
        &format!("M: minimapa  |  {fps} FPS"),
        10,
        10,
        18,
        Color::WHITESMOKE,
    );

    draw_crosshair(d);

    if won {
        d.draw_text(
            "GANASTE!",
            WINDOW_WIDTH / 2 - 90,
            WINDOW_HEIGHT / 2 - 20,
            40,
            Color::LIME,
        );
    }
}

/// Length of each arm of the crosshair, in pixels.
const CROSSHAIR_LENGTH: i32 = 8;
/// Space left empty around the exact center, so the aimed point stays visible.
const CROSSHAIR_GAP: i32 = 5;
const CROSSHAIR_THICKNESS: i32 = 2;

/// A fixed cross at the center of the screen.
///
/// It never moves: the center column of the render is the ray cast at exactly
/// `player.a`, and every stake is centered on the horizon, so the center of the
/// window already *is* the point the player is looking at.
fn draw_crosshair(d: &mut RaylibDrawHandle) {
    let center_x = WINDOW_WIDTH / 2;
    let center_y = WINDOW_HEIGHT / 2;
    let thickness = CROSSHAIR_THICKNESS;
    let half = thickness / 2;

    // (x, y, width, height) of the four arms, growing away from the center.
    let arms = [
        (
            center_x - half,
            center_y - CROSSHAIR_GAP - CROSSHAIR_LENGTH,
            thickness,
            CROSSHAIR_LENGTH,
        ),
        (
            center_x - half,
            center_y + CROSSHAIR_GAP,
            thickness,
            CROSSHAIR_LENGTH,
        ),
        (
            center_x - CROSSHAIR_GAP - CROSSHAIR_LENGTH,
            center_y - half,
            CROSSHAIR_LENGTH,
            thickness,
        ),
        (
            center_x + CROSSHAIR_GAP,
            center_y - half,
            CROSSHAIR_LENGTH,
            thickness,
        ),
    ];

    for (x, y, width, height) in arms {
        // Dark outline first, so the crosshair reads against light walls too.
        d.draw_rectangle(
            x - 1,
            y - 1,
            width + 2,
            height + 2,
            Color::new(0, 0, 0, 160),
        );
        d.draw_rectangle(x, y, width, height, Color::WHITESMOKE);
    }
}

/// Look for the maze next to the working directory first, then fall back to the
/// project root so `cargo run` works from anywhere.
fn maze_path() -> PathBuf {
    let local = PathBuf::from("maze.txt");
    if local.exists() {
        return local;
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("maze.txt")
}
