# RUST OPTIMIZATION PLAYBOOK V2 (OpenGMK / Android ARM64)

> **Enfoque:** Micro-optimizaciones, arquitectura de memoria, latencia de renderizado (GLES 3.0/Vulkan) y throughput del intérprete GML en arquitecturas móviles (ARM64/ARMv7 TBDR).

---

## 1. Matriz de Ingesta de Enlaces y Recursos

| ID | Fecha | Categoría | Fuente / Título | Enlace | Estado | Impacto Esperado |
|---|---|---|---|---|---|---|
| REF-001 | 2026-10-30 | Interpreter | *A tail-call interpreter in (nightly) Rust* (Matt Keeter) | https://www.mattkeeter.com/blog/2026-04-05-tailcall/ | Incorporado | Tail call dispatch con `become` vs match loop; supera ASM en ARM64 |
| REF-002 | 2026-10-30 | VM Memory | *NaN Boxing & Value Representation* (Bob Nystrom / Crafting Interpreters) | https://craftinginterpreters.com/optimization.html#nan-boxing | Incorporado | Reducción de 24/16 bytes a 8 bytes exactos por variable GM8 |
| REF-003 | 2026-10-30 | VM Memory | *value-lang: Safe 8-byte NaN-boxed runtime value in Rust* | https://github.com/value-lang/value-lang | Incorporado | Implementación Rust pura sin unsafe para dynamic values en stack |
| REF-004 | 2026-10-30 | Mobile GPU | *Arm Mali GPU Best Practices: Buffers & Memory* | https://developer.arm.com/documentation/101897/latest/Buffers/Buffer-allocation | Incorporado | Uncached write-only buffers, interleaving de vértices, eliminación de stalls |
| REF-005 | 2026-10-30 | Mobile GPU | *Qualcomm Adreno GPU Best Practices Guide* | https://developer.qualcomm.com | Incorporado | Binning pass vs Tile shading, GMEM spill prevention, Early-Z / LRZ |
| REF-006 | 2026-10-30 | Realtime Audio | *Real-time audio programming 101: time waits for nothing* (Ross Bencina) | http://www.rossbencina.com/code/real-time-audio-programming-101-time-waits-for-nothing | Incorporado | Regla cero-alloc / cero-mutex en audio callback AAudio |
| REF-007 | 2026-10-30 | Concurrency | *rtrb: Real-Time Ring Buffer (Lock-free SPSC)* | https://docs.rs/rtrb | Incorporado | SPSC wait-free ring buffer para streaming de audio y JNI events |
| REF-008 | 2026-10-30 | SIMD & Cache | *Flavors of SIMD & Efficient Jagged Arrays* (Arseny Kapoulkine / zeux) | https://zeux.io | Incorporado | SoA vs AoS para vectorización NEON y aplanamiento de grafos/arrays |
| REF-009 | 2026-10-30 | Mobile GPU | *PowerVR TBDR Architecture & Hidden Surface Removal (HSR)* | https://docs.imgtec.com/starter-guides/powervr-architecture/html/topics/tile-based-deferred-rendering-index.html | Incorporado | Rompe-stalls en TBDR: `discard` fragmentario bloquea ISP y HSR de hardware |
| REF-010 | 2026-10-30 | DSP / SIMD | *64-bit ARM Optimization for Audio Signal Processing* (Superpowered) | https://superpowered.com/64-bit-arm-optimization-audio-signal-processing | Incorporado | Duplicación de registros ARM64 (32 GPRs + 32 VFP/NEON 128-bit) para hot-paths |
| REF-011 | 2026-10-30 | DSP / NDK | *ARM NEON SIMD for Real-Time Audio in Android NDK* | https://dev.to/software_mvp-factory/arm-neon-simd-intrinsics-for-real-time-audio-processing-in-android-ndk-fpb | Incorporado | Ocultamiento de latencia *load-to-use* (3-4 ciclos) mediante unroll x2/x4 en NEON |
| **REF-015** | 2026-10-30 | Rust Perf | *Oxc Performance Guide (The JavaScript Oxidation Compiler)* | https://oxc.rs/docs/learn/performance | **ORO PURO** | **Técnicas de compilador:** Arena allocation total, structs compactos con `u32` vs `usize`, string interning |
| **REF-016** | 2026-10-30 | Mobile GPU | *Tailor Your Apps for Apple GPUs & TBDR* (Apple Developer) | https://developer.apple.com/documentation/metal/tailor-your-apps-for-apple-gpus-and-tile-based-deferred-rendering | **ORO** | **Memoryless attachments:** render targets transitorios que jamás tocan la DRAM física |
| REF-017 | 2026-10-30 | Mobile GPU | *PowerVR Performance Recommendations* (Imagination) | https://powervr-graphics.github.io/WebGL_SDK/WebGL_SDK/Documentation/Architecture%20Guides/PowerVR.Performance%20Recommendations.pdf | Incorporado | Prevención de saturación del *Parameter Buffer* (PB) reduciendo varyings en vertex shaders |

