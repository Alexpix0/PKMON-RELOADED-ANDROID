# ROADMAP TÉCNICO Y ESPECIFICACIÓN ARQUITECTÓNICA: v9.7 (BUILD 38)
**Proyecto:** Pokémon Reloaded Android (OpenGMK Native Engine + Android NDK Wrapper)  
**Versión Objetivo:** `v9.7` (`versionCode = 38`, `versionName = "9.7"`)  
**Base Oficial Portada:** Pokémon Reloaded Android Oficial v0.3.7  
**Enfoque:** Alto rendimiento nativo (60 FPS estables), Culling en Overworld, Soporte Total PC/Caja con Gamepad y Desbloqueo de Configuración de Teclas.

---

## 1. MÓDULOS NATIVOS CLAVE IMPLEMENTADOS

### 1.1 Native 60 FPS en Combates (Fix cuello de botella 25-35 FPS)
- **Problema:** En combates cargados de partículas de clima, efectos de ataque y refresco de interfaz, el frametime superaba 16.6ms, provocando bloqueo del divisor VSync a 30 FPS y tirones por cambio continuo de blend mode (`bm_normal` vs `bm_add`).
- **Causa Raíz:** `System::draw` ejecutaba llamadas continuas a `set_blend_mode` por cada partícula individual sin persistir estado de mezcla previo en bucle y sin permitir desenganche de VSync.
- **Solución Nativa:** Estado de mezcla cacheador en `particle.rs`, descarte temprano de partículas invisibles (`alpha <= 0.001` o `size <= 0.0`), y sincronización EGL dinámica con `eglSwapInterval(0)` y presentación no bloqueante ante picos.

### 1.2 Optimización de Movimiento en Overworld y Ciudades (Fix 26 FPS al caminar)
- **Problema:** Caída severa a 26 FPS en ciudades extensas con miles de tiles de fondo y capas decorativas mientras el personaje camina.
- **Causa Raíz:** `draw_tile` iteraba sobre toda la lista global `self.room.tile_list` dibujando todos los tiles del mapa completo sin comprobar si se encontraban dentro del viewport actual de la cámara.
- **Solución Nativa:** Implementación de *Viewport Frustum Culling* en `src/OpenGMK/gm8emulator/src/game/draw.rs`. Cada tile calcula su caja delimitadora `[tx, ty, tx + tw, ty + th]` y se descarta instantáneamente si está fuera de `[src_x, src_y, src_x + src_w, src_y + src_h]` (+32px margen de seguridad).

### 1.3 Fix Pantalla Negra en Menú de Comandos / Teclas del Juego
- **Problema:** Al acceder a la configuración interna de controles en el menú del juego, la pantalla quedaba negra y sin renderizado.
- **Causa Raíz:** El script GML de GameMaker 8 consultaba `joystick_exists(1)`, `joystick_name(1)`, `joystick_axes(1)` y `joystick_buttons(1)`. En `src/OpenGMK/gm8emulator/src/gml/kernel.rs`, estas funciones estaban stubbeadas retornando `FALSE` o 0, lo que provocaba un fallo de inicialización en el subsistema de entrada y un cuelgue del loop de pintado.
- **Solución Nativa:** Emulación de descriptor de gamepad virtual nativo (`joystick_exists` -> TRUE, `joystick_name` -> "Android Virtual Gamepad", `joystick_axes` -> 6, `joystick_buttons` -> 14, `joystick_has_pov` -> TRUE). La escena de configuración de controles inicializa con éxito y dibuja directamente en la superficie activa.

### 1.4 Gamepad Virtual Mouse en PC / Caja + Controles Globales de Ratón
- **Problema:** En el PC / Caja de Pokémon, la interacción requiere cursor de ratón. Los mandos físicos no podían mover el cursor ni realizar clics contextuales o selección rápida de cantidades.
- **Causa Raíz:** No existía traducción de coordenadas analógicas a eventos de puntero cuando `box_open()` o el menú estaban activos. La paleta de botones táctiles carecía de botones directos para Clic Izquierdo/Derecho y Scroll.
- **Solución Nativa:**
  - Inyección de cursor virtual impulsado por Stick izquierdo/derecho en modo PC / Caja (`update_cursor_delta`).
  - Nuevos controles táctiles y remapeables en catálogo nativo y Android: `lmb` ("Clic Izquierdo"), `rmb` ("Clic Derecho"), `qty_plus` ("+"), `qty_minus` ("-").
  - Mapeo bidireccional en `TouchLayouts.kt`, `touch_controls.rs`, `Controls.kt` y `android_main.rs`.

### 1.5 Port e Integración de Oficial v0.3.7
- **Novedades Portadas:**
  - Paquete `com.klisee.reloaded.discord.*`: Soporte completo de Discord Rich Presence (RPC Service, Gateway, Login Activity, Token Store y presencia dinámica de personaje/sala).
  - Assets oficiales: `characters.json`, `room_names.json`, `ui_en.json` (soporte multi-idioma ES/EN).
  - Seguridad en FBO de OpenGMK: Guardias contra `unwrap()` nulos en `duplicate_sprite` de `render/opengl.rs` para prevenir cierres por pérdida de contexto EGL.
  - Sincronización de controles `Z` y `X` en menú de navegación y mochila.
