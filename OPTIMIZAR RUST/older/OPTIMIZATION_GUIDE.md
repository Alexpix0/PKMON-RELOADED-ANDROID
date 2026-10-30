# Guía Maestra de Optimización en Rust para Emuladores en Android NDK
**Target:** `libgm8emulator.so` (ARM64 / ARMv7)  
**Herramienta de compilación:** `cargo-ndk`  
**Objetivo de diseño:** 60/120 FPS estables, cero asignaciones en el hot loop, saturación de caché L1 y latencia JNI determinista.

---

## 1. Directivas de Compilación y Perfil Release (`Cargo.toml` & NDK)

El compilador de Rust (`rustc`) y el backend LLVM aplican por defecto directivas conservadoras orientadas a velocidad de compilación y trazabilidad de errores. Para emulación de hardware a alta velocidad en SoCs ARM de dispositivos móviles, es imperativo forzar optimizaciones agresivas inter-módulos y mitigar sobrecostes de desenrollado de pila (*unwinding*).

### 1.1. Configuración de `Cargo.toml`

```toml
[profile.release]
# Nivel máximo de optimización de LLVM (vectorización agresiva, fusión de bucles)
opt-level = 3

# Link-Time Optimization 'fat': optimización inter-crate completa en todo el grafo de dependencias
lto = "fat"

# Reduce las unidades de generación de código a 1 para permitir a LLVM razonar sobre
# todo el binario simultáneamente (elimina barreras entre CGUs para inlining global)
codegen-units = 1

# Elimina el runtime de unwinding de excepciones. Los panics abortan inmediatamente el proceso,
# reduciendo el tamaño del binario y eliminando tablas de desenrollado EH de la caché L1i.
panic = "abort"

# Remueve símbolos de debug y tablas de reubicación no esenciales del .so final
strip = "symbols"

# Deshabilita comprobaciones numéricas de desbordamiento en release
overflow-checks = false

# Deshabilita assertions de debug en dependencias
debug-assertions = false
```

### 1.2. Flags LLVM y Configuración de Target (`.cargo/config.toml`)

Crea o edita `.cargo/config.toml` para inyectar flags de arquitectura al backend de LLVM para las arquitecturas Android prioritarias (`aarch64-linux-android` y `armv7-linux-androideabi`):

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

#### Justificación Técnica de Flags:
* `+neon`: Habilita instrucciones vectoriales SIMD de 128 bits para el pipeline de video (PPU) y mezcla de audio (APU).
* `-Wl,-Bsymbolic`: Enlace dinámico preferente intra-biblioteca. En Android, evita que las llamadas a funciones internas de `libgm8emulator.so` pasen por la PLT (Procedure Linkage Table), ahorrando indirecciones en llamadas frecuentes.
* `-Wl,--gc-sections`: Elimina funciones y datos muertos no referenciados, reduciendo el footprint en RAM y almacenamiento.

### 1.3. Pipeline de Compilación con `cargo-ndk`

```bash
# Compilación para arquitectura estándar moderna (64 bits - ARMv8-A)
cargo ndk -t arm64-v8a -o ./android/app/src/main/jniLibs build --release

# Compilación para dispositivos heredados (32 bits - ARMv7)
cargo ndk -t armeabi-v7a -o ./android/app/src/main/jniLibs build --release
```

---

## 2. Catálogo de Buenas Prácticas (Síntesis Técnica: Leapcell)

### 2.1. Gestión de Memoria y Supresión de Asignaciones en el Heap

#### Cuello de botella en CPU/Memoria
En Android, el asignador de memoria global (`jemalloc` o `scudo` del NDK) introduce contención de locks, llamadas al sistema (`mmap`/`brk`) y fragmentación de la memoria virtual. En un emulador que ejecuta a 60 FPS, cada frame dispone de 16.6 ms (o 8.3 ms a 120 FPS). Si el ciclo de instrucciones ejecuta 10 millones de ciclos por segundo, cualquier viaje al heap dentro de los loops de dispatch o decodificación introduce stalls térmicos y jitter en el frametime.

#### Técnicas de Mitigación:
1. **Pre-reserva (`with_capacity`):** Previene reasignaciones geométricas amortizadas (`realloc` + `memcpy`).
2. **Reutilización de Búferes:** Reutilizar estructuras persistentes mediante `.clear()` (conserva el buffer de memoria subyacente y reinicia la longitud a 0).
3. **Asignación en Pila (`SmallVec` / `ArrayVec`):** Mantener micro-buffers (ej. colas de interrupciones, estados temporales de micro-operaciones) estrictamente en el stack frame del hilo.
4. **Erradicación de Formateo Dinámico:** `format!`, `to_string()` y concatenaciones `+` asignan dinámicamente en el heap. Deben desterrarse de rutas de decodificación y ejecución.