---

## 2. HALLAZGOS CRÍTICOS: "EL ORO" NO DOCUMENTADO PREVIAMENTE

### 🌟 ORO 4: Lecciones del Compilador Oxc para el Intérprete OpenGMK (Rust Puro)
* **Fuente:** *Oxc Performance Guide* (el parser de Rust más rápido del mundo).
* **1. Reemplazo de Punteros/`usize` de 64 bits por `u32`:**
  - En arquitecturas de 64 bits (ARM64 / x86_64), cada puntero o `usize` ocupa 8 bytes.
  - Almacenar offsets de scripts, IDs de instancias y referencias a variables en `u32` (4 bytes) reduce el tamaño de las estructuras del VM a la mitad.
  - **Ganancia directa:** Duplica la cantidad de opcodes e instancias que caben en una línea de caché L1 (64 bytes).
* **2. Arena Allocation con Reset Global de Tick (`bumpalo`):**
  - En lugar de alocar y liberar memoria dinámica para evaluar expresiones intermedias de GML, utilizar un *Frame Arena Allocator*.
  - Al final del tick (`step_event`), se resetea el puntero de la arena a cero en $O(1)$ sin llamar a `free()` ni desfragmentar la memoria.

### 🌟 ORO 5: Memoryless Attachments en TBDR (Apple / Mali / PowerVR)
* **Fuente:** *Tailor Your Apps for Apple GPUs and Tile-Based Deferred Rendering*.
* **Descubrimiento:** En GPUs TBDR, los buffers intermedios (como buffers de stencil para máscaras de sprites o buffers de profundidad para ordenamiento Z) pueden declararse como **Memoryless / Transient**.
* **Impacto:** Viven 100% dentro de la memoria on-chip (Tile Memory / GMEM). Nunca se les asigna memoria RAM física del teléfono, reduciendo el consumo de ancho de banda y batería a cero durante esos pases intermedios.

### 🌟 ORO 6: Saturación del Parameter Buffer (PB) en TBDR por Exceso de Varyings
* **Fuente:** *PowerVR Performance Recommendations*.
* **Descubrimiento:** Cada atributo que un vertex shader pasa al fragment shader (`varying` en GLSL ES) se almacena en el *Parameter Buffer* (PB) de la GPU móvil antes de rasterizar los tiles.
* **Impacto en OpenGMK:**
  - En sprites 2D, pasar datos innecesarios por vértice satura el PB y fuerza a la GPU a volcar tiles incompletos a la memoria principal (*mid-frame render pass flush*), congelando el renderizador.
  - Mantener la estructura del vértice compacta: solo `vec2 a_Position`, `vec2 a_TexCoord` y `vec4 a_Color` empaquetado en 4 bytes (`GL_UNSIGNED_BYTE`).
