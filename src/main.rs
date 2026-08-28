mod caster;
mod combat;
mod framebuffer;
mod gamepad;
mod maze;
mod player;
mod render;
mod sprites;
mod textures;

use raylib::prelude::*;

use combat::{Combat, ShotResult};
use framebuffer::Framebuffer;
use maze::{ENEMY, GOAL, Maze, SPAWN};
use player::Player;
use sprites::{DrawItem, Enemy};
use textures::{TextureManager, asset_path};

const WINDOW_WIDTH: i32 = 1280;
const WINDOW_HEIGHT: i32 = 720;

const MAX_FRAME_TIME: f32 = 0.1;

const TITLE: &str = "BOB ESPONJA CON PISTOLA";

const MENU_BACKGROUNDS: [&str; 2] = ["assets/menu.png", "assets/pared2.png"];

const STEPS_SOUNDS: [&str; 2] = ["assets/pasos.ogg", "assets/caminar.mp3"];
const STEPS_VOLUME: f32 = 0.7;

const MUSIC_FILES: [&str; 3] = ["assets/musica.ogg", "assets/musica.mp3", "assets/music.ogg"];
const MUSIC_VOLUME: f32 = 0.35;

const WALKING_EPSILON: f32 = 0.05;

/// Efectos one-shot. Cada uno con su lista de candidatos: si no está el archivo,
/// esa acción simplemente no suena.
const SHOT_SOUNDS: [&str; 2] = ["assets/disparo.ogg", "assets/disparo.wav"];
const DEATH_SOUNDS: [&str; 2] = ["assets/muerte.ogg", "assets/muerte.wav"];
const IMPACT_SOUNDS: [&str; 2] = ["assets/impacto.ogg", "assets/impacto.wav"];
const RELOAD_SOUNDS: [&str; 2] = ["assets/recarga.ogg", "assets/recarga.wav"];
const HURT_SOUNDS: [&str; 2] = ["assets/dolor.ogg", "assets/dolor.wav"];
/// El clic seco del gatillo con el cargador vacío.
const EMPTY_SOUNDS: [&str; 2] = ["assets/vacio.ogg", "assets/vacio.wav"];

/// Zumbido de los enemigos. **Suena siempre en loop** y su volumen sube al
/// acercarse: es la pista de que hay uno cerca antes de verlo.
const ENEMY_AMBIENT: [&str; 3] = [
    "assets/enemigo.ogg",
    "assets/enemigo.mp3",
    "assets/enemigo.wav",
];
const ENEMY_AMBIENT_VOLUME: f32 = 0.9;
/// Desde cuántas celdas se empieza a oír al enemigo más cercano.
const ENEMY_HEAR_RANGE: f32 = 7.0;

/// Arma en pantalla. El segundo es el frame del fogonazo y es opcional.
const WEAPON_IDLE: [&str; 2] = ["assets/arma.png", "assets/pistola.png"];
const WEAPON_FIRE: [&str; 2] = ["assets/arma_disparo.png", "assets/pistola_disparo.png"];

/// Char con el que se dibuja la salida como sprite.
const EXIT_KIND: char = GOAL;

#[derive(PartialEq)]
enum Screen {
    Menu,
    Playing,
    Victory,
    GameOver,
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

/// Qué se ofrece al morir. Sin "seguir explorando", claro.
const GAMEOVER_OPTIONS: [&str; 2] = ["Reintentar", "Elegir otro nivel"];
const OVER_RETRY: usize = 0;

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
        .map(|(i, j)| Enemy::new(maze.cell_center(i, j, block_size), ENEMY))
        .collect();

    Level {
        maze,
        block_size,
        spawn,
        goal,
        enemies,
    }
}

