use raylib::prelude::*;

use crate::player::Player;
use crate::sprites::{Enemy, project};

/// Balas por cargador.
pub const MAG_SIZE: i32 = 8;
/// Cuánto tarda una recarga, en segundos.
pub const RELOAD_TIME: f32 = 1.0;
/// Tiempo mínimo entre dos disparos.
pub const FIRE_COOLDOWN: f32 = 0.35;
/// Vida con la que arranca el jugador.
pub const MAX_HEALTH: i32 = 100;

/// Tocar a un enemigo mata de una. Son estatuas venenosas: no persiguen, pero
/// encimárseles se paga caro, que es lo que les devuelve el peligro ahora que no
/// hay que matarlos a todos para salir.
const CONTACT_DAMAGE: i32 = MAX_HEALTH;
const ATTACK_COOLDOWN: f32 = 0.8;
/// A qué distancia (en fracción de celda) el enemigo alcanza al jugador.
const REACH: f32 = 0.6;

/// Cuánto dura el destello rojo al recibir daño.
const HURT_FLASH_TIME: f32 = 0.35;

/// Munición, vida y temporizadores del jugador.
pub struct Combat {
    pub health: i32,
    pub ammo: i32,
    pub reload_timer: f32,
    pub shot_timer: f32,
    pub hurt_flash: f32,
    pub attack_timer: f32,
}

impl Default for Combat {
    fn default() -> Self {
        Combat {
            health: MAX_HEALTH,
            ammo: MAG_SIZE,
            reload_timer: 0.0,
            shot_timer: 0.0,
            hurt_flash: 0.0,
            attack_timer: 0.0,
        }
    }
}

impl Combat {
    pub fn alive(&self) -> bool {
        self.health > 0
    }

    pub fn reloading(&self) -> bool {
        self.reload_timer > 0.0
    }

    /// Se puede disparar si hay balas, pasó la cadencia y no está recargando.
    pub fn can_shoot(&self) -> bool {
        self.alive() && self.ammo > 0 && self.shot_timer <= 0.0 && !self.reloading()
    }

    /// Gasta una bala. Devuelve `false` si no se podía disparar.
    pub fn spend_round(&mut self) -> bool {
        if !self.can_shoot() {
            return false;
        }
        self.ammo -= 1;
        self.shot_timer = FIRE_COOLDOWN;
        true
    }

    /// Empieza una recarga. No hace nada si el cargador ya está lleno.
    pub fn start_reload(&mut self) -> bool {
        if self.reloading() || self.ammo >= MAG_SIZE || !self.alive() {
            return false;
        }
        self.reload_timer = RELOAD_TIME;
        true
    }

    pub fn take_damage(&mut self, amount: i32) {
        self.health = (self.health - amount).max(0);
        self.hurt_flash = HURT_FLASH_TIME;
    }

    /// Corre los temporizadores y completa la recarga cuando toca.
    pub fn tick(&mut self, dt: f32) {
        self.shot_timer = (self.shot_timer - dt).max(0.0);
        self.hurt_flash = (self.hurt_flash - dt).max(0.0);
        self.attack_timer = (self.attack_timer - dt).max(0.0);

        if self.reloading() {
            self.reload_timer -= dt;
            if self.reload_timer <= 0.0 {
                self.reload_timer = 0.0;
                self.ammo = MAG_SIZE;
            }
        }
    }
}

/// Qué pasó al apretar el gatillo, para que el llamador decida qué sonido suena.
pub enum ShotResult {
    /// No se pudo disparar (sin balas, recargando o cadencia).
    Blocked,
    /// Salió el tiro pero no le dio a nadie.
    Missed,
    Hit {
        killed: bool,
    },
}