```rust
// ============================================================================
// ❌ ANTIPATRÓN: Asignaciones dinámicas continuas en el hot path del emulador
// ============================================================================
pub struct CpuBad {
    pub pending_interrupts: Vec<u8>,
}

impl CpuBad {
    pub fn step(&mut self) {
        // Asignación dinámica en cada llamada al step
        let mut executed_ops = Vec::new();
        executed_ops.push(0xEA);

        // format! asigna un nuevo String en el heap en cada ciclo
        if self.pending_interrupts.len() > 10 {
            let log_msg = format!("Alerta: Cola saturada con {} elementos", self.pending_interrupts.len());
            native_log(&log_msg);
        }
    }
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Zero-heap allocation y retención de memoria
// ============================================================================
use smallvec::SmallVec;

pub struct CpuOptimized {
    // SmallVec<[T; N]> almacena hasta N elementos directamente en el stack.
    // Solo migra al heap si excede la capacidad estática (improbable para IRQs).
    pub pending_irqs: SmallVec<[u8; 8]>,
    // Buffer pre-asignado persistente
    pub trace_buffer: Vec<u32>,
}

impl CpuOptimized {
    pub fn new() -> Self {
        Self {
            pending_irqs: SmallVec::new(),
            trace_buffer: Vec::with_capacity(1024), // Capacidad fija pre-asignada
        }
    }

    #[inline(always)]
    pub fn step(&mut self) {
        // Reutilización del buffer sin liberar memoria (capacidad intacta, len = 0)
        self.trace_buffer.clear();
        self.trace_buffer.push(0xEA);

        // Sin asignaciones: array de bytes estático o buffers fijos en pila
        if self.pending_irqs.len() > 8 {
            static MSG: &[u8] = b"Alerta: Cola de IRQ saturada\0";
            native_log_raw(MSG);
        }
    }
}
```

---

### 2.2. Iteradores vs. Bucles Tradicionales y Bounds Checking

#### Cuello de botella en CPU/Memoria
En Rust, el acceso a arrays e índices mediante `slice[i]` evalúa de forma obligatoria que `i < slice.len()`. Si el compilador no puede deducir los invariantes de longitud en tiempo de compilación, inyecta bifurcaciones condicionales (`cmp` + `b.ge` + salto a `panic_bounds_check`). Esto causa:
1. Contaminación de la tabla de predicción de saltos (*Branch Target Buffer* / BTB de ARM).
2. Inhibición de la auto-vectorización por parte de LLVM (el compilador no puede fusionar instrucciones en registros NEON si existen posibles bifurcaciones de pánico dentro del cuerpo del bucle).

Los iteradores de Rust implementan el rasgo `ExactSizeIterator` y `TrustedLen`, lo que permite a LLVM demostrar los límites y eliminar por completo el *bounds checking*, desbloqueando vectorización SIMD automática.

```rust
// ============================================================================
// ❌ ANTIPATRÓN: Indexación manual con bounds check en cada acceso (Scanline PPU)
// ============================================================================
pub fn render_scanline_indexed(framebuffer: &mut [u32], vram_palette: &[u32], tile_pixels: &[u8]) {
    // LLVM no puede garantizar que tile_pixels.len() == framebuffer.len()
    // Genera 2 comprobaciones de límites por iteración + saltos a panic
    for i in 0..framebuffer.len() {
        let palette_idx = tile_pixels[i] as usize; // Check 1
        framebuffer[i] = vram_palette[palette_idx]; // Check 2 & Check 3
    }
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Iteradores acoplados (Zip) y trozos exactos
// ============================================================================
pub fn render_scanline_iter(framebuffer: &mut [u32], vram_palette: &[u32; 256], tile_pixels: &[u8]) {
    // zip asegura que la iteración termine en el menor de ambos sin verificaciones intermedias.
    // vram_palette con tamaño fijo [u32; 256] convierte el índice u8 (0..255) en infallible: 0 checks.
    // LLVM auto-vectoriza este bucle usando registros NEON de ARM (vld1 / vst1).
    framebuffer.iter_mut()
        .zip(tile_pixels.iter())
        .for_each(|(pixel_out, &color_idx)| {
            *pixel_out = vram_palette[color_idx as usize];
        });
}

// Alternativa con chunks exactos para procesamiento SIMD manual o de 4 píxeles:
pub fn render_scanline_chunks(framebuffer: &mut [u32], color: u32) {
    // chunks_exact evita la comprobación de restos en cada bloque
    let mut chunks = framebuffer.chunks_exact_mut(4);
    for chunk in chunks.by_ref() {
        chunk[0] = color;
        chunk[1] = color;
        chunk[2] = color;
        chunk[3] = color;
    }
    // Procesa el remanente si la resolución no es múltiplo de 4
    for pixel in chunks.into_remainder() {
        *pixel = color;
    }
}
```

---

### 2.3. Estructuras de Datos Óptimas y Omisión de SipHash

#### Cuello de botella en CPU/Memoria
`std::collections::HashMap` utiliza por defecto `SipHash-1-3`, un algoritmo criptográficamente resistente contra ataques de denegación de servicio por colisión de hashes (Hash DoS). Para un emulador local:
* El coste computacional de calcular `SipHash` por cada lectura/escritura de MMIO, breakpoint o tabla de símbolos es prohibitivo (decenas de ciclos de CPU por lookup).
* Las estructuras dispersas con nodos enlazados generan saltos aleatorios en la memoria física (*pointer chasing*), rompiendo las líneas de caché L1/L2.

