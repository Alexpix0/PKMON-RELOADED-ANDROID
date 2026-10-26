# 🚀 ROADMAP v0.8.8: "LA REVOLUCIÓN DE LA ARQUITECTURA"
## *Ingeniería Extrema de Emulación, SIMD ARM NEON, Audio < 3ms, Snapshots Lock-Free y Cero Stutter*

---

## 📑 ÍNDICE GENERAL DEL ROADMAP v0.8.8

1. [Visión y Objetivos Cuantitativos de la Versión 0.8.8](#1-visión-y-objetivos-cuantitativos-de-la-versión-088)
2. [🎮 Bloque I: Pipeline Gráfico, Shaders Binarios y Compresión de VRAM](#2-🎮-bloque-i-pipeline-gráfico-shaders-binarios-y-compresión-de-vram)
   - [1.1 Caché Binaria Persistente de Shaders (`glProgramBinary` / `VkPipelineCache`)](#11-caché-binaria-persistente-de-shaders)
   - [1.2 Compresión de Texturas GPU (ASTC / ETC2) con Reducción del 60% de VRAM](#12-compresión-de-texturas-gpu-astc--etc2)
   - [1.3 Desbloqueo Seguro de `AHardwareBuffer` Staging Zero-Copy (API 26+)](#13-desbloqueo-seguro-de-ahardwarebuffer)
   - [1.4 Post-Procesado Ligero: Dual-Kawase Bloom 1/4 Res & Retro Subpixel LCD](#14-post-procesado-ligero-bloom-y-retro-lcd)
3. [⚡ Bloque II: Audio de Ultrabaja Latencia y Telemetría en Tiempo Real](#3-⚡-bloque-ii-audio-de-ultrabaja-latencia-y-telemetría-en-tiempo-real)
   - [2.1 Backend AAudio / Google Oboe (< 3 ms) con Modo Exclusivo](#21-backend-aaudio--google-oboe-exclusivo)
   - [2.2 Gestión de Desconexión en Caliente de Auriculares (Hot-Plug)](#22-gestión-de-desconexión-en-caliente)
   - [2.3 Telemetría Superpuesta Estilo MangoHud / RivaTuner (1% Low y Frametime Graph)](#23-telemetría-superpuesta-estilo-mangohud)
4. [🔬 Bloque III: Algoritmos O(1), Vectorización SIMD y Rebobinado Temporal](#4-🔬-bloque-iii-algoritmos-o1-vectorización-simd-y-rebobinado-temporal)
   - [3.1 Cuadrícula de Dispersión Espacial O(1) (*Spatial Hash Grid*) con Bitmasks](#31-cuadrícula-de-dispersión-espacial-o1)
   - [3.2 Vectorización ARM NEON (`<arm_neon.h>`) para Clima y Partículas](#32-vectorización-arm-neon-para-partículas)
   - [3.3 Rebobinado Temporal en Vivo (*Live Time Rewind* de 15 Segundos en Batalla)](#33-rebobinado-temporal-en-vivo-15s)
5. [🧙 Bloque IV: Superación de Fronteras de GM8 (Sistemas Desacoplados)](#5-🧙-bloque-iv-superación-de-fronteras-de-gm8)
   - [4.1 Threaded Game Loop Lock-Free SPSC con Interpolación 90/120 Hz](#41-threaded-game-loop-lock-free-spsc)
   - [4.2 AOT / Fast-Path Nativo para Bytecode GML Crítico](#42-aot--fast-path-nativo-gml)
6. [🗺️ Matriz de Verificación y Resiliencia](#6-🗺️-matriz-de-verificación-y-resiliencia)

---

## 1. Visión y Objetivos Cuantitativos de la Versión 0.8.8

La versión **0.8.7** resolvió el cuello de botella de renderizado en combates y eliminó la pantalla negra de Vulkan.
La versión **0.8.8** marca la transición a una arquitectura desacoplada de alto calibre, donde la CPU y GPU cooperan mediante estructuras libres de bloqueos (*lock-free*), aceleración vectorial nativa y latencia mínima de entrada/audio.

| Métrica | v0.8.7 | Meta v0.8.8 | Impacto |
| :--- | :---: | :---: | :--- |
| **Latencia de Audio** | ~35 ms | **< 3 ms (AAudio Exclusive Burst 96)** | Sonido instantáneo al pulsar botones |
| **Tiempo en Colisiones de Proyectiles** | 4 - 8 ms ($O(N \times M)$) | **< 0.05 ms ($O(1)$ Spatial Hash)** | Cero tirones en ataques masivos |
| **Procesamiento de Partículas Clima** | 18% CPU core | **< 3.5% CPU core (ARM NEON SIMD)** | 60 FPS estables bajo lluvia/tormenta |
| **Consumo de VRAM por Sprites** | ~240 MB | **< 95 MB (-60% ASTC/ETC2)** | Blindaje OOM en terminales de 2GB/3GB |
| **Transición de Shaders / Mapas** | 40 - 80 ms | **0 ms (Caché Binaria Persistente)** | Fin del hipo al cruzar puertas |
| **Tasa Visual en Paneles AMOLED** | 60 Hz fija | **90 / 120 Hz interpolados** | Suavidad cristalina sin romper la lógica GM8 |
| **Mapeo de Staging Zero-Copy** | Copias CPU->RAM->GPU | **AHardwareBuffer Directo** | Transferencias DMA directas |

---

## 2. 🎮 Bloque I: Pipeline Gráfico, Shaders Binarios y Compresión de VRAM

### 1.1 Caché Binaria Persistente de Shaders
* Almacena los ejecutables compilados de los shaders mediante `glProgramBinary` y `VkPipelineCache` en el directorio interno `code_cache/`.
* Clave de verificación de integridad: `SHA256(DriverVersion + GPU_Renderer + ShaderSource)`. Si el driver de GPU o la versión de la app cambia, la caché se invalida y regenera de manera transparente.

### 1.2 Compresión de Texturas GPU (ASTC / ETC2)
* Detección en tiempo de ejecución de las capacidades de descompresión por hardware del chip (Adreno, Mali, PowerVR).
* Carga directa de texturas en formatos nativos móviles `GL_COMPRESSED_RGBA8_ETC2_EAC` y `GL_COMPRESSED_RGBA_ASTC_4x4_KHR`, reduciendo el ancho de banda del bus de memoria y el calentamiento de la GPU.

### 1.3 Desbloqueo Seguro de `AHardwareBuffer`
* Encapsulación con enlace dinámico (`libc::dlsym` para API >= 26) para evitar caídas en terminales anteriores.
* Asignado exclusivamente como búfer de transferencia rápida (*staging*) y captura de pantalla/guardados rápidos, con canal de reserva estándar para dispositivos no compatibles.

### 1.4 Post-Procesado Ligero: Bloom y Retro LCD
* **Dual-Kawase Bloom a 1/4 Res:** Downsampling 4x con dos pasadas de filtrado Kawase para otorgar brillo a proyectiles y movimientos especiales legendarios con coste inferior a 0.2 ms en GPU.
* **Retro LCD / GBA Subpixel Grid:** Filtro óptico que emula la disposición RGB de las pantallas de Game Boy Advance / Nintendo DS con corrección cromática calibrada.

---

## 3. ⚡ Bloque II: Audio de Ultrabaja Latencia y Telemetría en Tiempo Real

### 2.1 Backend AAudio / Google Oboe Exclusivo
* Configuración con `AAUDIO_SHARING_MODE_EXCLUSIVE` y `AAUDIO_PERFORMANCE_MODE_LOW_LATENCY`.
* Búferes de ráfaga (*burst*) configurados a 96 muestras (a 48 kHz = 2.0 ms de latencia teórica).

### 2.2 Gestión de Desconexión en Caliente
* Callback de error registrado con `AAudioStreamBuilder_setErrorCallback`. Ante eventos `AAUDIO_ERROR_DISCONNECTED` (desconexión de auriculares jack/USB-C/Bluetooth), el stream se reconstruye sin bloquear el hilo del juego.

### 2.3 Telemetría Superpuesta Estilo MangoHud
* Visualizador superpuesto integrado que grafica el frametime histórico en tiempo real, marca las líneas objetivo de 16.66 ms y 8.33 ms, resalta caídas de 1% y 0.1% Low, y detalla la memoria consumida y llamadas de dibujado activas.

---

## 4. 🔬 Bloque III: Algoritmos O(1), Vectorización SIMD y Rebobinado Temporal

### 3.1 Cuadrícula de Dispersión Espacial O(1) (*Spatial Hash Grid*)
* Búsqueda espacial optimizada con máscaras de bits `uint64_t` preasignadas en memoria fija (cero asignaciones dinámicas por frame).
* Las entidades de combate y proyectiles se asignan a celdas de 32x32 píxeles, evaluando colisiones únicamente entre vecinos directos.

### 3.2 Vectorización ARM NEON para Partículas
* Reestructuración a Formato *Structure of Arrays (SoA)* alineado a 16 bytes.
* Actualización simultánea de 4 partículas en paralelo por ciclo con intrínsecos NEON `vaddq_f32`, `vmulq_f32` y `vfmaq_f32` con respaldo escalar automático para x86_64.

### 3.3 Rebobinado Temporal en Vivo (15s en Combate)
* Búfer circular de tamaño fijo (máximo 20 MB en RAM) que captura instantáneas delta periódicas de las instancias de batalla.
* Permite al jugador retroceder el combate ante errores críticos sin corromper el estado de la máquina de eventos.

---

## 5. 🧙 Bloque IV: Superación de Fronteras de GM8

### 4.1 Threaded Game Loop Lock-Free SPSC
* La lógica de GameMaker 8 permanece síncrona y segura a paso fijo (30 o 60 ticks/s).
* El estado visual resultante se empaqueta en un `RenderStateSnapshot` inmutable transferido por una cola atómica SPSC hacia el hilo de renderizado, el cual interpola $(x, y)$ a 90 o 120 FPS sin condiciones de carrera.

### 4.2 AOT / Fast-Path Nativo GML
* Tabla de saltos directos que desvía funciones y bucles críticos del bytecode GML (cálculo de daño, ordenamiento de profundidad, colisiones matemáticas) a funciones nativas compiladas en Rust/C++, reduciendo el overhead del intérprete.

---
