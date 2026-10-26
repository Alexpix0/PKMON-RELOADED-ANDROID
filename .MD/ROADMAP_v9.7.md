# ROADMAP TÉCNICO Y ESPECIFICACIÓN ARQUITECTÓNICA: v9.7 DEFINITIVE ENGINE OVERHAUL
**Proyecto:** Pokémon Reloaded Android (OpenGMK Native Engine + Android NDK Wrapper)  
**Versión Objetivo:** `v9.7` (`versionCode = 38`, `versionName = "9.7"`)  
**Base Oficial Portada:** Pokémon Reloaded Android Oficial v0.3.7  
**Enfoque:** Overhaul Definitivo de Rendimiento (Cero Caídas a 35ms / 31 FPS), Pipeline Zero-Copy TBDR, Hardware Z-Sort Y-Sorting & Tile Baking, Desacoplamiento de Renderizado, Limpieza de HUD e Integración de Complementos Avanzados de Motor.

---

## 1. OVERHAUL DE RENDIMIENTO NATIVO (DE FÁBRICA EN EL MOTOR OPEN_GMK)

### 1.1 Zero-PingPong Surface Atlas (Fix Definitivo 35ms -> <14ms en Combates)
- **Problema:** En combates, GameMaker alterna múltiples sub-superficies (fondo de batalla, sombras dinámicas, barras de HP, efectos de clima/partículas y cajas de texto), provocando *FBO/Surface Thrashing* severo. En GPUs móviles con arquitectura Tile-Based Deferred Rendering (TBDR: ARM Mali, PowerVR, Adreno), cada alternancia forzaba volcados de Tile Memory a RAM del sistema.
- **Solución:**
  - Persistencia del Draw FBO (`current_draw_fbo`) y filtrado redundante de enlaces a framebuffers en `src/OpenGMK/gm8emulator/src/render/opengl.rs`.
  - Eliminación de recreaciones y limpiezas de vista redundantes (`clear_view` duplicado) en operaciones de canvas y barras estáticas en `src/OpenGMK/gm8emulator/src/gml/kernel.rs`.
  - Flag nativo de fábrica `TBDR_SURFACE_FUSION` (`tbdr_surface_fusion=1` en `video.conf`).

### 1.2 Hardware Z-Buffer Y-Sorting & Static Map Baking (Fix Definitivo 31 FPS -> 60 FPS en Pueblos)
- **Problema:** En pueblos densos, el ordenamiento en CPU (`depth = -y`) de cientos de NPCs, árboles, farolas y capas de autotiles provocaba fragmentación continua de llamadas de dibujo (draw calls masivas e intercaladas).
- **Solución:**
  - Delegación del ordenamiento de profundidad a la GPU mediante el canal de profundidad con `glEnable(GL_DEPTH_TEST)` / `VK_COMPARE_OP_LESS_OR_EQUAL` y alpha-discard.
  - Agrupación por lotes masivos en `src/OpenGMK/gm8emulator/src/game/draw.rs` cuando `TOWN_ZSORT_BAKING` está activo, consolidando todos los tiles y sprites en 1-2 pases continuos.
  - Viewport Frustum Culling con margen seguro para suprimir llamadas fuera de pantalla.

### 1.3 Redundant State Filter & Estabilización OpenGL ES / Vulkan
- **OpenGL ES:**
  - Filtrado de estados redundantes en `src/OpenGMK/gm8emulator/src/render/opengl.rs`: comprobación de `current_program` antes de invocar `glUseProgram`, evitando paradas en el pipeline del controlador de gráficos móvil.
  - Caché binaria precompilada en disco (`shader_binary_precache`) para shaders GML/OpenGL y shaders de ataques, evitando micro-stutters al disparar habilidades por primera vez.
- **Vulkan:**
  - Persistencia de `VkPipelineCache` en `src/OpenGMK/gm8emulator/src/render/vulkan/pipeline.rs`.
  - Anillo de descriptores uniformes dinámicos (`Dynamic Uniform Ring Buffer`) con alineación para evitar reasignaciones en tiempo de dibujo.
  - Sincronización precisa de frames con seguimiento de fences (`advance_frame_sync`).

### 1.4 Smart Script Limiter (NPC Tick Culling)
- En `src/OpenGMK/gm8emulator/src/game/events.rs`, los eventos `ev::STEP` de NPCs estáticos o sin alarmas fuera de la vista de la cámara son pausados (`NPC_TICK_CULLING`), ahorrando valiosos ciclos de CPU en mapas poblados como Ciudad Carmín o Ciudad Azulona.

---

## 2. LIMPIEZA DE INTERFAZ Y CONTROLES BAJO DEMANDA

### 2.1 Botones de Ratón de PC 100% Ocultos por Defecto
- **Problema:** En perfiles normales (`Combate`, `Overworld`, `Menús y caja`), los botones `LMB`, `RMB`, `+` y `-` aparecían en el HUD estorbando la vista.
- **Solución:**
  - En `android/app/src/main/java/com/klisee/reloaded/TouchLayouts.kt` y `touch_controls.rs`, los botones PC (`lmb`, `rmb`, `qty_plus`, `qty_minus`) tienen contexto `emptySet()` / `contexts: 0`.
  - No aparecen en el HUD por defecto en ningún perfil.
  - Permanecen 100% disponibles en el catálogo del editor ("Personalizar controles") para que el usuario los active únicamente si lo desea.
  - El puntero virtual por joystick solo actúa cuando se habilita o en el contexto de gestión de cajas con mando físico.

---

## 3. COMPLEMENTOS AVANZADOS EN "RENDIMIENTO Y MOTOR" (FRONTEND + JNI)

Se integran 5 nuevos complementos en Ajustes -> Rendimiento y Motor, conectados bidireccionalmente a `Preferences.kt`, `Video.kt` (`video.conf`) y el motor OpenGMK:

1. **Fusión de Superficies TBDR (Zero-Copy Battle FBO)**:
   *Unifica las capas de interfaz, sombras y efectos de combate en un solo pase de memoria para eliminar la subida a 35ms.*
2. **Hilo de Renderizado Desacoplado (Async Render + Frame Interpolation)**:
   *Separa el motor lógico del hilo de dibujo OpenGL/Vulkan e interpola movimiento a 60 FPS constantes en pueblos y combates.*
3. **Acelerador de Pueblos (Z-Buffer Hardware Y-Sort + Tile Baking)**:
   *Delega el orden de profundidad de edificios/NPCs a la GPU y pre-renderiza el suelo estático para evitar caídas a 30 FPS al caminar.*
4. **Caché Binaria Precompilada de Shaders (Vulkan PSO / GL Program Binary)**:
   *Guarda los shaders de ataques y efectos en disco para eliminar tirones y errores gráficos al iniciar combates.*
5. **Limitador Inteligente de Scripts Lejanos (NPC Tick Culling)**:
   *Pausa la evaluación de colisiones y animaciones de eventos que están fuera de la pantalla del jugador.*

---

## 4. INTEGRACIÓN OFICIAL v0.3.7 PRESERVADA
- Integración completa de Discord RPC (`com.klisee.reloaded.discord.*`).
- Diccionarios multi-idioma oficial (`ui_en.json`, `room_names.json`, `characters.json`).
- Guardias de robustez contra pérdidas de contexto EGL en sprites duplicados.
- Regla crítica: `/SOURCE CODE DEL OFICIAL/` en `.gitignore` estricto y fuera de Git.