#### Solución:
* **Lookup de claves enteras o breakpoints:** Usar `FxHashMap` (algoritmo FxHash de Firefox, basado en multiplicación y rotación rápida) o `ahash` (aprovecha aceleración por hardware AES presente en ARMv8-A).
* **Mapeo de Memoria (RAM, VRAM, ROM):** Descartar HashMaps. Emplear arrays contiguos `[u8; N]`, slices planos o tablas de páginas paginadas de tamaño fijo indexadas directamente por la dirección de memoria desplazada.

```rust
// ============================================================================
// ❌ ANTIPATRÓN: HashMap con SipHash para mapeo de memoria y breakpoints
// ============================================================================
use std::collections::HashMap;

pub struct MemoryBusBad {
    // SipHash por cada acceso: desastre de rendimiento en hot path
    pub mmio_registers: HashMap<u16, u8>,
    // Punteros dispersos que destruyen la coherencia de la caché L1
    pub page_table: HashMap<u16, Vec<u8>>,
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Tablas directas planas y FxHashMap para depuración
// ============================================================================
use rustc_hash::FxHashMap;

pub const PAGE_SIZE: usize = 4096;
pub const NUM_PAGES: usize = 16; // Ej. 64KB divididos en páginas de 4KB

pub struct MemoryBusOptimized {
    // Array continuo plano: acceso en O(1) puro mediante suma y desplazamiento binario
    pub fast_ram: [u8; 64 * 1024],
    
    // Tabla de páginas plana mediante punteros directos: indexado inmediato
    pub read_pages: [*const u8; NUM_PAGES],
    pub write_pages: [*mut u8; NUM_PAGES],

    // Para tablas dispersas no críticas (ej. breakpoints en debug), FxHashMap es ultrarrápido
    pub breakpoints: FxHashMap<u32, u8>,
}

impl MemoryBusOptimized {
    #[inline(always)]
    pub fn read8(&self, addr: u16) -> u8 {
        // Cálculo aritmético puro: 0 latencia de hashing
        let page = (addr >> 12) as usize;
        let offset = (addr & 0x0FFF) as usize;
        unsafe {
            let base_ptr = self.read_pages[page];
            *base_ptr.add(offset)
        }
    }
}
```

---

### 2.4. Control de Inlining y Predicción de Bifurcaciones

#### Cuello de botella en CPU/Memoria
El bucle central de decodificación y ejecución de instrucciones (*instruction dispatch*) se ejecuta miles de millones de veces.
* El exceso de llamadas a función añade sobrecoste de prólogo/epílogo de pila (instrucciones `stp`, `ldp`, guardado de registros temporales `x19-x28` en ARM64).
* El exceso de inlining indiscriminado genera *código gigante* (*code bloat*), expulsando las instrucciones críticas fuera de la memoria caché de instrucciones L1 (L1i, típicamente de solo 32KB o 64KB en CPUs móviles Cortex-A55/A78).
* Las ramas impredecibles generan *pipeline flushes* (coste: 12 a 20 ciclos de penalización por fallo en CPUs ARM OoO modernas).

```rust
// ============================================================================
// ❌ ANTIPATRÓN: Inlining descontrolado o llamadas indirectas sin priorización
// ============================================================================
pub fn execute_instruction_bad(opcode: u8) {
    match opcode {
        0x00 => nop(),
        0xEA => jump_unconditional(),
        0xFF => illegal_opcode_handler(), // Raro, pero tratado igual por el compilador
        _ => fallback(),
    }
}

fn illegal_opcode_handler() {
    // Código extenso de dump de registros, formateo de logs, trace de stack...
    // Si se hace inline, infla la función principal y destruye la caché L1i.
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Segregación #[inline(always)] vs #[cold]
// ============================================================================
pub struct Cpu;

impl Cpu {
    // Inlining forzado en el dispatcher de alta frecuencia
    #[inline(always)]
    pub fn decode_and_step(&mut self, opcode: u8) {
        match opcode {
            0x00 => self.op_nop(),
            0x01 => self.op_lda_imm(),
            0xFF => self.handle_illegal_opcode(opcode), // Rama fría
            _ => self.handle_unimplemented(opcode),
        }
    }

    #[inline(always)]
    fn op_nop(&mut self) {
        // Ejecución en 1-2 ciclos
    }

    #[inline(always)]
    fn op_lda_imm(&mut self) {
        // Acceso directo a registros
    }

    // Marca la función como fría: LLVM la coloca al final de la sección .text,
    // garantizando que las páginas calientes de la caché L1i contengan solo código frecuente.
    #[cold]
    #[inline(never)]
    fn handle_illegal_opcode(&mut self, opcode: u8) {
        panic!("Opcode ilegal detectado: 0x{:02X}", opcode);
    }

    #[cold]
    #[inline(never)]
    fn handle_unimplemented(&mut self, opcode: u8) {
        eprintln!("Opcode no implementado: 0x{:02X}", opcode);
    }
}
```

---

### 2.5. Layout de Memoria, Alineación y Coherencia de Caché

