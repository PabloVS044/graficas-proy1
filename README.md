# Proyecto 1 — Raycaster

Laberinto en primera persona al estilo Wolfenstein 3D: se carga un mapa de texto,
se lanza un rayo por cada columna de la pantalla y cada rayo se dibuja como una
"stake" vertical cuya altura es inversamente proporcional a la distancia a la pared.

## Correr

```bash
cargo run --release
```

La vista es siempre 3D, con la vista top-down del laberinto dibujada encima como
minimapa en la esquina superior derecha. Para exportar un frame a PNG sin jugar:

```bash
cargo run --release -- --screenshot   # genera screenshot.png, HUD incluido
```

## Controles

| Tecla | Acción |
|-------|--------|
| ← / → | rotar el punto de vista |
| ↑ / ↓ | avanzar / retroceder en la dirección de vista |
| M     | mostrar / ocultar el minimapa |
| ESC   | salir |

El jugador arranca en la celda `p` y gana al llegar a la `g` (aparece `GANASTE!`
en pantalla). Hay colisión con paredes, con deslizamiento: al chocar en diagonal
se sigue avanzando sobre el eje que sí está libre.

## Estructura

| Archivo | Contenido |
|---------|-----------|
| `maze.txt` | el laberinto: `+ - \|` paredes, ` ` piso, `p` spawn, `g` meta |
| `src/maze.rs` | tipo `Maze` (grid de chars + dimensiones), carga del archivo, `is_wall`, búsqueda de `p`/`g` |
| `src/player.rs` | `Player { pos, a, fov }` y `process_events` (input + colisión) |
| `src/caster.rs` | `Intersect { distance, impact }` y `cast_ray` |
| `src/render.rs` | `render_world` (3D con stakes) y `render_minimap` (top-down encima) |
| `src/framebuffer.rs` | buffer de pixeles en CPU, se sube a la GPU una vez por frame |
| `src/main.rs` | ventana, main loop, HUD |

Origen del framebuffer: **arriba-izquierda**, x a la derecha y y hacia abajo, para
que la fila 0 de `maze.txt` sea la fila de arriba en pantalla. Por eso el ángulo
`a` crece en sentido de las agujas del reloj.

`block_size` (el tamaño de una celda en pixeles del mundo) se calcula al arrancar:
24 px con este mapa de 33×11.

## El minimapa

`render_minimap` es exactamente el mismo render top-down de siempre; lo único que
cambia es el **transform del framebuffer**. `set_transform(scale, offset_x, offset_y)`
aplica `pixel = mundo * scale + offset` dentro de `set_pixel`, `rect` y `circle`, así
que el mapa entero se encoge en la esquina sin que el renderer ni `cast_ray` sepan
nada: los dos siguen trabajando en coordenadas del mundo.

Detalle: `rect` mapea las dos esquinas del rectángulo en lugar de escalar su tamaño.
Escalando el tamaño, el redondeo deja costuras de 1 px entre celda y celda.

En el minimapa se ven las paredes, el cono de visión (rayos de `cast_ray` con
`draw_line = true`), el jugador en rojo y la meta en verde.

## El HUD y el crosshair

`draw_hud` corre dentro del closure de `swap_buffers`, o sea **después** del
`draw_texture` del framebuffer: se dibuja en coordenadas y resolución de ventana,
encima de la imagen del raycaster y sin gastar pixeles del buffer.

El crosshair es una cruz fija en `(WINDOW_WIDTH/2, WINDOW_HEIGHT/2)`: cuatro brazos
con un gap en el centro y un contorno oscuro para que se lea sobre paredes claras.
No se calcula ni se mueve con el ángulo, y la razón es geométrica: la columna
central del render es el rayo lanzado a exactamente `player.a` (con
`current_ray = 0.5` la fórmula del abanico se reduce a `a = player.a`), y toda
stake se dibuja centrada en el horizonte `hh`. El centro de la ventana ya *es* el
punto al que apunta el jugador.

Pendiente para cuando haya con qué interactuar (puertas, enemigos, disparar): ese
rayo central ya trae su `Intersect` calculado dentro del loop de `render_world`,
así que guardarlo en lugar de descartarlo da distancia y tipo de pared apuntada
gratis, y con eso el crosshair puede volverse contextual.

## La ecuación de `stake_height`

Los `???` de la diapositiva quedaron así:

```
distance_to_projection_plane = hw / tan(fov / 2)
distance_to_wall             = intersect.distance * cos(a - player.a)
stake_height                 = (block_size / distance_to_wall) * distance_to_projection_plane
```

- `hw / tan(fov / 2)` es la distancia al plano de proyección que hace que una pared
  llene exactamente la pantalla cuando ocupa todo el campo de visión.
- Multiplicar por `cos(a - player.a)` convierte el largo del rayo en distancia
  **perpendicular** al plano de proyección. Sin esa corrección las paredes se
  abomban en los bordes de la pantalla (efecto ojo de pez).
- Una pared mide un bloque de alto, de ahí el `block_size` en el numerador.

## Diferencias con el código de referencia de las diapositivas

- No hay switch de modo: la vista top-down se dibuja siempre encima de la 3D como
  minimapa, y `M` solo la muestra u oculta. El flag vive **fuera** del main loop y
  usa `is_key_pressed`; en el snippet original `let mut mode = "2D"` está dentro del
  loop (se reinicia cada frame) y con `is_key_down` alternaría 60 veces por segundo
  mientras la tecla esté abajo.
- El rayo avanza `d += 1.0` en vez de `d += 10.0`. Con pasos de 10 px y bloques de
  24 px el rayo se salta esquinas y las paredes salen escalonadas.
- `cast_ray` corta si `d` pasa la diagonal del mapa, para que un laberinto con un
  hueco en el borde no cuelgue el programa en un `loop` infinito.
- El framebuffer no voltea la y (lab 2 sí lo hacía); acá el origen arriba-izquierda
  coincide con el orden de las filas del archivo.
- Las stakes se dibujan con un rectángulo de 1 px de ancho en vez de un `for` de
  `point()` por pixel: una llamada por columna en lugar de una por pixel.

## Notas del mapa

`maze.txt` es el archivo de `sources/laberinto.txt` sin cambios: 33×11, `p` en
(2,1) y `g` en (30,9), y las 195 celdas caminables son alcanzables desde el spawn.

Como el mapa se indexa carácter por carácter, los pasillos horizontales quedan de
3 bloques de ancho y las aperturas verticales de 1 bloque de alto — es la geometría
que tiene el archivo, no un bug del render.

## Siguientes pasos

Texturas en las paredes, sprites/enemigos, pantalla de bienvenida con selección de
nivel y música. El minimapa y el contador de FPS ya están.
