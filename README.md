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
cargo run --release -- --screenshot             # genera screenshot.png, HUD incluido
cargo run --release -- --screenshot --menu      # la pantalla de bienvenida
cargo run --release -- --screenshot --victory   # la pantalla de victoria
cargo run --release -- --screenshot --level 3   # otro nivel
```

## Controles

| Control | Acción |
|---------|--------|
| ENTER / (A) | empezar, desde la pantalla de bienvenida |
| stick izquierdo | caminar y moverse de costado |
| stick derecho | rotar el punto de vista |
| cruceta | caminar y moverse de costado, sin analógico |
| mouse | rotar el punto de vista (solo horizontal) |
| W / S | avanzar / retroceder en la dirección de vista |
| A / D | moverse de costado sin girar (strafe) |
| ← / → | rotar, alternativa al mouse |
| ↑ / ↓ | avanzar / retroceder, alias de W/S |
| TAB   | soltar / recapturar el cursor |
| M     | mostrar / ocultar el minimapa |
| N     | silenciar / reanudar la música |
| ESC   | salir |

El cursor se captura al arrancar, como en cualquier FPS. `TAB` lo libera (y el HUD lo
avisa, porque un cursor suelto se confunde fácil con una cámara rota) y lo vuelve a
capturar.

Capturarlo tiene una trampa que costó dos intentos. Bajo XWayland el lock de cursor no
encierra nada, así que un barrido horizontal largo saca el puntero de la ventana, aparece
en el escritorio y el personaje deja de girar sin ninguna señal de por qué. La solución es
medir el movimiento contra el centro de la ventana y devolver el puntero ahí en cada frame
(`read_mouse_look`).

Pero eso **solo funciona con el cursor escondido, no deshabilitado**. `disable_cursor()`
pone a GLFW en `CURSOR_DISABLED`, y en ese modo `glfwSetCursorPos` deja de mover el puntero
real: solo actualiza una posición *virtual* interna. O sea que el recentrado no hacía nada
y el puntero se escapaba igual. Con `hide_cursor()` (modo `CURSOR_HIDDEN`) el warp sí es un
`XWarpPointer` de verdad, y ahí el puntero no se va a ningún lado.

Centrar al capturar (`capture_cursor`) tampoco es cosmético: como el movimiento se mide
contra el centro, arrancar o volver de `TAB` con el puntero en otro lado se leería como un
movimiento gigante y la cámara pegaría un salto.

El jugador arranca en la celda `p` y gana al llegar a la `g`, que abre la pantalla de
victoria con el tiempo de la partida. Hay colisión con paredes, con deslizamiento: al
chocar en diagonal se sigue avanzando sobre el eje que sí está libre.

## Estructura

| Archivo | Contenido |
|---------|-----------|
| `maze.txt`, `maze2.txt`, `maze3.txt` | los tres niveles: `+ - \|` paredes, ` ` piso, `p` spawn, `g` meta, `e` enemigo |
| `assets/` | texturas y fondo del menú (PNG), y los sonidos (OGG) |
| `src/maze.rs` | tipo `Maze` (grid de chars + dimensiones), carga del archivo, `is_wall`, búsqueda de `p`/`g`/`e` |
| `src/player.rs` | `Player { pos, a, fov }`, el `Intent` de entrada y `apply_intent` (movimiento + colisión) |
| `src/caster.rs` | `Intersect { distance, impact, tx, side }` y `cast_ray` |
| `src/textures.rs` | `TextureManager`: mantiene las imágenes en memoria y devuelve pixeles sueltos |
| `src/render.rs` | `render_world` (3D con stakes texturizadas, devuelve el z-buffer) y `render_minimap` |
| `src/sprites.rs` | `Enemy` y `render_enemies` (billboarding + z-buffer) |
| `src/framebuffer.rs` | buffer de pixeles en CPU, se sube a la GPU una vez por frame |
| `src/gamepad.rs` | lectura del mando: sticks con zona muerta, cruceta y botones |
| `src/main.rs` | ventana, main loop, máquina de pantallas, HUD y menús |

Origen del framebuffer: **arriba-izquierda**, x a la derecha y y hacia abajo, para
que la fila 0 de `maze.txt` sea la fila de arriba en pantalla. Por eso el ángulo
`a` crece en sentido de las agujas del reloj.

`block_size` (el tamaño de una celda en pixeles del mundo) se calcula al cargar cada
nivel: 24 px en el mapa de 33×11, 16 en el de 49×15 y 12 en el de 65×19.

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

## Pantallas

`Screen { Menu, Playing, Victory }` en `main.rs`. El juego abre en la bienvenida, que es
también el **selector de nivel**: se elige con las flechas (o la cruceta) entre los tres
laberintos y `ENTER` (o **A**) empieza. Llegar a la `g` abre la pantalla de victoria.

Los niveles viven en `LEVELS`, de más fácil a más difícil:

| Nivel | Archivo | Tamaño | Enemigos |
|-------|---------|--------|----------|
| 1 - Facil | `maze.txt` | 33×11 | 3 |
| 2 - Normal | `maze2.txt` | 49×15 | 6 |
| 3 - Dificil | `maze3.txt` | 65×19 | 9 |

Todo lo que cambia entre niveles vive en un `Level { maze, block_size, spawn, goal,
enemies }`, y `load_level` lo arma leyendo el archivo. Cambiar de nivel es construir otro:
nada del juego se queda con el laberinto viejo. El `block_size` sale de
`min(ancho_ventana / columnas, alto_ventana / filas)`, así que los mapas grandes usan
celdas más chicas y entran en la misma ventana — y de paso el costo del raycast no se
dispara, porque el mundo mide más o menos lo mismo en píxeles en los tres.

En `Menu` no se llama a `process_events` ni se chequea la meta: si se llamara, el jugador
se movería detrás del menú y podría ganar sin haber jugado. El cursor también depende del
estado — libre en la bienvenida y en la victoria, capturado mientras se juega.

La pantalla de victoria muestra el tiempo de la partida y deja elegir entre **seguir
explorando** o **reiniciar**. El nivel se sigue renderizando detrás, con un velo encima:
así se lee como una capa sobre el laberinto y no como un corte a otro lugar.

Elegir "seguir explorando" prende `freeroam`, que **desactiva la meta**. Sin eso, volver a
pisar la `g` tiraría la pantalla de victoria de nuevo a la cara. El HUD lo dice en pantalla,
porque si no parece que la meta se rompió.

Desde la victoria también se puede volver al menú a elegir otro nivel.

Las dos pantallas con opciones comparten `menu_step` y `draw_option_list`. La selección se
mueve con flechas (una tecla, un paso) y con la cruceta o el stick del mando. El stick
necesita un *latch*: es un eje que se queda empujado, así que sin recordar si estaba en
reposo el frame anterior, mantenerlo recorrería las opciones sesenta veces por segundo. La
flecha `> opción <` marca la selección además del color, que sobre un nivel lleno de
textura es fácil de no ver.

Para revisar las pantallas sin jugar: `--screenshot --menu`, `--screenshot --victory` y
`--screenshot --level 3`.

El fondo sale de `MENU_BACKGROUNDS`, con la misma lógica de candidatos que las texturas de
pared: gana el primero que cargue, hoy `assets/menu.png` y si no `assets/pared.png` como
provisorio. Se carga como `Texture2D` de GPU y no por el `TextureManager`: ese guarda
pixeles en CPU para muestrearlos de a uno con `tx`/`ty`, y acá la imagen se dibuja entera
de una vez, dentro del closure del HUD (o sea después del framebuffer, tapándolo).

`draw_background_cover` recorta en vez de estirar: toma el pedazo centrado más grande que
tenga la proporción de la ventana. Las imágenes que usamos son verticales, y estiradas a
800×600 se ven deformadas.

En el menú no se rayescanea nada: la imagen cubre la pantalla entera sobre un buffer ya
limpio, así que renderizar un frame que nadie va a ver sería trabajo tirado.

`draw_centered_text` mide el ancho con `measure_text` en vez de usar un offset a ojo (que
se descuadra apenas cambia el texto) y dibuja una sombra debajo, por lo mismo que el
crosshair tiene contorno: texto plano sobre una foto desaparece en las zonas claras.


## El mando

Teclado, mouse y mando no se leen por separado dentro del movimiento: cada uno produce un
`Intent { forward, strafe, turn, look_dx }` y se combinan con `Intent::merge`, que **suma y
clampea cada eje a `[-1, 1]`**. Ese clamp es lo que evita que apretar W y empujar el stick
a la vez camine al doble de velocidad. `look_dx` es la excepción y se suma sin clampear:
son pixeles de mouse, no un eje analógico, y recortarlo limitaría en silencio qué tan
rápido se puede girar.

Partirlo así también hace que el input se pueda testear sin abrir una ventana, que es la
única forma de probar esto: `--screenshot` no sirve para verificar un stick.

`gamepad::deadzone` no solo ignora los valores chicos, los **reescala**: `[0.18, 1]` se
estira de vuelta a `[0, 1]`. Devolver el valor crudo apenas cruza el umbral haría que el
stick salte de 0 a 0.18 de golpe, y eso se siente como un tirón en la mano. La zona muerta
existe porque los sticks de Xbox no descansan exactamente en cero, y sin ella la cámara
gira sola para siempre.

Si no hay mando conectado, `gamepad::intent` devuelve todo en cero, así que desenchufarlo a
mitad de partida solo deja al teclado a cargo en vez de romper algo. El nombre del control
se muestra en pantalla cuando está conectado — es la prueba de que está funcionando de
verdad.

**Sin vibración**: `SetGamepadVibration` existe en la API de raylib, pero en el backend
GLFW (el que usa este build) es un no-op que solo logea `"not available on target
platform"`. Tenerla implicaría hablarle a `/dev/input/eventX` por force-feedback, saltando
raylib.