#### Cuello de botella en CPU/Memoria
Las arquitecturas ARM leen y escriben datos en líneas de caché L1D (típicamente de 64 bytes).
* Un struct mal organizado genera relleno (*padding*) inútil que desperdicia espacio en caché.
* Si múltiples hilos (ej. hilo de CPU y el hilo de rasterización de la PPU) modifican campos adyacentes ubicados en la misma línea de 64 bytes, la CPU activa el protocolo de coherencia de caché (MESI/MOESI), invalidando la línea en todos los núcleos. Este fenómeno se denomina **False Sharing** y degrada dramáticamente el paralelismo.

```rust
// ============================================================================
// ❌ ANTIPATRÓN: Struct desorganizado con padding excesivo y riesgo de False Sharing
// ============================================================================
pub struct BadState {
    pub flag_a: bool,     // 1 byte
    // 7 bytes de padding aquí
    pub counter: u64,     // 8 bytes
    pub flag_b: bool,     // 1 byte
    // 3 bytes de padding
    pub cpu_cycle: u32,   // 4 bytes
    // Compartido entre hilos sin separación de línea de caché:
    pub ppu_vblank_irq: bool, // Mismo segmento de 64 bytes que las variables de CPU
}

// ============================================================================
// ✅ IMPLEMENTACIÓN OPTIMIZADA: Alineación explícita #[repr(C)], packing y #[repr(align(64))]
// ============================================================================
#[repr(C)]
pub struct Registers {
    // Agrupación contigua por tamaño decreciente para eliminar padding
    pub pc: u32,
    pub sp: u32,
    pub eax: u32,
    pub ebx: u32,
    pub flags: u16,
    pub irq_enabled: bool,
    pub halted: bool,
} // Tamaño exacto y predecible compatible con C / NDK

// Aislamiento de variables de hilos para eliminar False Sharing
#[repr(align(64))]
pub struct CpuThreadState {
    pub cycle_count: u64,
    pub registers: Registers,
}

#[repr(align(64))]
pub struct PpuThreadState {
    pub scanline: u32,
    pub frame_counter: u32,
    pub vblank_requested: bool,
}
```

---

## 3. Trampas Críticas de Rendimiento (Síntesis Técnica: Llogiq)

### 3.1. El Costo Oculto de `.clone()`

#### Causa del problema
En Rust, `.clone()` es explícito, pero la facilidad sintáctica para sortear las restricciones del *borrow checker* lleva a duplicar estructuras de datos completas de forma silenciosa.
* Clonar un `Vec<u8>` que representa un banco de ROM o una tabla de patrones genera una asignación en el heap (`malloc`) seguida de una copia masiva de memoria (`memcpy`).
* Pasar parámetros por valor que implementan `Clone` sin necesidad retiene ciclos innecesarios y destruye el throughput.

```rust
// ============================================================================
// ❌ CÓDIGO INEFICIENTE: Clonación defensiva para satisfacer al borrow checker
// ============================================================================
pub struct RomCartridge {
    pub rom_data: Vec<u8>,
}

impl RomCartridge {
    // Clona todo el vector de ROM (ej. 4MB) solo para leer el header
    pub fn read_header(&self) -> Vec<u8> {
        let cloned_rom = self.rom_data.clone();
        cloned_rom[0..16].to_vec()
    }
}

// ============================================================================
// ✅ ALTERNATIVA DE ALTO RENDIMIENTO: Préstamos contiguos (&[u8]) y tipos Copy
// ============================================================================
pub struct RomCartridgeOptimized {
    pub rom_data: Vec<u8>,
}

impl RomCartridgeOptimized {
    // Retorna una referencia a una rodaja (&[u8]): 0 copias, 0 bytes asignados
    #[inline(always)]
    pub fn read_header(&self) -> &[u8] {
        &self.rom_data[0..16]
    }

    // Si se requiere un buffer de tamaño fijo, retornar array por valor [u8; 16] (implementa Copy)
    #[inline(always)]
    pub fn read_header_fixed(&self) -> [u8; 16] {
        let mut header = [0u8; 16];
        header.copy_from_slice(&self.rom_data[0..16]);
        header
    }
}
```

---

### 3.2. La Trampa de `.collect()`

#### Causa del problema
Encadenar `.collect::<Vec<_>>()` en pipelines de iteradores fuerza la materialización completa de una colección intermedia en el heap. 
* Si se procesan datos de audio, listas de sprites visibles o transformaciones de bytes, cada `.collect()` introduce una asignación dinámica seguida de su inmediata destrucción en el scope siguiente.
* Se satura el asignador de memoria y se expulsa el conjunto de trabajo de la memoria caché.

```rust
// ============================================================================
// ❌ CÓDIGO INEFICIENTE: Materialización de vectores intermedios innecesarios
// ============================================================================
pub fn get_visible_sprites_bad(sprites: &[Sprite], current_scanline: i32) -> Vec<Sprite> {
    // collect() asigna un Vec en el heap en CADA SCANLINE (ej. 240 veces por frame)
    sprites.iter()
        .filter(|s| s.y <= current_scanline && current_scanline < s.y + 8)
        .cloned()
        .collect::<Vec<Sprite>>()
}

// ============================================================================
// ✅ ALTERNATIVA DE ALTO RENDIMIENTO: Consumo directo sobre buffer existente o iterador
// ============================================================================
#[derive(Clone, Copy)]
pub struct Sprite {
    pub x: u8,
    pub y: i32,
    pub tile: u8,
}

pub struct PpuRenderer {
    // Buffer reutilizable de tamaño fijo (máximo 8 sprites por scanline, estándar hardware)
    pub scanline_sprites: [Option<Sprite>; 8],
    pub sprite_count: usize,
}

impl PpuRenderer {
    #[inline(always)]
    pub fn evaluate_sprites(&mut self, sprites: &[Sprite], current_scanline: i32) {
        self.sprite_count = 0;
        
        // Iteración directa sin asignación en el heap
        for sprite in sprites.iter() {
            if sprite.y <= current_scanline && current_scanline < sprite.y + 8 {
                if self.sprite_count < 8 {
                    self.scanline_sprites[self.sprite_count] = Some(*sprite);
                    self.sprite_count += 1;
                } else {
                    break; // Límite de hardware alcanzado
                }
            }
        }
    }
}
```

