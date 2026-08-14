mod caster;
mod framebuffer;
mod gamepad;
mod maze;
mod player;
mod render;
mod sprites;
mod textures;

use raylib::prelude::*;

use framebuffer::Framebuffer;
use maze::{ENEMY, GOAL, Maze, SPAWN};
use player::Player;
use sprites::Enemy;
use textures::{TextureManager, asset_path};

const WINDOW_WIDTH: i32 = 800;
const WINDOW_HEIGHT: i32 = 600;

/// Upper bound for the frame time fed to the movement code, in seconds. Without
/// it, the very first frame (or a hitch while another window steals the CPU)
/// would be integrated as one huge step.
const MAX_FRAME_TIME: f32 = 0.1;

const TITLE: &str = "PROYECTO 1 - RAYCASTER";

/// Background of the title screen, in order of preference: the first one that
/// loads is used. `pared.png` is the placeholder until there is a proper
/// `menu.png`.
const MENU_BACKGROUNDS: [&str; 2] = ["assets/menu.png", "assets/pared.png"];

/// Which screen the game is on. The level selector is the one still missing.
#[derive(PartialEq)]
enum Screen {
    Menu,
    Playing,
    Victory,
}

/// The two things offered after winning.
const VICTORY_OPTIONS: [&str; 2] = ["Seguir explorando", "Reiniciar el nivel"];
const OPTION_FREEROAM: usize = 0;
const OPTION_RESTART: usize = 1;

