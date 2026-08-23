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

const MAX_FRAME_TIME: f32 = 0.1;

const TITLE: &str = "PROYECTO 1 - RAYCASTER";

const MENU_BACKGROUNDS: [&str; 2] = ["assets/menu.png", "assets/pared2.png"];

const STEPS_SOUNDS: [&str; 2] = ["assets/pasos.ogg", "assets/caminar.mp3"];
const STEPS_VOLUME: f32 = 0.7;

const MUSIC_FILES: [&str; 3] = [
    "assets/musica.ogg",
    "assets/musica.mp3",
    "assets/music.ogg",
];
const MUSIC_VOLUME: f32 = 0.35;

const WALKING_EPSILON: f32 = 0.05;

#[derive(PartialEq)]
enum Screen {
    Menu,
    Playing,
    Victory,
}

const LEVELS: [(&str, &str); 3] = [
    ("1 - Facil", "maze.txt"),
    ("2 - Normal", "maze2.txt"),
    ("3 - Dificil", "maze3.txt"),
];

const VICTORY_OPTIONS: [&str; 3] = [
    "Seguir explorando",
    "Reiniciar el nivel",
    "Elegir otro nivel",
];
const OPTION_FREEROAM: usize = 0;
const OPTION_RESTART: usize = 1;
const OPTION_MENU: usize = 2;

struct Level {
    maze: Maze,
    block_size: usize,
    spawn: Vector2,
    goal: Option<(usize, usize)>,
    enemies: Vec<Enemy>,
}

fn load_level(file: &str) -> Level {
    let maze = Maze::load(asset_path(file).to_str().unwrap());

    let block_size = (WINDOW_WIDTH as usize / maze.width)
        .min(WINDOW_HEIGHT as usize / maze.height)
        .max(8);

    let spawn = maze
        .find(SPAWN)
        .map(|(i, j)| maze.cell_center(i, j, block_size))
        .unwrap_or_else(|| panic!("el laberinto '{file}' no tiene celda 'p' de spawn"));
    let goal = maze.find(GOAL);

    let enemies: Vec<Enemy> = maze
        .find_all(ENEMY)
        .into_iter()
        .map(|(i, j)| Enemy {
            pos: maze.cell_center(i, j, block_size),
            kind: ENEMY,
        })
        .collect();

    Level {
        maze,
        block_size,
        spawn,
        goal,
        enemies,
    }
}