---

### 3.3. Indirección y Pérdida de Localidad (*Pointer Chasing*)

#### Causa del problema
El despacho dinámico (`Box<dyn Trait>` o `&dyn Trait`) introduce dos penalizaciones mayores a nivel de hardware:
1. **Doble indirección:** Para invocar un método, la CPU debe cargar el puntero al objeto, luego cargar el puntero a la vtable (`*vtable`), buscar la dirección de la función y finalmente saltar indirectamente (`blr` en ARM64).
2. **Inhibición de inlining:** El compilador no puede resolver la llamada en tiempo de compilación, impidiendo la optimización global inter-procedimental.
3. **Fallo de predicción:** Las llamadas indirectas provocan fallos constantes en el predictor de saltos indirectos del procesador móvil.

```rust
// ============================================================================
// ❌ CÓDIGO INEFICIENTE: Dynamic dispatch con Box<dyn BusDevice> en el bus de memoria
// ============================================================================
pub trait BusDevice {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);
}

pub struct BusWithDyn {
    // Cada elemento es un puntero en el heap con vtable: pointer chasing constante
    pub devices: Vec<Box<dyn BusDevice>>,
}

impl BusWithDyn {
    pub fn read(&self, addr: u16) -> u8 {
        for dev in &self.devices {
            let v = dev.read(addr); // Salto indirecto a través de vtable
            if v != 0xFF { return v; }
        }
        0xFF
    }
}

// ============================================================================
// ✅ ALTERNATIVA DE ALTO RENDIMIENTO: Polimorfismo estático cerrado mediante Enums
// ============================================================================
pub struct RamDevice { pub data: [u8; 2048] }
pub struct RomDevice { pub data: &'static [u8] }
pub struct PpuRegisters { pub status: u8 }

// Enum discriminador: sin vtables, sin heap, sin indirección de punteros
pub enum HardwareDevice {
    Ram(RamDevice),
    Rom(RomDevice),
    Ppu(PpuRegisters),
}

impl HardwareDevice {
    #[inline(always)]
    pub fn read(&self, addr: u16) -> u8 {
        match self {
            HardwareDevice::Ram(ram) => ram.data[(addr & 0x7FF) as usize],
            HardwareDevice::Rom(rom) => rom.data[addr as usize],
            HardwareDevice::Ppu(ppu) => ppu.status,
        }
    }
}

pub struct StaticBus {
    pub ram: RamDevice,
    pub rom: RomDevice,
    pub ppu: PpuRegisters,
}

impl StaticBus {
    // Inlining completo: LLVM convierte esto en un branch directo altamente predecible
    #[inline(always)]
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.ram.data[(addr & 0x7FF) as usize],
            0x2000..=0x3FFF => self.ppu.status,
            0x8000..=0xFFFF => self.rom.data[(addr - 0x8000) as usize],
            _ => 0xFF,
        }
    }
}
```

---

### 3.4. Sobrecarga de Monomorfización (*Code Bloat*)

#### Causa del problema
El polimorfismo paramétrico mediante Generics (`fn execute<T: CpuMode>(mode: T)`) duplica el código binario generado para cada tipo `T` concreto con el que se instancie.
* Si una función central de decodificación o ejecución de 2,000 líneas de código LLVM se monomorfiza para 8 combinaciones de estado de CPU, se generarán 8 copias idénticas en el archivo `.so`.
* El tamaño del segmento `.text` se dispara, saturando la caché de instrucciones L1i (32KB) del núcleo de CPU del SoC móvil y provocando *L1i cache thrashing*.

#### Solución: "Thin Generics" (Separación de núcleo no genérico)
Extraer toda la lógica pesada e invariable a funciones internas sin generics que operen sobre slices o tipos estándar, dejando el wrapper genérico únicamente para la adaptación de interfaces.

