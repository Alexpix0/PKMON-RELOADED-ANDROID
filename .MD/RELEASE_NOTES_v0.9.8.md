# Pokémon Reloaded Android — Versión 0.9.8 (Build 39)

### 🚀 Arquitectura y Rendimiento (Qué se mejoró respecto a v0.9.7)
- **Triple Buffer Lock-Free Zero-Alloc:** Sustituido el antiguo mutex y clonado de memoria por un canal SPSC con intercambio atómico de índices (`AtomicU8` con bit `DIRTY` y `UnsafeCell` alineado a 64 bytes). El hilo de lógica nunca se bloquea ni espera al renderizador.
- **Eliminación Total de `std::env::var` en Hot-Loops:** Implementada la estructura estática atómica `RenderConfig` en memoria (`filter_mode`, `render_scale`, `bezel_active`, `vsync`). Cero consultas de entorno por frame en los bucles críticos de dibujado y presentación de OpenGL ES y Vulkan.
- **Optimización de Partículas y Clima en Combates:** Filtrado temprano de partículas no visibles (`alpha <= 0.001` o `size <= 0.0`) antes de alternar modos de mezcla (`bm_normal` vs `bm_add`). Evita paradas continuas del pipeline gráfico y reduce drásticamente los flushes de VBO en GPUs móviles TBDR (Mali, PowerVR, Adreno).
- **Protección de Runtime y JIT:** Compilador JIT ARM64 fijado como desactivado por defecto (`JIT_ARM64_ENABLED = false`) con kill-switch persistente para erradicar fallos de protección `W^X` / SELinux en kernels OEM.

### 🛠️ Errores Corregidos (Bug Fixes & Auditoría)
- **Solución al Cuello de Botella en Combates (25 FPS / 35 ms):** Desacoplamiento real de lógica y renderizado, supresión de contención entre hilos y reducción de cambios de estado en shaders.
- **Corrección de Pantalla Negra en Menú de Controles Interno:**
  - Watchdog no bloqueante en `keyboard_wait` para evitar bloqueos indefinidos si no se reciben pulsaciones.
  - Reset forzado de superficies FBO (`current_draw_fbo = None`) antes de cada llamada a `present()` en OpenGL ES.
  - Gestión resiliente de archivos de configuración (`ini_open`/`ini_close`) con creación automática de directorios padre y captura de errores sin abortar el motor.
- **Reconciliación de Blit y Filtros en GPUs PowerVR:** Selección segura de filtros (`GL_NEAREST` vs `GL_LINEAR`) en `BlitFramebuffer` con coordenadas invertidas, previniendo framebuffers incompletos o negros en chips Rogue GE8320.
- **Transparencia Real de Backends (Vulkan vs. OpenGL ES):**
  - Nuevo puente JNI `nativeBackendInfo` para reportar el backend solicitado, el backend efectivo real y el motivo exacto de fallback.
  - La pantalla de Ajustes ahora notifica claramente al usuario cuando el controlador Vulkan falla y el motor recurre a OpenGL ES.
- **Blindaje de Seguridad y Limpieza de Firma (AUD-01):**
  - Retirado el almacén de claves del índice de Git y blindado `.gitignore`.
  - Supresión de contraseñas hardcodeadas en texto plano en Gradle.
  - Soporte para firma automática efímera en CI y fallback seguro.
- **Controles de Ratón Ocultos de Fábrica (AUD-08):** Los botones `LMB`, `RMB`, `+` y `-` permanecen 100% invisibles en nuevas instalaciones y actualizaciones, activándose únicamente si el usuario los añade manualmente desde el editor de controles táctiles.
- **Ajustes y Áreas Seguras (AUD-09):** Soporte completo de `WindowInsets` de sistema y espaciado inferior seguro respecto a la barra flotante de navegación, garantizando visibilidad de todas las opciones en pantallas con notch, navegación por gestos y 3 botones.

### 📊 Métricas y Verificación de Gates (R0 - R6)
- **Gate R0:** Aislamiento de material de firma en Git (`AUD-01`), `versionCode = 39`, `versionName = "0.9.8"` (`AUD-02`).
- **Gate R1 & R2:** Base nativa verificada, telemetría periódica cada 120 frames (`RLD_PERF`), canal triple buffer lock-free zero-alloc implementado (`AUD-03`, `AUD-04`).
- **Gate R3:** Cero `env::var` en caliente (`AUD-05`), transparencia JNI de Vulkan/GL (`AUD-06`), mitigación PowerVR y reset de FBO (`AUD-07`), controles de ratón ocultos (`AUD-08`) y safe insets en Ajustes (`AUD-09`).
- **Gate R4:** Optimización de draw calls en partículas y compatibilidad TBDR asegurada.
- **Gate R5:** Runtime GML estabilizado y JIT deshabilitado por defecto para estabilidad universal.
- **Gate R6:** Compilación exitosa en GitHub Actions (Rust ARM64 + Gradle Release APK). Artefacto generado: `pokemon-reloaded-0.9.8-release` (Build 39).
