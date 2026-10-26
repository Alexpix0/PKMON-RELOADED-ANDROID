# 🛡️ ROADMAP v0.7.1: ESTABILIZACIÓN CRÍTICA, PULIDO DE RENDIMIENTO Y ACTIVACIÓN RUNTIME

---

## 📑 ÍNDICE GENERAL DEL PARCHE DE ESTABILIZACIÓN

1. [Objetivo de la Versión v0.7.1](#1-objetivo-de-la-versión-v071)
2. [Correcciones Críticas de Estabilidad y Cero Fugas](#2-correcciones-críticas-de-estabilidad-y-cero-fugas)
   - [2.1 Eliminación de Fuga de Memoria Heap (`Box::leak`)](#21-eliminación-de-fuga-de-memoria-heap-boxleak)
   - [2.2 Blindaje contra Caídas Fatales en Instantáneas de Guardado (`.unwrap()`)](#22-blindaje-contra-caídas-fatales-en-instantáneas-de-guardado-unwrap)
   - [2.3 Tolerancia a Fallos de Concurrencia y Envenenamiento de Cerrojos Mutex](#23-tolerancia-a-fallos-de-concurrencia-y-envenenamiento-de-cerrojos-mutex)
   - [2.4 Máscara de Bits a Prueba de Futuro (`AtomicU64`)](#24-máscara-de-bits-a-prueba-de-futuro-atomicu64)
3. [Activación en Tiempo de Ejecución de Ajustes "Juego"](#3-activación-en-tiempo-de-ejecución-de-ajustes-juego)
   - [3.1 Carrera Automática Inteligente (`auto_run`)](#31-carrera-automática-inteligente-auto_run)
   - [3.2 Avance Universal de Diálogos por Pantalla Táctil (`dialog_tap_advance`)](#32-avance-universal-de-diálogos-por-pantalla-táctil-dialog_tap_advance)
   - [3.3 Escalado Real del Buffer Maestro de Audio (`bgm_volume`)](#33-escalado-real-del-buffer-maestro-de-audio-bgm_volume)
   - [3.4 Temporizador no Intrusivo de Guardado (`save_reminder`)](#34-temporizador-no-intrusivo-de-guardado-save_reminder)
   - [3.5 Respuesta Háptica de Captura e Interacción (`fishing_haptic`)](#35-respuesta-háptica-de-captura-e-interacción-fishing_haptic)
4. [Ergonomía Táctil y Pantallas Ultra-Panorámicas (20:9 y 21:9)](#4-ergonomía-táctil-y-pantallas-ultra-panorámicas-209-y-219)
5. [Aislamiento Seguro y Transparente del Backend Vulkan](#5-aislamiento-seguro-y-transparente-del-backend-vulkan)
6. [Verificación de Integridad y Empaquetado](#6-verificación-de-integridad-y-empaquetado)

---

## 1. Objetivo de la Versión v0.7.1

Consolidar y blindar la base tecnológica introducida en la versión v0.7.0. Mientras que la v0.7.0 aportó el gran salto de infraestructura (Dual Engine Vulkan/OpenGL, ensamblador AArch64, JIT Cranelift y UI de ajustes), la versión **v0.7.1** es la entrega de **estabilidad absoluta**:
* Cero fugas de memoria nativa (*Zero Leak Architecture*).
* Cero llamadas desprotegidas `.unwrap()` en hilos de persistencia y entrada.
* Activación efectiva y en tiempo real de los interruptores de comodidad sin tocar el ROM original de GameMaker 8.

---

## 2. Correcciones Críticas de Estabilidad y Cero Fugas

### 2.1 Eliminación de Fuga de Memoria Heap (`Box::leak`)
* **Diagnóstico:** En `src/OpenGMK/gm8emulator/src/android_main.rs`, las invocaciones al banner de estado del modo Turbo realizaban `Box::leak(format!(...).into_boxed_str())`. Esto causaba una fuga irreversible de fragmentos de memoria en cada alternancia del multiplicador de velocidad.
* **Solución aplicada:** Se sustituyó por préstamo temporal `&format!(...)`, aprovechando que el gestor de Toasts clona internamente el contenido solo durante su tiempo de vida de 2500 ms.

### 2.2 Blindaje contra Caídas Fatales en Instantáneas de Guardado (`.unwrap()`)
* **Diagnóstico:** En la rutina `perform_quick_snapshot`, la llamada directa a `path.file_name().unwrap()` exponía al proceso a un cierre forzado (*panic*) si el sistema de archivos de Android devolvía una entrada sin nombre estándar.
* **Solución aplicada:** Se implementó una verificación con coincidencia de patrones segura: `if let Some(file_name) = path.file_name()`.

### 2.3 Tolerancia a Fallos de Concurrencia y Envenenamiento de Cerrojos Mutex
* **Diagnóstico:** Múltiples secciones críticas en `touch_controls.rs` y `android_main.rs` utilizaban `.lock().unwrap()`. Un bloqueo transitorio o pánico en un hilo secundario de la app provocaba `PoisonError`, derribando en cascada todo el emulador.
* **Solución aplicada:** Migración a desbloqueo tolerante a envenenamiento mediante `.lock().unwrap_or_else(|e| e.into_inner())`.

### 2.4 Máscara de Bits a Prueba de Futuro (`AtomicU64`)
* **Diagnóstico:** El registro atómico de botones presionados (`PRESSED`) estaba implementado en un entero de 32 bits (`AtomicU32`). Con 28 botones ya registrados, cualquier expansión futura por encima del botón 31 provocaba desbordamiento de bits (`1 << 32`).
* **Solución aplicada:** Se migró la estructura a `AtomicU64` con verificación de límites estricta (`index < 64`).

---

## 3. Activación en Tiempo de Ejecución de Ajustes "Juego"

### 3.1 Carrera Automática Inteligente (`auto_run`)
* Vinculada tanto al joystick táctil en pantalla (`joystick_update`) como al stick de mandos físicos Bluetooth/USB (`stick_update`).
* Al detectar desplazamiento en cualquier dirección, el emulador inyecta automáticamente la pulsación de la tecla `Key::E` (correr), y la libera instantáneamente cuando el stick vuelve a su punto muerto o se levanta el dedo.

### 3.2 Avance Universal de Diálogos por Pantalla Táctil (`dialog_tap_advance`)
* Cuando el emulador detecta que la caja de texto está abierta (`current_context() == DIALOG` o `dialogue_open()`), cualquier toque en cualquier zona de la pantalla emula la pulsación de la tecla de confirmación (`Key::Z`), permitiendo leer y jugar con una mano sin buscar el botón pequeño en pantalla.

### 3.3 Escalado Real del Buffer Maestro de Audio (`bgm_volume`)
* En `game/audio/aaudio.rs`, los buffers de muestras flotantes PCM (`&mut [f32]`) ahora multiplican su amplitud por el valor normalizado del control deslizante de volumen de la aplicación, garantizando que el audio responda fielmente al nivel elegido por el jugador en lugar de actuar solo como un interruptor de silencio.

### 3.4 Temporizador no Intrusivo de Guardado (`save_reminder`)
* Monitorización en segundo plano en `tick_save_reminder`: tras 25 minutos de juego continuo, el emulador proyecta un aviso en el HUD recordando al jugador salvar la partida desde el menú.

### 3.5 Respuesta Háptica de Captura e Interacción (`fishing_haptic`)
* Retroalimentación háptica reforzada (28 ms) al emplear la caña de pescar (`Key::V`) o interactuar con el entorno.

---

## 4. Ergonomía Táctil y Pantallas Ultra-Panorámicas (20:9 y 21:9)

* Se ajustó el centro vertical del gatillo del Menú Radial Drastic a `cy: 0.945` (desde `0.958`), creando un margen de despeje seguro que previene que los gestos de navegación inferiores de Android (barra de inicio / recientes) intercepten accidentalmente los toques del jugador.

---

## 5. Aislamiento Seguro y Transparente del Backend Vulkan

* La opción de motor gráfico Vulkan en el Launcher queda catalogada como *"Vulkan (Experimental / Auto-fallback a OpenGL ES)"*.
* Ante cualquier falta de compatibilidad en la superficie o la GPU del dispositivo, el motor realiza una transición instantánea y silenciosa a OpenGL ES 3.0, garantizando cero pantallas negras.

---

## 6. Verificación de Integridad y Empaquetado

* **Version Code:** 20
* **Version Name:** `0.7.1`
* **Artifact CI Name:** `pokemon-reloaded-0.7.1-release`