```rust
// ============================================================================
// ❌ CÓDIGO INEFICIENTE: Monomorfización masiva que duplica toda la lógica
// ============================================================================
pub trait MemoryAccess {
    fn fetch(&self, addr: u32) -> u32;
}

// Esta función completa de 500 líneas se duplicará para cada tipo M
pub fn run_cpu_cycle_bloated<M: MemoryAccess>(mem: &M, pc: u32) {
    // ... Cientos de líneas de decodificación compleja ...
    let _opcode = mem.fetch(pc);
    // ... Operaciones aritméticas ...
}

// ============================================================================
// ✅ ALTERNATIVA DE ALTO RENDIMIENTO: Thin Generic delegando a core monomórfico
// ============================================================================
// Función central: compilada EXACTAMENTE UNA VEZ en el binario final
fn run_cpu_cycle_core(fetch_fn: fn(*const (), u32) -> u32, context: *const (), pc: u32) {
    let _opcode = fetch_fn(context, pc);
    // Toda la lógica pesada reside aquí de forma unificada
}

// Wrapper inline liviano: no infla la caché L1i
#[inline(always)]
pub fn run_cpu_cycle_thin<M: MemoryAccess>(mem: &M, pc: u32) {
    fn trampoline<M: MemoryAccess>(ptr: *const (), addr: u32) -> u32 {
        unsafe { (&*(ptr as *const M)).fetch(addr) }
    }
    run_cpu_cycle_core(trampoline::<M>, mem as *const M as *const (), pc);
}
```

---

### 3.5. Manejo de Cadenas vs. Slices de Bytes

#### Causa del problema
Utilizar `String` o `&str` para nombres de registros, títulos de ROM, validaciones de cabeceras o tags de estado obliga al runtime a validar codificación UTF-8 en cada instancia (`str::from_utf8`). 
* En hardware y emulación, los datos binarios son colecciones arbitrarias de bytes. Validar UTF-8 consume ciclos innecesarios y puede disparar panics accidentales ante caracteres no estándar de ROMs japonesas o volcados hexadecimales.
* Utilizar siempre rodajas de bytes sin formato: `&[u8]`, literales de bytes `b"MAGIC_HEADER"` o arrays `[u8; N]`.

```rust
// ============================================================================
// ❌ CÓDIGO INEFICIENTE: Verificación de cadenas UTF-8 dinámicas
// ============================================================================
pub fn validate_cartridge_bad(header: &[u8]) -> bool {
    // Intenta decodificar a String: asigna memoria y verifica UTF-8 byte a byte
    if let Ok(magic) = String::from_utf8(header[0..4].to_vec()) {
        return magic == "GM8A";
    }
    false
}

// ============================================================================
// ✅ ALTERNATIVA DE ALTO RENDIMIENTO: Comparación de enteros o slices de bytes
// ============================================================================
pub fn validate_cartridge_optimized(header: &[u8]) -> bool {
    // Comparación directa de slices de bytes: compilado a instrucción 'cmp' directa
    if header.len() >= 4 && &header[0..4] == b"GM8A" {
        return true;
    }
    false
}

// Variante aún más rápida: reinterpretación como entero nativo u32 (1 sola instrucción de CPU)
#[inline(always)]
pub fn validate_cartridge_u32(header: &[u8]) -> bool {
    if header.len() < 4 { return false; }
    // En little-endian: b"GM8A" = 0x41384D47
    let magic = u32::from_le_bytes(header[0..4].try_into().unwrap());
    magic == 0x41384D47
}
```

---

## 4. Arquitectura Crítica para Emulación en Android

### 4.1. Loop Principal de Ejecución: Zero-Allocation por Frame

El loop principal de emulación orquesta el paso coordinado entre los tres componentes principales: CPU (ciclos de instrucción), PPU (generación de píxeles/scanlines) y APU (muestras de audio).

#### Regla de Oro:
**Durante la ejecución de `run_frame()`, la tasa de asignaciones en el heap (`malloc` / `realloc`) debe ser estrictamente CERO.** Toda la memoria requerida para el estado, búferes de scanline, framebuffer y anillo de audio se pre-asigna en el arranque del emulador (`Emulator::new()`).

```rust
pub const FRAME_WIDTH: usize = 256;
pub const FRAME_HEIGHT: usize = 240;
pub const AUDIO_BUFFER_SIZE: usize = 1024;

pub struct EmulatorState {
    // Framebuffer continuo de video (RGBA8888 empaquetado como u32)
    pub video_buffer: Box<[u32; FRAME_WIDTH * FRAME_HEIGHT]>,
    
    // Búfer circular de audio pre-asignado
    pub audio_ring_buffer: Box<[i16; AUDIO_BUFFER_SIZE]>,
    pub audio_write_head: usize,

    // Estados internos
    pub cpu_cycles: u64,
    pub is_frame_ready: bool,
}

impl EmulatorState {
    pub fn new() -> Self {
        Self {
            // Asignación única inicial durante la carga
            video_buffer: Box::new([0xFF000000; FRAME_WIDTH * FRAME_HEIGHT]),
            audio_ring_buffer: Box::new([0; AUDIO_BUFFER_SIZE]),
            audio_write_head: 0,
            cpu_cycles: 0,
            is_frame_ready: false,
        }
    }

    /// Hot loop por frame. Ejecuta hasta que la PPU señale que se completó un cuadro de video (VBlank)
    #[inline(always)]
    pub fn run_frame(&mut self) {
        self.is_frame_ready = false;
        
        while !self.is_frame_ready {
            // 1. Ejecutar ciclo de CPU (0 asignaciones)
            let cycles_spent = self.step_cpu();

            // 2. Sincronizar PPU por los ciclos transcurridos
            self.step_ppu(cycles_spent);

            // 3. Sincronizar APU por los ciclos transcurridos
            self.step_apu(cycles_spent);
        }
    }

    #[inline(always)]
    fn step_cpu(&mut self) -> u32 {
        // Ejecución de la instrucción en curso
        4 // Retorna ciclos transcurridos
    }

    #[inline(always)]
    fn step_ppu(&mut self, _cycles: u32) {
        // Renderizado directo en self.video_buffer
        // Al terminar el último scanline, conmuta la bandera
        self.is_frame_ready = true;
    }

    #[inline(always)]
    fn step_apu(&mut self, _cycles: u32) {
        // Inserción en el anillo de audio modular sin reasignación
        let head = self.audio_write_head;
        self.audio_ring_buffer[head] = 0; // Muestra calculada
        self.audio_write_head = (head + 1) % AUDIO_BUFFER_SIZE;
    }
}
```

