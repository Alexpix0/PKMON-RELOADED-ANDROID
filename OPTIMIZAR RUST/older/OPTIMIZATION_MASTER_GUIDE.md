# OPTIMIZATION_MASTER_GUIDE.md: Guía Maestra de Ingeniería de Rendimiento de Bajo Nivel
**Target de Compilación:** `libgm8emulator.so` (Android NDK - `aarch64-linux-android` / `armv7-linux-androideabi`)  
**Ecosistema:** Rust + LLVM + Android Native Development Kit (NDK)  
**Objetivo de Rendimiento:** Latencia de frame determinista (<16.6ms @ 60 FPS / <8.3ms @ 120 FPS), cero asignaciones dinámicas en ejecución continua, erradicación de fallos de caché L1/L2, predicción de saltos branchless y saturación de paralelismo SIMD (ARM NEON).

---

## ÍNDICE GENERAL
1. **Directivas de Compilación y Perfil Release (`Cargo.toml` & NDK)**
2. **Literatura y Documentos Técnicos Fundamentales**
   - 2.1. The Rust Performance Book (Nicholas Nethercote et al.)
   - 2.2. "What Every Programmer Should Know About Memory" (Ulrich Drepper)
   - 2.3. "Algorithms for Modern Hardware" (Sergey Slotin)
   - 2.4. "Rust Atomics and Memory Ordering" (Mara Bos)
3. **Conferencias Técnicas y Análisis de Rendimiento**
   - 3.1. Casey Muratori: "Clean" Code, Horrible Performance
   - 3.2. Nicholas Nethercote: "In Pursuit of Rustc Speed"
   - 3.3. Jon Gjengset: "Crust of Rust: Profiling & Benchmarking"
   - 3.4. Matt Godbolt: "The Bits Between the Bits: How Compilers Optimize Code"
   - 3.5. Fedor Pikus: "Branchless Programming in Systems Code"
4. **Catálogo de Buenas Prácticas (Síntesis Leapcell)**
   - 4.1. Gestión de memoria y supresión de asignaciones en heap
   - 4.2. Iteradores vs. bucles y eliminación de bounds checks
   - 4.3. Estructuras óptimas y omisión de SipHash
   - 4.4. Control de inlining y segregación de ramas frías
   - 4.5. Layout de memoria, alineación y eliminación de False Sharing
5. **Trampas Críticas de Rendimiento (Síntesis Llogiq)**
   - 5.1. El costo oculto de `.clone()`
   - 5.2. La trampa de `.collect()`
   - 5.3. Indirección y pérdida de localidad (Pointer Chasing)
   - 5.4. Sobrecarga de monomorfización (Code Bloat en L1i)
   - 5.5. Manejo de cadenas vs. slices de bytes planos
6. **Módulos Técnicos Críticos Aterrizados al Motor (`libgm8emulator.so`)**
   - 6.1. Profile-Guided Optimization (PGO) en Android NDK (End-to-End)
   - 6.2. Branchless Instruction Dispatch (LUTs alineadas vs. match masivo)
   - 6.3. Erradicación de Bounds Checks en la Memoria Emulada (Power-of-2 MMU)
   - 6.4. Vectorización SIMD / ARM NEON para Scanline Rendering (PPU)
   - 6.5. Frontera JNI y Despliegue de Framebuffer Zero-Copy (`ANativeWindow`)
7. **Herramientas de Auditoría, Profiling e Inspección de Ensamblador**
   - 7.1. Inspección de ensamblador con `cargo-show-asm`
   - 7.2. Auditoría en Compiler Explorer (Godbolt) para ARM64
   - 7.3. Perfilado de hardware con `simpleperf` en Android NDK
8. **Arquitectura y Ejecución Nativa en Android NDK (Mobile Deep Dive)**
   - 8.1. Arquitectura de memoria móvil y soporte de páginas de 16 KB (Android 15+)
   - 8.2. Zero-Copy Native Library Loading (`extractNativeLibs = false`)
   - 8.3. Subsistema gráfico nativo de ultra-alto rendimiento (`ANativeWindow`)
   - 8.4. Audio de ultra-baja latencia con AAudio / Oboe y ring buffer lock-free
   - 8.5. Sincronización, Frame Pacing y mitigación de judder (AGDK Swappy / VSYNC)
   - 8.6. Optimización de tiempos de CI en GitHub Actions (`cargo-binstall` & caches)
9. **Arquitectura de Alto Rendimiento para Emulación / Port del Runtime de GameMaker 8 en Rust**
   - 9.1. Gestión de variables dinámicas con NaN-Boxing
   - 9.2. Pipeline de renderizado 2D: Dynamic Sprite Batching
   - 9.3. Sistema espacial de colisiones y bitmasking de 64 bits
   - 9.4. Diseño Orientado a Datos (SoA vs. AoS) para el loop de eventos
   - 9.5. Carga eficiente de recursos con Memory-Mapped I/O (`memmap2`)
10. **Compendio de Ingeniería Avanzada: Literatura, Blogs de Autores Clave y Patrones Críticos en Rust**
   - 10.1. Literatura técnica fundamental (The Rustonomicon, `std::simd`, Criterion.rs)
   - 10.2. Artículos técnicos de autores destacados (Matklad, Cliff L. Biffle, Fasterthanlime)
   - 10.3. Catálogo de patrones de alto rendimiento en Rust para sistemas y emulación
11. **Plantilla Estandarizada para Futuras Incorporaciones**

---

## 1. Directivas de Compilación y Perfil Release (`Cargo.toml` & NDK)

Para extraer el rendimiento máximo del compilador `rustc` y del backend LLVM sobre núcleos ARM Cortex (arquitecturas big.LITTLE / DynamIQ de Android), la configuración del perfil de release debe forzar análisis inter-módulos globales y eliminar sobrecostes de desenrollado de pila.

### 1.1. Configuración de `Cargo.toml`

```toml
[profile.release]
# Nivel 3 de optimización: vectorización automática agresiva, fusión de bucles y unrolling
opt-level = 3

# Link-Time Optimization (LTO) completo: permite inlining entre crates independientes
lto = "fat"

# Reduce la generación de código a una única unidad de compilación, permitiendo
# a LLVM aplicar propagación de constantes y dead-code elimination a nivel global
codegen-units = 1

# Elimina el runtime de excepciones EH (unwinding). Los panics finalizan el proceso
# inmediatamente (abort), reduciendo el tamaño del binario y eliminando tablas EH de la caché L1i.
panic = "abort"

# Remueve símbolos de depuración del binario final para reducir el footprint de memoria
strip = "symbols"

# Deshabilita comprobaciones aritméticas de desbordamiento en release
overflow-checks = false

# Deshabilita aserciones de depuración
debug-assertions = false
```

### 1.2. Flags LLVM y Arquitectura ARM en `.cargo/config.toml`

```toml
[target.aarch64-linux-android]
rustflags = [
    "-C", "target-feature=+neon,+fp-armv8",
    "-C", "link-arg=-Wl,--gc-sections",
    "-C", "link-arg=-Wl,-Bsymbolic",
    "-C", "link-arg=-Wl,--no-undefined",
]

[target.armv7-linux-androideabi]
rustflags = [
    "-C", "target-cpu=cortex-a7",
    "-C", "target-feature=+v7,+vfp3-d16,+neon",
    "-C", "link-arg=-Wl,--gc-sections",
    "-C", "link-arg=-Wl,-Bsymbolic",
]
```

* **`-C target-feature=+neon`:** Activa el motor vectorial SIMD de 128 bits para el pipeline de video (PPU) y audio (APU).
* **`-Wl,-Bsymbolic`:** Enlace directo intra-biblioteca. En Android, evita que las llamadas a funciones internas de `libgm8emulator.so` pasen por la PLT (*Procedure Linkage Table*), suprimiendo saltos indirectos innecesarios.

---

## 2. Literatura y Documentos Técnicos Fundamentales