## El audio

`RaylibAudio` y los clips viven sueltos en `main` y no en un struct propio: un `Music`
toma prestado el device del que salió, así que juntarlos en un tipo lo volvería
autorreferencial, que en Rust es pelearse con el borrow checker sin ganar nada.

Los pasos son un **loop que se pausa**, no un sample por pisada. El clip de Calamardo es
rítmico —cuatro golpes con su propia cadencia—, y recortarlo en pisadas sueltas mataría
justamente lo que lo hace reconocible. Entonces: si el jugador se mueve, `resume_stream`;
si se detiene, `pause_stream`. `resume` y no `play`, porque `play` reinicia desde el
principio y caminar a tirones repetiría el primer golpe cada vez.

Lo que decide si "se está moviendo" es la **distancia real recorrida** entre frames
(`player.pos` antes y después), no lo que pidió el input. Empujar contra una pared tiene
intención de movimiento pero no mueve nada, y ahí los pasos tienen que callarse.

El clip original traía 200 ms de silencio al principio, que en loop se escucha como un
hipo en cada vuelta. `assets/pasos.ogg` es el original recortado desde el primer golpe y
rellenado al final hasta 1.708 s, que son exactamente 4 × 427 ms: así el salto del final al
principio mantiene la cadencia en vez de tropezar.

