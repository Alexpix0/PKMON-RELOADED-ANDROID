# ENGINE_OPTIMIZATIONS_RESEARCH.md: Compendio de Optimización, Arquitectura de Motor y Fuentes Técnicas

**Objetivo:** Guía de optimización técnica de vanguardia para `libgm8emulator.so` (OpenGMK / Rust) + Android Wrapper (Kotlin/JNI/Compose).  
**Enfoque:** Arquitectura móvil (ARM64 big.LITTLE, GPUs TBDR Adreno/Mali), tuberías gráficas (GLES 3.0 / Vulkan), colisiones espaciales $O(1)$, puente JNI lock-free y sincronización de entrada a 0ms de latencia.

---

## 1. Mapa de Innovaciones Técnicas (Edición EX / Builds 40 & 41)

```
+-----------------------------------------------------------------------------------+
|                        FRONTEND KOTLIN / JETPACK COMPOSE                         |
|  - Panel Modular QoL     - HUD Táctil Zero-Alloc      - Gestor Cloud / Saves      |
+-----------------------------------------------------------------------------------+
                                        │
                         Canal Atómico SPSC / HardwareBuffer
                                        ▼
+-----------------------------------------------------------------------------------+
|                           PUENTE JNI ULTRA-BAJA LATENCIA                          |
|  - Inout Event Buffer (0ms)   - Zero-Copy Shaders/Textures   - JNI LocalFrame Guard|
+-----------------------------------------------------------------------------------+
                                        │
                                Syscalls nativas
                                        ▼
+-----------------------------------------------------------------------------------+
|                            MOTOR NATIVO RUST (OpenGMK)                            |
|  - Spatial Hash Grid O(1)     - Pinning Big Cores (big.LITTLE)                    |
|  - Pipeline GLES 3.0 / Vulkan - AAudio Stream / Oboe Ringbuffer                   |
+-----------------------------------------------------------------------------------+
```

---

## 2. Motor Nativo Rust (OpenGMK Core)

### 2.1. Spatial Hashing y Detección de Colisiones $O(1)$
* **Problema:** GameMaker 8 ejecuta bucles $O(N^2)$ o $O(N \cdot M)$ en eventos de colisión (`place_meeting`, `instance_place`), iterando sobre todas las instancias activas.
* **Solución Técnica:**
  * **2D Spatial Hash Grid con Morton Codes (Z-Order Curve):** Transformar coordenadas $(x, y)$ a un hash entero continuo de 32 o 64 bits para maximizar localidad espacial en caché L1/L2.
  * **Bitmask Collisions con SIMD (ARM NEON):** Prueba preliminar AABB branchless con registros `int32x4_t`, seguido de comprobación precisa píxel a píxel empaquetando máscaras de 64 bits por línea (`uint64_t`) y operando con instrucciones `AND` y `CNT` (Population Count).
