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
| **REF-009** | 2026-10-30 | Mobile GPU | *PowerVR TBDR Architecture & Hidden Surface Removal (HSR)* | https://docs.imgtec.com/starter-guides/powervr-architecture/html/topics/tile-based-deferred-rendering-index.html | **ORO** | **Rompe-stalls en TBDR:** `discard` fragmentario bloquea ISP y HSR de hardware |
| **REF-010** | 2026-10-30 | DSP / SIMD | *64-bit ARM Optimization for Audio Signal Processing* (Superpowered) | https://superpowered.com/64-bit-arm-optimization-audio-signal-processing | **ORO** | Duplicación de registros ARM64 (32 GPRs + 32 VFP/NEON 128-bit) para hot-paths |
| **REF-011** | 2026-10-30 | DSP / NDK | *ARM NEON SIMD for Real-Time Audio in Android NDK* | https://dev.to/software_mvp-factory/arm-neon-simd-intrinsics-for-real-time-audio-processing-in-android-ndk-fpb | **ORO** | Ocultamiento de latencia *load-to-use* (3-4 ciclos) mediante unroll x2/x4 en NEON |
| REF-012 | 2026-10-30 | Mobile GPU | *Vulkan Mobile Development: Rendering Approaches* | https://docs.vulkan.org/tutorial/latest/Building_a_Simple_Engine/Mobile_Development/04_rendering_approaches.html | Incorporado | Subpasses y `STORE_OP_DONT_CARE` para retener datos en on-chip Tile Memory |
| REF-013 | 2026-10-30 | SIMD / ARM | *Arm Coding Best Practice: Optimizing with Neon SIMD* | https://support.arm.com/documentation/101458/2010/Coding-best-practice/Optimizing-C-C---code-with-Arm-SIMD--Neon | Incorporado | Reglas de alineación y dual-issue en microarquitecturas Cortex-A |
| REF-014 | 2026-10-30 | Engine / 2D | *Unity Mobile Optimisation Guide* | https://docs.unity3d.com/2019.4/Documentation/Manual/MobileOptimisation.html | Referencia | Batching de draw calls y reducción de fillrate/overdraw en 2D |

---

## 2. HALLAZGOS CRÍTICOS: "EL ORO" NO DOCUMENTADO PREVIAMENTE

### 🌟 ORO 1: El cuello de botella de `discard` / Alpha-Test en GPUs TBDR (PowerVR / Apple / Mali)
* **Descubrimiento:** Las GPUs TBDR cuentan con un procesador de síntesis de imagen por hardware (ISP) que realiza **Hidden Surface Removal (HSR)** *antes* de ejecutar el fragment shader, eliminando el 100% del overdraw de fragmentos ocultos.
* **El Fallo Crítico en 2D:** Si un fragment shader contiene la instrucción `discard` (típico para transparencia de sprites: `if (tex.a < 0.1) discard;`), el hardware **no puede saber si el fragmento será visible hasta que el shader termine**.
* **Impacto:** Provoca *deferred depth writes*, **desactiva completamente el HSR acelerado por hardware** y congela el pipeline de rasterización de la GPU (pipeline stall) para todos los triángulos posteriores del tile.
* **Regla de Oro para OpenGMK:**
  1. **Evitar `discard` en shaders de sprites 2D siempre que sea posible.** Usar Alpha Blending estándar (`glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA)`).
  2. Si el corte alfa es obligatorio, ordenar la cola de renderizado en 3 pases estrictos:
     $$\text{1. Opacos (100\% HSR)} \longrightarrow \text{2. Alpha-Tested} \longrightarrow \text{3. Blended (Transparentes)}$$
  3. Nunca intercalar quads con `discard` en medio del lote de renderizado de opacos.

---

### 🌟 ORO 2: Explotación del Banco de 32 Registros ARM64 (Superpowered Pattern)
* **Descubrimiento:** La transición de ARMv7 (32-bit) a ARM64 (AArch64) no solo amplía punteros a 64 bits; **duplica los registros físicos**:
  - 31 registros de propósito general de 64 bits (`x0`-`x30`) frente a 14 en ARM32.
  - 32 registros SIMD/punto flotante de 128 bits (`v0`-`v31`) frente a 16 o 32 de 64-bit (`d0`-`d31`).
  - Convención ABI AArch64: 8 registros para argumentos de función y 8 para floats/vectores.