`update_stream()` corre todos los frames aunque esté pausado: un sonido en streaming hay
que alimentarlo o se queda sin buffer.

La **música de fondo** usa el mismo mecanismo, con dos diferencias: arranca sola al abrir
el juego y no se detiene nunca, así que suena también bajo los menús y la pantalla de
victoria; y va a `0.35` de volumen contra el `0.7` de los pasos, porque es una cama sonora
y además tiene que dejar lugar a los efectos que faltan. `N` la silencia.

La música se sirve acelerada **×2** (`atempo`, que sube la velocidad sin tocar el tono):
`musica.ogg` sale de `musica.mp3`, que queda como original por si hay que rehacerla.

Al convertir hay una trampa: el MP3 traía **carátula embebida**, y ffmpeg la convirtió en un
stream de video Theora dentro del OGG. raylib no abre un OGG multiplexado con video, así que
el archivo fallaba en silencio y el juego caía al MP3 sin acelerar. La conversión lleva
`-vn` justamente por eso:

```bash
ffmpeg -i assets/musica.mp3 -vn -af "atempo=2.0" -c:a libvorbis -q:a 5 assets/musica.ogg
```

Los dos clips se cargan con el mismo `load_loop`, que recorre una lista de candidatos y se
queda con el primero que exista — el mismo criterio que las texturas, así que agregar el
archivo alcanza para que suene, sin tocar código. Todo es `Music` y no `Sound` incluso para
clips cortos: solo el tipo con streaming se puede pausar y reanudar.