/// Arma la lista de billboards del frame: los enemigos vivos, con parpadeo si
/// acaban de recibir un tiro, y la salida.
///
/// Matar a los enemigos es opcional, así que la salida se dibuja tal cual, sin
/// nada que señale si está abierta.
fn build_sprites(level: &Level) -> Vec<DrawItem> {
    let mut items: Vec<DrawItem> = level
        .enemies
        .iter()
        .filter(|e| e.alive())
        .map(|e| DrawItem {
            pos: e.pos,
            kind: e.kind,
            tint: (e.hit_flash > 0.0).then_some(Color::WHITE),
        })
        .collect();

    if let Some((gi, gj)) = level.goal {
        items.push(DrawItem {
            pos: level.maze.cell_center(gi, gj, level.block_size),
            kind: EXIT_KIND,
            // Sin tinte: la salida tiene su propia imagen y ya no hay estado
            // "bloqueada" que señalar.
            tint: None,
        });
    }

    items
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
        .title(TITLE)
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

    let weapon_idle = load_first_texture(&mut window, &thread, &WEAPON_IDLE);
    let weapon_fire = load_first_texture(&mut window, &thread, &WEAPON_FIRE);
    if weapon_idle.is_none() {
        eprintln!("aviso: sin arma en pantalla (se buscó {WEAPON_IDLE:?})");
    }

    if std::env::args().any(|arg| arg == "--screenshot") {
        let menu_shot = std::env::args().any(|arg| arg == "--menu");
        let victory_shot = std::env::args().any(|arg| arg == "--victory");
        let pad_name = gamepad::name(&window);
        let shot_remaining = combat::remaining(&level.enemies);
        framebuffer.clear();
        let zbuffer = render::render_world(
            &mut framebuffer,
            &level.maze,
            &player,
            &texture_manager,
            level.block_size,
        );
        let items = build_sprites(&level);
        sprites::render_sprites(
            &mut framebuffer,
            &player,
            &items,
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
                draw_weapon(
                    d,
                    weapon_idle.as_ref(),
                    weapon_fire.as_ref(),
                    &Combat::default(),
                    false,
                    0.0,
                );
                draw_hud(
                    d,
                    0,
                    false,
                    true,
                    pad_name.as_deref(),
                    &Combat::default(),
                    shot_remaining,
                );
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

    let shot_sound = audio
        .as_ref()
        .and_then(|d| load_sound(d, &SHOT_SOUNDS, "disparo"));
    let death_sound = audio
        .as_ref()
        .and_then(|d| load_sound(d, &DEATH_SOUNDS, "muerte"));
    let impact_sound = audio
        .as_ref()
        .and_then(|d| load_sound(d, &IMPACT_SOUNDS, "impacto"));
    let reload_sound = audio
        .as_ref()
        .and_then(|d| load_sound(d, &RELOAD_SOUNDS, "recarga"));
    let hurt_sound = audio
        .as_ref()
        .and_then(|d| load_sound(d, &HURT_SOUNDS, "dolor"));
    let empty_sound = audio
        .as_ref()
        .and_then(|d| load_sound(d, &EMPTY_SOUNDS, "gatillo vacío"));

    // El zumbido arranca junto con el juego y no para nunca: lo que cambia es su
    // volumen, no si está sonando.
    let enemy_ambient = audio
        .as_ref()
        .and_then(|device| load_loop(device, &ENEMY_AMBIENT, 0.0, "zumbido de enemigos"));
    if let Some(track) = &enemy_ambient {
        track.play_stream();
    }

    let mut combat = Combat::default();
    let mut gameover_choice = OVER_RETRY;
    // El z-buffer del último frame dibujado, que es contra lo que se resuelve el
    // disparo: el tiro sale en el mismo instante en que el jugador ve la escena.
    let mut last_zbuffer: Vec<f32> = vec![f32::INFINITY; WINDOW_WIDTH as usize];

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
                    combat = Combat::default();
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

                combat.tick(dt);

                if intent.reload && combat.start_reload() {
                    play(&reload_sound);
                }

                if intent.shoot {
                    // El z-buffer del frame anterior: la geometría no cambió lo
                    // suficiente en 16 ms como para que importe, y evita tener que
                    // rayescanear de nuevo solo para disparar.
                    let aspect = texture_manager
                        .size(ENEMY)
                        .map_or(1.0, |(w, h)| w as f32 / h as f32);
                    match combat::shoot(
                        &mut combat,
                        &player,
                        &mut level.enemies,
                        &last_zbuffer,
                        Vector2::new(WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32),
                        level.block_size,
                        aspect,
                    ) {
                        // Sin balas el gatillo hace clic: sin eso, disparar en
                        // seco no se distingue de un juego colgado.
                        ShotResult::Blocked => {
                            if combat.ammo == 0 && !combat.reloading() {
                                play(&empty_sound);
                            }
                        }
                        ShotResult::Missed => play(&shot_sound),
                        ShotResult::Hit { killed } => {
                            play(&shot_sound);
                            if killed {
                                play(&death_sound);
                            } else {
                                play(&impact_sound);
                            }
                        }
                    }
                }

                if combat::update_enemies(
                    &mut level.enemies,
                    &player,
                    &mut combat,
                    level.block_size,
                    dt,
                ) {
                    play(&hurt_sound);
                }

                if !combat.alive() {
                    screen = Screen::GameOver;
                    gameover_choice = OVER_RETRY;
                    release_cursor(&mut window);
                }

                let on_goal =
                    level.goal == Some(level.maze.cell_at_pixel(player.pos, level.block_size));
                if on_goal && !freeroam {
                    screen = Screen::Victory;
                    run_time = (window.get_time() - run_started) as f32;
                    victory_choice = OPTION_FREEROAM;
                    release_cursor(&mut window);
                }
            }
            Screen::GameOver => {
                let step = menu_step(&window, &mut pad_menu_rested);
                if step != 0 {
                    gameover_choice = wrap_choice(gameover_choice, step, GAMEOVER_OPTIONS.len());
                }

                if confirm_pressed(&window) {
                    if gameover_choice == OVER_RETRY {
                        level = load_level(LEVELS[level_choice].1);
                        player = Player::new(level.spawn);
                        combat = Combat::default();
                        freeroam = false;
                        run_started = window.get_time();
                        screen = Screen::Playing;
                        capture_cursor(&mut window);
                        mouse_look = true;
                    } else {
                        screen = Screen::Menu;
                    }
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
                            level = load_level(LEVELS[level_choice].1);
                            player = Player::new(level.spawn);
                            combat = Combat::default();
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

        // El zumbido sigue al enemigo vivo más cercano, y se calla fuera del
        // juego: en los menús no hay a quién temerle.
        if let Some(track) = &enemy_ambient {
            track.update_stream();
            let cercania = if screen == Screen::Playing {
                combat::proximity_volume(
                    combat::nearest_alive(&level.enemies, player.pos),
                    ENEMY_HEAR_RANGE * level.block_size as f32,
                )
            } else {
                0.0
            };
            track.set_volume(cercania * ENEMY_AMBIENT_VOLUME);
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
            let items = build_sprites(&level);
            sprites::render_sprites(
                &mut framebuffer,
                &player,
                &items,
                &texture_manager,
                &zbuffer,
                level.block_size,
            );
            last_zbuffer = zbuffer;
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
        let remaining = combat::remaining(&level.enemies);

        framebuffer.swap_buffers(&mut window, &thread, |d| match screen {
            Screen::Menu => draw_menu(
                d,
                menu_background.as_ref(),
                time,
                level_choice,
                pad.as_deref(),
            ),
            Screen::Playing => {
                draw_weapon(
                    d,
                    weapon_idle.as_ref(),
                    weapon_fire.as_ref(),
                    &combat,
                    walking,
                    time,
                );
                draw_hud(
                    d,
                    fps,
                    freeroam,
                    mouse_look,
                    pad.as_deref(),
                    &combat,
                    remaining,
                );
            }
            Screen::Victory => draw_victory(d, victory_choice, run_time, pad.as_deref()),
            Screen::GameOver => draw_gameover(d, gameover_choice, pad.as_deref()),
        });
    }
}

/// Una posición vertical dada como fracción del alto de la ventana.
///
/// Las pantallas se colocan así y no en píxeles fijos: los valores estaban
/// calculados a ojo para 600 px de alto y se apelotonaban arriba al agrandar la
/// ventana. En fracciones, el layout sobrevive a cualquier resolución.
fn y_frac(fraction: f32) -> i32 {
    (WINDOW_HEIGHT as f32 * fraction) as i32
}

/// Tamaño de fuente proporcional al alto de la ventana, para que el texto no
/// encoja en pantallas grandes.
fn font(size: f32) -> i32 {
    ((WINDOW_HEIGHT as f32 / 600.0) * size) as i32
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

    draw_centered_text(d, TITLE, y_frac(0.32), font(40.0), Color::WHITESMOKE);

    let blink = (time * 3.0).sin() * 0.5 + 0.5;
    let alpha = (150.0 + 105.0 * blink) as u8;
    draw_centered_text(
        d,
        "elige un nivel",
        y_frac(0.42),
        font(20.0),
        Color::new(0xE8, 0xC0, 0x50, alpha),
    );

    let names: Vec<&str> = LEVELS.iter().map(|(name, _)| *name).collect();
    draw_option_list(d, &names, choice, y_frac(0.49), font(40.0));

    let confirm = match pad {
        Some(_) => "flechas o cruceta para elegir  |  ENTER o (A) para jugar",
        None => "flechas para elegir  |  ENTER para jugar",
    };
    draw_centered_text(d, confirm, y_frac(0.72), font(18.0), Color::LIGHTGRAY);

    let controls = match pad {
        Some(_) => "WASD + mouse  o  sticks del mando  |  TAB cursor  |  M minimapa  |  N musica",
        None => "WASD + mouse  |  TAB cursor  |  M minimapa  |  N musica",
    };
    draw_centered_text(d, controls, y_frac(0.77), font(16.0), Color::GRAY);
    draw_centered_text(d, "ESC para salir", y_frac(0.81), font(16.0), Color::GRAY);

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

    draw_centered_text(
        d,
        "GANASTE!",
        y_frac(0.23),
        font(56.0),
        Color::new(0x5E, 0xD9, 0x8A, 255),
    );
    draw_centered_text(
        d,
        &format!("llegaste a la meta en {run_time:.1} s"),
        215,
        20,
        Color::LIGHTGRAY,
    );

    draw_option_list(d, &VICTORY_OPTIONS, choice, y_frac(0.47), font(42.0));

    let hint = match pad {
        Some(_) => "flechas o cruceta para elegir  |  ENTER o (A) para confirmar",
        None => "flechas para elegir  |  ENTER para confirmar",
    };
    draw_centered_text(d, hint, y_frac(0.73), font(18.0), Color::LIGHTGRAY);
    draw_centered_text(d, "ESC para salir", y_frac(0.78), font(18.0), Color::GRAY);
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

/// Carga un efecto corto: `Sound` y no `Music` porque estos se disparan y se
/// olvidan, y varios pueden solaparse.
/// Primera textura de GPU que exista de la lista. Mismo criterio de candidatos
/// que las texturas del mundo y el fondo del menú.
fn load_first_texture(
    window: &mut RaylibHandle,
    thread: &RaylibThread,
    candidates: &[&str],
) -> Option<Texture2D> {
    candidates.iter().find_map(|path| {
        let full = asset_path(path);
        full.exists()
            .then(|| {
                window
                    .load_texture(thread, full.to_str().unwrap_or(path))
                    .ok()
            })
            .flatten()
    })
}

fn load_sound<'a>(device: &'a RaylibAudio, candidates: &[&str], what: &str) -> Option<Sound<'a>> {
    for path in candidates {
        let full = asset_path(path);
        if !full.exists() {
            continue;
        }
        match device.new_sound(full.to_str().unwrap_or(path)) {
            Ok(sound) => return Some(sound),
            Err(e) => eprintln!("aviso: no se pudo cargar '{path}' ({e})"),
        }
    }
    eprintln!("aviso: sin sonido de {what} (se buscó {candidates:?})");
    None
}

/// Reproduce si el efecto existe. Sin archivo, silencio y a otra cosa.
fn play(sound: &Option<Sound<'_>>) {
    if let Some(s) = sound {
        s.play();
    }
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

/// Barra de vida, munición y enemigos restantes, abajo a la izquierda.
fn draw_status(d: &mut RaylibDrawHandle, combat: &Combat, remaining: usize) {
    let margin = 16;
    let bar_w = 220;
    let bar_h = font(16.0);
    let y = WINDOW_HEIGHT - margin - bar_h;

    let ratio = combat.health as f32 / combat::MAX_HEALTH as f32;
    d.draw_rectangle(
        margin - 2,
        y - 2,
        bar_w + 4,
        bar_h + 4,
        Color::new(0, 0, 0, 160),
    );
    d.draw_rectangle(margin, y, bar_w, bar_h, Color::new(0x3A, 0x1E, 0x1E, 255));
    d.draw_rectangle(
        margin,
        y,
        (bar_w as f32 * ratio) as i32,
        bar_h,
        // El color acompaña la urgencia: verde con vida, rojo cuando queda poca.
        if ratio > 0.5 {
            Color::new(0x5E, 0xD9, 0x8A, 255)
        } else if ratio > 0.25 {
            Color::new(0xE8, 0xC0, 0x50, 255)
        } else {
            Color::new(0xD9, 0x4A, 0x4A, 255)
        },
    );
    d.draw_text(
        &format!("{} HP", combat.health),
        margin + 6,
        y,
        bar_h,
        Color::WHITESMOKE,
    );

    let ammo = if combat.reloading() {
        "recargando...".to_string()
    } else {
        format!("{} / {}", combat.ammo, combat::MAG_SIZE)
    };
    let ammo_y = y - font(30.0);
    d.draw_text(&ammo, margin, ammo_y, font(26.0), Color::WHITESMOKE);

    // Contador informativo: matarlos suma, pero no hace falta para salir.
    let (texto, color) = if remaining == 0 {
        (
            "nivel limpio".to_string(),
            Color::new(0x5E, 0xD9, 0x8A, 255),
        )
    } else {
        (
            format!("enemigos: {remaining}"),
            Color::new(0xE8, 0xC0, 0x50, 255),
        )
    };
    d.draw_text(&texto, margin, ammo_y - font(24.0), font(20.0), color);
}

/// El arma en primera persona.
///
/// Va dentro del closure del HUD, o sea en coordenadas de ventana y encima del
/// framebuffer, igual que el crosshair.
fn draw_weapon(
    d: &mut RaylibDrawHandle,
    idle: Option<&Texture2D>,
    fire: Option<&Texture2D>,
    combat: &Combat,
    walking: bool,
    time: f32,
) {
    // Mientras dura el fogonazo se muestra el otro frame, si existe.
    let firing = combat.shot_timer > combat::FIRE_COOLDOWN * 0.6;
    let Some(texture) = (if firing { fire.or(idle) } else { idle }) else {
        return;
    };

    // El sprite es apaisado, así que la escala se mide contra el alto: al 45%
    // ocupaba media pantalla y el cañón tapaba el crosshair.
    let scale = WINDOW_HEIGHT as f32 * 0.34 / texture.height as f32;
    let w = texture.width as f32 * scale;
    let h = texture.height as f32 * scale;

    // Bobbing al caminar: un arma perfectamente quieta parece una calcomanía
    // pegada a la pantalla.
    let bob = if walking {
        (time * 9.0).sin() * 6.0
    } else {
        0.0
    };
    // Retroceso: salta y vuelve durante la cadencia del disparo.
    let recoil = (combat.shot_timer / combat::FIRE_COOLDOWN) * 18.0;

    d.draw_texture_pro(
        texture,
        Rectangle::new(0.0, 0.0, texture.width as f32, texture.height as f32),
        Rectangle::new(
            // Corrida a la derecha y hundida abajo: el cañón apunta arriba a la
            // izquierda, así que así queda apuntando al centro sin taparlo.
            WINDOW_WIDTH as f32 * 0.66 - w / 2.0,
            WINDOW_HEIGHT as f32 - h * 0.86 + recoil + bob,
            w,
            h,
        ),
        Vector2::zero(),
        0.0,
        Color::WHITE,
    );
}

/// La pantalla de derrota, sobre el último frame del nivel.
fn draw_gameover(d: &mut RaylibDrawHandle, choice: usize, pad: Option<&str>) {
    d.draw_rectangle(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT, Color::new(40, 0, 0, 170));
    draw_centered_text(
        d,
        "TE MATARON",
        y_frac(0.25),
        font(56.0),
        Color::new(0xD9, 0x4A, 0x4A, 255),
    );
    draw_option_list(d, &GAMEOVER_OPTIONS, choice, y_frac(0.48), font(42.0));

    let hint = match pad {
        Some(_) => "flechas o cruceta para elegir  |  ENTER o (A) para confirmar",
        None => "flechas para elegir  |  ENTER para confirmar",
    };
    draw_centered_text(d, hint, y_frac(0.72), font(18.0), Color::LIGHTGRAY);
}

fn draw_hud(
    d: &mut RaylibDrawHandle,
    fps: u32,
    freeroam: bool,
    mouse_look: bool,
    pad: Option<&str>,
    combat: &Combat,
    remaining: usize,
) {
    // Destello rojo al recibir daño: sin esto, perder vida no se siente, solo se
    // lee en un número.
    if combat.hurt_flash > 0.0 {
        let alpha = (combat.hurt_flash / 0.35 * 120.0) as u8;
        d.draw_rectangle(
            0,
            0,
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            Color::new(200, 0, 0, alpha),
        );
    }

    draw_status(d, combat, remaining);

    let input = match pad {
        Some(_) => "mando + WASD",
        None => "WASD + mouse",
    };
    d.draw_text(
        &format!("{input}  |  TAB: cursor  |  M: minimapa  |  N: musica  |  {fps} FPS"),
        10,
        10,
        font(18.0),
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