### 2.1. The Rust Performance Book (Nicholas Nethercote et al.)
* **Referencia:** [https://nnethercote.github.io/perf-book/](https://nnethercote.github.io/perf-book/)
* **Fundamento Teórico:**
  - El perfilado empírico precede a la optimización: las conjeturas sobre cuellos de botella suelen ser erróneas.
  - La representación de memoria define el rendimiento: reorganizar los campos de una estructura para minimizar el *padding* y alinearla con el ancho de palabra de la CPU reduce el tráfico en el bus de memoria.
  - Evitar asignaciones invisibles producidas por conversiones implícitas o clones defensivos.

```rust
// ============================================================================
// ❌ ANTIPATRÓN: Struct con padding masivo y orden subóptimo de campos
// ============================================================================
pub struct InefficientRegisters {
    pub zero_flag: bool,   // 1 byte + 3 bytes padding
    pub pc: u32,           // 4 bytes
    pub carry_flag: bool,  // 1 byte + 7 bytes padding
    pub cycle_count: u64,  // 8 bytes
    pub interrupt: bool,   // 1 byte + 7 bytes padding
} // Tamaño total: 32 bytes (17 bytes útiles, 15 bytes desperdiciados en padding)

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Alineación descendente contigua sin padding
// ============================================================================
#[repr(C)]
pub struct OptimizedRegisters {
    pub cycle_count: u64,  // 8 bytes (offset 0)
    pub pc: u32,           // 4 bytes (offset 8)
    pub zero_flag: bool,   // 1 byte  (offset 12)
    pub carry_flag: bool,  // 1 byte  (offset 13)
    pub interrupt: bool,   // 1 byte  (offset 14)
    pub _reserved: u8,     // 1 byte padding explícito para cerrar a 16 bytes
} // Tamaño total: 16 bytes exactos (cabe completo en un cuarto de línea de caché L1 de 64 bytes)
```

---

### 2.2. "What Every Programmer Should Know About Memory" (Ulrich Drepper)
* **Referencia:** [https://people.freebsd.org/~lstewart/articles/cpumemory.pdf](https://people.freebsd.org/~lstewart/articles/cpumemory.pdf)
* **Fundamento Teórico:**
  - **Jerarquía de latencias:** Acceder a L1 (~4 ciclos / 1 ns), L2 (~12 ciclos / 3 ns), L3 (~40 ciclos / 12 ns), DRAM principal (~200 ciclos / 70 ns).
  - **Líneas de Caché (64 bytes):** La CPU nunca carga bytes individuales de la memoria principal; carga ráfagas de 64 bytes contiguos alineados a 64 bytes.
  - **Translation Lookaside Buffer (TLB):** Traduce direcciones de memoria virtual a física. Saltos aleatorios a través de buffers fragmentados causan *TLB misses*, forzando al hardware a realizar un costoso *page table walk*.
  - **Impacto en Emulación:** La RAM (64KB - 4MB) y VRAM deben ser bloques continuos lineales. El acceso a matrices multidimensionales dispersas destruye la prebúsqueda por hardware (*hardware prefetcher*).

```rust
// ============================================================================
// ❌ ANTIPATRÓN: Memoria de video representada mediante dispersión de punteros
// ============================================================================
pub struct DispersedVram {
    // Matriz de vectores: cada fila es una asignación separada en el heap
    // Los píxeles adyacentes verticalmente residen en direcciones virtuales arbitrarias.
    // Resultado: Cache miss y TLB miss por cada línea de escaneo.
    pub scanlines: Vec<Vec<u32>>,
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Buffer lineal contiguo alineado a línea de caché
// ============================================================================
pub const VRAM_WIDTH: usize = 256;
pub const VRAM_HEIGHT: usize = 240;

#[repr(align(64))]
pub struct ContiguousVram {
    // Array plano continuo de 256 * 240 = 61,440 elementos u32 (240 KB contiguos)
    // El hardware prefetcher de ARM detecta accesos secuenciales y precarga L1D sin latencia.
    pub pixels: [u32; VRAM_WIDTH * VRAM_HEIGHT],
}

impl ContiguousVram {
    #[inline(always)]
    pub fn get_pixel(&self, x: usize, y: usize) -> u32 {
        // Cálculo aritmético plano: dirección base + (y * stride + x) * 4
        self.pixels[y * VRAM_WIDTH + x]
    }
}
```

---

### 2.3. "Algorithms for Modern Hardware" (Sergey Slotin)
* **Referencia:** [https://en.algorithmica.org/hpc/](https://en.algorithmica.org/hpc/)
* **Fundamento Teórico:**
  - **Instruction-Level Parallelism (ILP):** Los procesadores superescalares modernos (como Cortex-A78/X1) tienen múltiples puertos de ejecución (ALUs, FPUs, Load/Store units) capaces de despachar 4 a 8 micro-operaciones por ciclo si no existen dependencias de datos (*Read-After-Write*).
  - **Branchless Programming:** Las bifurcaciones impredecibles detienen el pipeline y vacían las instrucciones especulativas. Emplear operaciones lógicas y selección condicional (`csel` en ARM64) mantiene la tubería de instrucciones llena.

```rust
// ============================================================================
// ❌ ANTIPATRÓN: Actualización de flags de CPU mediante bifurcaciones condicionales
// ============================================================================
pub fn update_zero_and_sign_flags_branchy(result: u8, flags: &mut u8) {
    // Genera 2 instrucciones condicionales (b.ne / b.pl)
    // Provoca fallos en el predictor de saltos ante resultados aleatorios
    if result == 0 {
        *flags |= 0x01; // Zero flag
    } else {
        *flags &= !0x01;
    }

    if (result as i8) < 0 {
        *flags |= 0x80; // Sign flag
    } else {
        *flags &= !0x80;
    }
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Operaciones aritméticas y máscaras de bits branchless
// ============================================================================
#[inline(always)]
pub fn update_zero_and_sign_flags_branchless(result: u8, flags: &mut u8) {
    // Compila directamente a instrucciones lógicas sin saltos (eor, lsr, orr)
    let is_zero = ((result == 0) as u8) & 0x01;
    let is_negative = (result >> 7) & 0x01;

    // Limpia bits 0 y 7, e inyecta los nuevos flags mediante OR bit a bit
    *flags = (*flags & !0x81) | is_zero | (is_negative << 7);
}
```

---

### 2.4. "Rust Atomics and Memory Ordering" (Mara Bos)
* **Referencia:** [https://marabos.nl/atomics/](https://marabos.nl/atomics/)
* **Fundamento Teórico:**
  - Los mutexes (`std::sync::Mutex`) en Android delegan en llamadas al sistema operativo (`futex`), suspendiendo el hilo en el kernel ante contención. Esto arruina la tasa de refresco de 60/120 FPS.
  - La comunicación entre el hilo de emulación de la CPU y los hilos de renderizado de video o streaming de audio debe resolverse mediante primitivas atómicas sin bloqueo (*lock-free*).
  - **Memory Orderings:**
    * `Relaxed`: Garantiza atomicidad de lectura/escritura sin barreras de ordenamiento ni sincronización de caché entre hilos.
    * `Release`: Todas las escrituras de memoria previas en el hilo actual son visibles para otros hilos que realicen un `Acquire`.
    * `Acquire`: Garantiza que las lecturas subsecuentes vean los valores sincronizados por el `Release` del otro hilo.
    * `SeqCst`: Consistencia secuencial global; introduce pesadas barreras de memoria (`dmb ish` en ARM64) que degradan el throughput.

```rust
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub const FRAME_BUFFER_SIZE: usize = 256 * 240;

// Estado compartido lock-free entre el hilo del emulador y el hilo de video Android
pub struct LockFreeFrameBuffer {
    pub raw_pixels: [u32; FRAME_BUFFER_SIZE],
    pub frame_ready: AtomicBool,
    pub frame_counter: AtomicUsize,
}

impl LockFreeFrameBuffer {
    pub const fn new() -> Self {
        Self {
            raw_pixels: [0xFF000000; FRAME_BUFFER_SIZE],
            frame_ready: AtomicBool::new(false),
            frame_counter: AtomicUsize::new(0),
        }
    }

    /// Llamado exclusivamente por el hilo de CPU/PPU del emulador
    #[inline(always)]
    pub fn publish_frame(&mut self) {
        // Incrementa contador con ordenamiento relajado (estadística interna)
        self.frame_counter.fetch_add(1, Ordering::Relaxed);
        
        // Release: garantiza que todas las escrituras a raw_pixels estén completas
        // en la caché coherente antes de que la bandera pase a true.
        self.frame_ready.store(true, Ordering::Release);
    }

    /// Llamado por el hilo de la Surface de Android (ANativeWindow)
    #[inline(always)]
    pub fn try_consume_frame<F>(&self, mut render_fn: F) -> bool 
    where
        F: FnMut(&[u32; FRAME_BUFFER_SIZE]),
    {
        // Acquire: garantiza que si lee true, todas las escrituras previas del productor son visibles
        if self.frame_ready.swap(false, Ordering::Acquire) {
            render_fn(&self.raw_pixels);
            true
        } else {
            false
        }
    }
}
```

---

## 3. Conferencias Técnicas y Análisis de Rendimiento

### 3.1. Casey Muratori: "Clean" Code, Horrible Performance
* **Referencia:** Conferencia técnica (YouTube / Substack: *"Clean" Code, Horrible Performance*).
* **Fundamento Teórico:**
  - Las abstracciones orientadas a objetos tradicionales (polimorfismo masivo mediante jerarquías de clases, encapsulamiento granular de propiedades y funciones virtuales) imponen penalizaciones severas:
    1. Vtables y llamadas indirectas que el compilador no puede resolver en tiempo de compilación.
    2. Imposibilidad de vectorizar bucles heterogéneos.
    3. Destrucción de la localidad de datos (*Data-Oriented Design*).
  - En un emulador, procesar entidades (sprites, canales de audio, timers) debe estructurarse como operaciones sobre arrays homogéneos continuos (*Structure of Arrays* o arrays planos contiguos) procesados de forma tabular.

```rust
// ============================================================================
// ❌ ANTIPATRÓN: OOP "Clean Code" con jerarquías polimórficas y vtables
// ============================================================================
pub trait RenderableEntity {
    fn update_and_render(&mut self, screen: &mut [u32]);
}

pub struct SceneGraphBad {
    pub entities: Vec<Box<dyn RenderableEntity>>, // Vtable indirection + punteros dispersos
}

impl SceneGraphBad {
    pub fn draw_frame(&mut self, screen: &mut [u32]) {
        for entity in &mut self.entities {
            entity.update_and_render(screen); // Llamada indirecta por cada objeto
        }
    }
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Data-Oriented Design (DOD) sobre arrays planos
// ============================================================================
pub const MAX_SPRITES: usize = 64;

#[repr(C)]
pub struct SpritePoolDOD {
    pub x: [u8; MAX_SPRITES],
    pub y: [u8; MAX_SPRITES],
    pub tile_id: [u8; MAX_SPRITES],
    pub flags: [u8; MAX_SPRITES],
}

impl SpritePoolDOD {
    // Ejecución continua: los atributos x e y están contiguos en memoria (caben en 1 línea de caché)
    #[inline(always)]
    pub fn render_scanline(&self, scanline: u8, scanline_buffer: &mut [u32; 256]) {
        for i in 0..MAX_SPRITES {
            let sy = self.y[i];
            if sy <= scanline && scanline < sy + 8 {
                let sx = self.x[i] as usize;
                let tile = self.tile_id[i];
                // Renderizado directo con acceso predecible
                scanline_buffer[sx.min(255)] = tile as u32;
            }
        }
    }
}
```

---

### 3.2. Nicholas Nethercote: "In Pursuit of Rustc Speed"
* **Referencia:** Blog técnico y conferencias en RustConf.
* **Fundamento Teórico:**
  - El algoritmo criptográfico estándar `SipHash-1-3` en `HashMap` penaliza severamente los lookups en compiladores y emuladores.
  - La sustitución sistemática por algoritmos de hashing orientados a enteros pequeños (`FxHash` o `ahash`) reduce los ciclos de hashing en más del 70%.
  - Erradicar estructuras intermedias de asignación corta previene la fragmentación del asignador de memoria global.

```rust
// ============================================================================
// ❌ ANTIPATRÓN: std::collections::HashMap con SipHash para MMIO y Breakpoints
// ============================================================================
use std::collections::HashMap;

pub struct DebuggerBad {
    pub breakpoints: HashMap<u32, u8>, // Sobrecoste criptográfico en cada fetch
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: rustc_hash::FxHashMap (multiplicación y rotación)
// ============================================================================
use rustc_hash::FxHashMap;

pub struct DebuggerOptimized {
    pub breakpoints: FxHashMap<u32, u8>, // Hashing ultra rápido para enteros de 32 bits
}

impl DebuggerOptimized {
    #[inline(always)]
    pub fn is_breakpoint(&self, pc: u32) -> bool {
        self.breakpoints.contains_key(&pc)
    }
}
```

---

### 3.3. Jon Gjengset: "Crust of Rust: Profiling & Benchmarking"
* **Referencia:** Conferencia técnica (YouTube: *Crust of Rust: Profiling & Benchmarking*).
* **Fundamento Teórico:**
  - Los benchmarks manuales basados en `std::time::Instant` sufren de ruido por cambios de frecuencia del procesador (*DVFS* en Android), interrupciones del kernel y optimizaciones de eliminación de código muerto de LLVM.
  - Es imperativo aislar las mediciones usando `criterion::black_box` para forzar a LLVM a materializar los valores de entrada y salida sin eliminar las computaciones intermedias.

```rust
// Configuración de micro-benchmark estable en benches/cpu_dispatch.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

pub fn benchmark_instruction_execution(c: &mut Criterion) {
    let mut opcode_stream = [0xEA; 1000]; // Stream de NOPs
    let mut cpu_pc = 0usize;

    c.bench_function("cpu_step_nop_stream", |b| {
        b.iter(|| {
            // black_box impide que LLVM optimice y colapse el bucle a un valor constante
            let op = black_box(opcode_stream[cpu_pc]);
            cpu_pc = (cpu_pc + 1) % 1000;
            black_box(op.wrapping_add(1));
        })
    });
}

criterion_group!(benches, benchmark_instruction_execution);
criterion_main!(benches);
```

---

### 3.4. Matt Godbolt: "The Bits Between the Bits: How Compilers Optimize Code"
* **Referencia:** Conferencia CppCon / YouTube.
* **Fundamento Teórico:**
  - El backend LLVM ejecuta análisis estricto de alias de memoria (*Strict Aliasing*). Si dos punteros son de tipos distintos o se garantiza mediante `&mut` que no solapan, LLVM asume que no hay mutación cruzada y puede almacenar variables en registros en lugar de escribir a RAM en cada ciclo.
  - Para que LLVM auto-vectorice un bucle (Loop Vectorizer), el número de iteraciones debe ser predecible o múltiplo del tamaño de registro vectorial, y no debe existir posibilidad de salir anticipadamente por pánico.

```rust
// ============================================================================
// ❌ ANTIPATRÓN: Aliasing potencial que fuerza a LLVM a recargar memoria
// ============================================================================
pub fn accumulate_samples_bad(samples: &[i16], total: &mut i32) {
    for &sample in samples {
        // En cada iteración, LLVM debe escribir a *total en memoria porque
        // no puede demostrar formalmente que total no solapa con el slice samples.
        *total += sample as i32;
    }
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Registro local acumulador sin writes a RAM
// ============================================================================
#[inline(always)]
pub fn accumulate_samples_optimized(samples: &[i16], total: &mut i32) {
    // Variable acumuladora local mantenida estrictamente en un registro de CPU (x0 / w0)
    let mut acc: i32 = *total;
    for &sample in samples {
        acc += sample as i32;
    }
    // Una única escritura final a memoria
    *total = acc;
}
```

---

### 3.5. Fedor Pikus: "Branchless Programming in Systems Code"
* **Referencia:** Conferencia CppCon / YouTube.
* **Fundamento Teórico:**
  - El costo de una rama condicional no es la instrucción en sí (`b.eq`), sino la penalización asociada a una predicción errónea (*branch misprediction*).
  - En arquitecturas fuera de orden (*Out-of-Order Execution*) como ARM Cortex-A78/X1, una bifurcación fallida vacía entre 15 y 20 etapas de pipeline, descartando todas las instrucciones especulativas.
  - Sustituir sentencias condicionales `if-else` por máscaras aritméticas o instrucciones condicionales directas (`csel` / `cset` / `cinc` en ARM64).

```rust
// ============================================================================
// ❌ ANTIPATRÓN: Clamp condicional con ramas bifurcadas en mezcla de audio
// ============================================================================
pub fn clamp_audio_sample_branchy(sample: i32) -> i16 {
    // 2 ramas de predicción que fallan aleatoriamente con señales de audio dinámicas
    if sample > i16::MAX as i32 {
        i16::MAX
    } else if sample < i16::MIN as i32 {
        i16::MIN
    } else {
        sample as i16
    }
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Clamp branchless intrínseco (compila a smin/smax)
// ============================================================================
#[inline(always)]
pub fn clamp_audio_sample_branchless(sample: i32) -> i16 {
    // En Rust, clamp compila directamente a instrucciones vectoriales/escalares smax y smin en ARM64
    sample.clamp(i16::MIN as i32, i16::MAX as i32) as i16
}
```

---

## 4. Catálogo de Buenas Prácticas (Síntesis Leapcell)

### 4.1. Gestión de Memoria y Supresión de Asignaciones en el Heap
* **Causa:** En Android, los allocators nativos (`scudo` / `jemalloc`) introducen contención de cerraduras y transiciones al kernel (`mmap`). En un loop a 60 FPS, cualquier viaje al heap produce jitter en el frametime.
* **Solución:** Pre-reserva (`with_capacity`), reciclaje de estructuras (`.clear()`) y uso de `SmallVec` / `ArrayVec` para el stack frame.

```rust
// ❌ ANTIPATRÓN
pub fn process_irqs_bad() -> Vec<u8> {
    let mut vec = Vec::new(); // Asigna dinámicamente en el heap
    vec.push(0x01);
    vec
}

// ✅ IMPLEMENTACIÓN OPTIMIZADA
use smallvec::SmallVec;

#[inline(always)]
pub fn process_irqs_opt() -> SmallVec<[u8; 8]> {
    let mut vec = SmallVec::new(); // Reside estrictamente en el stack
    vec.push(0x01);
    vec
}
```

---

### 4.2. Iteradores vs. Bucles y Eliminación de Bounds Checks
* **Causa:** La indexación directa `slice[i]` evalúa en cada paso `i < slice.len()`, emitiendo `cmp` + saltos condicionales a pánico, lo que inhibe la auto-vectorización por parte de LLVM.
* **Solución:** Iteradores con tamaños conocidos (`zip`, `chunks_exact`).

```rust
// ❌ ANTIPATRÓN: Comprobaciones continuas de límites en el hot loop de video
pub fn fill_pixels_bad(dest: &mut [u32], colors: &[u32]) {
    for i in 0..dest.len() {
        dest[i] = colors[i]; // Bounds check redundante en cada iteración
    }
}

// ✅ IMPLEMENTACIÓN OPTIMIZADA: Zip infalible auto-vectorizable a NEON
#[inline(always)]
pub fn fill_pixels_opt(dest: &mut [u32], colors: &[u32]) {
    dest.iter_mut().zip(colors.iter()).for_each(|(d, &c)| *d = c);
}
```

---

### 4.3. Estructuras Óptimas y Omisión de SipHash
* **Causa:** `std::collections::HashMap` usa SipHash-1-3 para mitigar ataques DoS. En emulación local, introduce decenas de ciclos de CPU de latencia innecesaria.
* **Solución:** `rustc_hash::FxHashMap` o `ahash` para mapeos dispersos, y arrays planos `[u8; N]` para memoria física.

---

### 4.4. Control de Inlining y Segregación de Ramas Frías
* **Causa:** Inlinear funciones gigantescas de manejo de errores, panics o volcados de registro dentro del loop de dispatch infla el código ejecutable y expulsa las instrucciones calientes de la caché L1i.
* **Solución:** Marcar el despacho con `#[inline(always)]` y las rutas de error con `#[cold]` y `#[inline(never)]`.

```rust
pub struct Cpu;

impl Cpu {
    #[inline(always)]
    pub fn step_instruction(&mut self, opcode: u8) {
        match opcode {
            0xEA => self.op_nop(),
            _ => self.handle_illegal_opcode(opcode),
        }
    }

    #[inline(always)]
    fn op_nop(&mut self) {}

    #[cold]
    #[inline(never)]
    fn handle_illegal_opcode(&mut self, op: u8) {
        panic!("Opcode inválido: 0x{:02X}", op);
    }
}
```

---

### 4.5. Layout de Memoria, Alineación y Eliminación de False Sharing
* **Causa:** Si el hilo de CPU y el hilo de renderizado modifican variables ubicadas dentro de la misma línea de caché de 64 bytes, los núcleos de la CPU invalidan recíprocamente sus cachés L1D (protocolo MESI), degradando el paralelismo.
* **Solución:** Separación explícita mediante `#[repr(align(64))]`.

```rust
#[repr(align(64))]
pub struct CpuContext {
    pub cycles: u64,
    pub pc: u32,
}

#[repr(align(64))]
pub struct PpuContext {
    pub current_scanline: u32,
    pub frame_ready: bool,
}
```

---

## 5. Trampas Críticas de Rendimiento (Síntesis Llogiq)

### 5.1. El Costo Oculto de `.clone()`
* **Causa:** Duplicar estructuras (`Vec<u8>`) de bancos de ROM o tablas de patrones para sortear restricciones del borrow checker desencadena `malloc` + `memcpy`.
* **Solución:** Préstamos contiguos (`&[u8]`) o arrays de longitud fija que implementan `Copy`.

---

### 5.2. La Trampa de `.collect()`
* **Causa:** Invocar `.collect::<Vec<_>>()` en cada scanline (240+ veces por frame) materializa colecciones intermedias en el heap, saturando el asignador y destruyendo la memoria caché.
* **Solución:** Procesamiento en línea sobre buffers mutables reutilizables de longitud fija.

---

### 5.3. Indirección y Pérdida de Localidad (*Pointer Chasing*)
* **Causa:** El despacho dinámico `Box<dyn Trait>` impone doble indirección (objeto -> vtable -> función) y genera fallos en el predictor de saltos indirectos de ARM.
* **Solución:** Polimorfismo cerrado mediante `enum` sin vtables ni saltos indirectos.

---

### 5.4. Sobrecarga de Monomorfización (*Code Bloat* en L1i)
* **Causa:** Funciones genéricas gigantes (`fn run<M: Bus>(bus: &mut M)`) generan una copia binaria completa por cada tipo `M`, saturando los 32KB/64KB de la caché de instrucciones L1i.
* **Solución:** Separación de interfaces genéricas delgadas (*thin generics*) delegando a un núcleo no genérico compilado una sola vez.

---

### 5.5. Manejo de Cadenas vs. Slices de Bytes Planos
* **Causa:** `String` y `&str` fuerzan validaciones continuas de codificación UTF-8. En emuladores de hardware, los datos son bytes puros.
* **Solución:** Operar estrictamente sobre `&[u8]` o tipos escalares `u32`/`u64` para verificaciones de cabeceras binarias.

---

## 6. Módulos Técnicos Críticos Aterrizados al Motor (`libgm8emulator.so`)

### 6.1. Profile-Guided Optimization (PGO) en Android NDK (End-to-End)

El optimizador guiado por perfiles (PGO) permite a LLVM reestructurar el diseño de las funciones en el binario final, alineando los bloques básicos de ensamblador de acuerdo a las ramas que realmente se toman durante la ejecución de los juegos, y reduciendo al mínimo los *icache misses*.

#### Flujo de Trabajo Paso a Paso:

```bash
# ----------------------------------------------------------------------------
# PASO 1: Compilación instrumentada para generar perfiles de ejecución
# ----------------------------------------------------------------------------
RUSTFLAGS="-Cprofile-generate=/data/local/tmp/pgo-data" \
cargo ndk -t arm64-v8a -o ./android/app/src/main/jniLibs build --release

# Desplegar APK en el dispositivo Android físico
./gradlew installRelease

# ----------------------------------------------------------------------------
# PASO 2: Ejecución del emulador en el dispositivo real para recolectar datos
# ----------------------------------------------------------------------------
# Ejecutar el emulador durante 60 segundos cargando ROMs representativas
adb shell am start -n com.gm8.emulator/.MainActivity
# Esperar que se ejecute la carga de trabajo...

# Volcar los datos del perfil desde el almacenamiento del dispositivo
adb shell mkdir -p /data/local/tmp/pgo-data
adb pull /data/local/tmp/pgo-data ./pgo-data

# ----------------------------------------------------------------------------
# PASO 3: Fusión de perfiles con llvm-profdata
# ----------------------------------------------------------------------------
# Ubicar la herramienta llvm-profdata incluida en la toolchain del NDK de Android
NDK_LLVM_BIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"
$NDK_LLVM_BIN/llvm-profdata merge -output=./merged.profdata ./pgo-data

# ----------------------------------------------------------------------------
# PASO 4: Compilación final de producción consumiendo el perfil fusionado
# ----------------------------------------------------------------------------
RUSTFLAGS="-Cprofile-use=$(pwd)/merged.profdata -Cllvm-args=-pgo-warn-missing" \
cargo ndk -t arm64-v8a -o ./android/app/src/main/jniLibs build --release
```

---

### 6.2. Branchless Instruction Dispatch (Look-Up Tables vs. Match Masivo)

#### Problema en Hardware:
Un bloque `match opcode` con más de 200 ramas compila a un árbol binario de bifurcaciones condicionales o a una tabla de salto (*jump table*) indirecta. Cuando el flujo de opcodes de una ROM emulada es dinámico, la tabla de predicción de saltos (*Branch Target Buffer* / BTB) de ARM se satura, disparando fallos de predicción constantes.

#### Solución: Look-Up Table (LUT) Alineada
Una tabla plana de 256 punteros a funciones directas `unsafe fn(&mut CpuState)` alineada a 64 bytes. El opcode actúa directamente como índice aritmético sin comprobación de rango (un valor `u8` siempre está entre 0 y 255).

```rust
// ============================================================================
// ❌ ANTIPATRÓN: Match gigantesco con fallos continuos de branch prediction
// ============================================================================
pub fn dispatch_match_bad(cpu: &mut CpuState, opcode: u8) {
    match opcode {
        0x00 => cpu.op_nop(),
        0x01 => cpu.op_lda_imm(),
        0x02 => cpu.op_sta_abs(),
        // ... 250 ramas adicionales que saturan el Branch Target Buffer ...
        _ => cpu.op_unimplemented(),
    }
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Dispatcher LUT plano alineado a 64 bytes
// ============================================================================
pub struct CpuState {
    pub a: u8,
    pub pc: u16,
    pub cycles: u32,
}

// Tipo de puntero a función sin sobrecoste de vtable
pub type OpcodeHandler = unsafe fn(&mut CpuState);

// Alineación a 64 bytes para abarcar exactamente 4 o 8 líneas de caché completas
#[repr(align(64))]
pub struct DispatchTable {
    pub handlers: [OpcodeHandler; 256],
}

// Funciones individuales de instrucciones con inlining específico
unsafe fn op_nop(cpu: &mut CpuState) {
    cpu.pc = cpu.pc.wrapping_add(1);
    cpu.cycles += 2;
}

unsafe fn op_lda_imm(cpu: &mut CpuState) {
    cpu.a = 0x42; // Carga inmediata simulada
    cpu.pc = cpu.pc.wrapping_add(2);
    cpu.cycles += 2;
}

unsafe fn op_unimplemented(cpu: &mut CpuState) {
    cpu.pc = cpu.pc.wrapping_add(1);
}

// Inicialización estática de la LUT en tiempo de compilación
pub static DISPATCH_LUT: DispatchTable = {
    let mut table = DispatchTable {
        handlers: [op_unimplemented; 256],
    };
    table.handlers[0x00] = op_nop;
    table.handlers[0x01] = op_lda_imm;
    table
};

impl CpuState {
    #[inline(always)]
    pub fn step_lut(&mut self, opcode: u8) {
        // Acceso aritmético directo indexado por u8 (garantizado en rango 0..255)
        // No genera ninguna comprobación de límites ni salto condicional
        unsafe {
            let handler = *DISPATCH_LUT.handlers.get_unchecked(opcode as usize);
            handler(self);
        }
    }
}
```

---

### 6.3. Erradicación de Bounds Checks en la Memoria Emulada (Power-of-2 MMU)

#### Problema en Hardware:
Cada acceso a la memoria del emulador mediante `ram[addr as usize]` genera una instrucción `cmp` y una bifurcación a `core::panicking::panic_bounds_check`. Esto añade 2 a 3 instrucciones por lectura/escritura e impide la canalización de instrucciones de carga/almacenamiento (*pipelined load/store*).

#### Solución:
Modelar el espacio de memoria como potencias de 2 (ej. 8KB, 64KB) y aplicar una máscara binaria (`& 0x1FFF`). Al enmascarar la dirección, el índice queda matemáticamente acotado dentro de la capacidad física, permitiendo el uso seguro de `get_unchecked` y `get_unchecked_mut`.

```rust
pub const RAM_CAPACITY: usize = 8192; // 8KB (potencia de 2: 2^13)
pub const RAM_MASK: usize = RAM_CAPACITY - 1; // 0x1FFF

pub struct FastMmu {
    pub ram: [u8; RAM_CAPACITY],
}

impl FastMmu {
    pub const fn new() -> Self {
        Self { ram: [0; RAM_CAPACITY] }
    }

    // ❌ ANTIPATRÓN: Indexado estándar con bounds check forzado
    pub fn read_safe_bad(&self, addr: u16) -> u8 {
        self.ram[(addr as usize) % RAM_CAPACITY] // Módulo: división aritmética costosa
    }

    // ✅ IMPLEMENTACIÓN OPTIMIZADA: Máscara bit a bit y acceso unchecked garantizado
    #[inline(always)]
    pub fn read8(&self, addr: u16) -> u8 {
        // Enmascaramiento bit a bit: 1 ciclo de reloj (instrucción 'and')
        let masked_idx = (addr as usize) & RAM_MASK;
        
        // Invariante de seguridad: masked_idx nunca puede exceder RAM_CAPACITY - 1
        unsafe { *self.ram.get_unchecked(masked_idx) }
    }

    #[inline(always)]
    pub fn write8(&mut self, addr: u16, val: u8) {
        let masked_idx = (addr as usize) & RAM_MASK;
        unsafe { *self.ram.get_unchecked_mut(masked_idx) = val; }
    }
}
```

---

### 6.4. Vectorización SIMD / ARM NEON para Scanline Rendering (PPU)

#### Problema en Hardware:
Un rasterizador de PPU que procesa píxeles individualmente en un bucle escalar ejecuta 256 operaciones por scanline. En arquitecturas ARM64, las unidades NEON procesan 128 bits de datos en un solo ciclo de CPU (16 píxeles de 8 bits o 4 píxeles RGBA de 32 bits en paralelo).

#### Solución con Intrínsecos ARM64 (`std::arch::aarch64`):
Procesar bloques de 8 píxeles simultáneamente, aplicando máscaras de transparencia y mezcla de capas de fondo/sprites en una única ráfaga vectorial.

```rust
#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

/// Mezcla vectorial de una capa de sprites sobre un fondo usando ARM NEON
/// fondo: slice de píxeles RGBA (u32) del scanline actual
/// sprites: slice de píxeles RGBA (u32) con canal alfa de 0 o 255
#[inline(always)]
pub unsafe fn blend_scanline_neon(fondo: &mut [u32], sprites: &[u32]) {
    debug_assert!(fondo.len() >= 8 && sprites.len() >= 8);
    
    // Procesa bloques de 4 píxeles (16 bytes = 128 bits por registro NEON)
    let mut f_ptr = fondo.as_mut_ptr() as *mut u8;
    let mut s_ptr = sprites.as_ptr() as *const u8;
    
    // Carga 4 píxeles RGBA (128 bits) de fondo y 4 de sprite en registros vectoriales
    let v_fondo: uint8x16_t = vld1q_u8(f_ptr);
    let v_sprite: uint8x16_t = vld1q_u8(s_ptr);

    // Genera una máscara de transparencia: compara si el sprite no es negro/transparente
    let v_zero: uint8x16_t = vdupq_n_u8(0);
    let mask: uint8x16_t = vcgtq_u8(v_sprite, v_zero); // Mascara 0xFF donde hay color

    // Selección condicional vectorial bit a bit: vbslq selecciona bits de v_sprite si mask es 1, o v_fondo si es 0
    let v_resultado: uint8x16_t = vbslq_u8(mask, v_sprite, v_fondo);

    // Almacena los 4 píxeles resultantes directamente en memoria de video sin saltos condicionales
    vst1q_u8(f_ptr, v_resultado);
}

/// Alternativa portable de alto nivel auto-vectorizable por LLVM
#[inline(always)]
pub fn blend_scanline_portable(fondo: &mut [u32], sprites: &[u32]) {
    fondo.chunks_exact_mut(4)
        .zip(sprites.chunks_exact(4))
        .for_each(|(f_chunk, s_chunk)| {
            for i in 0..4 {
                let s = s_chunk[i];
                // Expresión branchless que LLVM convierte a selección vectorial
                f_chunk[i] = if s != 0 { s } else { f_chunk[i] };
            }
        });
}
```

---

### 6.5. Frontera JNI y Despliegue de Framebuffer Zero-Copy (`ANativeWindow`)

#### Arquitectura Zero-Copy:
Evitar transferir arrays Java (`byte[]`) hacia el runtime ART mediante llamadas JNI intermedias. En su lugar, bloquear directamente el búfer físico provisto por la superficie de Android (`ANativeWindow_lock`) y realizar una copia no solapada respetando el *stride* de pantalla de la GPU.

```rust
use std::ffi::c_void;
use jni::JNIEnv;
use jni::objects::JObject;
use jni::sys::{jobject, jlong};

type ANativeWindow = c_void;

#[repr(C)]
pub struct ANativeWindow_Buffer {
    pub width: i32,
    pub height: i32,
    pub stride: i32,
    pub format: i32,
    pub bits: *mut c_void,
    pub reserved: [u32; 6],
}

#[link(name = "android")]
extern "C" {
    fn ANativeWindow_fromSurface(env: *mut jni::sys::JNIEnv, surface: jobject) -> *mut ANativeWindow;
    fn ANativeWindow_release(window: *mut ANativeWindow);
    fn ANativeWindow_lock(window: *mut ANativeWindow, outBuffer: *mut ANativeWindow_Buffer, inOutDirtyBounds: *mut c_void) -> i32;
    fn ANativeWindow_unlockAndPost(window: *mut ANativeWindow) -> i32;
}

pub struct EmulatorBridge {
    pub framebuffer: Box<[u32; 256 * 240]>,
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_gm8_emulator_NativeBridge_presentFrame(
    mut env: JNIEnv,
    _class: JObject,
    emulator_handle: jlong,
    surface: jobject,
) {
    let emulator = &mut *(emulator_handle as *mut EmulatorBridge);
    let raw_env = env.get_raw();

    let window = ANativeWindow_fromSurface(raw_env, surface);
    if window.is_null() { return; }

    let mut window_buffer: ANativeWindow_Buffer = std::mem::zeroed();

    // Bloquea el búfer de display provisto por SurfaceFlinger
    if ANativeWindow_lock(window, &mut window_buffer, std::ptr::null_mut()) == 0 {
        let dest_ptr = window_buffer.bits as *mut u32;
        let stride = window_buffer.stride as usize;

        // Copia directa de línea a línea respetando el alineamiento de hardware de la GPU
        for y in 0..240 {
            let src_row = emulator.framebuffer.as_ptr().add(y * 256);
            let dst_row = dest_ptr.add(y * stride);
            std::ptr::copy_nonoverlapping(src_row, dst_row, 256);
        }

        // Envía el cuadro directamente al compositor de Android sin intervención del GC
        ANativeWindow_unlockAndPost(window);
    }

    ANativeWindow_release(window);
}
```

---

## 7. Herramientas de Auditoría, Profiling e Inspección de Ensamblador

### 7.1. Inspección de Ensamblador con `cargo-show-asm`
Para verificar si una función crítica de CPU o MMIO ha sido inilineada, si sus límites han sido eliminados y si está haciendo uso efectivo de registros escalares o vectoriales, se audita el código máquina emitido:

```bash
# 1. Instalar herramienta de inspección
cargo install cargo-show-asm

# 2. Inspeccionar el código generado para el target ARM64 en release
cargo asm --target aarch64-linux-android --release "libgm8emulator::mmu::FastMmu::read8"

# 3. Validar presencia de instrucciones ARM64 óptimas:
# - Ausencia de: 'bl core::panicking::panic_bounds_check'
# - Presencia de: 'and', 'ldr', 'ldrb', 'csel'
```

---

### 7.2. Auditoría en Compiler Explorer (Godbolt) para ARM64
Para aislar fragmentos matemáticos o decodificadores antes de integrarlos al crate:
* **Compilador:** `rustc (nightly o stable reciente)`
* **Flags de compilación:** `-C opt-level=3 --target aarch64-linux-android -C target-feature=+neon`
* **Directiva de validación:** Asegurarse de que el cuerpo de la función no genere llamadas a la pila (`stp x29, x30, [sp, #-16]!`), manteniéndose como una función hoja (*leaf function*) que opere puramente en registros `w0`-`w7`.

---

### 7.3. Perfilado de Hardware con `simpleperf` en Android NDK
`simpleperf` es la herramienta de perfilado de rendimiento nativa de Android que interactúa directamente con los Performance Monitoring Counters (PMC) del núcleo ARM:

```bash
# 1. Habilitar profiling en el dispositivo Android vía ADB
adb root
adb shell setenforce 0

# 2. Iniciar grabación de contadores de hardware sobre el proceso activo
adb shell simpleperf record \
    -p $(adb shell pidof com.gm8.emulator) \
    -e cpu-cycles,instructions,L1-dcache-load-misses,L1-icache-load-misses,branch-misses \
    --duration 10 \
    -o /data/local/tmp/perf.data

# 3. Descargar el reporte de datos a la máquina de desarrollo
adb pull /data/local/tmp/perf.data .

# 4. Generar reporte estadístico ordenado por símbolos nativos
simpleperf report --sort comm,dso,symbol -i perf.data -n --limit 15

# 5. Generar un Flamegraph interactivo SVG
python3 $ANDROID_NDK_HOME/simpleperf/report_sample.py perf.data > out.perf
python3 $ANDROID_NDK_HOME/simpleperf/flamegraph.py out.perf > flamegraph.svg
```

---

## 8. Arquitectura y Ejecución Nativa en Android NDK (Mobile Deep Dive)

Esta sección aborda la compilación, empaquetado, ciclo de vida y ejecución de `libgm8emulator.so` en el sistema operativo Android moderno (Android 10 hasta Android 15+), garantizando cumplimiento con los requisitos de silicio móvil ARM64 y eliminando cualquier sobrecoste del runtime ART/Java.

---

### 8.1. Arquitectura de Memoria Móvil y Soporte de Páginas de 16 KB (Android 15+)

#### Fundamento Teórico de Hardware:
Tradicionalmente, los núcleos ARM64 en Android operaban con un tamaño de página de memoria virtual de 4 KB (`PAGE_SIZE = 4096`). A partir de Android 15 (API 35+) y en microarquitecturas ARMv9 recientes (Cortex-X4, Cortex-A720), el kernel de Android soporta y comenzará a exigir páginas de **16 KB** (`PAGE_SIZE = 16384`).

* **Ventajas en Rendimiento:** Un tamaño de página de 16 KB cuadruplica la cobertura del *Translation Lookaside Buffer* (TLB) con el mismo número de entradas de hardware. En emuladores que leen y escriben masivamente en espacios de memoria mapeada (RAM, VRAM, ROM), esto reduce los *TLB misses* entre un 4% y un 12%, disminuyendo las costosas caminatas por la tabla de páginas (*page table walks*).
* **El Problema Crítico de Compatibilidad:** Si una biblioteca nativa `.so` ha sido enlazada asumiendo segmentos `PT_LOAD` alineados a 4 KB, el dynamic linker de Android (`/system/bin/linker64`) fallará al cargarla en un dispositivo con kernel de 16 KB, abortando el proceso inmediatamente con un error de `dlopen`:
  ```text
  dlopen failed: empty/missing PT_LOAD at 0x... (alignment 0x1000 is less than system page size 0x4000)
  ```

#### Configuración de Enlace para 16 KB en Rust:
Se deben inyectar flags específicos al linker de LLVM (`lld` / `mold`) a través de `.cargo/config.toml` o de la variable de entorno `RUSTFLAGS`:

```toml
# En .cargo/config.toml
[target.aarch64-linux-android]
rustflags = [
    "-C", "link-arg=-Wl,-z,max-page-size=16384",
    "-C", "link-arg=-Wl,-z,common-page-size=16384",
    "-C", "target-feature=+neon,+fp-armv8",
]
```

#### Verificación Determinista con `readelf`:
Tras compilar con `cargo-ndk`, se debe auditar el alineamiento de los segmentos `LOAD` de `libgm8emulator.so`:

```bash
# Auditar alineación de segmentos ELF
$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-readelf -l \
    ./android/app/src/main/jniLibs/arm64-v8a/libgm8emulator.so | grep -E "LOAD|ALIGN"
```

* **Salida esperada (Válida para 16 KB y 4 KB):**
  ```text
  Type           Offset   VirtAddr           PhysAddr           FileSiz  MemSiz   Flg Align
  LOAD           0x000000 0x0000000000000000 0x0000000000000000 0x01a450 0x01a450 R   0x4000
  LOAD           0x020000 0x0000000000020000 0x0000000000020000 0x0e5120 0x0e5120 R E 0x4000
  LOAD           0x110000 0x0000000000110000 0x0000000000110000 0x0081a0 0x008200 RW  0x4000
  ```
  *(El valor `Align` debe ser estrictamente `0x4000` = 16,384 bytes, nunca `0x1000` = 4,096 bytes).*

---

### 8.2. Zero-Copy Native Library Loading (`extractNativeLibs = false`)

#### Comparativa Arquitectural: Descompresión en Disco vs. `mmap` Directo

```
MÉTODO HEREDADO (extractNativeLibs = true / useLegacyPackaging = true):
[ APK en Disco ] --- (Descompresión en instalación) ---> [ /data/app/.../lib/arm64/libgm8emulator.so ]
                                                                       | (dlopen)
                                                                       v
                                                           [ Memoria Física RAM ]
* Doble espacio en almacenamiento flash.
* Mayor tiempo de instalación y actualización.
* Desgaste de celdas NAND UFS/eMMC.

MÉTODO OPTIMIZADO (extractNativeLibs = false / useLegacyPackaging = false):
[ APK en Disco (lib sin comprimir, 16KB-aligned) ] <--- (mmap directo por el Linker) ---> [ Memoria Física RAM ]
* 0 bytes de almacenamiento duplicado en disco.
* Carga inmediata en memoria virtual.
* Páginas limpias (R/O y código ejecutable) respaldadas por el APK: el kernel puede descartarlas
  ante presión de memoria sin necesidad de swapping a disco.
```

#### Configuración en `build.gradle.kts` (Android Gradle Plugin 8.0+):

```kotlin
// android/app/build.gradle.kts
android {
    packaging {
        jniLibs {
            // Deshabilita el empaquetado heredado: las bibliotecas .so se almacenan sin comprimir
            // y alineadas en el ZIP del APK para que el linker las mapee directamente.
            useLegacyPackaging = false
        }
    }
}
```

#### Configuración en `AndroidManifest.xml`:

```xml
<application
    android:extractNativeLibs="false"
    android:hasCode="true"
    android:allowBackup="false">
    <!-- ... componentes ... -->
</application>
```

#### Verificación de Alineación del ZIP con `zipalign`:
El archivo `.so` dentro del APK debe estar almacenado en modo `STORED` (nivel de compresión 0) y alineado a una frontera de 16 KB (16,384 bytes):

```bash
# Validar alineación dentro del APK final
zipalign -c -v -p 16 app-release.apk | grep "libgm8emulator.so"
# Debe reportar: "Verification SUCCESSFUL" y offset múltiplo de 16384
```

---

### 8.3. Subsistema Gráfico Nativo con ANativeWindow

#### Cuello de Botella del Antipatrón Tradicional:
Muchos proyectos de emulación principiantes cruzan la frontera JNI enviando arrays de píxeles (`jintArray` o `jbyteArray`) o buffers hacia un objeto `android.graphics.Bitmap` de Java.
1. **Sobrecoste de asignación JNI:** Cada llamada a `env.new_int_array()` o `env.set_int_array_region()` bloquea el recolector de basura o clona buffers de memoria.
2. **Copia de píxeles redundante:** Los datos van de Rust RAM -> Java Heap -> Skia/Bitmap -> Textura GPU.
3. **Pérdida de presupuesto de frame:** En resolución nativa escalada a pantalla móvil (ej. 1080p o 1440p), esta triple copia consume más de 6-8 ms del presupuesto de 16.6 ms de frame.

#### Patrón de Alto Rendimiento: Renderizado Directo sobre `ANativeWindow`
Se delega la presentación visual a un `SurfaceView` en la UI de Android. Su superficie física (`android.view.Surface`) se transfiere al motor Rust a través de JNI. Rust la asocia mediante `ANativeWindow_fromSurface`, bloquea el buffer de SurfaceFlinger y copia directamente al puntero de hardware mapeado.

```rust
// ============================================================================
// ❌ ANTIPATRÓN: Transferencia de píxeles cruzando JNI hacia Bitmap de Java
// ============================================================================
use jni::JNIEnv;
use jni::objects::{JClass, JIntArray};

#[no_mangle]
pub unsafe extern "C" fn Java_com_gm8_emulator_NativeBridge_renderFrameToBitmapBad(
    env: JNIEnv,
    _class: JClass,
    framebuffer_out: JIntArray,
) {
    let mut local_pixels = [0u32; 256 * 240];
    // Emula y llena local_pixels...
    
    // PELIGRO: Copia masiva de memoria cruzando el runtime de ART.
    // Dispara contención en el garbage collector y genera stalls de frame.
    let raw_slice = std::slice::from_raw_parts(local_pixels.as_ptr() as *const i32, 256 * 240);
    let _ = env.set_int_array_region(&framebuffer_out, 0, raw_slice);
}

// ============================================================================
// ✅ PATRÓN DE ALTO RENDIMIENTO: Enlace directo con ANativeWindow (Cero Copias ART)
// ============================================================================
use std::ffi::c_void;
use std::ptr::null_mut;
use jni::JNIEnv;
use jni::objects::JObject;
use jni::sys::{jobject, jlong};

pub const EMULATOR_WIDTH: usize = 256;
pub const EMULATOR_HEIGHT: usize = 240;
pub const WINDOW_FORMAT_RGBA_8888: i32 = 1;

type ANativeWindow = c_void;

#[repr(C)]
pub struct ANativeWindow_Buffer {
    pub width: i32,
    pub height: i32,
    pub stride: i32, // Stride de hardware en píxeles (puede ser mayor que width)
    pub format: i32,
    pub bits: *mut c_void, // Puntero mapeado a la memoria de la GPU
    pub reserved: [u32; 6],
}

#[link(name = "android")]
extern "C" {
    fn ANativeWindow_fromSurface(env: *mut jni::sys::JNIEnv, surface: jobject) -> *mut ANativeWindow;
    fn ANativeWindow_release(window: *mut ANativeWindow);
    fn ANativeWindow_setBuffersGeometry(window: *mut ANativeWindow, width: i32, height: i32, format: i32) -> i32;
    fn ANativeWindow_lock(window: *mut ANativeWindow, outBuffer: *mut ANativeWindow_Buffer, inOutDirtyBounds: *mut c_void) -> i32;
    fn ANativeWindow_unlockAndPost(window: *mut ANativeWindow) -> i32;
}

pub struct EmulatorGraphicsCore {
    // Buffer continuo interno del emulador alineado a línea de caché
    pub internal_framebuffer: [u32; EMULATOR_WIDTH * EMULATOR_HEIGHT],
}

#[no_mangle]
pub unsafe extern "C" fn Java_com_gm8_emulator_NativeBridge_renderDirectToSurface(
    mut env: JNIEnv,
    _class: JObject,
    emulator_core_ptr: jlong,
    surface: jobject,
) {
    let core = &mut *(emulator_core_ptr as *mut EmulatorGraphicsCore);
    let raw_env = env.get_raw();

    // 1. Obtiene el puntero a la ventana nativa de SurfaceFlinger
    let window = ANativeWindow_fromSurface(raw_env, surface);
    if window.is_null() { return; }

    // 2. Fija la resolución física interna del búfer nativo
    ANativeWindow_setBuffersGeometry(
        window,
        EMULATOR_WIDTH as i32,
        EMULATOR_HEIGHT as i32,
        WINDOW_FORMAT_RGBA_8888,
    );

    let mut window_buffer: ANativeWindow_Buffer = std::mem::zeroed();

    // 3. Bloquea el búfer de hardware provisto por el compositor
    if ANativeWindow_lock(window, &mut window_buffer, null_mut()) == 0 {
        let dest_base = window_buffer.bits as *mut u32;
        let gpu_stride = window_buffer.stride as usize;

        // 4. Transferencia de scanlines considerando el hardware stride (pitch) de la GPU
        // Si gpu_stride == EMULATOR_WIDTH, se puede hacer 1 solo memcpy continuo de 61440 palabras
        if gpu_stride == EMULATOR_WIDTH {
            std::ptr::copy_nonoverlapping(
                core.internal_framebuffer.as_ptr(),
                dest_base,
                EMULATOR_WIDTH * EMULATOR_HEIGHT,
            );
        } else {
            // Copia fila por fila para respetar el relleno de alineación de la GPU
            for y in 0..EMULATOR_HEIGHT {
                let src_offset = y * EMULATOR_WIDTH;
                let dst_offset = y * gpu_stride;
                std::ptr::copy_nonoverlapping(
                    core.internal_framebuffer.as_ptr().add(src_offset),
                    dest_base.add(dst_offset),
                    EMULATOR_WIDTH,
                );
            }
        }

        // 5. Presenta inmediatamente el búfer al display de Android
        ANativeWindow_unlockAndPost(window);
    }

    // 6. Libera la referencia nativa
    ANativeWindow_release(window);
}
```

---

### 8.4. Audio de Baja Latencia con AAudio / Oboe y SPSC Lock-Free Ring Buffer

#### Comparativa de Latencias de Audio en Android:
* **`android.media.AudioTrack` (Java / Kotlin):**
  - Latencia media: **40 ms - 100 ms**.
  - Pasa a través del servidor de audio de Android (`AudioFlinger`), aplicando mezclas de software y colas con locks.
  - Provoca que los efectos de sonido y la música de los juegos emulados se escuchen desfasados respecto a los cuadros de video (100 ms equivale a un desfase de 6 frames a 60 FPS).
* **AAudio / Oboe (C++ / Rust NDK Nativo):**
  - Latencia media: **< 10 ms** (en dispositivos certificados con soporte MMAP ALSA directo).
  - Modo `SharingMode::Exclusive`: El hilo de audio se conecta directamente con el buffer de hardware del DSP / ALSA del SoC, saltándose el mezclador general del sistema.
  - Modo `PerformanceMode::LowLatency`: Prioridad de tiempo real del kernel de Linux (`SCHED_FIFO`).

#### Arquitectura de Audio Desacoplada (SPSC Lock-Free Ring Buffer)
El hilo de ejecución de la CPU del emulador y el callback de hardware de audio corren a frecuencias dispares y asíncronas.
* **Productor:** Hilo de emulación (genera muestras de audio muestra por muestra o en bloques de scanline).
* **Consumidor:** Callback de alta prioridad de AAudio (solicita ráfagas de 96 a 192 muestras cada pocos milisegundos).
* **Regla estricta:** **Bajo ninguna circunstancia el callback de audio de AAudio puede bloquearse con un Mutex.** Si el callback se bloquea, se genera un *Audio Underrun* (chasquido audible / *glitch*).

```rust
// ============================================================================
// ✅ PATRÓN DE ALTO RENDIMIENTO: SPSC Lock-Free Ring Buffer para Streaming de Audio
// ============================================================================
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub const AUDIO_RING_CAPACITY: usize = 4096; // Potencia de 2 para indexación branchless
pub const AUDIO_RING_MASK: usize = AUDIO_RING_CAPACITY - 1;

pub struct SpscAudioRingBuffer {
    pub buffer: [i16; AUDIO_RING_CAPACITY],
    pub head: AtomicUsize, // Índice de escritura (controlado por el Productor: Emulador)
    pub tail: AtomicUsize, // Índice de lectura (controlado por el Consumidor: Callback AAudio)
}

impl SpscAudioRingBuffer {
    pub const fn new() -> Self {
        Self {
            buffer: [0i16; AUDIO_RING_CAPACITY],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// Llamado desde el ciclo de la CPU del emulador (Productor)
    #[inline(always)]
    pub fn try_push(&mut self, sample: i16) -> bool {
        let current_head = self.head.load(Ordering::Relaxed);
        let current_tail = self.tail.load(Ordering::Acquire);

        // Si el buffer está lleno, se descarta la muestra para evitar stalls en la CPU
        if current_head.wrapping_sub(current_tail) >= AUDIO_RING_CAPACITY {
            return false;
        }

        let idx = current_head & AUDIO_RING_MASK;
        self.buffer[idx] = sample;

        // Release: publica la muestra en memoria antes de actualizar la cabeza
        self.head.store(current_head.wrapping_add(1), Ordering::Release);
        true
    }

    /// Llamado por el callback de tiempo real de AAudio (Consumidor)
    #[inline(always)]
    pub fn pop_slice(&mut self, output: &mut [i16]) -> usize {
        let current_head = self.head.load(Ordering::Acquire);
        let current_tail = self.tail.load(Ordering::Relaxed);

        let available = current_head.wrapping_sub(current_tail);
        let count = available.min(output.len());

        for i in 0..count {
            let idx = (current_tail.wrapping_add(i)) & AUDIO_RING_MASK;
            output[i] = self.buffer[idx];
        }

        // Relleno de silencio en caso de sub-ejecución transitoria (evita ruidos de memoria basura)
        for i in count..output.len() {
            output[i] = 0;
        }

        // Release: libera espacio en el buffer informando al hilo del emulador
        self.tail.store(current_tail.wrapping_add(count), Ordering::Release);
        count
    }
}
```

#### Integración con el Crate `oboe` en Rust:

```rust
use oboe::{
    AudioOutputCallback, AudioOutputStream, AudioStream, AudioStreamBuilder,
    DataCallbackResult, PerformanceMode, SharingMode,
};

pub struct AudioEngine {
    stream: AudioStreamAsync<Output, EmulatorAudioCallback>,
}

pub struct EmulatorAudioCallback {
    pub ring_buffer: Arc<parking_lot::Mutex<SpscAudioRingBuffer>>, // O puntero UnsafeCell lock-free
}

impl AudioOutputCallback for EmulatorAudioCallback {
    type FrameType = (i16, i16); // Salida estéreo 16-bit PCM

    fn on_audio_ready(
        &mut self,
        _stream: &mut dyn AudioOutputStream,
        data: &mut [(i16, i16)],
    ) -> DataCallbackResult {
        // En un diseño puro lock-free, se consume sin locks
        // data se llena directamente desde el ring buffer
        DataCallbackResult::Continue
    }
}

pub fn create_low_latency_audio_stream() -> AudioStreamBuilder<Output> {
    AudioStreamBuilder::default()
        .set_performance_mode(PerformanceMode::LowLatency)
        .set_sharing_mode(SharingMode::Exclusive)
        .set_format(oboe::AudioFormat::I16)
        .set_channel_count(2)
        .set_sample_rate(44100)
}
```

---

### 8.5. Sincronización y Frame Pacing (AGDK Swappy / VSYNC)

#### El Problema del Desfase de Frecuencia (Judder / Micro-Stuttering):
Las consolas clásicas ejecutaban a frecuencias fijas dictadas por sus osciladores de cuarzo nativos:
* **Game Boy:** 59.7275 Hz (Período: ~16.743 ms)
* **NES / Famicom (NTSC):** 60.0988 Hz (Período: ~16.639 ms)
* **SNES (NTSC):** 60.0988 Hz

Por otro lado, los paneles de los smartphones Android modernos operan a frecuencias nominales fijas o dinámicas (**60.0 Hz**, **90.0 Hz**, **120.0 Hz**).
* Si un emulador de Game Boy corre a su velocidad nativa de 59.727 Hz y empuja frames a una pantalla de 60.000 Hz, cada **~3.66 segundos** la pantalla solicitará un cuadro nuevo antes de que el emulador lo haya completado.
* **Consecuencias:**
  1. Si se fuerza sincronización VSYNC ingenua: Se produce un *frame drop* o duplicación súbita de cuadro cada 3.6 segundos, percibido como un molesto tirón visual (*judder*).
  2. Si la emulación se amarra estrictamente al VSYNC de Android: El juego correrá un 0.45% más rápido de lo real, y el búfer de audio se vaciará rápidamente (*audio underflow*), provocando crujidos sonoros.

#### Solución Integral: Android Game Development Kit (AGDK) Swappy & Resampling
1. **AGDK Frame Pacing (Swappy):**
   - Se comunica con el `Choreographer` de Android y los callbacks de VSYNC de hardware.
   - Determina el instante exacto en que cada buffer debe ser presentado (`ANativeWindow_setActualPresentationTime`), ajustando dinámicamente el intercambio para evitar que se desfasen los buffers de SurfaceFlinger.
2. **Audio-Driven Master Clock con Resampling Adaptativo:**
   - En lugar de sincronizar el avance del tiempo con el reloj del sistema, el **reloj maestro** lo dicta la tasa de consumo de muestras del hardware de audio (AAudio).
   - Si el nivel del ring buffer de audio sube o baja ligeramente debido a la discrepancia de 59.7 Hz vs 60 Hz, un algoritmo de remuestreo polinómico ligero (o ajuste fino de ±0.5% en la tasa de muestra) estabiliza la tasa sin saltos perceptibles ni para el oído ni para la vista.

```rust
// ============================================================================
// ✅ PATRÓN DE FRAME PACING: Compensación de reloj guiada por el buffer de audio
// ============================================================================
pub struct FramePacer {
    pub target_buffer_level: usize,
    pub current_speed_multiplier: f64,
}

impl FramePacer {
    pub fn new(capacity: usize) -> Self {
        Self {
            target_buffer_level: capacity / 2, // Mitad del buffer como punto de equilibrio
            current_speed_multiplier: 1.0,
        }
    }

    #[inline(always)]
    pub fn compute_clock_adjustment(&mut self, current_audio_fill: usize) -> f64 {
        let error = current_audio_fill as f64 - self.target_buffer_level as f64;
        
        // Ajuste sub-audible y suave (máximo ±0.5% de desviación de velocidad)
        // Permite absorber la diferencia de 59.727 Hz vs 60.000 Hz de forma transparente
        let correction = error * 0.00005;
        self.current_speed_multiplier = (1.0 + correction).clamp(0.995, 1.005);
        self.current_speed_multiplier
    }
}
```

---

### 8.6. Optimización de Tiempos de CI en GitHub Actions

#### Los Dos Grandes Cuellos de Botella en CI Móvil para Rust NDK:
1. **Compilar `cargo-ndk` desde código fuente:** La ejecución habitual de `cargo install cargo-ndk` descarga e invoca a `rustc` para compilar todo el árbol de dependencias de la herramienta en cada workflow de CI, consumiendo entre **2 y 4 minutos** inútilmente.
2. **Recompilación total de dependencias nativas:** Sin una estrategia de caché atómica vinculada a los hashes de `Cargo.lock` y del NDK de Android, las crates del motor se compilan desde cero en cada commit.

#### Soluciones de Optimización:
1. **Uso de `cargo-binstall`:** Descarga los binarios precompilados de `cargo-ndk` directamente desde los releases de GitHub en menos de **4 segundos**.
2. **`Swatinem/rust-cache@v2`:** Almacena de forma inteligente las carpetas `target/` de los targets `aarch64-linux-android` y `armv7-linux-androideabi`.
3. **`gradle/actions/setup-gradle@v3`:** Manejo automatizado de la caché de compilación de Gradle y de las descargas del SDK/NDK.

#### Workflow Completo de Producción (`.github/workflows/ci.yml`):

```yaml
name: Rust NDK Android CI

on:
  push:
    branches: [ master, main ]
  pull_request:
    branches: [ master, main ]

env:
  CARGO_TERM_COLOR: always
  ANDROID_NDK_VERSION: "26.1.10909125"

jobs:
  build-emulator-android:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout del Repositorio
        uses: actions/checkout@v4

      - name: Configurar Java 17 (JDK)
        uses: actions/setup-java@v4
        with:
          distribution: 'temurin'
          java-version: '17'

      - name: Configurar Android SDK y NDK
        uses: android-actions/setup-android@v3

      - name: Instalar Versión Específica del NDK
        run: |
          sdkmanager --install "ndk;${{ env.ANDROID_NDK_VERSION }}"
          echo "ANDROID_NDK_HOME=${ANDROID_SDK_ROOT}/ndk/${{ env.ANDROID_NDK_VERSION }}" >> $GITHUB_ENV

      - name: Instalar Rust Toolchain Estable y Targets Android
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-linux-android, armv7-linux-androideabi

      - name: Configurar Caché Inteligente de Rust
        uses: Swatinem/rust-cache@v2
        with:
          key: ${{ runner.os }}-android-ndk-${{ env.ANDROID_NDK_VERSION }}

      - name: Instalar cargo-binstall (Ultra-rápido)
        uses: cargo-bins/cargo-binstall@main

      - name: Instalar cargo-ndk vía binario precompilado
        run: cargo binstall --no-confirm cargo-ndk

      - name: Compilar Motor Nativo libgm8emulator.so (ARM64 & ARMv7)
        run: |
          cargo ndk -t arm64-v8a -t armeabi-v7a -o ./android/app/src/main/jniLibs build --release

      - name: Configurar y Cachear Gradle
        uses: gradle/actions/setup-gradle@v3

      - name: Ensamblar APK de Release
        working-directory: ./android
        run: ./gradlew assembleRelease --stacktrace

      - name: Verificar Soporte de Páginas de 16 KB en el Binario Resultante
        run: |
          SO_PATH="./android/app/src/main/jniLibs/arm64-v8a/libgm8emulator.so"
          ${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-readelf -l $SO_PATH | grep -E "LOAD"
          # Validar que no existan segmentos LOAD con alineación menor a 0x4000 (16384 bytes)
          if ${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-readelf -l $SO_PATH | grep LOAD | grep -q "0x1000"; then
            echo "ERROR CRÍTICO: libgm8emulator.so contiene segmentos alineados a 4KB! Requiere 16KB para Android 15+."
            exit 1
          fi
          echo "Verificación Exitosa: Segmentos ELF correctamente alineados a 16 KB."
```

---

## 9. Arquitectura de Alto Rendimiento para Emulación / Port del Runtime de GameMaker 8 en Rust

Esta sección aborda los retos arquitecturales específicos de la reingeniería y emulación del runtime de **GameMaker 8 (GM8)** en Rust para Android (`libgm8emulator.so`). El motor original de GM8 (escrito en Delphi/C++) sufría de cuellos de botella severos: tipado dinámico boxed, llamadas de dibujo inmediatas sin agrupar, escaneos lineales $O(N)$ en colisiones y almacenamiento disperso orientado a objetos. Su reescritura en Rust requiere patrones de sistemas de bajo nivel para alcanzar 60/120 FPS sostenidos en SoCs móviles.

---

### 9.1. Gestión de Variables Dinámicas con NaN-Boxing

#### Cuello de Botella en Memoria y Registros de CPU:
En GML (GameMaker Language), las variables son dinámicamente tipadas: un identificador puede contener un número real de doble precisión (`f64`), una cadena (`String`), un ID de instancia (`i32`), un booleano o un puntero a array/recurso.

El enfoque ingenuo en Rust consiste en modelar este dinamismo mediante un `enum`:

```rust
// Tamaño del discriminante (8 bytes alineados) + variante mayor (String: 24 bytes) = 32 BYTES
pub enum GMValueBad {
    Real(f64),           // 8 bytes
    Str(String),         // 24 bytes (ptr, cap, len)
    InstanceId(i32),     // 4 bytes
    Bool(bool),          // 1 byte
    Array(Vec<GMValueBad>), // 24 bytes
}
```

* **Penalización en Caché L1D:** Un juego de GM8 con 10,000 instancias y 20 variables por instancia almacena $200,000 \times 32\text{ bytes} = 6.4\text{ MB}$ solo en variables. La memoria caché L1D (32 KB - 64 KB en ARM Cortex) se satura al instante, provocando continuos *cache misses*.
* **Penalización en Registros ARM64:** Una estructura de 32 bytes no cabe en los registros escalares de la ABI ARM64 (`x0`-`x7`). Toda operación aritmética en la máquina virtual obliga a derramar memoria en la pila (*stack spill*), generando instrucciones `stp` / `ldp` en cada suma o comparación.

#### Solución: Arquitectura NaN-Boxing (8 Bytes Exactos)
El estándar IEEE-754 para números en coma flotante de doble precisión (`f64`) define 64 bits:
- **Bit 63:** Signo (1 bit)
- **Bits 62-52:** Exponente (11 bits). Cuando todos los bits son `1` (`0x7FF`), el número representa Infinito o **NaN** (Not-a-Number).
- **Bits 51-0:** Mantisa o Carga Útil (*Payload*, 52 bits).

Si el bit 51 es `1`, se trata de un **Quiet NaN (QNaN)**, el cual no dispara excepciones en la FPU de hardware. Dado que las arquitecturas modernas ARM64 y x86_64 solo utilizan los 48 bits inferiores del espacio de direcciones virtuales (o 52 bits con LVA), **los 48 bits inferiores de un QNaN pueden almacenar cualquier puntero, entero o ID arbitrario sin colisionar con números reales válidos.**

#### Diagrama de Memoria en ASCII: NaN-Boxing de 64 Bits

```
 63 62         52 51 50 49 48 47                                0
+--+-------------+--+--+--+--+----------------------------------+
|S | 11111111111 |1 |  TAG   |      PAYLOAD (48 bits)           |
+--+-------------+--+--+--+--+----------------------------------+
 \      \         \     \                    \
  \      \         \     \                    +---> Puntero a Heap / String / ID
   \      \         \     +---> Tag de Tipo: 0001=Bool, 0010=InstanceId, 0011=Object, 0100=StringPtr...
    \      \         +---> Bit QNaN (Quiet NaN = 1)
     \      +---> Exponente IEEE-754 (Todo 1s = 0x7FF)
      +---> Bit de Signo

1. Número Real (f64): Cualquier valor cuyo exponente != 0x7FF o sea un NaN aritmético normalizado.
2. Booleano:          0x7FF8_0001_0000_0000 (False) / 0x7FF8_0001_0000_0001 (True)
3. Instance ID (i32): 0x7FF8_0002_XXXX_XXXX (ID empaquetado en 32 bits)
4. Puntero a Heap:    0x7FF8_0003_PPPP_PPPP_PPPP (Dirección de 48 bits de String u Objeto)
```

#### Código Comparativo:

```rust
// ============================================================================
// ❌ ANTIPATRÓN: Enum convencional con inflado a 32 bytes y derrames en stack
// ============================================================================
#[derive(Clone, Debug)]
pub enum GMValueSlow {
    Real(f64),
    StringVal(String),
    InstanceId(i32),
    Bool(bool),
}

pub fn add_gm_values_slow(a: &GMValueSlow, b: &GMValueSlow) -> GMValueSlow {
    match (a, b) {
        (GMValueSlow::Real(x), GMValueSlow::Real(y)) => GMValueSlow::Real(x + y),
        _ => panic!("Tipos incompatibles"),
    }
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: NaN-Boxing de 8 Bytes en un único registro u64
// ============================================================================
const QNAN_MASK: u64      = 0x7FF8_0000_0000_0000;
const TAG_MASK: u64       = 0x0007_0000_0000_0000;
const PAYLOAD_MASK: u64   = 0x0000_FFFF_FFFF_FFFF;

const TAG_BOOL: u64       = 0x0001_0000_0000_0000;
const TAG_INSTANCE_ID: u64= 0x0002_0000_0000_0000;
const TAG_HEAP_PTR: u64   = 0x0003_0000_0000_0000;

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct Value(pub u64);

impl Value {
    #[inline(always)]
    pub fn from_f64(val: f64) -> Self {
        let bits = val.to_bits();
        // Normaliza NaNs canónicos de hardware para que no colisionen con nuestros tags
        if (bits & QNAN_MASK) == QNAN_MASK {
            Self(QNAN_MASK) // Canonical NaN
        } else {
            Self(bits)
        }
    }

    #[inline(always)]
    pub fn from_bool(b: bool) -> Self {
        Self(QNAN_MASK | TAG_BOOL | (b as u64))
    }

    #[inline(always)]
    pub fn from_instance_id(id: i32) -> Self {
        Self(QNAN_MASK | TAG_INSTANCE_ID | ((id as u32) as u64))
    }

    #[inline(always)]
    pub fn from_ptr<T>(ptr: *const T) -> Self {
        let addr = ptr as u64;
        debug_assert!(addr & !PAYLOAD_MASK == 0, "Puntero excede 48 bits de memoria virtual");
        Self(QNAN_MASK | TAG_HEAP_PTR | addr)
    }

    #[inline(always)]
    pub fn is_f64(&self) -> bool {
        (self.0 & QNAN_MASK) != QNAN_MASK || self.0 == QNAN_MASK
    }

    #[inline(always)]
    pub fn as_f64(&self) -> f64 {
        debug_assert!(self.is_f64(), "El valor no es un número real f64");
        f64::from_bits(self.0)
    }

    #[inline(always)]
    pub fn is_instance_id(&self) -> bool {
        (self.0 & (QNAN_MASK | TAG_MASK)) == (QNAN_MASK | TAG_INSTANCE_ID)
    }

    #[inline(always)]
    pub fn as_instance_id(&self) -> i32 {
        debug_assert!(self.is_instance_id());
        (self.0 & 0xFFFF_FFFF) as i32
    }

    #[inline(always)]
    pub fn as_ptr<T>(&self) -> *const T {
        debug_assert!((self.0 & (QNAN_MASK | TAG_MASK)) == (QNAN_MASK | TAG_HEAP_PTR));
        (self.0 & PAYLOAD_MASK) as *const T
    }
}

// Operación aritmética rápida: el valor se pasa directamente en registros w0/x0 de ARM64
#[inline(always)]
pub fn fast_add(a: Value, b: Value) -> Value {
    if a.is_f64() && b.is_f64() {
        Value::from_f64(a.as_f64() + b.as_f64())
    } else {
        slow_add_fallback(a, b)
    }
}

#[cold]
#[inline(never)]
fn slow_add_fallback(_a: Value, _b: Value) -> Value {
    // Manejo de concatenación de cadenas u operaciones dinámicas complejas
    Value::from_f64(0.0)
}
```

---

### 9.2. Pipeline de Renderizado 2D: Dynamic Sprite Batching

#### El Cuello de Botella de GM8:
En GM8 clásico, cada llamada a `draw_sprite(spr, subimg, x, y)` emitía comandos inmediatos en la GPU (Direct3D 8/9). Al portar esto a Android (OpenGL ES 3.0 o Vulkan):
1. **Llamadas de dibujo masivas (Draw Calls):** Dibujar 1,000 partículas o sprites genera 1,000 llamadas `glDrawElements` o `vkCmdDrawIndexed`.
2. **GPU Context Switching y Driver Overhead:** El driver de GPU de Android (Qualcomm Adreno o ARM Mali) satura un núcleo completo de CPU gestionando el overhead del pipeline state.

#### Solución: Sprite Batcher con Vaciado Automático (Flush)
Se acumulan los vértices de los quads de los sprites en un buffer contiguo en memoria (`Vec<Vertex>`). Solo se envía una orden de dibujo a la GPU cuando:
1. El identificador de textura activa cambia.
2. La capacidad máxima del buffer del batcher se satura (ej. 2,048 quads = 8,192 vértices y 12,288 índices).

#### Diagrama del Batching de Vértices:

```
[ Sprite 0 (Tex A) ] --+
[ Sprite 1 (Tex A) ] --+--> [ Buffer Contiguo: 4 Vértices x N en RAM ] ---> 1 Sola Llamada GPU (glDrawElements)
[ Sprite 2 (Tex A) ] --+
[ Sprite 3 (Tex B) ] ------> CAMBIO DE TEXTURA -> FLUSH AUTOMÁTICO -> Nueva Llamada GPU
```

#### Código Comparativo:

```rust
// ============================================================================
// ❌ ANTIPATRÓN: Draw call individual e inmediata por cada sprite
// ============================================================================
pub fn draw_sprite_bad(texture_id: u32, x: f32, y: f32) {
    // Enlaza textura en cada invocación (stall del pipeline gráfico)
    unsafe {
        // glBindTexture(GL_TEXTURE_2D, texture_id);
        // glDrawArrays(GL_TRIANGLES, 0, 6);
    }
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Dynamic Sprite Batcher con Buffer Contiguo
// ============================================================================
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex2D {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color_rgba: u32, // RGBA8888 empaquetado en 4 bytes
}

pub const MAX_QUADS: usize = 2048;
pub const MAX_VERTICES: usize = MAX_QUADS * 4;
pub const MAX_INDICES: usize = MAX_QUADS * 6;

pub struct SpriteBatcher {
    pub vertices: Vec<Vertex2D>,
    pub current_texture: u32,
    pub quad_count: usize,
}

impl SpriteBatcher {
    pub fn new() -> Self {
        Self {
            vertices: Vec::with_capacity(MAX_VERTICES),
            current_texture: 0,
            quad_count: 0,
        }
    }

    #[inline(always)]
    pub fn draw_quad(
        &mut self,
        texture_id: u32,
        dst_x: f32, dst_y: f32, dst_w: f32, dst_h: f32,
        u0: f32, v0: f32, u1: f32, v1: f32,
        color: u32,
    ) {
        // Si cambia la textura o el buffer se satura, vacía el batch a la GPU
        if (texture_id != self.current_texture && self.quad_count > 0) || self.quad_count >= MAX_QUADS {
            self.flush();
        }

        self.current_texture = texture_id;

        // Genera los 4 vértices del quad directamente en el buffer contiguo
        let v0 = Vertex2D { position: [dst_x, dst_y],         uv: [u0, v0], color_rgba: color };
        let v1 = Vertex2D { position: [dst_x + dst_w, dst_y], uv: [u1, v0], color_rgba: color };
        let v2 = Vertex2D { position: [dst_x + dst_w, dst_y + dst_h], uv: [u1, v1], color_rgba: color };
        let v3 = Vertex2D { position: [dst_x, dst_y + dst_h], uv: [u0, v1], color_rgba: color };

        self.vertices.push(v0);
        self.vertices.push(v1);
        self.vertices.push(v2);
        self.vertices.push(v3);

        self.quad_count += 1;
    }

    pub fn flush(&mut self) {
        if self.quad_count == 0 { return; }

        // Aquí se emite UNA ÚNICA llamada de dibujo para todos los quads acumulados
        // render_backend::bind_texture(self.current_texture);
        // render_backend::upload_subdata(&self.vertices);
        // render_backend::draw_indexed(self.quad_count * 6);

        self.vertices.clear();
        self.quad_count = 0;
    }
}
```

---

### 9.3. Sistema Espacial de Colisiones y Bitmasking de 64 bits

#### El Cuello de Botella de GM8:
1. **Búsqueda Exhaustiva $O(N)$:** Las funciones `place_meeting(x, y, obj)` o `instance_place` escanean secuencialmente la lista completa de todas las instancias del room para comprobar solapamiento de cajas delimitadoras (*Bounding Boxes* / AABB). Con 2,000 instancias, cada frame ejecuta millones de comprobaciones inútiles.
2. **Colisiones Precisas por Píxel Lentas:** Al habilitar máscaras precisas, GM8 recorre bucles anidados en `px` y `py` evaluando píxel a píxel, causando miles de saltos condicionales y fallos de caché.

#### Solución 1: Spatial Hash Grid de Tamaño Fijo
La pantalla y el room se dividen en celdas de tamaño uniforme (ej. 64x64 píxeles). Cada instancia se registra únicamente en las celdas que intersecta su AABB. La búsqueda de colisiones pasa de complejidad $O(N)$ a $O(K)$, donde $K$ es el número de instancias presentes en esas celdas inmediatas.

#### Solución 2: Bitmasking de 64 Bits en Colisión de Máscaras
Las máscaras de sprites se empaquetan en arrays continuos de enteros de 64 bits (`[u64; N]`). Cada bit representa la opacidad de 1 píxel (`1 = sólido`, `0 = transparente`). 
* Evaluar una fila de 64 píxeles entre dos sprites toma **1 solo ciclo de CPU** mediante una operación AND a nivel de bits (`word_a & word_b`).
* Para desalineaciones horizontales sub-palabra, se aplican desplazamientos de bits (`>>` o `<<`).

#### Diagrama de Bitmasking de 64 Bits:

```
Sprite A Fila 0: [ 00011111 11000000 ... 64 bits ... ] (u64)
                 & (AND bit a bit)
Sprite B Fila 0: [ 00000111 00000000 ... 64 bits ... ] (u64 desplazado en X)
-------------------------------------------------------
Resultado:       [ 00000111 00000000 ... 64 bits ... ] != 0  ===> ¡COLISIÓN DETECTADA EN 1 CICLO!
```

#### Código Comparativo:

```rust
// ============================================================================
// ❌ ANTIPATRÓN: Comprobación de colisión precisa pixel a pixel mediante bucles
// ============================================================================
pub fn pixel_collision_slow(mask_a: &[bool], mask_b: &[bool], w: usize, h: usize) -> bool {
    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            if mask_a[idx] && mask_b[idx] { // Decenas de miles de lecturas y branches
                return true;
            }
        }
    }
    false
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Spatial Hash Grid + Bitmasking de 64 bits
// ============================================================================
pub const CELL_SIZE: i32 = 64;
pub const GRID_WIDTH: usize = 32; // Cuadrícula para room de 2048 x 2048
pub const GRID_HEIGHT: usize = 32;

pub struct SpatialHashGrid {
    // Celdas planas con capacidad fija para evitar heap allocations
    pub cells: Vec<Vec<u32>>, // Almacena IDs de instancia
}

impl SpatialHashGrid {
    #[inline(always)]
    pub fn cell_coords(x: f32, y: f32) -> (usize, usize) {
        let cx = ((x as i32) / CELL_SIZE).clamp(0, (GRID_WIDTH - 1) as i32) as usize;
        let cy = ((y as i32) / CELL_SIZE).clamp(0, (GRID_HEIGHT - 1) as i32) as usize;
        (cx, cy)
    }
}

/// Máscara binaria empaquetada: cada u64 almacena 64 píxeles horizontales
pub struct Bitmask64 {
    pub width: usize,
    pub height: usize,
    pub words_per_row: usize, // ceil(width / 64)
    pub words: Vec<u64>,
}

impl Bitmask64 {
    /// Comprueba colisión entre dos máscaras con desfase relativo (dx, dy)
    #[inline(always)]
    pub fn overlaps_at(&self, other: &Bitmask64, dx: i32, dy: i32) -> bool {
        let y_start = dy.max(0);
        let y_end = (self.height as i32).min(dy + other.height as i32);
        if y_start >= y_end { return false; }

        let shift_x = dx.rem_euclid(64) as u32;
        let word_offset_x = (dx.div_euclid(64)) as isize;

        for y in y_start..y_end {
            let row_self = y as usize;
            let row_other = (y - dy) as usize;

            let self_row_idx = row_self * self.words_per_row;
            let other_row_idx = row_other * other.words_per_row;

            for w in 0..self.words_per_row {
                let w_other = (w as isize) - word_offset_x;
                if w_other < 0 || w_other >= (other.words_per_row as isize) {
                    continue;
                }

                let word_a = self.words[self_row_idx + w];
                let raw_b = other.words[other_row_idx + (w_other as usize)];
                
                // Aplica alineación horizontal de bits
                let word_b_shifted = raw_b << shift_x;

                // Evaluación instantánea de 64 píxeles simultáneos
                if (word_a & word_b_shifted) != 0 {
                    return true;
                }
            }
        }
        false
    }
}
```

---

### 9.4. Diseño Orientado a Datos (SoA vs. AoS) para el Loop de Eventos

#### Cuello de Botella del Diseño Original de GM8:
En GM8, cada instancia es una estructura monolítica masiva (*Array of Structures* - AoS), dispersa por el heap y cargada de mapas hash dinámicos para sus variables:

```rust
pub struct InstanceAoSBad {
    pub id: u32,
    pub x: f64,
    pub y: f64,
    pub hspeed: f64,
    pub vspeed: f64,
    pub friction: f64,
    pub gravity: f64,
    pub alarms: [i32; 12],
    pub dynamic_vars: std::collections::HashMap<String, f64>, // Pointer chasing masivo
}
```

* Al procesar el evento `Step` de movimiento físico, la CPU necesita únicamente `x`, `y`, `hspeed` y `vspeed`.
* Sin embargo, al estar empaquetados en un struct AoS de más de 250 bytes, cada acceso a una instancia expulsa de la caché L1D los datos de las demás.
* **Resultado:** Imposibilidad total de auto-vectorización SIMD y saturación del bus de memoria.

#### Solución: Structure of Arrays (SoA)
Separar las propiedades físicas de alta frecuencia (*hot path*) en arrays contiguos paralelos planos. 
* Los vectores `xs`, `ys`, `hspeeds` y `vspeeds` residen contiguos en memoria física.
* El compilador LLVM auto-vectoriza la actualización del movimiento utilizando registros ARM NEON de 128 bits (`vaddq_f32`), actualizando 4 instancias simultáneamente por ciclo de CPU.

#### Diagrama SoA vs AoS:

```
AoS (Malo para Caché):
[ x0, y0, vars... (256B) ][ x1, y1, vars... (256B) ][ x2, y2, vars... (256B) ]
  \___ Trae 256 bytes de memoria basura a la línea de caché solo para leer x e y ___/

SoA (Óptimo para Caché L1D y ARM NEON):
X:       [ x0,  x1,  x2,  x3,  x4,  x5,  x6,  x7 ] (Contiguo, 32 bytes)
Y:       [ y0,  y1,  y2,  y3,  y4,  y5,  y6,  y7 ] (Contiguo, 32 bytes)
H_SPEED: [ h0,  h1,  h2,  h3,  h4,  h5,  h6,  h7 ] (Carga vectorial NEON inmediata)
```

#### Código Comparativo:

```rust
// ============================================================================
// ❌ ANTIPATRÓN: Iteración sobre instancias monolíticas (AoS disperso)
// ============================================================================
pub fn update_physics_aos_slow(instances: &mut [InstanceAoSBad]) {
    for inst in instances.iter_mut() {
        inst.x += inst.hspeed;
        inst.y += inst.vspeed;
        if inst.friction > 0.0 {
            inst.hspeed *= 1.0 - inst.friction;
        }
    }
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Structure of Arrays (SoA) Auto-Vectorizable
// ============================================================================
pub struct InstancePhysicsSoA {
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub hspeed: Vec<f32>,
    pub vspeed: Vec<f32>,
    pub friction: Vec<f32>,
    pub active: Vec<bool>,
}

impl InstancePhysicsSoA {
    #[inline(always)]
    pub fn update_physics_simd(&mut self) {
        let count = self.x.len();
        debug_assert_eq!(self.y.len(), count);
        debug_assert_eq!(self.hspeed.len(), count);
        debug_assert_eq!(self.vspeed.len(), count);

        // Procesamiento en trozos exactos de 4 (alineado a registros vectoriales ARM NEON)
        let mut chunks_x = self.x.chunks_exact_mut(4);
        let mut chunks_y = self.y.chunks_exact_mut(4);
        let chunks_h = self.hspeed.chunks_exact(4);
        let chunks_v = self.vspeed.chunks_exact(4);

        for (((x_quad, y_quad), h_quad), v_quad) in chunks_x.by_ref()
            .zip(chunks_y.by_ref())
            .zip(chunks_h)
            .zip(chunks_v) 
        {
            // LLVM auto-vectoriza este bloque exacto a instrucciones NEON (vaddq_f32)
            x_quad[0] += h_quad[0];
            x_quad[1] += h_quad[1];
            x_quad[2] += h_quad[2];
            x_quad[3] += h_quad[3];

            y_quad[0] += v_quad[0];
            y_quad[1] += v_quad[1];
            y_quad[2] += v_quad[2];
            y_quad[3] += v_quad[3];
        }

        // Remanente escalar para instancias impares
        let rem_x = chunks_x.into_remainder();
        let rem_y = chunks_y.into_remainder();
        let rem_offset = count - rem_x.len();

        for i in 0..rem_x.len() {
            rem_x[i] += self.hspeed[rem_offset + i];
            rem_y[i] += self.vspeed[rem_offset + i];
        }
    }
}
```

---

### 9.5. Carga Eficiente de Recursos con Memory-Mapped I/O (`memmap2`)

#### Cuello de Botella de Carga Tradicional:
Los juegos compilados en GM8 empaquetan todos los sprites, fondos, sonidos y scripts en un único archivo binario contenedor (ej. `game.data`, `data.win` o archivo `.gm8`).
* Utilizar `std::fs::read("game.data")` carga todo el archivo (ej. 150 MB) al heap de la aplicación.
* Duplica el consumo de RAM en dispositivos móviles y obliga al kernel a realizar costosas operaciones de copia física y asignación de páginas.
* Incrementa el tiempo de arranque inicial (*Cold Startup*) en 2 a 5 segundos.

#### Solución: Mapeo de Memoria con `memmap2`
Utilizar la llamada del sistema `mmap` a través del crate `memmap2`.
1. **Zero-Copy Instantáneo:** El kernel de Linux/Android mapea el archivo en el espacio de direcciones virtuales del proceso en **0.1 milisegundos**.
2. **Paginación Bajo Demanda (Demand Paging):** Las texturas y sonidos solo se transfieren desde el almacenamiento flash a la memoria RAM física en el momento exacto en que el juego las solicita por primera vez.
3. **Páginas Limpias Respaldadas por Archivo:** Si el sistema operativo experimenta presión de memoria en segundo plano, puede descartar las páginas del archivo sin necesidad de escribirlas en la partición de swap.

#### Consideraciones de Seguridad (`unsafe`) e Invariantes:
`memmap2::Mmap::map` es una operación marcada como `unsafe` porque si otro proceso externo trunca o modifica el archivo subyacente en el disco mientras el emulador lo lee, se disparará una señal `SIGBUS`.
* **Invariante de Seguridad del Motor:** Los assets del juego en Android residen dentro del sandbox privado de la aplicación (`/data/data/com.gm8.emulator/...` o dentro del APK cerrado), garantizando acceso exclusivo de solo lectura y blindando la invariante frente a mutaciones externas.

#### Código Comparativo:

```rust
// ============================================================================
// ❌ ANTIPATRÓN: Carga monolítica completa en heap mediante std::fs::read
// ============================================================================
pub fn load_assets_bad(path: &str) -> Vec<u8> {
    // Asigna 100+ MB en el heap, saturando el allocator scudo de Android
    std::fs::read(path).expect("Fallo al leer archivo de assets")
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Memory-Mapped I/O Zero-Copy con memmap2
// ============================================================================
use std::fs::File;
use std::sync::Arc;
use memmap2::Mmap;

pub struct GamePackage {
    mmap: Arc<Mmap>,
}

impl GamePackage {
    pub fn open(file_path: &str) -> Result<Self, std::io::Error> {
        let file = File::open(file_path)?;

        // Mapeo virtual sin lectura inmediata a RAM
        // INVARIANTE: El archivo es de solo lectura y reside en el sandbox seguro de la app
        let mmap = unsafe { Mmap::map(&file)? };

        Ok(Self {
            mmap: Arc::new(mmap),
        })
    }

    /// Obtiene una rodaja directa (&[u8]) del recurso sin copias intermedias
    #[inline(always)]
    pub fn get_asset_slice(&self, offset: usize, length: usize) -> &[u8] {
        &self.mmap[offset..offset + length]
    }

    /// Parsea una cabecera de textura directamente desde la memoria mapeada
    #[inline(always)]
    pub unsafe fn get_typed_header<T: Copy>(&self, offset: usize) -> &T {
        let slice = &self.mmap[offset..offset + std::mem::size_of::<T>()];
        &*(slice.as_ptr() as *const T)
    }
}
```

---

## 10. Compendio de Ingeniería Avanzada: Literatura, Blogs de Autores Clave y Patrones Críticos en Rust

Esta sección condensa los principios y técnicas de ingeniería de sistemas más avanzados extraídos de la literatura técnica oficial de Rust, publicaciones de ingenieros destacados de la comunidad de compiladores y un catálogo de patrones de optimización de bajo nivel directamente aplicables a `libgm8emulator.so`.

---

### 10.1. Literatura Técnica Fundamental de Rust

#### 10.1.1. The Rustonomicon ("The Dark Arts of Advanced and Unsafe Rust")
* **Referencia Oficial:** [https://doc.rust-lang.org/nomicon/](https://doc.rust-lang.org/nomicon/)
* **Fundamentos Críticos:**
  - **Layout de Structs y Modificadores de Representación:**
    * `repr(Rust)` (Por defecto): El compilador reordena los campos libremente para minimizar el padding. Su layout no es determinista entre versiones del compilador.
    * `repr(C)`: Garantiza que los campos se posicionen en el orden exacto de declaración respetando las reglas de la ABI de C. Imprescindible para estructuras que cruzan a `ANativeWindow`, descriptores de vértices para OpenGL/Vulkan y structs de hardware.
    * `repr(transparent)`: Garantiza que un struct con un único campo no-zero-sized tenga exactamente el mismo layout, tamaño y ABI que su campo interior. Crucial para wrappers de rendimiento como `struct Value(u64)` (NaN-Boxing) o identificadores fuertemente tipados (`struct EntityId(u32)`).
    * `repr(packed)`: Elimina todo el padding forzando alineación a 1 byte. **Advertencia de Rendimiento:** En CPUs ARM, acceder a un entero no alineado dentro de un struct packed puede disparar una penalización de múltiples ciclos de reloj o un fallo de hardware (*bus error* / `SIGBUS`).
  - **Reglas de Aliasing y Stacked Borrows / Tree Borrows:**
    * LLVM asume que dos referencias mutables (`&mut T`) nunca solapan en memoria física, emitiendo el atributo de metadatos `noalias`. Si el código inseguro (`unsafe`) muta datos a través de un puntero mientras existe una referencia activa, se genera comportamiento indefinido (UB), permitiendo a LLVM eliminar lecturas o reordenar instrucciones incorrectamente.
  - **Directivas para `slice::from_raw_parts` y Transmutes Seguros:**
    1. El puntero base no debe ser nulo y debe estar correctamente alineado con `align_of::<T>()`.
    2. El tamaño total `len * size_of::<T>()` no puede exceder `isize::MAX` bytes.
    3. La memoria debe estar totalmente inicializada.
    4. El ciclo de vida de la referencia resultante no puede exceder el tiempo de vida de la memoria física asignada.

```rust
// ============================================================================
// ❌ ANTIPATRÓN: slice::from_raw_parts con violación de alineación o aliasing
// ============================================================================
pub unsafe fn read_u32_slice_bad(bytes: &[u8]) -> &[u32] {
    // PELIGRO: bytes puede no estar alineado a 4 bytes. En ARM64, esto
    // genera lecturas desalineadas o comportamiento indefinido inmediato.
    std::slice::from_raw_parts(bytes.as_ptr() as *const u32, bytes.len() / 4)
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Verificación estricta de alineación y tamaño
// ============================================================================
pub fn read_u32_slice_safe(bytes: &[u8]) -> Option<&[u32]> {
    let ptr = bytes.as_ptr();
    let align = std::mem::align_of::<u32>();
    
    // Verifica formalmente que la dirección base sea múltiplo exacto de 4
    if (ptr as usize) % align != 0 || bytes.len() % 4 != 0 {
        return None;
    }

    // Cumple todas las invariantes de The Rustonomicon
    unsafe {
        Some(std::slice::from_raw_parts(ptr as *const u32, bytes.len() / 4))
    }
}
```

---

#### 10.1.2. The Portable SIMD Guide (`std::simd`)
* **Referencia Oficial:** [https://doc.rust-lang.org/nightly/std/simd/](https://doc.rust-lang.org/nightly/std/simd/)
* **Fundamentos Críticos:**
  - A diferencia de los intrínsecos específicos de plataforma (`std::arch::aarch64` para ARM NEON o `std::arch::x86_64` para AVX2), `std::simd` proporciona vectores portables de longitud fija (`f32x4`, `u8x16`, `i16x8`).
  - El backend LLVM traduce automáticamente estas operaciones a las instrucciones vectoriales nativas correspondientes del target (`vaddq_f32` en ARM64, `_mm_add_ps` en x86_64).
  - Permite escribir pipelines de rasterización de video (PPU) y mezcla de audio (APU) que compilan con rendimiento idéntico en Android ARM64, emuladores de escritorio x86_64 y targets WebAssembly.

```rust
// ============================================================================
// ❌ CÓDIGO INEFICIENTE: Procesamiento escalar de mezcla de audio estéreo
// ============================================================================
pub fn mix_audio_scalar(out: &mut [i16], bgm: &[i16], sfx: &[i16]) {
    for i in 0..out.len() {
        let mixed = (bgm[i] as i32 + sfx[i] as i32) / 2;
        out[i] = mixed.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    }
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Portable SIMD (std::simd) procesando 8 muestras simultáneas
// ============================================================================
#![feature(portable_simd)]
use std::simd::prelude::*;

pub fn mix_audio_simd(out: &mut [i16], bgm: &[i16], sfx: &[i16]) {
    let mut out_chunks = out.chunks_exact_mut(8);
    let bgm_chunks = bgm.chunks_exact(8);
    let sfx_chunks = sfx.chunks_exact(8);

    for ((out_chunk, bgm_chunk), sfx_chunk) in out_chunks.by_ref().zip(bgm_chunks).zip(sfx_chunks) {
        let v_bgm = i16x8::from_slice(bgm_chunk);
        let v_sfx = i16x8::from_slice(sfx_chunk);

        // Suma saturada vectorial: compila a instrucción 'sqadd' en ARM NEON (sin overflow)
        let v_mixed = v_bgm.saturating_add(v_sfx);
        
        v_mixed.copy_to_slice(out_chunk);
    }

    // Remanente escalar para muestras sobrantes
    let remainder_offset = out.len() - out_chunks.into_remainder().len();
    for i in remainder_offset..out.len() {
        out[i] = bgm[i].saturating_add(sfx[i]);
    }
}
```

---

#### 10.1.3. Criterion.rs User Guide (Benchmarking Riguroso y Prevención de DCE)
* **Referencia Oficial:** [https://bheisler.github.io/criterion.rs/book/](https://bheisler.github.io/criterion.rs/book/)
* **Fundamentos Críticos:**
  - **Dead Code Elimination (DCE) y Constant Folding:** Si un microbenchmark ejecuta una función pura cuyos resultados no se consumen o dependen de entradas constantes, LLVM calculará el resultado en tiempo de compilación o eliminará el bucle completo, reportando falsos `0.00 ns`.
  - **Uso estricto de `std::hint::black_box`:** Obliga a LLVM a tratar el dato como opaco, impidiendo que el optimizador asuma constantes de entrada o elimine escrituras de salida.
  - **Análisis Estadístico Bootstrap:** Criterion ejecuta miles de iteraciones, calcula intervalos de confianza (95%) y aplica pruebas de hipótesis de Student para reportar variaciones reales frente a ruido térmico o DVFS en CPUs móviles.

```rust
// ============================================================================
// ❌ MICRO-BENCHMARK INVÁLIDO: LLVM elimina el cómputo por dead code elimination
// ============================================================================
pub fn bench_invalid(c: &mut criterion::Criterion) {
    c.bench_function("bench_bad", |b| {
        b.iter(|| {
            let mut sum = 0;
            for i in 0..1000 { sum += i; } // LLVM calcula sum = 499500 en compile time y borra el loop
            sum
        })
    });
}

// ============================================================================
// ✅ MICRO-BENCHMARK ROBUSTO: Uso riguroso de black_box y throughput configurado
// ============================================================================
use criterion::{black_box, Criterion, Throughput};

pub fn bench_robust(c: &mut Criterion) {
    let mut group = c.benchmark_group("mmu_operations");
    group.throughput(Throughput::Bytes(4096));

    let memory_block = vec![0x42u8; 4096];

    group.bench_function("checksum_4k", |b| {
        b.iter(|| {
            // black_box en la entrada evita que LLVM asuma que el buffer siempre contiene 0x42
            let slice = black_box(&memory_block[..]);
            let mut acc = 0u32;
            for &byte in slice {
                acc = acc.wrapping_add(byte as u32);
            }
            // black_box en la salida asegura que el acumulador se compute en runtime
            black_box(acc)
        })
    });
    group.finish();
}
```

---

### 10.2. Artículos Técnicos de Ingenieros Destacados

#### 10.2.1. Matklad (Aleksey Kladov - Creador de rust-analyzer)
* **Conceptos Clave:**
  1. **Arena Allocators (`bumpalo`):** En compiladores y motores de videojuegos, reservar y liberar millones de pequeños objetos en el heap global (`malloc`) degrada drásticamente el rendimiento. El patrón de Arena reserva bloques contiguos de memoria donde la asignación es un simple incremento de puntero (`bump pointer`). Al finalizar el frame o script, la arena se vacía en $O(1)$ sin invocar destructores individuales.
  2. **Diseño Orientado a Índices (Index-Based Architectures):** En lugar de grafos de punteros interconectados con `Rc<RefCell<Node>>` (que causan fragmentación, fugas de ciclos y pesadas comprobaciones de préstamo en tiempo de ejecución), las entidades se almacenan en un array plano (`Vec<Entity>`) y se referencian mediante índices enteros tipados (`struct EntityId(u32)`).
  3. **String Interning (`lasso` / `smol_str`):** Reemplazar instancias de `String` por identificadores enteros (`Spur` / `u32`). Las comparaciones de nombres de variables de GM8 pasan de ser comparaciones de cadenas byte a byte $O(N)$ a una simple comparación de enteros $O(1)$.

```rust
// ============================================================================
// ❌ ANTIPATRÓN: Grafo con punteros compartidos y mutabilidad interior costosa
// ============================================================================
use std::rc::Rc;
use std::cell::RefCell;

pub struct AstNodeBad {
    pub name: String, // String asignada en heap para cada identificador
    pub children: Vec<Rc<RefCell<AstNodeBad>>>, // Sobrecoste de conteo de referencias y runtime checks
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Arena Allocator + String Interning (Patrón Matklad)
// ============================================================================
use bumpalo::Bump;
use lasso::Spur;

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct NodeId(pub u32);

pub struct AstNodeOptimized<'a> {
    pub symbol: Spur, // Entero u32 que representa la cadena (comparación O(1))
    pub children: &'a [NodeId], // Slice ubicado en memoria contigua de la Arena
}

pub struct AstArenaContext {
    pub bump: Bump,
}

impl AstArenaContext {
    pub fn new() -> Self {
        Self { bump: Bump::with_capacity(256 * 1024) } // 256 KB pre-asignados
    }

    #[inline(always)]
    pub fn alloc_node(&self, symbol: Spur, children_ids: &[NodeId]) -> &AstNodeOptimized {
        let children_slice = self.bump.alloc_slice_copy(children_ids);
        self.bump.alloc(AstNodeOptimized {
            symbol,
            children: children_slice,
        })
    }

    #[inline(always)]
    pub fn reset_frame(&mut self) {
        // Libera toda la memoria acumulada en 1 sola operación instantánea
        self.bump.reset();
    }
}
```

---

#### 10.2.2. Cliff L. Biffle
* **Referencia Técnica:** [cliffle.com](http://cliffle.com) (*"Learn Rust the Dangerous Way"*, estudios de layout en sistemas embebidos).
* **Conceptos Clave:**
  1. **Optimización de Nichos de Discriminante (*Niche-Filling Optimization*):**
     Rust optimiza los enums aprovechando representaciones inválidas de tipos (*niches*). Por ejemplo, una referencia `&T` o un `NonNull<T>` nunca pueden ser nulos (`0x0`). Por tanto, `Option<&T>` utiliza el valor `0x0` para representar `None`, haciendo que el tamaño de `Option<&T>` sea idéntico al de un puntero nativo (8 bytes), sin discriminante adicional.
  2. **Uso de `NonZeroU32`:** Aplicar tipos `NonZero` a identificadores de instancias o entidades permite envolverlos en `Option<NonZeroU32>` ocupando exactamente 4 bytes en lugar de 8 bytes (4 bytes de valor + 4 bytes de discriminante y padding).
  3. **Inspección de Representaciones LLVM (LLVM IR):** Auditar el código intermedio de LLVM con `cargo rustc -- --emit=llvm-ir` permite validar si una bifurcación ha sido colapsada a una instrucción condicional o si se ha eliminado el almacenamiento del discriminante.

```rust
// ============================================================================
// ❌ CÓDIGO SUBÓPTIMO: Enum con discriminante explícito que añade 4 bytes de padding
// ============================================================================
pub struct EntityHandleBad {
    pub id: Option<u32>, // 4 bytes (u32) + 1 byte (tag) + 3 bytes padding = 8 BYTES
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Niche Optimization mediante NonZeroU32 (4 Bytes)
// ============================================================================
use std::num::NonZeroU32;

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct EntityHandleOptimized {
    // 0x0 representa 'None', cualquier otro valor es 'Some(id)'.
    // Tamaño exacto: 4 BYTES. 0 padding, cabe perfectamente en registros de 32 bits.
    pub id: Option<NonZeroU32>,
}

impl EntityHandleOptimized {
    #[inline(always)]
    pub fn new(id: u32) -> Self {
        Self { id: NonZeroU32::new(id) }
    }

    #[inline(always)]
    pub fn is_active(&self) -> bool {
        self.id.is_some()
    }
}
```

---

#### 10.2.3. Fasterthanlime (Amos Wenger)
* **Referencia Técnica:** [fasterthanli.me](https://fasterthanli.me) (*"A story of memory layouts"*, auditoría de binarios y llamadas al sistema).
* **Conceptos Clave:**
  1. **Inspección Profunda de Layouts Binarios:** Auditar el tamaño, la alineación y los offsets de cada campo con `std::mem::offset_of!` para erradicar huecos ocultos de alineación (*padding gaps*).
  2. **Peligros de Desalineación en Hardware Móvil:** CPUs ARM pueden requerir múltiples ciclos para recomponer lecturas que cruzan fronteras de palabras de memoria, y punteros desalineados pasados a llamadas del NDK pueden disparar excepciones de hardware.
  3. **Eliminación de Syscalls Inútiles:** Reducir las llamadas al kernel (`read`, `write`, `gettimeofday`). En emuladores, consultar el reloj del sistema mediante llamadas repetidas a `std::time::Instant::now()` introduce *syscall overhead*. En su lugar, el tiempo debe sincronizarse por ciclos de CPU emulados o eventos VSYNC.

```rust
// ============================================================================
// ❌ ANTIPATRÓN: Medición de tiempo basada en llamadas continuas al sistema
// ============================================================================
pub fn step_emulator_with_syscalls(cpu: &mut CpuState) {
    for _ in 0..10_000 {
        cpu.step();
        // Syscall clock_gettime en cada instrucción: desastre de rendimiento
        let now = std::time::Instant::now();
        if now.elapsed().as_millis() > 16 { break; }
    }
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Sincronización determinista basada en ciclos de CPU
// ============================================================================
pub fn step_emulator_by_cycles(cpu: &mut CpuState, target_cycles: u32) {
    let mut accumulated_cycles = 0;
    // 0 syscalls en el hot loop: control por aritmética de registros locales
    while accumulated_cycles < target_cycles {
        let cycles = cpu.step();
        accumulated_cycles += cycles;
    }
}
```

---

### 10.3. Catálogo de Patrones de Alto Rendimiento en Rust

A continuación se documenta el catálogo de patrones de optimización de sistemas críticos para emulación y runtime de videojuegos:

---

#### 10.3.1. Branchless Bit Manipulation & Population Count (`count_ones` / `trailing_zeros`)
* **Problema:** Determinar qué instancias están activas o qué interrupciones están pendientes escaneando un array booleano genera bifurcaciones condicionales continuas.
* **Solución:** Utilizar bitsets de 64 bits (`u64`) donde cada bit representa el estado de una entidad. Utilizar instrucciones de hardware dedicadas (`trailing_zeros` compila a `rbit` + `clz` en ARM64, ejecutándose en **1 solo ciclo de reloj**).

```rust
pub struct EntityBitset {
    pub mask: u64,
}

impl EntityBitset {
    /// Itera únicamente sobre los índices activos sin evaluar ramas condicionales
    #[inline(always)]
    pub fn for_each_active<F: FnMut(usize)>(&mut self, mut f: F) {
        let mut bits = self.mask;
        while bits != 0 {
            // Encuentra el índice del bit activo más bajo en 1 ciclo de CPU
            let idx = bits.trailing_zeros() as usize;
            f(idx);
            // Limpia el bit procesado: bits = bits & (bits - 1)
            bits &= bits - 1;
        }
    }
}
```

---

#### 10.3.2. Zero-Copy Type Casting & Reinterpretación Pod con `bytemuck`
* **Problema:** Copiar bytes de una ROM emulada a un struct de cabecera mediante serializadores o `memcpy` manual introduce asignaciones y sobrecoste de CPU.
* **Solución:** Utilizar el crate `bytemuck` con los rasgos `Pod` (*Plain Old Data*) y `Zeroable`. Garantiza conversiones de tipos seguras en tiempo de compilación sin copiar un solo byte de memoria.

```rust
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct RomHeader {
    pub magic: [u8; 4],
    pub rom_size: u32,
    pub entry_point: u32,
    pub checksum: u32,
}

pub fn parse_header_zero_copy(rom_bytes: &[u8]) -> Option<&RomHeader> {
    // Conversión segura instantánea: valida longitud y alineación sin copias
    bytemuck::try_from_bytes(&rom_bytes[0..std::mem::size_of::<RomHeader>()]).ok()
}
```

---

#### 10.3.3. Cache-Conscious Lookup Tables (LUTs) con `const fn`
* **Problema:** Calcular funciones trigonométricas (`sin`, `cos`) o transformaciones de color en tiempo de ejecución satura las unidades de coma flotante (FPU).
* **Solución:** Precalcular tablas de búsqueda estáticas alineadas a líneas de caché L1 mediante evaluación de constantes en tiempo de compilación (`const fn`), erradicando el cálculo durante el juego.

```rust
#[repr(align(64))]
pub struct TrigLut {
    pub sin_table: [f32; 360],
}

// Generación en tiempo de compilación: 0 costo en runtime
pub const fn generate_sin_table() -> TrigLut {
    let mut table = [0.0f32; 360];
    let mut i = 0;
    while i < 360 {
        // Aproximación polinómica constexpr para compilación
        let rad = (i as f32) * (3.14159265 / 180.0);
        table[i] = rad; // En producción: serie de Taylor para aproximación exacta
        i += 1;
    }
    TrigLut { sin_table: table }
}

pub static GLOBAL_TRIG_LUT: TrigLut = generate_sin_table();

#[inline(always)]
pub fn fast_sin(angle_degrees: u16) -> f32 {
    let idx = (angle_degrees % 360) as usize;
    unsafe { *GLOBAL_TRIG_LUT.sin_table.get_unchecked(idx) }
}
```

---

#### 10.3.4. Small String Optimization (SSO) e Identifier Interning (`smol_str` / `lasso`)
* **Problema:** Las cadenas cortas en Rust (`String`) siempre viajan al heap global (24 bytes de puntero/capacidad/longitud + reserva en memoria dinámica). En GM8, los nombres de variables como `"x"`, `"y"`, `"speed"`, `"score"` son cadenas breves que saturan el allocator.
* **Solución:** `smol_str::SmolStr`. Cadenas de hasta 22 bytes se almacenan **directamente en el stack o registro** de forma compacta (24 bytes totales), evitando cualquier llamada al heap. Para símbolos repetitivos, el interning con `lasso` los comprime a un entero de 32 bits (`Spur`).

```rust
use smol_str::SmolStr;

pub struct DynamicVariableMap {
    // Si el nombre tiene <= 22 caracteres, 0 asignaciones en heap
    pub name: SmolStr,
    pub value: f64,
}

impl DynamicVariableMap {
    #[inline(always)]
    pub fn new(name: &str, value: f64) -> Self {
        Self {
            name: SmolStr::new(name), // Stack-allocated si es corta
            value,
        }
    }
}
```

---

#### 10.3.5. Unchecked Pointer Arithmetic bajo Invariantes Demostradas
* **Problema:** En el hot loop de emulación de memoria (MMU), el acceso a buffers garantizados de 64 KB sigue emitiendo verificaciones de límites (*bounds checks*).
* **Solución:** Emplear aritmética de punteros crudos (`ptr.add()`) envuelta en funciones puras seguras donde la invariante esté matemáticamente blindada.

```rust
pub struct MemoryController {
    raw_ptr: *mut u8,
    capacity: usize,
}

impl MemoryController {
    pub fn new(capacity: usize) -> Self {
        let mut buffer = vec![0u8; capacity];
        let raw_ptr = buffer.as_mut_ptr();
        std::mem::forget(buffer); // Gestión manual garantizada
        Self { raw_ptr, capacity }
    }

    /// INVARIANTE DEMOSTRADA: addr < capacity garantizado por la máscara binaria previa
    #[inline(always)]
    pub unsafe fn read_byte_unchecked(&self, addr: usize) -> u8 {
        debug_assert!(addr < self.capacity, "Violación crítica de invariante MMU");
        *self.raw_ptr.add(addr)
    }

    #[inline(always)]
    pub unsafe fn write_byte_unchecked(&mut self, addr: usize, val: u8) {
        debug_assert!(addr < self.capacity, "Violación crítica de invariante MMU");
        *self.raw_ptr.add(addr) = val;
    }
}
```

---

## 11. Plantilla Estandarizada para Futuras Incorporaciones

Utiliza la siguiente plantilla estandarizada para registrar futuras notas técnicas, análisis de papers, optimizaciones de compilador o microbenchmarks:

```markdown
### [OPT-XXX] Título Técnico de la Técnica o Recurso
* **Nombre del Recurso / Referencia:** [Título del Paper, Video o Artículo]
* **Enlace / Autor:** [URL Oficial y Autor / Organización]
* **Subsistema Afectado:** [CPU / PPU / APU / MMU / JNI / Memoria]

#### 1. Problema de Hardware o Compilador que Resuelve
[Explicación concisa y técnica sobre por qué se degrada el rendimiento a nivel de:
Líneas de caché L1/L2, saturación del TLB, fallos de predicción de saltos (BTB),
dependencias RAW en el pipeline o contención en el kernel/allocator de Android].

#### 2. Impacto Estimado en Rendimiento
* **Ciclos de CPU / Latencia:** [ej. Reducción de 12 ciclos por instrucción a 2 ciclos]
* **Uso de Memoria / Allocations:** [ej. 0 bytes asignados en runtime / eliminación de 15 bytes de padding]
* **Throughput:** [ej. Aumento de 45 FPS a 60 FPS estables en dispositivos Cortex-A55]

#### 3. Snippet Comparativo

```rust
// ============================================================================
// ❌ ANTIPATRÓN / CÓDIGO INEFICIENTE
// ============================================================================
// [Código fuente antes de optimizar, señalando el cuello de botella exacto]

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA
// ============================================================================
// [Código fuente optimizado, con atributos de compilación y justificación técnica]
```

#### 4. Checklist de Verificación en Compilación
- [ ] Inspeccionado en ensamblador con `cargo asm` (verificado ausencia de `panic_bounds_check` o llamadas indirectas).
- [ ] Verificado en perfil release con `lto = "fat"` y `codegen-units = 1`.
- [ ] Ejecutado micro-benchmark con `criterion` demostrando mejora estadística sin regresiones.
- [ ] Comprobado en dispositivo físico Android mediante `simpleperf` (sin incremento de `L1-dcache-load-misses` ni `branch-misses`).
```