## El piso y el cielo

Las dos bandas planas no llevan colores inventados: están **muestreados de las propias
texturas** — la mediana del agua abierta en `pared3.png` da el techo (`#3E9EAA`) y la
mediana de la arena en la base de `pared.png` da el piso (`#BFB693`). Así las bandas
continúan el arte en lugar de aproximarlo.

Se probó también una versión más apagada, pensando que un piso claro le comería
protagonismo a las paredes; el resultado fue peor: la arena desaturada se lee como barro
gris y rompe justo lo que se buscaba.

## Los sprites y el piso

El enemigo se dibuja al **55% de la altura de un bloque** a la misma distancia
(`SPRITE_SCALE` en `src/sprites.rs`), no del tamaño de la pared.

Va **apoyado en el piso**: la base de un bloque cae en `hh + block_height / 2` y el sprite
cuelga hacia arriba desde ahí. Centrarlo en el horizonte —lo natural cuando mide exactamente
un bloque— dejaría al personaje flotando en el aire.

El **ancho sale de la proporción de la textura**, no de un cuadrado: `enemigo.png` es más
alto que ancho (401×512), y forzarlo a cuadrado engordaba al personaje un 28%.

La imagen se recorta al *bounding box* de su canal alpha antes de usarse. Los márgenes
transparentes que traen los PNG recortados no son inocuos acá: con el sprite anclado al
suelo, un margen vacío abajo lo deja levitando esa cantidad de píxeles.

## La cámara y el tiempo

`process_events` recibe el `dt` del frame y **las velocidades del teclado son por
segundo**: `MOVE_SPEED` en px/s y `ROTATION_SPEED` en rad/s, multiplicadas por `dt`. Antes
estaban en px *por frame*, o sea atadas a los 60 FPS del `set_target_fps`: en una máquina
más lenta el jugador caminaba más lento. El `dt` se recorta a 0.1 s, si no el primer frame
(o un tirón mientras otra ventana se lleva la CPU) se integra como un paso gigante y puede
teletransportar al jugador a través de una pared.

El **delta del mouse no se multiplica por `dt`**, y es la parte que más se equivoca: un
delta ya es un desplazamiento, no una velocidad. Escalarlo por el tiempo haría que el
mismo movimiento de la mano gire distinto según los FPS, que es justo lo contrario de lo
que se busca.

Sí se recorta a ±200 px por frame. Al volver de un alt-tab o al arrastrar la ventana llega
un salto enorme de una sola vez, y sin el recorte la cámara pega un latigazo.

El strafe reusa `try_move` sin tocarlo: la dirección lateral es la de vista rotada un
cuarto de vuelta, `(-sin a, cos a)`, y como `try_move` ya prueba un eje a la vez, deslizar
de costado contra una pared sigue funcionando.

Las funciones `look_delta` y `wrap_angle` están separadas del input justamente para poder
testearlas sin abrir una ventana: `cargo test` cubre el signo del giro, la proporción, el
recorte simétrico y que el ángulo no se escape de `[0, 2π)`.

## Texturas

`TextureManager` (`src/textures.rs`) carga un PNG por cada char del mapa y guarda sus
pixeles ya convertidos a RGBA en un `Vec<u8>`. Convertir una sola vez al cargar (en vez
de leer `image.data` crudo en cada muestra, como el snippet de la diapositiva) deja el
camino por pixel sin `unsafe` y sin depender del formato del archivo.

Cada char tiene una lista de candidatos en `TEXTURE_FILES` y gana el primero que exista,
así que cambiar una textura es pegar un archivo. Hoy los tres tipos de pared tienen imagen
propia: `+` una placa de metal, `-` la esponja amarilla y `|` metal con ventanas. Los chars
apuntan a un índice en un `Vec<Texture>`, así que si dos compartieran archivo se cargaría
una sola vez, y la búsqueda por pixel sigue siendo un solo hash sobre un `char`.

