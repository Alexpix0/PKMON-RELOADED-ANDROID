# 🌟 ROADMAP v0.7.3: COMPLETACIÓN TOTAL DE FUNCIONALIDADES RUNTIME Y VULKAN REAL

---

## 📑 ÍNDICE GENERAL DEL PARCHE v0.7.3

1. [Objetivo de la Versión v0.7.3](#1-objetivo-de-la-versión-v073)
2. [Vulkan Real: Paso de Superficie Nativa ANativeWindow](#2-vulkan-real-paso-de-superficie-nativa-anativewindow)
3. [Reescalado Interno Real de Renderizado (`OPENGMK_RENDER_SCALE`)](#3-reescalado-interno-real-de-renderizado-opengmk_render_scale)
4. [Menú Radial Drastic con Acceso Rápido a Objetos Clave (`key_items_radial`)](#4-menú-radial-drastic-con-acceso-rápido-a-objetos-clave-key_items_radial)
5. [Pausa Táctica Instantánea en Tiempo de Ejecución (`puzzle_pause`)](#5-pausa-táctica-instantánea-en-tiempo-de-ejecución-puzzle_pause)
6. [Recordatorio Inteligente de Repelente en Sobremundo (`repel_reminder`)](#6-recordatorio-inteligente-de-repelente-en-sobremundo-repel_reminder)
7. [Contador de Pasos para Huevos Pokémon (`egg_hatch_timer`)](#7-contador-de-pasos-para-huevos-pokémon-egg_hatch_timer)
8. [Baliza Háptica de Encuentro en Batalla (`shiny_beacon`)](#8-baliza-háptica-de-encuentro-en-batalla-shiny_beacon)
9. [Resumen Técnico de Archivos y Sincronización](#9-resumen-técnico-de-archivos-y-sincronización)

---

## 1. Objetivo de la Versión v0.7.3

Consolidar y completar el 100% de los elementos planificados en el Roadmap de emulación para Android:
* Conectar el puntero nativo `ANativeWindow` de Android a la inicialización de superficies de Vulkan.
* Implementar el reescalado interno real del Framebuffer Object (FBO) para duplicar el rendimiento en dispositivos de gama media y baja al elegir 0.75x o 0.5x.
* Activar las funciones de comodidad restantes en tiempo de ejecución: Objetos clave en el menú radial, Pausa táctica instantánea, Recordatorio de repelente, Contador de pasos de huevos y Alerta táctil de combate.

---

## 2. Vulkan Real: Paso de Superficie Nativa ANativeWindow

* **Diagnóstico anterior:** `VulkanRenderer::new` invocaba `create_android_surface(&context, std::ptr::null_mut())`. Al recibir un puntero nulo, la comprobación de seguridad retornaba error forzando el fallback a OpenGL ES.
* **Solución aplicada:** Se enlaza el handle nativo de ventana entregado por Ramen (`_window.xid()`), pasando el puntero real `ANativeWindow` a la creación de superficie de Vulkan (`create_android_surface`). Si el dispositivo cuenta con drivers Vulkan operativos en Android, el renderizador se enlaza directamente a la ventana nativa. Si los controladores del fabricante fallan, el emulador activa su fallback limpio a OpenGL ES 3.0.

---

## 3. Reescalado Interno Real de Renderizado (`OPENGMK_RENDER_SCALE`)

* **Diagnóstico anterior:** La preferencia de reescalado (0.75x, 1.0x, 1.25x) guardaba una variable de entorno pero no alteraba la resolución de las texturas internas de dibujo.
* **Solución aplicada:**
  * En `render/opengl.rs`, la creación inicial del FBO y las rutinas de cambio de sala (`resize_framebuffer`) ahora escalan las dimensiones internas por el factor configurado (`width * scale`, `height * scale`).
  * En `set_view`, el `gl::Viewport` y `gl::Scissor` se adaptan proporcionalmente cuando se dibuja al FBO del emulador.
  * Al presentar a pantalla, `BlitFramebuffer` realiza el reescalado final a pantalla completa con interpolación bilinear (`gl::LINEAR`), reduciendo el coste de relleno de píxeles hasta en un **44% (a 0.75x)** y aumentando drásticamente la tasa de fotogramas en terminales modestos.

---

## 4. Menú Radial Drastic con Acceso Rápido a Objetos Clave (`key_items_radial`)

* **Activación:** Al activar la opción en Ajustes -> Juego, el menú radial circular inferior reconfigura sus sectores para acceso rápido a herramientas esenciales del juego:
  * **Sector 0 (Arriba):** Bicicleta rápida (`Key::C`).
  * **Sector 1 (Arriba-Derecha):** Caña de Pescar (`Key::V`).
  * **Sector 2 (Derecha):** Guardado Rápido (`Snapshot` en ranura rápida).
  * **Sector 3 (Abajo-Derecha):** Mochila de objetos (`Key::O`).
  * **Sector 4 (Abajo):** Cerrar Menú.
  * **Sector 5 (Abajo-Izquierda):** Pausa Táctica (`Pause`).
  * **Sector 6 (Izquierda):** Alternar Turbo.
  * **Sector 7 (Arriba-Izquierda):** Modo edición de controles táctiles.

---

## 5. Pausa Táctica Instantánea en Tiempo de Ejecución (`puzzle_pause`)

* **Mecánica:** Permite congelar la simulación de GameMaker 8 al instante (física, temporizadores, animadores y pasos de objetos) sin salir a la pantalla de inicio del sistema.
* **Disparador:** Accesible desde el sector 5 del Menú Radial o mediante la API `toggle_puzzle_pause()`. Muestra un banner en el HUD: `⏸️ JUEGO PAUSADO (Toca para reanudar)`. Al tocar cualquier punto de la pantalla, el juego se reanuda suavemente de inmediato.

---

## 6. Recordatorio Inteligente de Repelente en Sobremundo (`repel_reminder`)

* **Mecánica:** En salas de mapa (`current_context() == WORLD`), el emulador monitoriza los ciclos de movimiento continuo del stick o joystick táctil (`tick_movement_step`).
* Al acumular aproximadamente 250 pasos de desplazamiento activo (la duración estándar del Repelente Máximo), emite una vibración leve y un mensaje en el HUD: `REPELENTE: Revisa si el efecto se ha agotado`.

---

## 7. Contador de Pasos para Huevos Pokémon (`egg_hatch_timer`)

* **Mecánica:** Monitorea los ciclos de eclosión de huevos en el sobremundo.
* Al cumplir ~500 pasos de movimiento continuo (un ciclo base de incubación), lanza una notificación táctil y visual en el HUD: `HUEVO: Ciclo de eclosión alcanzado (~500 pasos)`.

---

## 8. Baliza Háptica de Encuentro en Batalla (`shiny_beacon`)

* **Mecánica:** Al transicionar hacia una sala de combate salvaje o combate especial (`set_room` con id `47 | 191 | 709`), el emulador genera un pulso háptico distintivo de 40 ms para advertir al jugador de prestar atención a la pantalla de combate.

---

## 9. Resumen Técnico de Archivos y Sincronización

| Archivo Modificado | Cambios Introducidos |
| :--- | :--- |
| `src/OpenGMK/gm8emulator/src/touch_controls.rs` | Acciones `Bike`, `Rod`, `Bag`, `Pause` en `RadialAction`; control de pausa táctica, contador de pasos para repelente/huevo y baliza de combate |
| `src/OpenGMK/gm8emulator/src/android_main.rs` | Enlace de inyección de teclas para objetos clave, despausado táctil al tocar pantalla, y paso de pasos de movimiento en `joystick_update` |
| `src/OpenGMK/gm8emulator/src/game.rs` | Pausa del bucle en `frame()` ante `is_puzzle_paused()` |
| `src/OpenGMK/gm8emulator/src/render/vulkan/mod.rs` | Paso de `_window.xid()` nativo en lugar de puntero nulo |
| `src/OpenGMK/gm8emulator/src/render/vulkan/surface.rs` | Enlace y registro de `ANativeWindow` real |
| `src/OpenGMK/gm8emulator/src/render/opengl.rs` | Reescalado proporcional FBO en `new`, `resize_framebuffer` y `set_view` |
| `android/app/src/main/java/com/klisee/reloaded/AppInfo.kt` | Sincronización de versión a `0.7.3 EX`, `versionCode = 22` |
| `android/app/build.gradle.kts` | Version bump a `22` / `"0.7.3"` |
| `.github/workflows/build.yml` | Nombre de artefacto `pokemon-reloaded-0.7.3-release` |