---

### 4.2. Búfer de Video y Frontera JNI: Zero-Copy

Transferir el framebuffer de 60 cuadros por segundo desde Rust hacia la capa Java/Kotlin en Android puede provocar saturación del bus de memoria y recolectores de basura si se utilizan mecanismos ingenuos como `env.byte_array_from_slice()` (el cual reserva un nuevo array Java `byte[]` en el heap ART y ejecuta un `memcpy`).

Existen dos arquitecturas de alto rendimiento para eliminar por completo la copia intermodal:

---

#### Opción A: Renderizado Directo sobre `ANativeWindow` (Recomendado)
Se pasa una instancia de `android.view.Surface` directamente al código nativo mediante JNI. Rust adquiere la ventana nativa mediante el NDK (`ANativeWindow_fromSurface`), bloquea el buffer de píxeles (`ANativeWindow_lock`), renderiza directamente sobre la memoria mapeada de GPU/SurfaceFlinger y publica el frame (`ANativeWindow_unlockAndPost`).

```rust
use std::ffi::c_void;
use jni::JNIEnv;
use jni::objects::JObject;
use jni::sys::{jobject, jlong};

// Declaraciones de tipos y funciones NDK de libandroid.so
type ANativeWindow = c_void;

#[repr(C)]
struct ANativeWindow_Buffer {
    width: i32,
    height: i32,
    stride: i32,
    format: i32,
    bits: *mut c_void,
    reserved: [u32; 6],
}

#[link(name = "android")]
extern "C" {
    fn ANativeWindow_fromSurface(env: *mut jni::sys::JNIEnv, surface: jobject) -> *mut ANativeWindow;
    fn ANativeWindow_release(window: *mut ANativeWindow);
    fn ANativeWindow_lock(window: *mut ANativeWindow, outBuffer: *mut ANativeWindow_Buffer, inOutDirtyBounds: *mut c_void) -> i32;
    fn ANativeWindow_unlockAndPost(window: *mut ANativeWindow) -> i32;
}

/// JNI Hook exportado para dibujar el frame directamente en la Surface de Android
#[no_mangle]
pub unsafe extern "C" fn Java_com_gm8_emulator_NativeBridge_renderNativeSurface(
    mut env: JNIEnv,
    _class: JObject,
    emulator_ptr: jlong,
    surface: jobject,
) {
    let emulator = &mut *(emulator_ptr as *mut EmulatorState);
    let raw_env = env.get_raw();
    
    // 1. Obtener el descriptor nativo de la ventana
    let window = ANativeWindow_fromSurface(raw_env, surface);
    if window.is_null() { return; }

    let mut buffer: ANativeWindow_Buffer = std::mem::zeroed();

    // 2. Bloquear el búfer del compositor (SurfaceFlinger)
    if ANativeWindow_lock(window, &mut buffer, std::ptr::null_mut()) == 0 {
        let dest_ptr = buffer.bits as *mut u32;
        let stride = buffer.stride as usize;

        // 3. Copia directa de línea a línea respetando el pitch/stride de la GPU
        // Zero copias intermedias en Java. Transmisión directa a memoria de display.
        for y in 0..FRAME_HEIGHT {
            let src_offset = y * FRAME_WIDTH;
            let dst_offset = y * stride;
            
            std::ptr::copy_nonoverlapping(
                emulator.video_buffer.as_ptr().add(src_offset),
                dest_ptr.add(dst_offset),
                FRAME_WIDTH,
            );
        }

        // 4. Desbloquear y enviar al compositor de pantalla
        ANativeWindow_unlockAndPost(window);
    }

    ANativeWindow_release(window);
}
```

---

#### Opción B: Compartición de Memoria via `DirectByteBuffer`
Si la arquitectura requiere que la capa Kotlin/Java o un shader OpenGL/Vulkan consuma el buffer, se crea un `DirectByteBuffer` en el lado nativo o Java. Este búfer reside fuera del Garbage Collector (`DirectMemory`), permitiendo a Rust y a la GPU acceder al mismo puntero físico sin intervención del recolector de basura.

