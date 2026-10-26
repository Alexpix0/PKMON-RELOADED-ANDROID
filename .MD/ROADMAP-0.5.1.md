# 🗺️ Hoja de Ruta — Pokémon Reloaded Android (v0.5.1 Milestone)

> **Documento de Entrega y Planificación**  
> **Proyecto:** Pokémon Reloaded para Android  
> **Versión:** 0.5.1 (`versionCode = 16`, `versionName = "0.5.1"`)  
> **Autores:** Alexpix0 / Armando  
> **Stack tecnológico:** Rust (OpenGMK Engine) + ARM64 NEON SIMD + Kotlin (Jetpack Compose Launcher)

---

## 🎯 Resumen Ejecutivo de la Versión 0.5.1
La versión 0.5.1 consolida las mayores demandas de rendimiento, ergonomía de control y fidelidad audiovisual solicitadas por la comunidad:

1. **⚡ Selector de Motor Gráfico (Backend):**
   * **OpenGL ES (Nativo Estable):** Compatibilidad y estabilidad probada con todos los chips y GPUs móviles (Mali, Adreno, PowerVR).
   * **Vulkan (Experimental - Pipeline Optimizado):** Menor sobrecarga de llamadas del procesador, agrupación agresiva de sprites en Texture Atlases y reducción de consumo en terminales modernos.

2. **🚀 Desbloqueo de 60 FPS en Zonas Capadas a 30 FPS (`force_60fps`):**
   * Elimina la restricción original de 30 FPS en menús, salas fijas y cinemáticas del motor Game Maker, adaptando la cadencia de fotogramas (`effective_speed = 60`) para una fluidez de imagen constante a 60 FPS.

3. **🔫 Autofire / Disparo Continuo Individual (15 pulsaciones por segundo):**
   * Manteniendo presionado `Z` o `X` (así como `C`, `V` o el botón contextual de `diálogo`), el motor genera un ciclo automático y continuo de 15 pulsaciones por segundo (2 fotogramas pulsado, 2 fotogramas liberado a 60 FPS).
   * Pasa conversaciones, textos y diálogos extensos descansando el pulgar y ejecuta ráfagas de ataques instantáneas en combate sin fatiga muscular.

4. **🎵 Separación Completa de Volumen BGM (Música) y SFX (Efectos de Sonido):**
   * Control independiente para silenciar la música al 0% y escuchar podcasts o Spotify propio de fondo mientras se conservan los efectos sonoros de los ataques, Pokéballs y menús al 100%.
   * Modulación nativa en el sintetizador MIDI de canciones y en el mezclador de pistas exclusivas y efectos.

5. **🕹️ D-Pad Flotante Dinámico:**
   * Al tocar en la mitad izquierda de la pantalla, la base del joystick analógico se ancla al instante debajo de donde el jugador apoya su dedo pulgar (`DYNAMIC_JOYSTICK_ANCHOR`), calculando el rumbo y la dirección con respecto a esa posición natural.

6. **🔥 Aceleración Matemática SIMD NEON ARM64 y Optimización de Colisiones:**
   * Multiplicación de matrices de 4x4 acelerada con instrucciones vectoriales nativas ARM64 NEON (`core::arch::aarch64::*` con `vld1q_f32`, `vdupq_n_f32`, `vmulq_f32`, `vmlaq_f32` y `vst1q_f32`), resolviendo transformaciones en registros de 128 bits.
   * Verificación precisa de colisiones pixel-perfect con corte temprano por bounding box (`if !within_bbox { return false; }`), ahorrando millones de iteraciones de ciclos de CPU en combates masivos y clima adverso.

---

## 📋 Estado de Implementación y Fases

| Componente | Característica | Estado | Ubicación |
|---|---|---|---|
| **Controles** | Autofire individual Z/X (15 puls/s) | ✅ Completado | `touch_controls.rs`, `android_main.rs`, `game.rs` |
| **Controles** | D-Pad Flotante Dinámico bajo el pulgar | ✅ Completado | `touch_controls.rs`, `android_main.rs` |
| **Audio** | BGM independiente (0% a 100%) | ✅ Completado | `midi.rs`, `mixer.rs`, `touch_controls.rs` |
| **Audio** | SFX independiente (0% a 100%) | ✅ Completado | `mixer.rs`, `touch_controls.rs` |
| **Vídeo** | Desbloquear 60 FPS en salas a 30 FPS | ✅ Completado | `game.rs`, `touch_controls.rs`, `SettingsScreen.kt` |
| **Vídeo** | Selector OpenGL ES / Vulkan Backend | ✅ Completado | `android_main.rs`, `SettingsScreen.kt`, `Video.kt` |
| **Rendimiento**| Aceleración SIMD NEON ARM64 | ✅ Completado | `render.rs` (`mat4mult`) |
| **Rendimiento**| Optimización de Colisiones Bounding Box | ✅ Completado | `asset/sprite.rs` (`check_collision_point_precise`) |
| **Release** | Bump a v0.5.1 (`versionCode = 16`) | ✅ Completado | `build.gradle.kts`, `build.yml` |

---

## 🔮 Futuras Exploraciones para v0.6.0
* Compilación JIT de GML a código máquina ARM64 utilizando Cranelift en rutinas matemáticas de combate intensivo.
* Sincronización en la nube (WebDAV / Google Drive) para partidas guardadas entre móvil, tablet y PC.
* Gestor integrado de Regalos Misteriosos y códigos promocionales.