Las imágenes originales son panorámicas (3168×1344) y **generadas con IA, con las franjas
superior e inferior desenfocadas**. Se recortaron a la banda nítida —detectada midiendo el
gradiente horizontal fila por fila, que el desenfoque aplasta— y de ahí a un cuadrado de
512×512. El recorte cuadrado no es capricho: el renderer estira la imagen entera sobre cada
cara de bloque, así que una panorámica de 3168 px se vería como toda la escena aplastada y
repetida en cada bloque. Los panorámicos quedaron en `assets/originales/` por si hay que
rehacer el recorte.

Cada stake, en lugar de un color plano, muestrea la textura:

```
tx = intersect.tx * ancho_de_la_textura              // dónde pegó el rayo sobre la cara
ty = (y - stake_top) / (stake_bottom - stake_top) * alto_de_la_textura
```

`stake_top` y `stake_bottom` van **sin clampear** en la fórmula de `ty` aunque solo se
pinte la parte visible: clampearlos estiraría la textura al acercarse a la pared.

`intersect.tx` sale de refinar el impacto. La marcha de `d += 1.0` sirve para saber *qué*
celda es sólida, pero la fracción sacada del punto muestreado tiene hasta 1 px de error
sobre 24 px de bloque (~4% del ancho de la textura) y la textura tiembla al caminar. Al
detectar la celda se resuelve el cruce exacto con sus dos planos de entrada: el rayo entra
en el más tardío de los dos, y ese `t` pasa a ser también la distancia. `side` dice por
qué cara entró (plano vertical u horizontal), que es la que define de qué coordenada del
impacto sale `tx` — y de paso se usa para oscurecer un 25% las caras horizontales, si no
dos caras perpendiculares del mismo bloque comparten textura y distancia y la esquina
entre ellas desaparece.

Si falta el PNG de un char, ese stake vuelve al color plano de `cell_color`: el juego
corre con `assets/` vacío.

## Sprites

Los enemigos son celdas `e` del `maze.txt` (caminables: el rayo las atraviesa, si no
serían paredes). `render_enemies` los dibuja como billboards, ordenados de lejos a cerca:

1. `sprite_a = atan2(dy, dx)` — `atan2` y no `atan(y/x)`, que pierde el cuadrante
2. la diferencia con `player.a` normalizada a `[-π, π]`, para que un sprite justo en el
   salto 0/2π no parezca estar a 359 grados
3. fuera del FOV (más un margen, para que entre deslizándose y no de golpe) no se dibuja
4. `sprite_size = (block_size / distancia) * distance_to_projection_plane`
5. `screen_x = hw + tan(diff) * distance_to_projection_plane` — proyectar con la tangente
   y no interpolar linealmente sobre el FOV deja al sprite pegado a la columna de pared
   que tiene detrás
6. **z-buffer**: `render_world` devuelve la distancia perpendicular de cada columna, y el
   sprite se salta las columnas donde la pared está más cerca. Por eso un enemigo
   desaparece detrás de una esquina en vez de flotar sobre ella.

Los pixeles transparentes (alpha < 128 o el magenta llave `152,0,136`) no se pintan.

El framebuffer pasó a tener sus pixeles en un `Vec<u8>` propio, con `set_pixel_color`
escribiendo directo: paredes y sprites cambian de color en cada pixel, y con la API vieja
eso era una llamada FFI a raylib por pixel, hasta un millón por frame. El `Image` quedó
solo como buffer de staging hacia la GPU. Medido: 3.6 ms por frame de render completo
(mundo + sprites + minimapa) a 800×600.

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

`maze.txt` es el archivo de `sources/laberinto.txt` con tres `e` agregadas: 33×11, `p` en
(2,1) y `g` en (30,9), y las 195 celdas caminables son alcanzables desde el spawn. Las
`e` marcan enemigos y siguen siendo piso, así que no cambian la geometría del laberinto.

Como el mapa se indexa carácter por carácter, los pasillos horizontales quedan de
3 bloques de ancho y las aperturas verticales de 1 bloque de alto — es la geometría
que tiene el archivo, no un bug del render.

## Siguientes pasos

Pantalla de bienvenida con selección de nivel y música. El minimapa, el contador de FPS,
las texturas y los sprites ya están.