* **Fuentes y Referencias Clave:**
  * *Paper:* "Optimized Spatial Hashing for Collision Detection of Deformable Objects" (Matthias Müller et al.) – [PDF en matthias-research.github.io](https://matthias-research.github.io/pages/publications/tetraSim.pdf).
  * *Libro:* "Real-Time Collision Detection" (Christer Ericson, Morgan Kaufmann). Capítulo 7: *Spatial Partitioning*.
  * *Repo Rust:* `spade` (Delaunay/Spatial) y `kd-tree` crate.
  * *Reddit r/emudev:* "Spatial partitioning for 2D retro physics engines with dense particle counts".

### 2.2. Afinidad de Hilos CPU en Arquitecturas big.LITTLE / DynamIQ
* **Problema:** El planificador CFS de Linux/Android migra el hilo de emulación de GameMaker entre núcleos LITTLE (eficiencia/lentos) y núcleos Cortex-X / Cortex-A7x (rendimiento), induciendo micro-stutter (judder) y caídas de framerate térmicas.
* **Solución Técnica:**
  * **Thread Pinning a Núcleos de Alto Rendimiento:** Uso de `sched_setaffinity` / `pthread_setaffinity_np` desde C/Rust identificando CPUs mediante `/sys/devices/system/cpu/cpu*/cpufreq/cpuinfo_max_freq`.
  * **Prioridad Real-Time:** Configuración de `sched_setscheduler` con política `SCHED_FIFO` o `SCHED_RR` (o nice value `-20`) en el hilo de render y audio.
* **Fuentes y Referencias Clave:**
  * *Google Android NDK Guide:* "CPU Affinity and Thread Scheduling on Android" – [developer.android.com/ndk/guides/neuralnetworks/cpu-affinity](https://developer.android.com/ndk/guides/neuralnetworks/cpu-affinity).
  * *Android Open Source Project (AOSP):* Documentación de `sched_setaffinity` y `taskset`.
  * *Reddit r/androiddev:* "Pinning audio and game loop threads to Big cores on Snapdragon/Dimensity".

### 2.3. Pipeline Gráfico OpenGL ES 3.0 & Vulkan (Optimizaciones TBDR)
* **Problema:** Las GPUs móviles (Qualcomm Adreno, ARM Mali, PowerVR) usan arquitecturas *Tile-Based Deferred Rendering* (TBDR). Llamadas como `glReadPixels`, lecturas de buffer síncronas o `glBufferData(..., null)` causan rotura de tiling (*tile thrashing*) y congelamientos de GPU.
* **Solución Técnica:**
  * **Persistent Mapped Buffers (GLES 3.0 / Vulkan):** `glBufferStorageEXT` o buffers anulares con doble/triple buffering para vértices de sprites.
  * **Evitar `glGetError` en Hot Paths:** Incurre en sincronización obligatoria CPU-GPU eliminando paralelismo del driver.
  * **Dynamic Sprite Batching con Instanced Rendering:** Agrupar quads en batches de hasta 4096 sprites con una sola llamada `glDrawElementsInstanced`.
* **Fuentes y Referencias Clave:**
  * *Qualcomm Adreno GPU Guide:* "Snapdragon Game Development Guide & Adreno Best Practices" – [developer.qualcomm.com](https://developer.qualcomm.com/software/snapdragon-game-toolkit).
  * *Arm Developer:* "Mali GPU Best Practices: Developer Guide (Tile-Based Rendering Architecture)" – [developer.arm.com/documentation](https://developer.arm.com/documentation/100019/latest/).
  * *GDC Vault:* "Tiled Shading and Mobile GPU Optimizations for 60FPS Native Runtimes".

### 2.4. Subsistema de Audio Ultra-Low Latency (AAudio / Oboe)
* **Problema:** El audio emulado en Android suele sufrir de subruns o latencias elevadas (>50ms) usando OpenSL ES o AudioTrack.
* **Solución Técnica:**
  * **AAudio Exclusive Mode:** Stream configurado con `AAUDIO_SHARING_MODE_EXCLUSIVE` y `AAUDIO_PERFORMANCE_MODE_LOW_LATENCY`.
  * **Ring Buffer SPSC Lock-Free:** Productor en hilo de audio de GameMaker (Rust), consumidor en el callback de audio nativo de AAudio (`rtrb` crate en Rust).
* **Fuentes y Referencias Clave:**
  * *Google Oboe Repo:* [github.com/google/oboe](https://github.com/google/oboe) y [Full Low Latency Guide](https://github.com/google/oboe/blob/main/docs/FullLatencyTuningGuide.md).
  * *Android Developers:* "Low Latency Audio with AAudio" – [developer.android.com/ndk/guides/audio/aaudio/aaudio](https://developer.android.com/ndk/guides/audio/aaudio/aaudio).

---

## 3. Puente JNI de Ultra Baja Latencia (Zero-Copy & 0ms Input)

### 3.1. Canal de Entrada Atómico (Lock-Free Input Dispatch)
* **Problema:** Enviar eventos de botones/joysticks desde Android UI a Rust mediante métodos JNI (`nativeOnTouchEvent(...)`) genera overhead de llamada JNI (30-100ns por evento) y contención de hilos.
* **Solución Técnica:**
  * **Memoria Compartida Atómica (`AtomicU32` / `AtomicU64`):** Kotlin y Rust comparten una región fija mapeada (direct byte buffer o `mmap`). La pulsación actualiza una máscara de bits atómica (`fetch_or` / `fetch_and`) con ordenamiento `Relaxed` o `Acquire`/`Release`.
  * Latencia de paso de entrada: **0 ms** sin llamadas JNI en el loop de eventos.
* **Fuentes y Referencias Clave:**
  * *Paper:* "Correct and Efficient Lock-Free FIFO Queues" (Maged M. Michael, Michael L. Scott).
  * *Repo Rust:* `crossbeam-queue` y `rtrb`.
  * *Blog:* "Fast JNI Calls and Critical Natives" (Android NDK `@FastNative` y `@CriticalNative`).

### 3.2. Paso de Texturas y Shaders sin Copias (`AHardwareBuffer`)
* **Problema:** Transferir texturas o buffers entre Kotlin y Rust mediante arrays de bytes duplica uso de RAM y dispara el Garbage Collector (GC).
* **Solución Técnica:**
  * Uso de `AHardwareBuffer` (API 26+) para vincular buffers nativos de GPU compartidos directamente con EGL (`EGLImageKHR`) o Vulkan (`VkImportAndroidHardwareBufferInfoANDROID`).
* **Fuentes y Referencias Clave:**
  * *Android NDK:* `ndk/system/hardware_buffer.h`.
  * *Google Samples:* `android/ndk-samples/camera-texture-view` y `vulkan-hardware-buffer`.

---

## 4. UI y HUD Táctil en Kotlin + Jetpack Compose

### 4.1. HUD Táctil Adaptativo Zero-Allocation
* **Problema:** Objetos `MotionEvent` y asignaciones en lambdas de Compose durante gestos táctiles provocan churn en memoria joven y pausas GC de Android Runtime (ART).
* **Solución Técnica:**
  * **Procesamiento de Batch de `MotionEvent`:** Reutilización de estructuras mutables recicladas (`Pool` de eventos).
  * **Dibujo Nativo en Compose:** Usar `drawBehind` o `Canvas` con buffers preasignados evitando recombinaciones de composables.
  * Sincronización con `Choreographer.postFrameCallback` para alinear frames al refresco real de la pantalla (60/90/120 Hz).
* **Fuentes y Referencias Clave:**
  * *Android Developers:* "Optimizing Compose Performance" – [developer.android.com/jetpack/compose/performance](https://developer.android.com/jetpack/compose/performance).
  * *Google Codelabs:* "High Refresh Rate & Motion Handling on Android".

---

## 5. Dónde y Cómo Buscar Recursos Avanzados (Guía para el Investigador)

Para recopilar más papers, repositorios y discusiones técnicas precisas, utiliza estos repositorios de conocimiento y términos de búsqueda especializados:

### 5.1. Repositorios de Código y Motores Abiertos
1. **OpenGMK y Runtimes de GameMaker:**
   * GitHub: `OpenGMK/OpenGMK` – motor base en Rust.
   * GitHub: `ENIGMA-Dev/enigma-dev` – compilador y runtime C++ alternativo de GameMaker (revisar sus implementaciones de colisión y batching).
   * GitHub: `UnderminersTeam/Underanalyzer` / `UndertaleModTool` – especificaciones de bytecode y estructuras de datos de GameMaker Studio y legado GM8.
2. **Emuladores de Alto Rendimiento en Rust:**
   * Repositorio de `dolphin-emu` (Dolphin Android JNI y pipeline Vulkan).
   * `yuzu-emu` / `suyu` (arquitecturas de asignación de memoria ARM64 y core pinning).
   * `skyline-emu` (Strata / skyline: emulación optimizada para Adreno con extensiones directas de GPU).

### 5.2. Comunidades Técnicas y Reddit
* **r/emudev:** Búsquedas clave:
  * `"spatial hash collision 2d"`
  * `"android big.little thread affinity"`
  * `"TBDR mobile tile thrashing OpenGL ES"`
* **r/rust:** Búsquedas clave:
  * `"real time audio lock-free ringbuffer"`
  * `"ARM NEON SIMD pixel collision"`
  * `"JNI zero copy direct byte buffer performance"`
* **r/androiddev:** Búsquedas clave:
  * `"16kb page alignment NDK"`
  * `"AHardwareBuffer zero copy rendering"`
  * `"Choreographer frame pacing Swappy"`

### 5.3. Búsquedas Específicas en Google / Google Scholar (Dorks Técnicos)
* `filetype:pdf "spatial hashing" "collision detection" "cache locality"`
* `site:developer.arm.com "Mali" "TBDR" "best practices" "tile memory"`
* `site:developer.qualcomm.com "Adreno" "persistent buffer" "subpass"`
* `"GameMaker" "runtime internals" OR "virtual machine" filetype:pdf`
* `"pthread_setaffinity_np" "big.LITTLE" "Android" core affinity`

---

## 6. Checklist de Implementación Inmediata

- [ ] **Core Pinning en `gm8emulator`:** Integrar llamada nativa `pthread_setaffinity_np` al iniciar el hilo principal de emulación para fijarlo en CPUs >= 4 (núcleos Gold/Prime).
- [ ] **Spatial Grid para `obj_pokemon` / Proyectiles:** Reemplazar el loop lineal de instancias por tabla hash espacial de celdas $64 \times 64$ píxeles.
- [ ] **Buffer de Entrada Atómico:** Migrar dispatch de controles desde JNI sincrónico a buffer de flags atómicos compartidos `AtomicU32`.
- [ ] **Optimizaciones de Shader en GLES 3.0:** Garantizar que los fragment shaders no usen descartes (`discard`) en caminos críticos para no deshabilitar el *Early-Z rejection* de las GPUs Mali y Adreno.