/// Dispara al centro de la pantalla.
///
/// Le pega al enemigo más cercano cuyo sprite cubre la columna central, siempre
/// que no haya pared en medio: `zbuffer` guarda la distancia a la pared en cada
/// columna, así que compararla con la del enemigo es toda la prueba que hace
/// falta. Se usa la misma proyección que el dibujo, así que la puntería coincide
/// exactamente con lo que se ve.
pub fn shoot(
    combat: &mut Combat,
    player: &Player,
    enemies: &mut [Enemy],
    zbuffer: &[f32],
    screen: Vector2,
    block_size: usize,
    aspect: f32,
) -> ShotResult {
    let (screen_width, screen_height) = (screen.x, screen.y);
    if !combat.spend_round() {
        return ShotResult::Blocked;
    }

    let center = screen_width / 2.0;
    let wall_distance = zbuffer
        .get(center as usize)
        .copied()
        .unwrap_or(f32::INFINITY);

    let mut best: Option<(usize, f32)> = None;
    for (idx, enemy) in enemies.iter().enumerate() {
        if !enemy.alive() {
            continue;
        }
        let Some(p) = project(
            player,
            enemy.pos,
            screen_width,
            screen_height,
            block_size,
            aspect,
        ) else {
            continue;
        };
        if !p.covers_column(center) || p.distance > wall_distance {
            continue;
        }
        if best.is_none_or(|(_, d)| p.distance < d) {
            best = Some((idx, p.distance));
        }
    }

    match best {
        Some((idx, _)) => ShotResult::Hit {
            killed: enemies[idx].hit(),
        },
        None => ShotResult::Missed,
    }
}

/// Corre los temporizadores de los enemigos y aplica el daño por contacto.
///
/// Los enemigos **no se mueven**: son obstáculos fijos que lastiman si el jugador
/// se les encima. Sin movimiento no hace falta línea de vista tampoco — estar
/// tocando a uno ya significa que no hay pared en medio.
///
/// Devuelve `true` si el jugador recibió daño este frame.
pub fn update_enemies(
    enemies: &mut [Enemy],
    player: &Player,
    combat: &mut Combat,
    block_size: usize,
    dt: f32,
) -> bool {
    let reach = REACH * block_size as f32;
    let mut hurt = false;

    for enemy in enemies.iter_mut() {
        enemy.hit_flash = (enemy.hit_flash - dt).max(0.0);

        if !enemy.alive() || !combat.alive() {
            continue;
        }

        let distance = (player.pos.x - enemy.pos.x).hypot(player.pos.y - enemy.pos.y);
        if distance <= reach && combat.attack_timer <= 0.0 {
            combat.take_damage(CONTACT_DAMAGE);
            combat.attack_timer = ATTACK_COOLDOWN;
            hurt = true;
        }
    }

    hurt
}

/// Cuántos enemigos quedan vivos.
pub fn remaining(enemies: &[Enemy]) -> usize {
    enemies.iter().filter(|e| e.alive()).count()
}

/// Distancia al enemigo vivo más cercano, o `None` si no queda ninguno.
pub fn nearest_alive(enemies: &[Enemy], from: Vector2) -> Option<f32> {
    enemies
        .iter()
        .filter(|e| e.alive())
        .map(|e| (e.pos.x - from.x).hypot(e.pos.y - from.y))
        .min_by(f32::total_cmp)
}

