# 🚀 ROADMAP v0.9.1: "COMPATIBILIDAD UNIVERSAL, BLINDAJE INFINIX HOT 20i Y REFACTORIZACIÓN ERGONÓMICA"
## *Misión Crítica: Apertura y Fluidez 60 FPS en Infinix Hot 20i (Helio G25 / PowerVR GE8320), Vulkan Oficial y Hub de Ajustes Anidado*

---

## 📑 ÍNDICE GENERAL DEL ROADMAP v0.9.1

1. [Visión Suprema de la Versión 0.9.1: Compatibilidad de Campo](#1-visión-suprema-de-la-versión-091-compatibilidad-de-campo)
2. [📱 Bloque I: Misión Crítica Infinix Hot 20i (MediaTek Helio G25 + PowerVR GE8320)](#2-📱-bloque-i-misión-crítica-infinix-hot-20i-mediatek-helio-g25--powervr-ge8320)
   - [1.1 Causa Raíz del Pantallazo Negro Post-Loading y Reparación de Controladores de Sala](#11-causa-raíz-del-pantallazo-negro-post-loading-y-reparación-de-controladores-de-sala)
   - [1.2 Saneamiento del Fragment Shader: Compatibilidad con PowerVR Rogue HSR](#12-saneamiento-del-fragment-shader-compatibilidad-con-powervr-rogue-hsr)
   - [1.3 Topología de CPU Homogénea (8x Cortex-A53) en `big_little.rs`](#13-topología-de-cpu-homogénea-8x-cortex-a53-en-big_littlers)
   - [1.4 Detección de Hardware y Valores Predefinidos a Prueba de Fallos (*Fail-Safe Defaults*)](#14-detección-de-hardware-y-valores-predefinidos-a-prueba-de-fallos-fail-safe-defaults)
3. [🌋 Bloque II: Consolidación Definitiva de Vulkan y Limpieza en Ajustes](#3-🌋-bloque-ii-consolidación-definitiva-de-vulkan-y-limpieza-en-ajustes)
   - [2.1 Vulkan Oficial en el Selector (Eliminación de "Experimental" y Reducción de Texto)](#21-vulkan-oficial-en-el-selector-eliminación-de-experimental-y-reducción-de-texto)
   - [2.2 Robustez del Pipeline Vulkan 1.1/1.2 y Manejo Silencioso de Fallbacks](#22-robustez-del-pipeline-vulkan-1112-y-manejo-silencioso-de-fallbacks)
4. [🎨 Bloque III: Refactorización Visual de Ajustes (Hub Principal y Navegación Anidada Drill-Down)](#4-🎨-bloque-iii-refactorización-visual-de-ajustes-hub-principal-y-navegación-anidada-drill-down)
   - [3.1 Hub Principal con Índice de Categorías (Reducción de Carga Visual)](#31-hub-principal-con-índice-de-categorías-reducción-de-carga-visual)
   - [3.2 Navegación Anidada a Sub-Vistas Dedicadas (Vídeo, Audio, Mandos y Rendimiento)](#32-navegación-anidada-a-sub-vistas-dedicadas-vídeo-audio-mandos-y-rendimiento)
   - [3.3 Preservación Estricta del Orden, Lógica y Persistencia de cada Ajuste](#33-preservación-estricta-del-orden-lógica-y-persistencia-de-cada-ajuste)
5. [💎 Bloque IV: Blindaje de las Optimizaciones de v0.9.0 sin Regresiones](#5-💎-bloque-iv-blindaje-de-las-optimizaciones-de-v090-sin-regresiones)
   - [4.1 Coexistencia de AMD FSR, xBRZ, JIT ARM64 y Tablas LUT](#41-coexistencia-de-amd-fsr-xbrz-jit-arm64-y-tablas-lut)
   - [4.2 Verificación de Audio 3D, Stems Reactivos, WSOLA y Rueda Radial](#42-verificación-de-audio-3d-stems-reactivos-wsola-y-rueda-radial)
6. [🗺️ Matriz de Verificación y Control de Calidad](#6-🗺️-matriz-de-verificación-y-control-de-calidad)

---

## 1. Visión Suprema de la Versión 0.9.1: Compatibilidad de Campo

La versión **0.9.0** introdujo la mayor cantidad de innovaciones de bajo nivel en la historia de OpenGMK (JIT ARM64 Tier-2, AMD FidelityFX FSR 1.0, tablas LUT L1, audio HRTF y WSOLA).
Sin embargo, las pruebas reales en terminales modestos como el **Infinix Hot 20i** demostraron que ciertas optimizaciones hiper-agresivas (especialmente el filtrado de instancias estáticas y el descarte de alfa temprano) provocan que el juego complete la barra de carga pero se quede en **pantalla negra permanente**.

La versión **0.9.1** tiene como directiva suprema la **COMPATIBILIDAD ABSOLUTA**:
1. El **Infinix Hot 20i** debe abrir, cargar la barra y entrar fluidamente a la pantalla de título y al juego en su primer arranque sin pantalla negra.
2. Todas las optimizaciones previas de la v0.9.0 se conservan intactas, pero protegidas con salvaguardas que respetan la arquitectura del chip **MediaTek Helio G25** y la GPU **PowerVR Rogue GE8320**.
3. El panel de Ajustes se refactoriza en un **Hub limpio con navegación anidada (Drill-Down)** para eliminar la saturación visual.

---

## 2. 📱 Bloque I: Misión Crítica Infinix Hot 20i (MediaTek Helio G25 + PowerVR GE8320)

### 1.1 Causa Raíz del Pantallazo Negro Post-Loading y Reparación de Controladores de Sala
* **Diagnóstico Técnico:**
  * En el motor GameMaker (`gm8emulator`), la transición de la pantalla de carga al menú principal está gobernada por objetos controladores de flujo (`obj_intro`, `obj_fade`, `obj_control`, `obj_pantalla_negra`, `obj_presentacion`).
  * Estas instancias son lógicamente inmóviles: tienen `speed = 0`, `hspeed = 0`, `vspeed = 0`, `gravity = 0`, no tienen timeline activa y no usan alarmas (gestionan su estado mediante variables locales como `alpha`, `step_count` dentro de su evento `ev_step`).
  * En la v0.9.0, `active_static_partition_enabled()` clasificó erróneamente a estas instancias como "escenografía sólida estática" e ignoró su evento `STEP`. Como resultado, el código que disminuye el alfa del fundido negro (`alpha -= 0.05`) **nunca se ejecutó**, atrapando la pantalla en negro perpetuo.
* **Corrección Definitiva:**
  * **Regla de Inmunidad para Controladores:** En `events::step()`, una instancia **NUNCA** puede ser omitida si:
    1. Es invisible (`visible == false`).
    2. No tiene sprite asignado (`sprite_index < 0`).
    3. Su objeto tiene implementado código ejecutable en `ev::STEP`.
  * Solo se puede omitir instancias que sean puramente bloques colisionables sólidos sin lógica (`solid == true` y sin código en `ev_step`).
  * Por seguridad absoluta, en el perfil predeterminado inicial (*Out-of-the-Box*), `activeStaticPartition` vendrá **desactivado (`false`)**, activable de forma voluntaria en terminales de gama alta dentro de ciudades con 3.000+ árboles.

### 1.2 Saneamiento del Fragment Shader: Compatibilidad con PowerVR Rogue HSR
* **Diagnóstico Técnico:**
  * El Infinix Hot 20i monta una GPU **IMG PowerVR GE8320** (arquitectura PowerVR Rogue), no una GPU ARM Mali.
  * En PowerVR, el acelerador de hardware **Hidden Surface Removal (HSR)** es extremadamente sensible: la presencia indiscriminada de `discard` (`if (colour.a < 0.05) discard;`) en el Fragment Shader principal rompe la optimización de profundidad temprana y fuerza a la GPU a descartar tiles de color completos o generar un fallo de pintado en transiciones con fundido a negro.
* **Corrección Definitiva:**
  * En `fragment.glsl`, eliminar el descarte incondicional de bajo alfa y restringirlo a:
    ```glsl
    if (alpha_test && colour.a <= 0.0) discard;
    ```
  * El descarte de píxeles para optimización de overdraw se aplica exclusivamente a través de la bandera del motor `alpha_test` cuando se dibujan sprites o proyectiles, protegiendo las primitivas de fundido (`draw_rectangle`), overlays y framebuffers.

### 1.3 Topología de CPU Homogénea (8x Cortex-A53) en `big_little.rs`
* **Diagnóstico Técnico:**
  * El procesador **MediaTek Helio G25** posee una arquitectura octa-core **100% Cortex-A53** (4 núcleos a 2.0 GHz y 4 núcleos a 1.5 GHz), sin núcleos Cortex-A7x o Cortex-X.
  * Si el detector `big_little.rs` busca frecuencias superiores a 2.2 GHz o arquitecturas mixtas, puede clasificar todos los núcleos como "LITTLE" o generar llamadas erráticas a `sched_setaffinity`.
* **Corrección Definitiva:**
  * Adaptar `big_little.rs` para detectar clústeres por frecuencia relativa: en procesadores homogéneos A53/A55, asignar los núcleos de mayor frecuencia (clúster 2.0 GHz, cores 4-7) al hilo de render y lógica, y los núcleos de 1.5 GHz (cores 0-3) a tareas auxiliares. Si no se puede determinar con certeza, operar en modo libre sin forzar afinidad que sature un solo núcleo.

### 1.4 Detección de Hardware y Valores Predefinidos a Prueba de Fallos (*Fail-Safe Defaults*)
* **Perfil de Arranque 100% Estable:**
  * Backend por defecto: **OpenGL ES** (la GPU PowerVR GE8320 no posee drivers Vulkan estables en Android 12).
  * Shaders xBRZ y FSR: Desactivados por defecto (activables en ajustes), asegurando que terminales modestos arranquen con la máxima tasa de cuadros nativa.
  * Comprobación de integridad: si el juego detecta GPU PowerVR en el primer inicio, fija automáticamente las opciones que garantizan 60 FPS sin bloqueos.

---

## 3. 🌋 Bloque II: Consolidación Definitiva de Vulkan y Limpieza en Ajustes

### 2.1 Vulkan Oficial en el Selector (Eliminación de "Experimental" y Reducción de Texto)
* **Interfaz Limpia en Ajustes:**
  * Sustituir el texto extenso por el título directo y conciso **"Vulkan"**.
  * Subtítulo minimalista: *"Pipeline gráfico de bajo nivel de alto rendimiento"*.
  * Selector balanceado y simétrico frente a **"OpenGL ES"**, eliminando cualquier referencia a fallbacks o advertencias que transmitan sensación de inestabilidad al usuario.

### 2.2 Robustez del Pipeline Vulkan 1.1/1.2 y Manejo Silencioso de Fallbacks
* **Gestión de Errores Transparente:**
  * Si un terminal carece de compatibilidad física con Vulkan (como el Infinix Hot 20i), el emulador realiza un fallback silencioso e instantáneo a OpenGL ES en el arranque nativo, sin mostrar ventanas de error ni alertas innecesarias.
  * Para dispositivos con soporte Vulkan (Snapdragon Adreno 6xx/7xx, Mali-G7x), el pipeline se inicializa con swapchain optimizada (`VK_KHR_swapchain`) y sincronización limpia.

---

## 4. 🎨 Bloque III: Refactorización Visual de Ajustes (Hub Principal y Navegación Anidada Drill-Down)

### 3.1 Hub Principal con Índice de Categorías (Reducción de Carga Visual)
* **Fin al Scroll Infinito de 40+ Opciones:**
  * La pantalla principal de Ajustes se transforma en un **Hub Central de Tarjetas** con botones de acceso directo:
    - 📺 **Vídeo y Pantalla:** Acceso a backend, resolución, FSR, filtros y bezels.
    - 🎵 **Audio y Sonido:** Acceso a volúmenes BGM/SFX, audio 3D y WSOLA.
    - 🎮 **Mandos y Controles:** Acceso a controles táctiles, D-Pad, rueda radial y gamepads.
    - ⚡ **Rendimiento y Motor:** Acceso a JIT ARM64, Big.LITTLE, DRS PID y memoria.
    - 💾 **Seguridad y Partidas:** Acceso a Scoped Storage Shield, snapshots y diagnóstico.
    - ℹ️ **Acerca de y Ayuda:** Versión, logs y comprobador de actualizaciones.
* **Estética Moderna Material 3:**
  * Tarjetas limpias con iconografía destacada, subtítulos descriptivos breves y respuesta táctil interactiva.

### 3.2 Navegación Anidada a Sub-Vistas Dedicadas (Vídeo, Audio, Mandos y Rendimiento)
* **Estructura Modular de Sub-Pantallas (*Drill-Down*):**
  - **Sub-Vista "Vídeo" (`VideoSettingsSubScreen`):**
    - Contiene: Backend (OpenGL ES / Vulkan), Perfil Combate Extremo, AMD FSR 1.0 (EASU + RCAS), Iluminación 2D Dinámica, Filtro Gráfico (Original, xBRZ 4x/6x, CRT, FXAA), Accesibilidad Cromática, Refresco Adaptativo LTPO, Modo de Pantalla (4:3, Rellenar, Pixel-Perfect), Escala de Renderizado (0.75x a 1.5x) y Bezels.
  - **Sub-Vista "Audio" (`AudioSettingsSubScreen`):**
    - Contiene: Volumen BGM (100%, 50%, 0%), Volumen SFX (100%, 0%), Audio Espacial 3D Binaural HRTF, Música Reactiva de Combate y Preservación de Tono en Turbo (WSOLA).
  - **Sub-Vista "Mandos y Controles" (`ControlsSettingsSubScreen`):**
    - Contiene: Editor táctil, Mapeo de mandos físicos, Autofire continuo, Tipo de D-Pad (Fijo / Flotante Dinámico), Rueda Radial Táctica Doom/GTA, Gestos Multitáctiles, Disposición (Estándar / Una Mano) y Vibración Háptica Dual HD.
  - **Sub-Vista "Rendimiento y Motor" (`PerformanceSettingsSubScreen`):**
    - Contiene: Compilador JIT Tier-2 ARM64, Afinidad Big.LITTLE de CPU, Resolución Dinámica DRS con PID, Despertador GPU Mali, Mega-Atlas Virtual y Particionado Activo/Estático.
* **Barra de Navegación con Retorno Inmediato:**
  - Botón de retroceso (`ArrowBack`) en la barra superior para volver al Hub Principal en cualquier momento con un solo toque y sin perder los cambios.

### 3.3 Preservación Estricta del Orden, Lógica y Persistencia de cada Ajuste
* **Cero Regresiones de Funcionamiento:**
  - Cada interruptor (`SwitchRow`), selector de opciones (`ChoiceRow`) y botón conserva con exactitud su lógica previa, sus valores por defecto y su guardado automático en `video.conf`.
  - El orden de las opciones dentro de cada sub-pantalla es idéntico al establecido, garantizando coherencia para los usuarios habituales.
  - Toda modificación se aplica en caliente (*Hot-Reload*) de inmediato en el motor.

---

## 5. 💎 Bloque IV: Blindaje de las Optimizaciones de v0.9.0 sin Regresiones

### 4.1 Coexistencia de AMD FSR, xBRZ, JIT ARM64 y Tablas LUT
* Todas las innovaciones introducidas en v0.9.0 permanecen operativas en el motor, disponibles y activables a través de sus respectivos interruptores en las nuevas sub-vistas.
* El Compilador JIT ARM64 Tier-2 mantiene su cumplimiento de seguridad $W \oplus X$ (`PROT_READ | PROT_WRITE` -> `__clear_cache` -> `PROT_READ | PROT_EXEC`).
* Las tablas de búsqueda rápida (LUT 1024) y la memoización de daño continúan acelerando los cálculos matemáticos en Rust.

### 4.2 Verificación de Audio 3D, Stems Reactivos, WSOLA y Rueda Radial
* El motor DSP de audio mantiene el algoritmo WSOLA para aceleración de 2x a 8x sin distorsión tonal.
* La Rueda Radial Táctica mantiene su time-dilation a 0.2x para selección rápida de equipo en combate.

---

## 6. 🗺️ Matriz de Verificación y Control de Calidad

- [ ] **Arranque en Infinix Hot 20i:** Tras completar la barra de carga, el juego inicia de inmediato en la pantalla de título/menú sin pantalla negra con los ajustes predefinidos.
- [ ] **Controladores de Sala Operativos:** Los eventos `STEP` de objetos invisibles (`obj_intro`, `obj_fade`, `obj_control`) se ejecutan sin ser omitidos.
- [ ] **Compatibilidad PowerVR GE8320:** El fragment shader no descarta fragmentos de fade-in y el renderizado en OpenGL ES es completamente estable.
- [ ] **Hub de Ajustes Limpio:** La pantalla principal de Ajustes muestra únicamente las tarjetas/botones de categoría sin scroll interminable.
- [ ] **Navegación Anidada:** Al pulsar "Vídeo", "Audio", "Mandos y Controles" o "Rendimiento", se abre su sub-vista dedicada y el botón de retroceso vuelve al Hub.
- [ ] **Selector de Vulkan:** Muestra limpiamente "Vulkan" y "OpenGL ES" sin la palabra "experimental" ni textos excesivos.
- [ ] **Persistencia y Hot-Reload:** Cada opción modificada se persiste en `video.conf` y se refleja en el motor sin reiniciar la app.
