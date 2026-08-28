# Bob Esponja con pistola

Videojuego de laberintos en primera persona desarrollado en Rust con raylib. El motor usa
raycasting para dibujar un mundo 3D a partir de mapas de texto, con una estética inspirada
en Fondo de Bikini.

El objetivo de cada nivel es llegar a la salida. En el camino hay enemigos estáticos: se
pueden eliminar, pero no es obligatorio; entrar en contacto con uno provoca una derrota.

## Video de demostración

[![Ver gameplay en YouTube](https://img.youtube.com/vi/FJGRuNkv-3I/hqdefault.jpg)](https://youtu.be/FJGRuNkv-3I)

[Ver el gameplay en YouTube](https://youtu.be/FJGRuNkv-3I)

## Características

- Raycasting con paredes texturizadas, corrección de ojo de pez y sombreado por distancia.
- Tres niveles seleccionables, cargados desde archivos de texto.
- Enemigos y salida renderizados como sprites con billboarding y ocultamiento por paredes.
- Movimiento con colisiones, cámara con mouse y soporte para mando.
- Sistema de combate con ocho balas por cargador, recarga y enemigos de tres impactos.
- Minimapa opcional, HUD, pantalla de victoria y pantalla de derrota.
- Música, pasos y efectos de disparo. Los sonidos que no estén disponibles se omiten sin
  impedir que el juego inicie.

## Requisitos

- Rust con Cargo (edición 2024).
- Un compilador de C/C++ y CMake para compilar raylib.
- Una sesión gráfica con soporte para OpenGL y audio.

En Debian o Ubuntu, las dependencias nativas habituales pueden instalarse con:

```bash
sudo apt install build-essential cmake libasound2-dev libx11-dev libxrandr-dev \
  libxi-dev libgl1-mesa-dev libxcursor-dev libxinerama-dev
```

## Ejecución

Desde la raíz del repositorio:

```bash
cargo run --release
```

También se puede iniciar con un nivel preseleccionado (del 1 al 3):

```bash
cargo run --release -- --level 2
```

La opción solo cambia la selección inicial del menú; hay que confirmar el nivel para
empezar a jugar.

## Controles

### Teclado y mouse

| Control | Acción |
| --- | --- |
| `W` / `S` o `↑` / `↓` | Avanzar / retroceder |
| `A` / `D` | Desplazarse lateralmente |
| Mouse o `←` / `→` | Girar la cámara |
| Clic izquierdo o `Ctrl izquierdo` | Disparar |
| `R` | Recargar |
| `M` | Mostrar u ocultar el minimapa |
| `N` | Silenciar o reanudar la música |
| `Tab` | Liberar o volver a capturar el cursor |
| `Enter` o `Espacio` | Confirmar una opción |
| `Esc` | Salir |

En los menús se usan `W` / `S` o `↑` / `↓` para cambiar la selección.

### Mando

| Control | Acción |
| --- | --- |
| Stick izquierdo o cruceta | Moverse |
| Stick derecho | Girar la cámara |
| Gatillo derecho | Disparar |
| Botón X | Recargar |
| Botón A o Start | Confirmar una opción |

El mando es opcional y el juego muestra su nombre cuando raylib lo detecta.

## Capturas de pantalla

El programa puede generar `screenshot.png` sin entrar al bucle del juego:

```bash
cargo run --release -- --screenshot
cargo run --release -- --screenshot --menu
cargo run --release -- --screenshot --victory
cargo run --release -- --screenshot --level 3
```

Los modificadores `--menu` y `--victory` cambian la pantalla capturada. `--level N` elige
el mapa usado para la captura.

## Mapas

Los niveles están definidos en `maze.txt`, `maze2.txt` y `maze3.txt`. Cada carácter
representa una celda:

| Carácter | Significado |
| --- | --- |
| Espacio | Piso transitable |
| `p` | Posición inicial del jugador |
| `g` | Salida del nivel |
| `e` | Enemigo |
| Cualquier otro carácter | Pared |

Los caracteres `+`, `-` y `|` tienen texturas distintas. Cada mapa debe contener al menos
una celda `p`; `g` y `e` son opcionales para el cargador, aunque un nivel normal necesita
una salida para poder completarse.

## Estructura del proyecto

| Ruta | Responsabilidad |
| --- | --- |
| `src/main.rs` | Ventana, estados del juego, entrada, audio, menús y HUD |
| `src/caster.rs` | Intersección de rayos con las celdas del mapa |
| `src/render.rs` | Renderizado del mundo 3D y del minimapa |
| `src/framebuffer.rs` | Buffer de píxeles en CPU y presentación mediante una textura de GPU |
| `src/maze.rs` | Carga y consulta de mapas de texto |
| `src/player.rs` | Movimiento, cámara y colisiones |
| `src/sprites.rs` | Enemigos, salida, proyección de sprites y z-buffer |
| `src/combat.rs` | Vida, munición, disparos y daño por contacto |
| `src/gamepad.rs` | Entrada de mando y zona muerta de los sticks |
| `src/textures.rs` | Carga y muestreo de texturas y sprites |
| `assets/` | Imágenes, música y efectos de sonido |

La ventana tiene una resolución fija de 1280 × 720 y un límite de 60 FPS. El tamaño de
las celdas se calcula para que cada mapa quepa dentro de esa resolución.

## Pruebas

Las pruebas unitarias cubren la cámara, combinación de entradas, zona muerta del mando,
proyección de sprites y reglas de combate:

```bash
cargo test
```