/// Qué tan fuerte se escucha el zumbido de los enemigos, de 0 a 1.
///
/// Cae con el cuadrado de la distancia y no de forma lineal: el oído percibe el
/// volumen de manera logarítmica, así que una rampa lineal se siente como si el
/// sonido apareciera de golpe cerca del final.
pub fn proximity_volume(distance: Option<f32>, range: f32) -> f32 {
    match distance {
        None => 0.0,
        Some(d) => {
            let cerca = (1.0 - d / range).clamp(0.0, 1.0);
            cerca * cerca
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_cargador_vacio_no_dispara() {
        let mut c = Combat::default();
        for _ in 0..MAG_SIZE {
            assert!(c.spend_round());
            c.shot_timer = 0.0; // saltear la cadencia
        }
        assert_eq!(c.ammo, 0);
        assert!(!c.can_shoot());
        assert!(!c.spend_round());
    }

    #[test]
    fn la_cadencia_bloquea_el_segundo_tiro() {
        let mut c = Combat::default();
        assert!(c.spend_round());
        assert!(!c.can_shoot(), "no puede disparar de nuevo al instante");
        c.tick(FIRE_COOLDOWN);
        assert!(c.can_shoot());
    }

    #[test]
    fn recargar_llena_el_cargador_pero_tarda() {
        let mut c = Combat::default();
        c.ammo = 2;
        assert!(c.start_reload());
        assert!(c.reloading());
        assert!(!c.can_shoot(), "no se dispara mientras recarga");
        c.tick(RELOAD_TIME);
        assert_eq!(c.ammo, MAG_SIZE);
        assert!(!c.reloading());
    }

    #[test]
    fn no_se_recarga_con_el_cargador_lleno() {
        let mut c = Combat::default();
        assert!(!c.start_reload());
    }

    #[test]
    fn la_vida_no_baja_de_cero() {
        let mut c = Combat::default();
        c.take_damage(MAX_HEALTH * 2);
        assert_eq!(c.health, 0);
        assert!(!c.alive());
    }

    #[test]
    fn un_muerto_no_dispara() {
        let mut c = Combat::default();
        c.take_damage(MAX_HEALTH);
        assert!(!c.can_shoot());
    }

    fn escenario(distancia: f32) -> (Vec<Enemy>, Player, Combat) {
        let enemigo = Enemy::new(Vector2::new(100.0, 100.0), 'e');
        let jugador = Player::new(Vector2::new(100.0 + distancia, 100.0));
        (vec![enemigo], jugador, Combat::default())
    }

    #[test]
    fn tocar_a_un_enemigo_hace_dano() {
        let bs = 20;
        let (mut enemies, player, mut combat) = escenario(REACH * bs as f32 * 0.5);
        assert!(update_enemies(
            &mut enemies,
            &player,
            &mut combat,
            bs,
            0.016
        ));
        assert_eq!(combat.health, MAX_HEALTH - CONTACT_DAMAGE);
    }

    #[test]
    fn de_lejos_no_hace_dano() {
        let bs = 20;
        let (mut enemies, player, mut combat) = escenario(bs as f32 * 3.0);
        assert!(!update_enemies(
            &mut enemies,
            &player,
            &mut combat,
            bs,
            0.016
        ));
        assert_eq!(combat.health, MAX_HEALTH);
    }

    #[test]
    fn un_jugador_muerto_no_sigue_recibiendo_golpes() {
        let bs = 20;
        let (mut enemies, player, mut combat) = escenario(REACH * bs as f32 * 0.5);
        update_enemies(&mut enemies, &player, &mut combat, bs, 0.016);
        assert!(!combat.alive());
        // Ya muerto, el enemigo deja de contar como golpe nuevo.
        assert!(!update_enemies(
            &mut enemies,
            &player,
            &mut combat,
            bs,
            0.016
        ));
    }

    #[test]
    fn un_enemigo_muerto_no_lastima() {
        let bs = 20;
        let (mut enemies, player, mut combat) = escenario(REACH * bs as f32 * 0.5);
        for _ in 0..3 {
            enemies[0].hit();
        }
        assert!(!update_enemies(
            &mut enemies,
            &player,
            &mut combat,
            bs,
            0.016
        ));
        assert_eq!(combat.health, MAX_HEALTH);
    }

    #[test]
    fn el_zumbido_sube_al_acercarse() {
        let rango = 300.0;
        let lejos = proximity_volume(Some(280.0), rango);
        let medio = proximity_volume(Some(150.0), rango);
        let encima = proximity_volume(Some(5.0), rango);
        assert!(lejos < medio && medio < encima, "{lejos} {medio} {encima}");
        assert!(encima <= 1.0);
    }

    #[test]
    fn fuera_de_rango_y_sin_enemigos_no_suena() {
        assert_eq!(proximity_volume(Some(400.0), 300.0), 0.0);
        assert_eq!(proximity_volume(None, 300.0), 0.0);
    }

    #[test]
    fn el_zumbido_sigue_al_enemigo_mas_cercano() {
        let enemies = vec![
            Enemy::new(Vector2::new(500.0, 0.0), 'e'),
            Enemy::new(Vector2::new(50.0, 0.0), 'e'),
        ];
        let d = nearest_alive(&enemies, Vector2::new(0.0, 0.0)).unwrap();
        assert_eq!(d, 50.0);
    }

    #[test]
    fn los_muertos_no_zumban() {
        let mut enemies = vec![Enemy::new(Vector2::new(50.0, 0.0), 'e')];
        for _ in 0..3 {
            enemies[0].hit();
        }
        assert!(nearest_alive(&enemies, Vector2::new(0.0, 0.0)).is_none());
    }

    #[test]
    fn remaining_cuenta_solo_los_vivos() {
        let mut enemies = vec![
            Enemy::new(Vector2::new(0.0, 0.0), 'e'),
            Enemy::new(Vector2::new(1.0, 1.0), 'e'),
        ];
        assert_eq!(remaining(&enemies), 2);
        for _ in 0..3 {
            enemies[0].hit();
        }
        assert_eq!(remaining(&enemies), 1, "todavía queda uno");
        for _ in 0..3 {
            enemies[1].hit();
        }
        assert_eq!(remaining(&enemies), 0);
    }
}
