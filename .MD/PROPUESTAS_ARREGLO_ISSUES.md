# Matriz Integral de Diagnóstico y Propuestas de Parches — Pokémon Reloaded Android

Documento consolidado de especificación técnica para agentes de IA y desarrolladores de sistemas.  
Repositorio upstream de incidencias: [Klisee/pokemon-reloaded-android/issues](https://github.com/Klisee/pokemon-reloaded-android/issues)  
Repositorio objetivo local: `c:\Users\ADV\Desktop\pokemon-reloaded-android-main` (`Alexpix0/PKMON-RELOADED-ANDROID`, rama `v0.9.7`)

---

## 📑 Tabla de Contenidos

| ID | Título de la Incidencia | Subsistema Afectado | Severidad | Prioridad |
| :-: | :--- | :--- | :---: | :---: |
| **#96** | Corrección antier: bug real del congelamiento | `android_main.rs` / `aaudio.rs` (Lifecycle) | **Crítica** | P0 |
| **#97** | Botones en almacenamiento de Pokémon (no hay) | `TouchLayouts.kt` (Perfil MENU) | **Alta** | P1 |
| **#95** | Botones en la caja | `TouchLayouts.kt` (Perfil MENU) | **Alta** | P1 |
| **#107** | Rompecabezas no permite girar la ficha | `Controls.kt` / `TouchLayouts.kt` | **Alta** | P1 |
| **#94** | Ruinas Omega (Rotación de piezas) | `Controls.kt` / `TouchLayouts.kt` | **Alta** | P1 |
| **#103** | 8 way D-pad (Cruceta octogonal) | `touch_controls.rs` | **Media** | P2 |
| **#101** | Tecla H (Hora / Reloj) y teclas F1... | `Controls.kt` | **Media** | P2 |
| **#98** | Frente de batalla y teclado Android | `GameActivity.kt` / `android_main.rs` (IME) | **Alta** | P1 |
| **#106** | Pokémon se mueve y ataca solo tras cambio | `touch_controls.rs` (Sticky Input) | **Alta** | P1 |
| **#99** | Errores en renderizado de capas (Z-Index / Surf) | `render/opengl.rs` / `egl.rs` | **Media** | P2 |
| **#102** | Ubicaciones de Pokémon en Pokédex no aparecen | `obj_pokedex` / GL Surface | **Media** | P2 |
| **#100** | English translation improvements | `res/values-en/` / `TouchProfile` | **Baja** | P3 |
| **#104** | Mis disculpas léeme (Feedback controles) | `TouchEditorScreen.kt` (UX) | **Baja** | P3 |

---

## 1. Issue #96: Freeze en el Último Frame por Hilo Worker Huérfano

- **Identificador:** Issue #96
- **Enlace GitHub:** https://github.com/Klisee/pokemon-reloaded-android/issues/96
- **Autor del Reporte:** `@nyxthor-dev`
- **Estado:** `OPEN`
- **Severidad:** **Crítica (Bloqueo total de la app)**

### Descripción Original
> El síntoma no era "pantalla negra"... Era: La pantalla se queda congelada mostrando el último frame que el juego logró dibujar, y nada reacciona nunca — ni toques, ni botones, ni back. Hay que force-cerrar la app para salir.  
> 4 de 12 sesiones en el log terminaron con `cerrando la actividad (juego terminado: false, destruyendo: true)`. Eso significa que Android destruyó la Activity pero el emulador NO terminó — el worker thread siguió corriendo.

### Consecuencias e Impacto Técnico
1. **Fuga de Recursos Gráficos:** El worker viejo retiene el contexto EGL como *current* en su hilo.
2. **Fallo en Inicialización de Nueva Sesión:** Cuando Android recrea la Activity en el mismo proceso (ej. tras minimizar 30 segundos o rotar pantalla), el nuevo worker intenta ejecutar `eglMakeCurrent` y falla con `GL_INVALID_OPERATION`.
3. **Pérdida de Entrada e Inactividad Total:** Los frames nuevos nunca se envían al surface. Android muestra perpetuamente el último frame disponible en el búfer de ventana nativo.
4. **Carrera Crítica en Audio:** El callback de AAudio de la sesión huérfana compite con el nuevo mixer (`*MIXER.lock() = Some(new_mixer)`), provocando Use-After-Free y cortes severos de audio.

### Causa Raíz
Ubicación en código: `src/OpenGMK/gm8emulator/src/android_main.rs:1978-2085`.  
Al recibirse `MainEvent::Destroy`, se asigna `destroying = true`, el bucle de polling de Android se rompe y la función `android_entry` retorna. Sin embargo, el hilo worker creado mediante `std::thread::spawn` que ejecuta `crate::run_game(...)` no recibe ninguna señal de cancelación ni se espera con un `.join()`.

### Propuesta de Parche de Código

#### Archivo: `src/OpenGMK/gm8emulator/src/android_main.rs`
```rust
// 1. Declarar flags atómicas de ciclo de vida
pub static SHUTDOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static WORKER_HANDLE: std::sync::Mutex<Option<std::thread::JoinHandle<()>>> = std::sync::Mutex::new(None);

// 2. Al lanzar el worker en android_entry:
SHUTDOWN.store(false, std::sync::atomic::Ordering::SeqCst);
let handle = std::thread::spawn(move || {
    // ... ejecución de crate::run_game
    game_over_thread.store(true, std::sync::atomic::Ordering::SeqCst);
});
*WORKER_HANDLE.lock().unwrap() = Some(handle);

// 3. Al terminar el bucle de eventos (después de líneas 2070-2075):
if !game_over.load(std::sync::atomic::Ordering::SeqCst) {
    // Señalizar al motor y audio que deben abortar inmediatamente
    SHUTDOWN.store(true, std::sync::atomic::Ordering::SeqCst);
    android::push_input(ramen::platform::android::InputEvent::CloseRequest);
    
    // Esperar hasta 2000 ms para cierre ordenado
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(2000);
    while !game_over.load(std::sync::atomic::Ordering::SeqCst) && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    
    // Si no respondió a tiempo, matar el proceso para garantizar arranque limpio en la próxima sesión
    if !game_over.load(std::sync::atomic::Ordering::SeqCst) {
        eprintln!("[LIFECYCLE] Worker thread no respondió a SHUTDOWN tras 2s. Abortando proceso de forma segura.");
        std::process::exit(0);
    }
}
```

#### Archivo: `src/OpenGMK/gm8emulator/src/game/audio/aaudio.rs`
```rust
extern "C" fn data_callback(stream: *mut AAudioStream, user_data: *mut c_void, audio_data: *mut c_void, num_frames: i32) -> i32 {
    if crate::android_main::SHUTDOWN.load(std::sync::atomic::Ordering::Relaxed) {
        return 1; // AAUDIO_CALLBACK_RESULT_STOP: detiene el stream nativo de inmediato
    }
    // ... resto del callback
}
```

#### Archivo: `src/OpenGMK/gm8emulator/src/game.rs`
```rust
// Al inicio de cada iteración del bucle principal de frame:
if crate::android_main::SHUTDOWN.load(std::sync::atomic::Ordering::Relaxed) {
    return SceneChange::End;
}
```

### Verificación y Testing
- Minimizar el juego durante 40 segundos, reanudar desde apps recientes y verificar que no hay pantalla congelada.
- Inspeccionar `adb logcat`: ninguna sesión debe culminar con `juego terminado: false, destruyendo: true`.

---

## 2. Issues #97 y #95: Ausencia de Botones en Menú de Almacenamiento y PC

- **Identificadores:** Issue #97 y Issue #95
- **Enlaces GitHub:** https://github.com/Klisee/pokemon-reloaded-android/issues/97 y https://github.com/Klisee/pokemon-reloaded-android/issues/95
- **Autores:** `@ejmarinzapata-tech` y `@cjtorres99cj-hue`
- **Estado:** `OPEN`
- **Severidad:** **Alta (Inoperabilidad de funciones de PC)**

### Descripción Original
> "Ya actualice a la última versión... ingresé al apartado de edición de botones y aún no me permite añadir botones a ese perfil"  
> "Aún se espera que coloquen botones en la caja y pc"

### Consecuencias e Impacto Técnico
Los jugadores no pueden mover, depositar, retirar ni liberar Pokémon dentro del Centro Pokémon. El modo híbrido de la interfaz carece de cruceta y botones de confirmación/cancelación dentro de la pantalla del PC, dejando al usuario bloqueado sin poder interactuar con las cajas de almacenamiento.

### Causa Raíz
Ubicación en código: `android/app/src/main/java/com/klisee/reloaded/TouchLayouts.kt:8-85`.  
El enum `TouchProfile` declara `MENU("menu", "Menús y caja", ...)`, pero en la definición de la colección `TouchLayouts.controls` ningún control contiene `TouchProfile.MENU` en su `setOf(...)`. Al seleccionar dicho perfil en el editor de controles, la interfaz detecta cero botones disponibles.

### Propuesta de Parche de Código

#### Archivo: `android/app/src/main/java/com/klisee/reloaded/TouchLayouts.kt`
Asignar `TouchProfile.MENU` a los controles de acción e input direccional requeridos:
```kotlin
// Líneas 40-75 en TouchLayouts.kt
TouchControlSpec("z", "Z", 0.865f, 0.851f, 0.033f, 
    setOf(TouchProfile.WORLD, TouchProfile.BATTLE, TouchProfile.CHOICE, TouchProfile.MENU_NAV, TouchProfile.MENU), 
    "z.png", aspect = 1.0f),

TouchControlSpec("x", "X", 0.933f, 0.700f, 0.033f, 
    setOf(TouchProfile.WORLD, TouchProfile.BATTLE, TouchProfile.CHOICE, TouchProfile.MENU_NAV, TouchProfile.MENU), 
    "x.png", aspect = 1.0f),

TouchControlSpec("c", "C", 0.797f, 0.700f, 0.033f, 
    setOf(TouchProfile.WORLD, TouchProfile.BATTLE, TouchProfile.MENU), 
    "c.png", aspect = 1.0f),

TouchControlSpec("a", "A", 0.640f, 0.900f, 0.024f, 
    setOf(TouchProfile.WORLD, TouchProfile.BATTLE, TouchProfile.SAFARI, TouchProfile.MENU_NAV, TouchProfile.MENU), 
    "a.png", aspect = 1.0f),

TouchControlSpec("return", "OK", 0.500f, 0.945f, 0.032f, 
    setOf(TouchProfile.WORLD, TouchProfile.SAFARI, TouchProfile.MENU_NAV, TouchProfile.MENU), 
    "return.png", aspect = 1.0f),

TouchControlSpec("joystick", "Joystick", .135f, .700f, .200f,
    setOf(TouchProfile.WORLD, TouchProfile.BATTLE, TouchProfile.CHOICE, TouchProfile.MENU_NAV, TouchProfile.MENU), 
    "joystick-base.png", joystick = true),
```

### Verificación y Testing
- Entrar a Ajustes > Perfiles de Controles > "Menús y caja".
- Verificar que el editor permite arrastrar, redimensionar y posicionar Z, X, C, A y el Joystick.
- Abrir la caja de almacenamiento en el juego y verificar que los controles responden.

---

## 3. Issues #107 y #94: Bloqueo en Puzzles de Ruinas Alfa / Omega (Giro de Fichas)

- **Identificadores:** Issue #107 e Issue #94
- **Enlaces GitHub:** https://github.com/Klisee/pokemon-reloaded-android/issues/107 y https://github.com/Klisee/pokemon-reloaded-android/issues/94
- **Autores:** `@johanstivven-art` y `@wrayanenmanuellopezlopez-creator`
- **Estado:** `OPEN`
- **Severidad:** **Alta (Bloqueo de progresión de historia/capturas)**

### Descripción Original
> "no me permite girar las piezas para resolver los puzzles, por favor una solución porque estoy súper enganchado con el juego"  
> "No se pueden dar vuelta a las piezas del rompe cabeza"

### Consecuencias e Impacto Técnico
Los rompecabezas líticos de las Ruinas Alfa y Ruinas Omega exigen orientar piezas a 0°, 90°, 180° y 270°. Al no existir asignación de teclas virtuales para rotar, el jugador no puede completar los puzzles ni desbloquear las cámaras de Pokémon legendarios/Unown.

### Causa Raíz
Ubicación en código: `android/app/src/main/java/com/klisee/reloaded/Controls.kt:29-45`.  
El motor original en PC lee las teclas de función de rotación (`Q` y `E`, o `A` y `D`, o `L` y `R`). La aplicación Android no expone en el enum `ControlAction` ni en el layout de pantalla ningún botón vinculado a los códigos de rotación.

### Propuesta de Parche de Código

#### Archivo: `android/app/src/main/java/com/klisee/reloaded/Controls.kt`
```kotlin
// En enum class ControlAction:
enum class ControlAction(val key: String, val defaultCode: Int, val label: String) {
    // ... controles existentes ...
    ROTATE_L("Q", KeyEvent.KEYCODE_Q, "Girar Pieza Izquierda (Q / L)"),
    ROTATE_R("E", KeyEvent.KEYCODE_E, "Girar Pieza Derecha (E / R)"),
}
```

#### Archivo: `android/app/src/main/java/com/klisee/reloaded/TouchLayouts.kt`
```kotlin
// Agregar controles virtuales específicos para rotación en el catálogo táctil:
TouchControlSpec("rotate_l", "⟲", 0.730f, 0.400f, 0.028f, setOf(TouchProfile.WORLD, TouchProfile.MENU_NAV), null),
TouchControlSpec("rotate_r", "⟳", 0.810f, 0.400f, 0.028f, setOf(TouchProfile.WORLD, TouchProfile.MENU_NAV), null),
```

### Verificación y Testing
- Iniciar el puzzle de Kabuto/Omanyte en Ruinas Alfa/Omega.
- Presionar los botones `⟲` y `⟳` y constatar la rotación de la ficha en incrementos de 90°.

---

## 4. Issue #103: Cruceta (D-Pad) de 8 Direcciones con Movimiento Diagonal

- **Identificador:** Issue #103
- **Enlace GitHub:** https://github.com/Klisee/pokemon-reloaded-android/issues/103
- **Autor:** `@michaelolumayuwabello-ux`
- **Estado:** `OPEN`
- **Severidad:** **Media (Ergonomía de combate en tiempo real)**

### Descripción Original
> "I would like to kindly suggest adding an 8-way D-pad control to the touchscreen controls. This would allow players to move diagonally in all directions, not just up, down, left, and right... when I want to press ⬆️➡️=↗️, ⬅️⬆️=↖️, ⬇️➡️=↘️, ⬅️⬇️=↙️"

### Consecuencias e Impacto Técnico
Pokémon Reloaded es un juego de combate en tiempo real. Al usar la cruceta táctil actual, los jugadores están limitados a 4 direcciones cardinales estrictas. Cualquier intento de presionar una diagonal es forzado al eje con mayor desplazamiento, provocando movimientos truncados y pérdida de fluidez en combate.

### Causa Raíz
Ubicación en código: `src/OpenGMK/gm8emulator/src/touch_controls.rs:1302-1312`.
```rust
// Código actual restrictivo:
if move_mode() == MoveMode::Dpad {
    const DPAD_DEAD: f32 = 0.26;
    if dx.abs().max(dy.abs()) < DPAD_DEAD {
        return [false; 4]
    }
    return if dx.abs() > dy.abs() {
        [false, false, dx < 0.0, dx > 0.0]
    } else {
        [dy < 0.0, dy > 0.0, false, false]
    }
}
```
El condicional `if dx.abs() > dy.abs()` descarta de forma absoluta el eje secundario, anulando la simultaneidad de teclas de dirección.

### Propuesta de Parche de Código

#### Archivo: `src/OpenGMK/gm8emulator/src/touch_controls.rs`
Reemplazar por cálculo de octantes diagonales con umbral trigonométrico (tan(22.5°) ≈ 0.414):
```rust
if move_mode() == MoveMode::Dpad {
    const DPAD_DEAD: f32 = 0.22;
    const DIAG_THRESHOLD: f32 = 0.4142; // Permite un sector de 45° para cada una de las 4 esquinas diagonales

    let ax = dx.abs();
    let ay = dy.abs();

    if ax.max(ay) < DPAD_DEAD {
        return [false; 4];
    }

    let is_diagonal = (ax.min(ay) / ax.max(ay)) > DIAG_THRESHOLD;

    let up = dy < 0.0 && (ay > ax || is_diagonal);
    let down = dy > 0.0 && (ay > ax || is_diagonal);
    let left = dx < 0.0 && (ax > ay || is_diagonal);
    let right = dx > 0.0 && (ax > ay || is_diagonal);

    return [up, down, left, right];
}
```

### Verificación y Testing
- Configurar el modo de movimiento en "Cruceta" en las opciones del juego.
- Tocar la esquina superior derecha del D-Pad y confirmar que el personaje se mueve a 45° hacia el noreste.

---

## 5. Issue #101: Tecla H (Reloj/Hora) y Teclas de Función F1-F12

- **Identificador:** Issue #101
- **Enlace GitHub:** https://github.com/Klisee/pokemon-reloaded-android/issues/101
- **Autor:** `@Deranged619` (con aportes de `@wilseavila3-sudo`)
- **Estado:** `OPEN`
- **Severidad:** **Media (Funcionalidad utilitaria)**

### Descripción Original
> "Para así poder visualizar la hora mientras se juega. Al igual faltarían ciertas teclas como el F1... También la tecla F para la flauta"

### Consecuencias e Impacto Técnico
Pokémon Reloaded cuenta con ciclo diurno/nocturno, eventos basados en días/horas y evoluciones por hora (ej. Eevee -> Umbreon/Espeon). En PC, la tecla `H` consulta el reloj en pantalla y `F` toca la flauta. Al no existir estos botones en Android, los jugadores no pueden verificar la hora ni utilizar ítems rápidos de flauta.

### Causa Raíz
Ubicación en código: `android/app/src/main/java/com/klisee/reloaded/Controls.kt:29`.  
Omisión en el mapa de teclas `ControlAction` y en las opciones de teclas virtuales del emulador.

### Propuesta de Parche de Código

#### Archivo: `android/app/src/main/java/com/klisee/reloaded/Controls.kt`
```kotlin
enum class ControlAction(val key: String, val defaultCode: Int, val label: String) {
    // ...
    H("H", KeyEvent.KEYCODE_H, "Mostrar Hora (H)"),
    F("F", KeyEvent.KEYCODE_F, "Flauta (F)"),
    F1("F1", KeyEvent.KEYCODE_F1, "Ayuda / Opciones (F1)"),
    F5("F5", KeyEvent.KEYCODE_F5, "Guardado Rápido (F5)"),
}
```

#### Archivo: `android/app/src/main/java/com/klisee/reloaded/TouchLayouts.kt`
```kotlin
TouchControlSpec("h", "H", 0.050f, 0.145f, 0.022f, setOf(TouchProfile.WORLD), "h.png", aspect = 1.0f),
TouchControlSpec("f", "F", 0.120f, 0.145f, 0.022f, setOf(TouchProfile.WORLD), "f.png", aspect = 1.0f),
```

### Verificación y Testing
- Asignar el botón `H` en el perfil de Exploración.
- Pulsar el botón durante la partida y verificar la aparición del HUD del reloj.

---

## 6. Issue #98: Teclado IME de Android y Acceso a Edificios en Frente de Batalla

- **Identificador:** Issue #98
- **Enlace GitHub:** https://github.com/Klisee/pokemon-reloaded-android/issues/98
- **Autor:** `@javiersarracent49-lgtm`
- **Estado:** `OPEN`
- **Severidad:** **Alta (Bloqueo en Frente de Batalla y escritura de texto)**

### Descripción Original
> "las puertas pentagonales que hay antes de llegar a cada edificio se abren pero no se puede pasar de ese punto, es como si hubiera una barrera invisible... Otro problema es el del teclado Android este aparece pero a la hora de escribir no se coloca las letras en el juego, lo único q se coloca son algunos símbolos"

### Consecuencias e Impacto Técnico
1. Los jugadores quedan bloqueados fuera de las instalaciones del Frente de Batalla y Cueva Griferial debido a colisiones no liberadas en el trigger de la puerta.
2. Al intentar nombrar Pokémon o ingresar textos, el teclado nativo de Android no transmite letras ordinarias.

### Causa Raíz
1. **IME Input:** En `android_main.rs:1614-1620`, la función `ime_char_to_key(c)` transforma caracteres a eventos `KeyDown`/`KeyUp`. GameMaker 8 utiliza `keyboard_string` para capturar texto directo en diálogos; si el buffer interno no se refresca mediante inyección directa de texto Unicode, el juego solo recibe códigos numéricos de scancode físico.
2. **Puertas Pentagonales:** Conflicto de colisión sólida (`solid = false` no se ejecuta al terminar la sub-imagen de la animación de apertura en GML).

### Propuesta de Parche de Código

#### Archivo: `src/OpenGMK/gm8emulator/src/android_main.rs`
En `ime_pump()`, alimentar directamente la variable global `keyboard_string` de GameMaker además de emitir la pulsación simulada:
```rust
ImeEvent::Text(text) => {
    for c in text.chars() {
        let key = ime_char_to_key(c);
        // Garantizar que el carácter viaje con el evento de texto
        android::push_input(ramen::platform::android::InputEvent::KeyDown { key, text: Some(c) });
        android::push_input(ramen::platform::android::InputEvent::KeyUp { key });
    }
}
```

#### Archivo: `android/app/src/main/java/com/klisee/reloaded/GameActivity.kt`
Configurar el `InputType` del anchor para compatibilidad total con texto plano:
```kotlin
anchor.inputType = InputType.TYPE_CLASS_TEXT or 
                   InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD or 
                   InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
```

### Verificación y Testing
- Abrir un diálogo para poner mote a un Pokémon.
- Escribir texto con mayúsculas y minúsculas desde el teclado virtual y validar que las letras se reflejan en pantalla.
- Caminar a través de las puertas pentagonales del Frente de Batalla tras completarse su animación.

---

## 7. Issue #106: Movimiento y Ataque Autónomo No Deseado (Sticky Input tras Debilitamiento)

- **Identificador:** Issue #106
- **Enlace GitHub:** https://github.com/Klisee/pokemon-reloaded-android/issues/106
- **Autor:** `@johnroed`
- **Estado:** `OPEN`
- **Severidad:** **Alta (Pérdida de control del jugador)**

### Descripción Original
> "When i was battling 2nd gym for water badge after i kill its pokemon the 2nd pokemon killed mine as well snd when it switch to my next pokemon which is lucario it was automatic moving and attacking the enemy i tried switching it only happens to my lucario i cant control it, its automatically fighting"

### Consecuencias e Impacto Técnico
Al cambiar de Pokémon tras un desmayo, el nuevo Pokémon realiza ataques y desplazamientos continuos de manera automática, sin obedecer los toques del jugador o ignorando nuevos comandos táctiles.

### Causa Raíz
Ubicación en código: `src/OpenGMK/gm8emulator/src/touch_controls.rs:860-880`.  
"Sticky Input": Cuando un Pokémon cae debilitado, el motor GameMaker suspende temporalmente el procesamiento de eventos o transiciona de room/subestado. Si el usuario mantenía el dedo apoyado en el Joystick o botón de ataque durante ese frame, el evento nativo `TouchUp` se pierde en la transición. El bitmask del estado de teclas queda fijado en 1 de forma persistente.

### Propuesta de Parche de Código

#### Archivo: `src/OpenGMK/gm8emulator/src/touch_controls.rs`
Implementar un reseteo de estados de toque cuando se detecte un cambio de contexto o una ventana de gracia de combate:
```rust
pub fn flush_held_inputs() {
    // Liberar direcciones virtuales sostenidas
    CURRENT_STICK_DIRS.store(0, std::sync::atomic::Ordering::Relaxed);
    // Limpiar botones virtuales activos
    let mut pressed = PRESSED_BUTTONS.lock().unwrap();
    pressed.clear();
}
```
Invocar `flush_held_inputs()` en `game.rs` cada vez que ocurra un evento de cambio de instancia de jugador o cambio de turno en combate.

### Verificación y Testing
- Entrar a un combate y mantener presionada una tecla de dirección en el momento exacto en que el Pokémon propio es debilitado.
- Enviar al siguiente Pokémon y confirmar que aparece completamente estático esperando input.

---

## 8. Issue #99: Glitches de Renderizado de Capas (Z-Index / Surf / Dulce Aroma / Gran Ojo)

- **Identificador:** Issue #99
- **Enlace GitHub:** https://github.com/Klisee/pokemon-reloaded-android/issues/99
- **Autor:** `@martinezlopezjosemaestre-cmd`
- **Estado:** `OPEN`
- **Severidad:** **Media (Inconsistencia visual de profundidad)**

### Descripción Original
> "Surf: El sprite del Pokémon no se sumerge ni tiene el corte visual en el agua; se renderiza completo con sombra flotando sobre la superficie... Dulce Aroma: Las partículas/efecto se dibujan en una capa plana en los pies del personaje... Gran Ojo: El rayo indicador pasa por debajo de los objetos del mapa"

### Consecuencias e Impacto Técnico
La estética del juego sufre notorias anomalías: los Pokémon navegando sobre el agua parecen flotar en el aire, y los rayos de búsqueda o partículas de aroma se dibujan con jerarquías de capa invertidas.

### Causa Raíz
Ubicación en código: `src/OpenGMK/gm8emulator/src/render/opengl.rs:630-636` y configuración EGL.  
GameMaker 8 gestiona la profundidad 2D mediante Painter's Algorithm (`depth` menor se dibuja encima). En Android GLES, cuando dos objetos comparten el mismo depth o se utilizan shaders con recorte de fragmentos de agua, la falta de un depth buffer y stencil buffer en el framebuffer EGL provoca que las capas transparentes sobrescriban las máscaras inferiores.

### Propuesta de Parche de Código

#### Archivo: `src/OpenGMK/gm8emulator/src/render/opengl/egl.rs`
Garantizar que la configuración de superficie EGL solicite bits de profundidad y stencil adecuados para soportar operaciones de máscara de agua:
```rust
let config_attribs = [
    egl::SURFACE_TYPE, egl::WINDOW_BIT,
    egl::RENDERABLE_TYPE, egl::OPENGL_ES2_BIT,
    egl::RED_SIZE, 8,
    egl::GREEN_SIZE, 8,
    egl::BLUE_SIZE, 8,
    egl::ALPHA_SIZE, 8,
    egl::DEPTH_SIZE, 16,     // Requerido para corte Z
    egl::STENCIL_SIZE, 8,    // Requerido para máscaras de efectos
    egl::NONE,
];
```

#### Archivo: `src/OpenGMK/gm8emulator/src/render/opengl.rs`
En el ordenamiento de batches de sprites en 2D, implementar desempate estable:
```rust
// Si depth_a == depth_b, desempatar determinísticamente por orden de creación de instancia
instances.sort_by(|a, b| {
    b.depth.partial_cmp(&a.depth).unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| a.id.cmp(&b.id))
});
```

### Verificación y Testing
- Utilizar Surf en una ruta marítima y verificar que la base del Pokémon queda parcialmente sumergida con animación de ondas de agua superpuestas.
- Activar Gran Ojo y comprobar que el haz de luz se proyecta sobre la vegetación.

---

## 9. Issue #102: Falta de Visualización de Hábitats en el Mapa de la Pokédex

- **Identificador:** Issue #102
- **Enlace GitHub:** https://github.com/Klisee/pokemon-reloaded-android/issues/102
- **Autor:** `@AhsterXD`
- **Estado:** `OPEN`
- **Severidad:** **Media (Información de Pokédex ausente)**

### Descripción Original
> "no me aparecen las ubicaciones de ningún Pokémon desde la pokédex. (Imagen adjunta de mapa regional sin áreas resaltadas)"

### Consecuencias e Impacto Técnico
Los jugadores no pueden consultar en qué rutas, cuevas o mares habitan las especies avistadas.

### Causa Raíz
En el script GML de visualización de hábitats de la Pokédex (`obj_pokedex`), las áreas coloreadas se renderizan en una superficie offscreen temporal (`surface_create`) que luego se superpone sobre el sprite del mapa regional. En Android GLES, cuando una surface se pierde o sus dimensiones no respetan el aspect ratio escalado de la ventana nativa, el renderizado de la capa de hábitat resulta transparente o es descartado silenciosamente.

### Propuesta de Parche de Código
1. En el wrapper de superficies GL (`src/OpenGMK/gm8emulator/src/render/opengl/cache.rs`), verificar que si `surface_exists(id)` es consultado en Android y el contexto EGL fue recreado, la superficie se regenere automáticamente en lugar de devolver un framebuffer nulo.
2. Comprobar que las coordenadas de dibujado de la Pokédex utilicen las dimensiones lógicas fijas del juego (640x480) antes de aplicar el escalado global del viewport.

### Verificación y Testing
- Abrir la Pokédex y navegar a la sección de área de Pidgey o Zubat.
- Verificar que las zonas geográficas del mapa se iluminan con los rectángulos de hábitat.

---

## 10. Issue #100: Cadenas de Texto Sin Traducir al Inglés en Ajustes

- **Identificador:** Issue #100
- **Enlace GitHub:** https://github.com/Klisee/pokemon-reloaded-android/issues/100
- **Autor:** `@michaelolumayuwabello-ux`
- **Estado:** `OPEN`
- **Severidad:** **Baja (Localización e internacionalización)**

### Descripción Original
> "I would also like to kindly report a few Spanish words that still appear untranslated in the settings: 'Perfil: Exploración' -> 'Profile: Exploration', 'Controles para caminar por el mundo.' -> 'Controls for walking around the world.', 'Cruceta' -> 'D-pad', 'Moderno' -> 'Modern', 'Clásico' -> 'Classic', 'Tamaño: 100 %' -> 'Size: 100%'"

### Consecuencias e Impacto Técnico
La experiencia de usuarios angloparlantes se ve afectada al encontrar términos en español mezclados en la configuración.

### Causa Raíz
Ubicación en código: `android/app/src/main/res/values/strings.xml` y `TouchLayouts.kt:9`.  
No existe el directorio de recursos `android/app/src/main/res/values-en/`. Adicionalmente, los textos descriptivos de perfiles táctiles se encuentran hardcodeados como literales en las clases Kotlin de Jetpack Compose en lugar de utilizar el resolvedor de recursos `stringResource(...)`.

### Propuesta de Parche de Código

#### Archivo: Crear `android/app/src/main/res/values-en/strings.xml`
```xml
<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="app_name">Pokemon Reloaded EX</string>
    <string name="play">Play</string>
    <string name="import_game">Import game</string>
    <string name="log">Log</string>
    <string name="about">About</string>
    <string name="profile_world_title">Profile: Exploration</string>
    <string name="profile_world_desc">Controls for walking around the world.</string>
    <string name="dpad_label">D-pad</string>
    <string name="style_modern">Modern</string>
    <string name="style_classic">Classic</string>
    <string name="size_format">Size: %1$d%%</string>
</resources>
```

#### Archivo: `android/app/src/main/java/com/klisee/reloaded/ui/ControlsScreen.kt`
Sustituir los textos estáticos por `stringResource(...)` para que el sistema seleccione automáticamente el idioma del dispositivo.

### Verificación y Testing
- Cambiar el idioma del sistema Android a Inglés (English - US).
- Abrir Ajustes de Controles y verificar que todos los títulos, descripciones y porcentajes aparecen en inglés.

---

## 11. Issue #104: Usabilidad del Editor de Controles y Guía de Inicio

- **Identificador:** Issue #104
- **Enlace GitHub:** https://github.com/Klisee/pokemon-reloaded-android/issues/104
- **Autor:** `@lachypj-alt`
- **Estado:** `OPEN`
- **Severidad:** **Baja (UX / Onboarding)**

### Descripción Original
> "He de pedir perdón, mi incompetencia evitó q me dice cuenta de lo bien elaborado q está el editor de botones q se integró al juego y me quejé sin sentido de botones faltantes..."

### Consecuencias e Impacto Técnico
Muchos usuarios reportan falsos problemas de botones faltantes simplemente porque no descubren cómo acceder al editor de perfiles táctiles o no comprenden que cada botón puede reposicionarse y activarse a gusto.

### Causa Raíz
Falta de una guía visual inicial (tooltip o banner explicativo) en el menú de controles.

### Propuesta de Parche de Código
En `android/app/src/main/java/com/klisee/reloaded/ui/ControlsScreen.kt`, agregar una tarjeta informativa persistente en la cabecera:
```kotlin
Card(
    modifier = Modifier.fillMaxWidth().padding(12.dp),
    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.primaryContainer)
) {
    Text(
        text = stringResource(R.string.editor_tip_banner),
        style = MaterialTheme.typography.bodyMedium,
        modifier = Modifier.padding(16.dp)
    )
}
```

### Verificación y Testing
- Comprobar que en la pantalla de controles se explica claramente la posibilidad de personalizar botones por perfil.

---

## 🛠️ Plan de Implementación Consolidado para la IA Ejecutora

Para aplicar estos parches de forma determinista, ejecutar las modificaciones en tres etapas:

1. **Fase 1 (Nivel Nativo Rust):**
   - Modificar `src/OpenGMK/gm8emulator/src/android_main.rs` (Parche SHUTDOWN + timeout para Issue #96).
   - Modificar `src/OpenGMK/gm8emulator/src/game/audio/aaudio.rs` (Retorno `AAUDIO_CALLBACK_RESULT_STOP` para Issue #96).
   - Modificar `src/OpenGMK/gm8emulator/src/touch_controls.rs` (D-Pad de 8 direcciones para Issue #103 y flush de inputs para Issue #106).
   - Modificar `src/OpenGMK/gm8emulator/src/render/opengl/egl.rs` y `opengl.rs` (Depth y Stencil para Issue #99).

2. **Fase 2 (Nivel Android / Kotlin):**
   - Modificar `android/app/src/main/java/com/klisee/reloaded/TouchLayouts.kt` (Habilitar `TouchProfile.MENU` para Issues #97 y #95).
   - Modificar `android/app/src/main/java/com/klisee/reloaded/Controls.kt` (Añadir `ROTATE_L`, `ROTATE_R`, `H`, `F` para Issues #107, #94, #101).
   - Modificar `android/app/src/main/java/com/klisee/reloaded/GameActivity.kt` (Ajustes de IME para Issue #98).

3. **Fase 3 (Localización y Recursos):**
   - Crear `android/app/src/main/res/values-en/strings.xml` (Issue #100).
   - Compilar y verificar el APK con `./gradlew assembleRelease` o `cargo check`.
