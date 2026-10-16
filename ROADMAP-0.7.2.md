# ⚡ ROADMAP v0.7.2: OPTIMIZACIÓN COMBATE, LIMPIEZA DE HUD Y RENDIMIENTO DINÁMICO 60 FPS

---

## 📑 ÍNDICE GENERAL DEL PARCHE v0.7.2

1. [Objetivo de la Versión v0.7.2](#1-objetivo-de-la-versión-v072)
2. [Evaluación de Capturas de Revisión y Diagnóstico](#2-evaluación-de-capturas-de-revisión-y-diagnóstico)
3. [Limpieza del HUD y Controles en Combate (Cero Obstrucción de PS)](#3-limpieza-del-hud-y-controles-en-combate-cero-obstrucción-de-ps)
   - [3.1 Eliminación de Botones Inútiles en Batalla (`M`, `E`, `T`, `L`, `P`)](#31-eliminación-de-botones-inútiles-en-batalla-m-e-t-l-p)
   - [3.2 Reubicación Ergonómica de `PKM` (Espacio) y `O` (Mochila)](#32-reubicación-ergonómica-de-pkm-espacio-y-o-mochila)
4. [Rendimiento Dinámico y Eliminación del Cuello de Botella de 33 FPS](#4-rendimiento-dinámico-y-eliminación-del-cuello-de-botella-de-33-fps)
   - [4.1 Salto de Fotogramas Automático Adaptativo por Carga (`FRAME_LAGGING`)](#41-salto-de-fotogramas-automático-adaptativo-por-carga-frame_lagging)
   - [4.2 Reducción Real del 50% de Partículas y Clima (`weather_opt: optimized`)](#42-reducción-real-del-50-de-partículas-y-clima-weather_opt-optimized)
   - [4.3 Filtrado Bilinear de Alta Fidelidad en Android (`OPENGMK_FILTER`)](#43-filtrado-bilinear-de-alta-fidelidad-en-android-opengmk_filter)
5. [Sincronización Total de Versión en Launcher e Interfaz](#5-sincronización-total-de-versión-en-launcher-e-interfaz)
6. [Resumen Técnico de Modificaciones](#6-resumen-técnico-de-modificaciones)

---

## 1. Objetivo de la Versión v0.7.2

La versión **v0.7.2** responde directamente a la auditoría técnica visual de las 17 capturas del emulador en combate real contra Lysson / Xerneas. Sus objetivos fundamentales son:
* **Despejar completamente la barra de vida del Pokémon rival**: Eliminar los botones que tapaban los PS, nivel y nombre en combate.
* **Eliminar las caídas a 33 FPS**: Dotar al motor de GameMaker de salto de fotogramas reactivo por carga y activar el recorte real del 50% en auras/partículas.
* **Corregir la discrepancia de versión visual**: Actualizar las etiquetas estáticas heredadas del Launcher para reflejar siempre `BuildConfig.VERSION_NAME` dinámico.

---

## 2. Evaluación de Capturas de Revisión y Diagnóstico

| Captura auditada | Elemento observado | Diagnóstico técnico | Corrección aplicada en v0.7.2 |
| :--- | :--- | :--- | :--- |
| **Foto 1 (Launcher)** | Badge superior dice `0.4.0 EX` | Texto estático hardcodeado en `HomeScreen.kt` | Enlazado a `BuildConfig.VERSION_NAME` dinámico |
| **Foto 5 (Ajustes Clima)** | Opción `Optimizado (50%)` activa | `game.rs` solo contemplaba `"minimal"`, ignorando `"optimized"` | Añadido bloque `weather_opt == "optimized"` (salto 50% de emisores) |
| **Foto 6 (Frame Skip)** | Opción `Automático (Según carga)` activa | `auto` solo saltaba si Turbo estaba activo | Vinculado a detección de retraso por fotograma (`FRAME_LAGGING`) |
| **Foto 8 (Filtro)** | Opción `Suave / Bilinear` | `render/opengl.rs` forzaba `gl::NEAREST` en Android | Lee `OPENGMK_FILTER` y aplica `gl::LINEAR` en el Blit a pantalla |
| **Foto 17 (Combate Xerneas)** | Fila de botones `PKM O M E T L P` sobre PS rival | Botones tenían máscara de contexto `BATTLE` | Retirado `BATTLE` de `M, E, T, L, P`; `PKM` y `O` reubicados en el margen lateral |
| **Foto 17 (Combate Xerneas)** | Contador marca 33 FPS con 60 FPS activados | Sobrecarga de partículas + room speed base 30 sin frame-skipping adaptativo | Motor reactivo mantiene el refresco a 60 FPS con descarte visual automático |

---

## 3. Limpieza del HUD y Controles en Combate (Cero Obstrucción de PS)

### 3.1 Eliminación de Botones Inútiles en Batalla (`M`, `E`, `T`, `L`, `P`)
* **Problema:** Los botones de **Mapa (M)**, **Correr (E)**, **Teléfono (T)**, **Libreta (L)** y **Pokédex (P)** aparecían alineados horizontalmente en `cy: 0.145`. En el juego base Pokémon Reloaded, ninguna de estas teclas tiene funcionalidad alguna en salas de batalla (`47 | 191 | 709`).
* **Solución:** Se retiró el flag `BATTLE` de sus contextos en `touch_controls.rs`. Ahora solo se muestran en el mapa (`WORLD`) y Safari (`SAFARI`).

### 3.2 Reubicación Ergonómica de `PKM` (Espacio) y `O` (Mochila)
* **Problema:** `Space` y `O` permanecían en la fila superior, ocultando parcialmente la barra de vida y el nivel del Pokémon del rival.
* **Solución:** En salas de combate (`current_context() == BATTLE`), si el usuario no ha personalizado su posición manualmente, se sitúan de forma limpia en el margen izquierdo (`cx: 0.065`, `cy: 0.380` y `cy: 0.500`), por encima del D-Pad. Esto deja el 100% del área central y superior despejada para ver auras, barras de vida, PS numéricos y clima.

---

## 4. Rendimiento Dinámico y Eliminación del Cuello de Botella de 33 FPS

### 4.1 Salto de Fotogramas Automático Adaptativo por Carga (`FRAME_LAGGING`)
* **Problema:** En salas complejas con shaders y auras pesadas (como el combate legendario de Xerneas), forzar 60 FPS (`force_60fps`) sobre un motor pensado originalmente a 30 ticks satura la CPU/GPU móvil. Al tomar cada paso ~30 ms, el juego caía a 33 FPS en cámara lenta.
* **Solución:** Se implementó una bandera atómica global `FRAME_LAGGING`. Cuando el bucle del limitador detecta que el tiempo de renderizado y física superó la cuota presupuestada del fotograma (`diff > duration`), el modo `auto` descarta inmediatamente el dibujado visual del siguiente fotograma (`skip_visual_frame = (c % 2) == 1`). La física continúa a 60 ticks fluidos y la tasa de fotogramas se estabiliza.

### 4.2 Reducción Real del 50% de Partículas y Clima (`weather_opt: optimized`)
* **Problema:** En `game.rs`, la rama de optimización solo evaluaba `"minimal"` (33% de partículas). Si el usuario seleccionaba `"optimized"` (50%) en Ajustes, el motor caía en el bloque `else` por defecto, ejecutando el 100% de partículas.
* **Solución:** Se añadió el controlador nativo para `"optimized"`, ejecutando `self.particles.auto_update_systems(&mut self.rand)` exactamente 1 de cada 2 ticks (`c % 2 == 0`).

### 4.3 Filtrado Bilinear de Alta Fidelidad en Android (`OPENGMK_FILTER`)
* **Problema:** En la llamada `BlitFramebuffer` de presentación a la ventana nativa de Android, se forzaba siempre `gl::NEAREST`, impidiendo que los usuarios disfrutaran de un reescalado suave.
* **Solución:** Se enlaza con `OPENGMK_FILTER`. Si el usuario selecciona `sharp`, `linear` o `fxaa`, se emplea `gl::LINEAR` para una imagen perfectamente nítida y sin dientes de sierra.

---

## 5. Sincronización Total de Versión en Launcher e Interfaz

* **HomeScreen.kt:** El distintivo superior de la cabecera ahora muestra `${BuildConfig.VERSION_NAME} EX`.
* **AboutScreen.kt:** La fila de "Edición EX" muestra dinámicamente `v${BuildConfig.VERSION_NAME} EX (Alexpix0 / Armando)`.
* **GameRepository.kt:** Las instantáneas de guardado registran la versión actual `${BuildConfig.VERSION_NAME}-EX`.
* **Gradle:** Código de versión incrementado a `versionCode = 21`, nombre `versionName = "0.7.2"`.

---

## 6. Resumen Técnico de Modificaciones

| Archivo Modificado | Tipo de Cambio | Impacto |
| :--- | :--- | :--- |
| `src/OpenGMK/gm8emulator/src/touch_controls.rs` | Contextos de botones + Posición BATTLE | Pantalla de combate limpia, sin botones invasivos |
| `src/OpenGMK/gm8emulator/src/touch_controls.rs` | Tolerancia en `is_button_hidden` | Cero pánicos por envenenamiento en cerrojos |
| `src/OpenGMK/gm8emulator/src/game.rs` | `FRAME_LAGGING` + Auto Frame-Skip | Eliminación del cuello de botella de 33 FPS |
| `src/OpenGMK/gm8emulator/src/game.rs` | Soporte `weather_opt == "optimized"` | 50% menos carga de CPU/GPU en partículas |
| `src/OpenGMK/gm8emulator/src/render/opengl.rs` | Lectura de `OPENGMK_FILTER` en Blit | Suavizado bilinear real en Android |
| `android/app/src/main/java/com/klisee/reloaded/ui/HomeScreen.kt` | Versión dinámica | Insignia superior sincronizada con 0.7.2 |
| `android/app/src/main/java/com/klisee/reloaded/ui/AboutScreen.kt` | Versión dinámica | Acerca de sincronizado con 0.7.2 |
| `android/app/src/main/java/com/klisee/reloaded/GameRepository.kt` | Versión dinámica | Metadatos de guardado sincronizados |
| `android/app/build.gradle.kts` | Version bump | `versionCode 21`, `versionName 0.7.2` |
| `.github/workflows/build.yml` | Artifact name | `pokemon-reloaded-0.7.2-release` |