* **Impacto en OpenGMK:**
  - En hot-paths de audio DSP y en el intérprete de bytecode, el compilador LLVM de Rust suele derramar variables locales a la pila (`stack spilling`) cuando faltan registros en x86 o ARM32.
  - Forzar funciones pequeñas (inlining agresivo en hot-paths del VM loop) permite que **todo el contexto de ejecución viva en registros CPU**, eliminando accesos al stack frame (`spill/reload`).

---

### 🌟 ORO 3: Ocultamiento de Latencia *Load-to-Use* en ARM NEON con Rust
* **Descubrimiento:** En los núcleos ARM Cortex-A (A55, A76, A78, etc.), una carga vectorial de memoria (`vld1q_f32` o `vld1q_u32`) tiene una latencia de **3 a 4 ciclos de reloj** antes de que el valor esté disponible para una operación aritmética (`vmulq_f32`, `vaddq_f32`).
* **El Error Común:**
  ```rust
  // Lento: la CPU se detiene esperando que cargue 'a' antes de multiplicar
  let a = vld1q_f32(ptr_a);
  let r = vmulq_f32(a, b); // STALL de 3 ciclos
  ```
* **Patrón de Alto Rendimiento (Interleaving & 4x Unroll):**
  Desenrollar 4 bloques vectoriales (16 floats / 16 instancias a la vez) e intercalar todas las cargas juntas:
  ```rust
  // Carga paralela: oculta la latencia de memoria
  let a0 = vld1q_f32(ptr.add(0));
  let a1 = vld1q_f32(ptr.add(4));
  let a2 = vld1q_f32(ptr.add(8));
  let a3 = vld1q_f32(ptr.add(12));
  // Las unidades de cómputo ejecutan sin stalls
  let r0 = vmulq_f32(a0, factor);
  let r1 = vmulq_f32(a1, factor);
  let r2 = vmulq_f32(a2, factor);
  let r3 = vmulq_f32(a3, factor);
  ```

---

## 3. Auditoría de Enlaces Proporcionados: Oro vs Ruido

| Enlace / Documento | Veredicto | Motivo Técnico |
|---|---|---|
| `docs.imgtec.com/.../powervr-architecture` | **ORO PURO** | Detalla el funcionamiento del ISP y el colapso del HSR ante `discard`. |
| `superpowered.com/64-bit-arm-optimization` | **ORO PURO** | Estrategia de registros ARM64 sin derrame a la pila para audio y DSP. |
| `dev.to/.../arm-neon-simd-intrinsics` | **ORO PURO** | Patrón completo NDK: AAudio + SPSC ringbuffer + NEON 128-bit unroll. |
| `docs.vulkan.org/.../rendering_approaches` | **ALTO VALOR** | Manejo de Tile Memory y attachments transitorios sin pisar la DRAM. |
| `support.arm.com/.../Arm-SIMD--Neon` | **ALTO VALOR** | Guía de microarquitectura de ejecución dual en ARM. |
| `docs.unity3d.com/.../MobileOptimisation` | **MEDIO VALOR** | Buenas prácticas generales de overdraw y batching 2D. |
| `scribd.com/.../Conceptos-Basicos-de-Videojuegos` | *RUIDO* | Contenido introductorio para principiantes (sin valor de bajo nivel). |
| `unipamplona.edu.co/.../guia03arduino.pdf` | *RUIDO* | Guía básica de Arduino/microcontroladores de 8 bits. |
| `slideshare.net/.../guia-estructuras-cclicas` | *RUIDO* | Nivel escolar sobre qué es un bucle `while`/`for`. |
| `apuntesdam.com/algoritmos` | *RUIDO* | Temario de FP general de algoritmos básicos de ordenamiento. |
| `meshy.ai/es/blog/game-loop` | *RUIDO* | Artículo divulgativo generalista sobre el game loop. |