```rust
use jni::JNIEnv;
use jni::objects::JObject;
use jni::sys::{jobject, jlong};

/// Inicializa y retorna un DirectByteBuffer mapeado al video_buffer de Rust
#[no_mangle]
pub unsafe extern "C" fn Java_com_gm8_emulator_NativeBridge_getDirectFrameBuffer(
    mut env: JNIEnv,
    _class: JObject,
    emulator_ptr: jlong,
) -> jobject {
    let emulator = &mut *(emulator_ptr as *mut EmulatorState);
    
    // Obtiene el puntero crudo al inicio del buffer de video
    let buffer_ptr = emulator.video_buffer.as_mut_ptr() as *mut u8;
    let buffer_size = (FRAME_WIDTH * FRAME_HEIGHT * 4) as usize;

    // Crea un ByteBuffer directo apuntando a la dirección física en memoria de Rust
    // Java puede acceder a este buffer inmediatamente sin copia alguna
    let direct_buffer = env.new_direct_byte_buffer(buffer_ptr, buffer_size)
        .expect("Fallo al crear DirectByteBuffer");

    direct_buffer.into_raw()
}
```

---

## 5. Sección Modular Extensible (Plantillas Estandarizadas)

Esta sección define las plantillas para incorporar de forma uniforme nuevas guías de optimización, análisis con herramientas de perfilado del NDK (ej. `simpleperf`) y reportes de benchmark.

---

### 5.1. Plantilla: Nueva Entrada de Guía / Nota de Optimización

```markdown
### [TAG-00X] Título Técnico de la Técnica / Optimización
* **Origen / Fuente:** [URL / Paper / Autor]
* **Subsistema Afectado:** [CPU / PPU / APU / MMIO / JNI]
* **Impacto Estimado:** [Crítico (Hot Path) / Medio (Warm Path) / Bajo (Inicialización)]

#### Cuello de Botella a Nivel de Hardware / Runtime
[Explicación concisa y técnica de por qué ocurre el problema a nivel de registros,
líneas de caché L1/L2, predicción de saltos o contención en el runtime del NDK].

#### Código Comparativo

```rust
// ❌ ANTIPATRÓN / CÓDIGO INEFICIENTE
// [Insertar snippet de código problemático con anotaciones]

// ✅ IMPLEMENTACIÓN OPTIMIZADA
// [Insertar snippet de código optimizado con anotaciones]
```

#### Verificación y Validación
* **Comando de prueba / benchmark:** `cargo bench --bench <nombre>`
* **Métricas objetivo:** [Instrucciones por ciclo (IPC), ciclos de CPU ahorrados, bytes asignados]
```

---

### 5.2. Plantilla: Análisis de Perfilado con `simpleperf` (Android NDK)

```markdown
### [PERF-00X] Análisis de Perfilado: [Nombre del Escenario / Bottleneck]
* **Dispositivo de prueba:** [Modelo / SoC (ej. Snapdragon 8 Gen 2 / Exynos 2200)]
* **Target ABI:** [arm64-v8a / armeabi-v7a]
* **Build:** `cargo ndk build --release` (con símbolos no strippeados para debug de perfiles)

#### 1. Comando de Captura con `simpleperf`
```bash
# Registrar eventos de CPU en el proceso del emulador por 10 segundos
adb shell simpleperf record \
    -p $(adb shell pidof com.gm8.emulator) \
    -e cpu-cycles,instructions,L1-dcache-load-misses,branch-misses \
    --duration 10 \
    -o /data/local/tmp/perf.data

# Descargar y reportar datos
adb pull /data/local/tmp/perf.data .
simpleperf report --sort comm,symbol -i perf.data -n --limit 10
```

#### 2. Flamegraph y Hotspots Detectados
| Símbolo / Función Nativa | % CPU Samples | Evento Dominante | Diagnóstico de Causa Raíz |
| :--- | :--- | :--- | :--- |
| `libgm8emulator::cpu::decode_step` | 42.5% | `L1-icache-miss` | Monomorfización excesiva |
| `libgm8emulator::mmu::read8` | 28.1% | `branch-misses` | Bounds check en cada acceso |
| `libgm8emulator::jni::render` | 14.3% | `L1-dcache-miss` | Copia no alineada en frame |

#### 3. Acción Correctiva y Resultado
* **Parche aplicado:** [Referencia a commit o PR]
* **Resultado post-optimización:**
  - `CPU Samples`: -X%
  - `Frametime medio`: Reducido de 18.2 ms a 13.9 ms (estable a 60 FPS)
```

---

### 5.3. Plantilla: Registro de Micro-Benchmarks (`criterion`)

```markdown
### [BENCH-00X] Micro-Benchmark: [Nombre del Módulo / Función]

#### Configuración del Benchmark (`benches/emu_bench.rs`)
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

pub fn bench_instruction_dispatch(c: &mut Criterion) {
    let mut emu = create_test_emulator();
    c.bench_function("dispatch_100k_instructions", |b| {
        b.iter(|| {
            for _ in 0..100_000 {
                emu.step_instruction(black_box(0xEA));
            }
        })
    });
}

criterion_group!(benches, bench_instruction_dispatch);
criterion_main!(benches);
```

#### Comparativa de Rendimiento
| Implementación | Tiempo Medio (ns / op) | Variación (%) | Memoria Asignada |
| :--- | :--- | :--- | :--- |
| **Línea base (Original)** | 4.82 ns | -- | 128 bytes / op |
| **Optimizado (Branchless + Zero-Heap)** | 0.91 ns | **-81.1%** | 0 bytes / op |
```