fn main() {
    let mut level_choice = std::env::args()
        .skip_while(|arg| arg != "--level")
        .nth(1)
        .and_then(|n| n.parse::<usize>().ok())
        .map(|n| n.saturating_sub(1).min(LEVELS.len() - 1))
        .unwrap_or(0);
    let mut level = load_level(LEVELS[level_choice].1);

    let mut player = Player::new(level.spawn);

    let (mut window, thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title("Proyecto 1 - Raycaster")
        .build();
    window.set_target_fps(60);

    let texture_manager = TextureManager::new();

    let mut framebuffer = Framebuffer::new(WINDOW_WIDTH, WINDOW_HEIGHT, Color::BLACK);
    let mut show_minimap = true;

    let mut freeroam = false;
    let mut run_started = 0.0;
    let mut run_time = 0.0;
    let mut victory_choice = OPTION_FREEROAM;
    let mut pad_menu_rested = true;

    let menu_background = MENU_BACKGROUNDS.iter().find_map(|path| {
        let full = asset_path(path);
        window
            .load_texture(&thread, full.to_str().unwrap_or(path))
            .ok()
    });
    if menu_background.is_none() {
        eprintln!("aviso: sin fondo de menú (se buscó {MENU_BACKGROUNDS:?}); queda en negro");
    }

    if std::env::args().any(|arg| arg == "--screenshot") {
        let menu_shot = std::env::args().any(|arg| arg == "--menu");
        let victory_shot = std::env::args().any(|arg| arg == "--victory");
        let pad_name = gamepad::name(&window);
        framebuffer.clear();
        let zbuffer = render::render_world(
            &mut framebuffer,
            &level.maze,
            &player,
            &texture_manager,
            level.block_size,
        );
        sprites::render_enemies(
            &mut framebuffer,
            &player,
            &level.enemies,
            &texture_manager,
            &zbuffer,
            level.block_size,
        );
        if !menu_shot && !victory_shot {
            render::render_minimap(
                &mut framebuffer,
                &level.maze,
                &player,
                &level.enemies,
                level.block_size,
            );
        }
        framebuffer.swap_buffers(&mut window, &thread, |d| {
            if menu_shot {
                draw_menu(d, menu_background.as_ref(), 0.0, 0, pad_name.as_deref());
            } else if victory_shot {
                draw_victory(d, OPTION_FREEROAM, 42.7, pad_name.as_deref());
            } else {
                draw_hud(d, 0, false, true, pad_name.as_deref());
            }
        });
        window.take_screenshot(&thread, "screenshot.png");
        return;
    }

    let audio = RaylibAudio::init_audio_device().ok();
    if audio.is_none() {
        eprintln!("aviso: no se pudo abrir el audio; el juego corre en silencio");
    }
    let steps = audio
        .as_ref()
        .and_then(|device| load_loop(device, &STEPS_SOUNDS, STEPS_VOLUME, "pasos"));

    let music = audio
        .as_ref()
        .and_then(|device| load_loop(device, &MUSIC_FILES, MUSIC_VOLUME, "música"));
    if let Some(track) = &music {
        track.play_stream();
    }
    let mut music_on = true;
    let mut steps_started = false;
    let mut steps_playing = false;

    let mut screen = Screen::Menu;
    let mut mouse_look = true;

    while !window.window_should_close() {
        let dt = window.get_frame_time().min(MAX_FRAME_TIME);

        let mut walking = false;

        match screen {
            Screen::Menu => {
                let step = menu_step(&window, &mut pad_menu_rested);
                if step != 0 {
                    level_choice = wrap_choice(level_choice, step, LEVELS.len());
                }

                if confirm_pressed(&window) {
                    level = load_level(LEVELS[level_choice].1);
                    player = Player::new(level.spawn);
                    freeroam = false;
                    screen = Screen::Playing;
                    capture_cursor(&mut window);
                    mouse_look = true;
                    run_started = window.get_time();
                }
            }
            Screen::Playing => {
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

                let mouse_dx = read_mouse_look(&mut window, mouse_look);
                let intent =
                    player::keyboard_intent(&window, mouse_dx).merge(gamepad::intent(&window));
                let was_at = player.pos;
                player::apply_intent(&mut player, intent, &level.maze, level.block_size, dt);
                walking = player.pos.distance(was_at) > WALKING_EPSILON;

                let on_goal =
                    level.goal == Some(level.maze.cell_at_pixel(player.pos, level.block_size));
                if on_goal && !freeroam {
                    screen = Screen::Victory;
                    run_time = (window.get_time() - run_started) as f32;
                    victory_choice = OPTION_FREEROAM;
                    release_cursor(&mut window);
                }
            }
            Screen::Victory => {
                let step = menu_step(&window, &mut pad_menu_rested);
                if step != 0 {
                    victory_choice = wrap_choice(victory_choice, step, VICTORY_OPTIONS.len());
                }

                if confirm_pressed(&window) {
                    match victory_choice {
                        OPTION_RESTART => {
                            player = Player::new(level.spawn);
                            freeroam = false;
                            run_started = window.get_time();
                            screen = Screen::Playing;
                            capture_cursor(&mut window);
                            mouse_look = true;
                        }
                        OPTION_MENU => screen = Screen::Menu,
                        _ => {
                            freeroam = true;
                            screen = Screen::Playing;
                            capture_cursor(&mut window);
                            mouse_look = true;
                        }
                    }
                }
            }
        }

        if let Some(track) = &music {
            track.update_stream();
            if window.is_key_pressed(KeyboardKey::KEY_N) {
                music_on = !music_on;
                if music_on {
                    track.resume_stream();
                } else {
                    track.pause_stream();
                }
            }
        }

        if let Some(steps_clip) = &steps {
            steps_clip.update_stream();

            if walking && !steps_playing {
                if steps_started {
                    steps_clip.resume_stream();
                } else {
                    steps_clip.play_stream();
                    steps_started = true;
                }
                steps_playing = true;
            } else if !walking && steps_playing {
                steps_clip.pause_stream();
                steps_playing = false;
            }
        }

        framebuffer.clear();

        if screen != Screen::Menu {
            let zbuffer = render::render_world(
                &mut framebuffer,
                &level.maze,
                &player,
                &texture_manager,
                level.block_size,
            );
            sprites::render_enemies(
                &mut framebuffer,
                &player,
                &level.enemies,
                &texture_manager,
                &zbuffer,
                level.block_size,
            );
            if show_minimap && screen == Screen::Playing {
                render::render_minimap(
                    &mut framebuffer,
                    &level.maze,
                    &player,
                    &level.enemies,
                    level.block_size,
                );
            }
        }

        let fps = window.get_fps();
        let time = window.get_time() as f32;
        let pad = gamepad::name(&window);

        framebuffer.swap_buffers(&mut window, &thread, |d| match screen {
            Screen::Menu => draw_menu(
                d,
                menu_background.as_ref(),
                time,
                level_choice,
                pad.as_deref(),
            ),
            Screen::Playing => draw_hud(d, fps, freeroam, mouse_look, pad.as_deref()),
            Screen::Victory => draw_victory(d, victory_choice, run_time, pad.as_deref()),
        });
    }
}

fn draw_menu(
    d: &mut RaylibDrawHandle,
    background: Option<&Texture2D>,
    time: f32,
    choice: usize,
    pad: Option<&str>,
) {
    if let Some(texture) = background {
        draw_background_cover(d, texture);
    }

    d.draw_rectangle(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT, Color::new(0, 0, 0, 130));

    draw_centered_text(d, TITLE, 190, 40, Color::WHITESMOKE);

    let blink = (time * 3.0).sin() * 0.5 + 0.5;
    let alpha = (150.0 + 105.0 * blink) as u8;
    draw_centered_text(
        d,
        "elegi un nivel",
        255,
        20,
        Color::new(0xE8, 0xC0, 0x50, alpha),
    );

    let names: Vec<&str> = LEVELS.iter().map(|(name, _)| *name).collect();
    draw_option_list(d, &names, choice, 295, 40);

    let confirm = match pad {
        Some(_) => "flechas o cruceta para elegir  |  ENTER o (A) para jugar",
        None => "flechas para elegir  |  ENTER para jugar",
    };
    draw_centered_text(d, confirm, 435, 18, Color::LIGHTGRAY);

    let controls = match pad {
        Some(_) => "WASD + mouse  o  sticks del mando  |  TAB cursor  |  M minimapa  |  N musica",
        None => "WASD + mouse  |  TAB cursor  |  M minimapa  |  N musica",
    };
    draw_centered_text(d, controls, 462, 16, Color::GRAY);
    draw_centered_text(d, "ESC para salir", 486, 16, Color::GRAY);

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

fn menu_step(window: &RaylibHandle, pad_rested: &mut bool) -> i32 {
    let mut step = 0;
    if window.is_key_pressed(KeyboardKey::KEY_UP) || window.is_key_pressed(KeyboardKey::KEY_W) {
        step -= 1;
    }
    if window.is_key_pressed(KeyboardKey::KEY_DOWN) || window.is_key_pressed(KeyboardKey::KEY_S) {
        step += 1;
    }

    let pad_axis = gamepad::menu_axis(window);
    if pad_axis != 0 && *pad_rested {
        step += pad_axis;
    }
    *pad_rested = pad_axis == 0;

    step
}

fn confirm_pressed(window: &RaylibHandle) -> bool {
    window.is_key_pressed(KeyboardKey::KEY_ENTER)
        || window.is_key_pressed(KeyboardKey::KEY_SPACE)
        || gamepad::confirm_pressed(window)
}

fn wrap_choice(choice: usize, step: i32, count: usize) -> usize {
    (choice as i32 + step).rem_euclid(count as i32) as usize
}

fn draw_option_list(
    d: &mut RaylibDrawHandle,
    options: &[&str],
    choice: usize,
    first_y: i32,
    spacing: i32,
) {
    for (i, option) in options.iter().enumerate() {
        let selected = i == choice;
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
        draw_centered_text(
            d,
            &label,
            first_y + i as i32 * spacing,
            if selected { 28 } else { 24 },
            color,
        );
    }
}

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

    draw_option_list(d, &VICTORY_OPTIONS, choice, 285, 42);

    let hint = match pad {
        Some(_) => "flechas o cruceta para elegir  |  ENTER o (A) para confirmar",
        None => "flechas para elegir  |  ENTER para confirmar",
    };
    draw_centered_text(d, hint, 440, 18, Color::LIGHTGRAY);
    draw_centered_text(d, "ESC para salir", 470, 18, Color::GRAY);
}

fn draw_background_cover(d: &mut RaylibDrawHandle, texture: &Texture2D) {
    let (tw, th) = (texture.width as f32, texture.height as f32);
    let window_ratio = WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32;
    let texture_ratio = tw / th;

    let (src_w, src_h) = if texture_ratio > window_ratio {
        (th * window_ratio, th)
    } else {
        (tw, tw / window_ratio)
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

fn draw_centered_text(d: &mut RaylibDrawHandle, text: &str, y: i32, size: i32, color: Color) {
    let x = (WINDOW_WIDTH - d.measure_text(text, size)) / 2;
    d.draw_text(text, x + 2, y + 2, size, Color::new(0, 0, 0, 180));
    d.draw_text(text, x, y, size, color);
}

fn load_loop<'a>(
    device: &'a RaylibAudio,
    candidates: &[&str],
    volume: f32,
    what: &str,
) -> Option<Music<'a>> {
    for path in candidates {
        let full = asset_path(path);
        if !full.exists() {
            continue;
        }
        match device.new_music(full.to_str().unwrap_or(path)) {
            Ok(mut clip) => {
                clip.set_looping(true);
                clip.set_volume(volume);
                return Some(clip);
            }
            Err(e) => eprintln!("aviso: no se pudo cargar '{path}' ({e})"),
        }
    }
    eprintln!("aviso: sin {what} (se buscó {candidates:?})");
    None
}

fn window_center() -> Vector2 {
    Vector2::new(WINDOW_WIDTH as f32 / 2.0, WINDOW_HEIGHT as f32 / 2.0)
}

fn capture_cursor(window: &mut RaylibHandle) {
    window.hide_cursor();
    window.set_mouse_position(window_center());
}

fn release_cursor(window: &mut RaylibHandle) {
    window.enable_cursor();
}

fn read_mouse_look(window: &mut RaylibHandle, mouse_look: bool) -> f32 {
    if !mouse_look || !window.is_window_focused() {
        return 0.0;
    }

    let center = window_center();
    let dx = window.get_mouse_position().x - center.x;
    window.set_mouse_position(center);
    dx
}

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
        &format!("{input}  |  TAB: cursor  |  M: minimapa  |  N: musica  |  {fps} FPS"),
        10,
        10,
        18,
        Color::WHITESMOKE,
    );

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

const CROSSHAIR_LENGTH: i32 = 8;
const CROSSHAIR_GAP: i32 = 5;
const CROSSHAIR_THICKNESS: i32 = 2;

fn draw_crosshair(d: &mut RaylibDrawHandle) {
    let center_x = WINDOW_WIDTH / 2;
    let center_y = WINDOW_HEIGHT / 2;
    let thickness = CROSSHAIR_THICKNESS;
    let half = thickness / 2;

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