fn main() {
    let maze = Maze::load(asset_path("maze.txt").to_str().unwrap());

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

    // Sprites are placed from the map itself: every `e` cell becomes an enemy
    // standing at the center of that cell.
    let enemies: Vec<Enemy> = maze
        .find_all(ENEMY)
        .into_iter()
        .map(|(i, j)| Enemy {
            pos: maze.cell_center(i, j, block_size),
            kind: ENEMY,
        })
        .collect();

    let mut player = Player::new(spawn);

    let (mut window, thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title("Proyecto 1 - Raycaster")
        .build();
    window.set_target_fps(60);

    // Loaded once: every wall pixel drawn from here on samples these images.
    let texture_manager = TextureManager::new();

    let mut framebuffer = Framebuffer::new(WINDOW_WIDTH, WINDOW_HEIGHT, Color::BLACK);
    let mut show_minimap = true;

    // After winning, the player can keep walking around the level. The goal stops
    // triggering then, or reaching it again would throw the victory screen back
    // in their face.
    let mut freeroam = false;
    let mut run_started = 0.0;
    let mut run_time = 0.0;
    let mut victory_choice = OPTION_FREEROAM;
    // Whether the pad's menu axis was resting last frame, so holding the stick
    // moves the selection once instead of every frame.
    let mut pad_menu_rested = true;

    // Background for the title screen. It is a plain GPU texture, not a
    // `TextureManager` entry: that one keeps pixels on the CPU to sample them one
    // by one with tx/ty, and here the image is blitted whole.
    //
    // Same idea as the wall textures: the first candidate that loads wins, so
    // dropping a `menu.png` in `assets/` takes over without touching any code.
    let menu_background = MENU_BACKGROUNDS.iter().find_map(|path| {
        let full = asset_path(path);
        window.load_texture(&thread, full.to_str().unwrap_or(path)).ok()
    });
    if menu_background.is_none() {
        eprintln!("aviso: sin fondo de menú (se buscó {MENU_BACKGROUNDS:?}); queda en negro");
    }

    // `cargo run -- --screenshot` draws one full frame, saves the window to disk
    // and exits, which is handy for checking the render without playing.
    // `--menu` and `--victory` do the same for the other two screens, which
    // otherwise can only be seen by playing.
    if std::env::args().any(|arg| arg == "--screenshot") {
        let menu_shot = std::env::args().any(|arg| arg == "--menu");
        let victory_shot = std::env::args().any(|arg| arg == "--victory");
        let pad_name = gamepad::name(&window);
        framebuffer.clear();
        let zbuffer =
            render::render_world(&mut framebuffer, &maze, &player, &texture_manager, block_size);
        sprites::render_enemies(
            &mut framebuffer,
            &player,
            &enemies,
            &texture_manager,
            &zbuffer,
            block_size,
        );
        if !menu_shot && !victory_shot {
            render::render_minimap(&mut framebuffer, &maze, &player, &enemies, block_size);
        }
        framebuffer.swap_buffers(&mut window, &thread, |d| {
            if menu_shot {
                draw_menu(d, menu_background.as_ref(), 0.0, pad_name.as_deref());
            } else if victory_shot {
                draw_victory(d, OPTION_FREEROAM, 42.7, pad_name.as_deref());
            } else {
                draw_hud(d, 0, false, true, pad_name.as_deref());
            }
        });
        window.take_screenshot(&thread, "screenshot.png");
        return;
    }

    // The title screen owns the cursor: it is only captured once the game starts.
    let mut screen = Screen::Menu;
    let mut mouse_look = true;

    while !window.window_should_close() {
        let dt = window.get_frame_time().min(MAX_FRAME_TIME);

        match screen {
            Screen::Menu => {
                if window.is_key_pressed(KeyboardKey::KEY_ENTER)
                    || window.is_key_pressed(KeyboardKey::KEY_SPACE)
                    || gamepad::confirm_pressed(&window)
                {
                    screen = Screen::Playing;
                    capture_cursor(&mut window);
                    mouse_look = true;
                    run_started = window.get_time();
                }
            }
            Screen::Playing => {
                // 1. show or hide the minimap, release or recapture the cursor
                if window.is_key_pressed(KeyboardKey::KEY_M) {
                    show_minimap = !show_minimap;
                }
                if window.is_key_pressed(KeyboardKey::KEY_TAB) {
                    mouse_look = !mouse_look;
                    if mouse_look {
                        capture_cursor(&mut window);
                    } else {
                        release_cursor(&mut window);
                    }
                }

                // 2. move the player on user input. Keyboard, mouse and pad all
                //    end up in the same `Intent`, merged so that using two of them
                //    at once does not move at double speed. Speeds are per second,
                //    so they need the frame time; the first frame (and any hitch)
                //    is clamped so a long one can't teleport the player through a
                //    wall.
                let mouse_dx = read_mouse_look(&mut window, mouse_look);
                let intent =
                    player::keyboard_intent(&window, mouse_dx).merge(gamepad::intent(&window));
                player::apply_intent(&mut player, intent, &maze, block_size, dt);

                if let Some((gi, gj)) = goal {
                    if !freeroam && maze.cell_at_pixel(player.pos, block_size) == (gi, gj) {
                        screen = Screen::Victory;
                        run_time = (window.get_time() - run_started) as f32;
                        victory_choice = OPTION_FREEROAM;
                        release_cursor(&mut window);
                    }
                }
            }
            Screen::Victory => {
                // Move the selection: one step per key press, and one step per
                // *new* push of the pad (hence the latch).
                let pad_axis = gamepad::menu_axis(&window);
                let mut step = 0;
                if window.is_key_pressed(KeyboardKey::KEY_UP)
                    || window.is_key_pressed(KeyboardKey::KEY_W)
                {
                    step -= 1;
                }
                if window.is_key_pressed(KeyboardKey::KEY_DOWN)
                    || window.is_key_pressed(KeyboardKey::KEY_S)
                {
                    step += 1;
                }
                if pad_axis != 0 && pad_menu_rested {
                    step += pad_axis;
                }
                pad_menu_rested = pad_axis == 0;

                if step != 0 {
                    // rem_euclid so that going up from the first option wraps to
                    // the last one instead of going negative.
                    let count = VICTORY_OPTIONS.len() as i32;
                    victory_choice = (victory_choice as i32 + step).rem_euclid(count) as usize;
                }

                if window.is_key_pressed(KeyboardKey::KEY_ENTER)
                    || window.is_key_pressed(KeyboardKey::KEY_SPACE)
                    || gamepad::confirm_pressed(&window)
                {
                    if victory_choice == OPTION_RESTART {
                        player = Player::new(spawn);
                        freeroam = false;
                        run_started = window.get_time();
                    } else {
                        // Free roam: the level stays as it is and the goal stops
                        // counting, so walking over it again changes nothing.
                        freeroam = true;
                    }
                    screen = Screen::Playing;
                    capture_cursor(&mut window);
                    mouse_look = true;
                }
            }
        }

        // 3. clear framebuffer
        framebuffer.clear();

        // 4. draw stuff: the 3D world first (which fills the z-buffer), then the
        //    sprites depth-tested against it, and the minimap on top of both.
        //    The menu draws none of it: it is a full screen image over a cleared
        //    buffer, so raycasting a frame nobody sees would be wasted work.
        //    The victory screen keeps the world behind it, so it reads as an
        //    overlay on the level instead of a hard cut to a different place.
        if screen != Screen::Menu {
            let zbuffer =
                render::render_world(&mut framebuffer, &maze, &player, &texture_manager, block_size);
            sprites::render_enemies(
                &mut framebuffer,
                &player,
                &enemies,
                &texture_manager,
                &zbuffer,
                block_size,
            );
            if show_minimap && screen == Screen::Playing {
                render::render_minimap(&mut framebuffer, &maze, &player, &enemies, block_size);
            }
        }

        let fps = window.get_fps();
        let time = window.get_time() as f32;
        let pad = gamepad::name(&window);

        framebuffer.swap_buffers(&mut window, &thread, |d| match screen {
            Screen::Menu => draw_menu(d, menu_background.as_ref(), time, pad.as_deref()),
            Screen::Playing => draw_hud(d, fps, freeroam, mouse_look, pad.as_deref()),
            Screen::Victory => draw_victory(d, victory_choice, run_time, pad.as_deref()),
        });
    }
}

/// The title screen: background, a dark veil over it, and the text.
fn draw_menu(
    d: &mut RaylibDrawHandle,
    background: Option<&Texture2D>,
    time: f32,
    pad: Option<&str>,
) {
    if let Some(texture) = background {
        draw_background_cover(d, texture);
    }

    // Veil, so the text stays readable over any image.
    d.draw_rectangle(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT, Color::new(0, 0, 0, 130));

    draw_centered_text(d, TITLE, 190, 40, Color::WHITESMOKE);

    // Blinking, so the eye goes to the one line that says what to do.
    let blink = (time * 3.0).sin() * 0.5 + 0.5;
    let alpha = (150.0 + 105.0 * blink) as u8;
    let start = match pad {
        Some(_) => "ENTER o (A) para jugar",
        None => "ENTER para jugar",
    };
    draw_centered_text(d, start, 300, 26, Color::new(0xE8, 0xC0, 0x50, alpha));

    let controls = match pad {
        Some(_) => "WASD + mouse  o  sticks del mando",
        None => "WASD + mouse para moverse",
    };
    draw_centered_text(d, controls, 380, 18, Color::LIGHTGRAY);
    draw_centered_text(d, "TAB libera el cursor  |  M minimapa", 404, 18, Color::GRAY);
    draw_centered_text(d, "ESC para salir", 440, 18, Color::GRAY);

    // Naming the pad is the proof it is really connected, which is the point of
    // the whole feature.
    if let Some(name) = pad {
        draw_centered_text(
            d,
            &format!("mando: {name}"),
            WINDOW_HEIGHT - 40,
            16,
            Color::new(0x8A, 0xD6, 0x9B, 255),
        );
    }
}

/// The success screen, drawn over the frozen level.
fn draw_victory(d: &mut RaylibDrawHandle, choice: usize, run_time: f32, pad: Option<&str>) {
    d.draw_rectangle(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT, Color::new(0, 0, 0, 160));

    draw_centered_text(d, "GANASTE!", 140, 56, Color::new(0x5E, 0xD9, 0x8A, 255));
    draw_centered_text(
        d,
        &format!("llegaste a la meta en {run_time:.1} s"),
        215,
        20,
        Color::LIGHTGRAY,
    );

    for (i, option) in VICTORY_OPTIONS.iter().enumerate() {
        let y = 300 + i as i32 * 46;
        let selected = i == choice;
        // The arrow carries the selection as much as the color does: a color
        // difference alone is easy to miss on top of a busy level.
        let label = if selected {
            format!("> {option} <")
        } else {
            option.to_string()
        };
        let color = if selected {
            Color::new(0xE8, 0xC0, 0x50, 255)
        } else {
            Color::GRAY
        };
        draw_centered_text(d, &label, y, if selected { 28 } else { 24 }, color);
    }

    let hint = match pad {
        Some(_) => "flechas o cruceta para elegir  |  ENTER o (A) para confirmar",
        None => "flechas para elegir  |  ENTER para confirmar",
    };
    draw_centered_text(d, hint, 440, 18, Color::LIGHTGRAY);
    draw_centered_text(d, "ESC para salir", 470, 18, Color::GRAY);
}

/// Draws `texture` filling the window without deforming it: the overflowing side
/// is cropped instead of squashed. The images used here are portrait, so
/// stretching them to a 800x600 window would be very visible.
fn draw_background_cover(d: &mut RaylibDrawHandle, texture: &Texture2D) {
    let (tw, th) = (texture.width as f32, texture.height as f32);
    let window_ratio = WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32;
    let texture_ratio = tw / th;

    // Take the biggest centered piece of the image with the window's proportions.
    let (src_w, src_h) = if texture_ratio > window_ratio {
        (th * window_ratio, th) // too wide: crop the sides
    } else {
        (tw, tw / window_ratio) // too tall: crop top and bottom
    };

    d.draw_texture_pro(
        texture,
        Rectangle::new((tw - src_w) / 2.0, (th - src_h) / 2.0, src_w, src_h),
        Rectangle::new(0.0, 0.0, WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32),
        Vector2::zero(),
        0.0,
        Color::WHITE,
    );
}

/// Text centered horizontally, with a dark shadow under it.
///
/// The width is measured instead of guessed: an eyeballed offset goes crooked as
/// soon as the string changes. The shadow is the same reason the crosshair has an
/// outline — plain text over a photo disappears on the light patches.
fn draw_centered_text(d: &mut RaylibDrawHandle, text: &str, y: i32, size: i32, color: Color) {
    let x = (WINDOW_WIDTH - d.measure_text(text, size)) / 2;
    d.draw_text(text, x + 2, y + 2, size, Color::new(0, 0, 0, 180));
    d.draw_text(text, x, y, size, color);
}

/// Center of the window, where the pointer is parked while the mouse aims.
fn window_center() -> Vector2 {
    Vector2::new(WINDOW_WIDTH as f32 / 2.0, WINDOW_HEIGHT as f32 / 2.0)
}

/// Hides the cursor and parks it in the center.
///
/// It hides the cursor instead of disabling it, and that distinction is the
/// whole trick. `disable_cursor` puts GLFW in `CURSOR_DISABLED`, and in that mode
/// `glfwSetCursorPos` stops warping the real pointer: it only updates an internal
/// *virtual* position. So the re-centering below would do nothing, and since the
/// pointer grab does not confine anything under XWayland, the pointer would walk
/// out of the window and the camera would stop turning. In `CURSOR_HIDDEN` the
/// warp is a real `XWarpPointer`, which is what keeps the pointer inside.
///
/// Centering is not cosmetic either: the mouse look measures against the center,
/// so starting (or coming back from TAB) with the pointer anywhere else would be
/// read as one big movement and snap the camera.
fn capture_cursor(window: &mut RaylibHandle) {
    window.hide_cursor();
    window.set_mouse_position(window_center());
}

/// Gives the cursor back to the desktop.
fn release_cursor(window: &mut RaylibHandle) {
    window.enable_cursor(); // also undoes `hide_cursor`
}

/// How many pixels the mouse moved horizontally this frame, and puts the pointer
/// back in the middle of the window.
///
/// The movement is measured against the center and the pointer is warped back
/// there every frame, because the cursor lock does not confine anything under
/// XWayland: a long horizontal sweep walks the pointer out of the window, onto
/// the desktop, and the camera silently stops turning. Warping it back keeps it
/// inside whatever the compositor does — see `capture_cursor` for why the cursor
/// is hidden rather than disabled, which is what makes the warp actually work.
fn read_mouse_look(window: &mut RaylibHandle, mouse_look: bool) -> f32 {
    // Not while the cursor is released, and not while another window has focus:
    // warping the pointer then would fight the desktop.
    if !mouse_look || !window.is_window_focused() {
        return 0.0;
    }

    let center = window_center();
    let dx = window.get_mouse_position().x - center.x;
    window.set_mouse_position(center);
    dx
}

/// Everything drawn on top of the raycast image, in window coordinates.
fn draw_hud(
    d: &mut RaylibDrawHandle,
    fps: u32,
    freeroam: bool,
    mouse_look: bool,
    pad: Option<&str>,
) {
    let input = match pad {
        Some(_) => "mando + WASD",
        None => "WASD + mouse",
    };
    d.draw_text(
        &format!("{input}  |  TAB: cursor  |  M: minimapa  |  {fps} FPS"),
        10,
        10,
        18,
        Color::WHITESMOKE,
    );

    // While the cursor is free the mouse doesn't aim, and that is easy to
    // mistake for the camera being broken. Say it on screen.
    if !mouse_look {
        d.draw_text(
            "cursor libre - TAB para volver a mirar con el mouse",
            10,
            32,
            18,
            Color::new(0xE8, 0xC0, 0x50, 255),
        );
    }

    draw_crosshair(d);

    // After winning, the goal no longer does anything: saying so avoids looking
    // like the victory screen is broken when walking over it again.
    if freeroam {
        d.draw_text(
            "modo libre - nivel completado",
            10,
            WINDOW_HEIGHT - 28,
            18,
            Color::new(0x5E, 0xD9, 0x8A, 255),
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
