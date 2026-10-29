# ENGINE_OPTIMIZATIONS_RESEARCH.md: Compendio Avanzado de Arquitectura de Motor y Técnicas de Alto Rendimiento

**Target:** `libgm8emulator.so` (OpenGMK / Rust) + Android Wrapper (Kotlin/JNI/Compose).  
**Hardware Target:** Arquitecturas móviles ARM64 (DynamIQ / big.LITTLE) y GPUs Tile-Based Deferred Rendering (TBDR: Qualcomm Adreno, ARM Mali Bifrost/Valhall, PowerVR).  
**Meta de Frame Pacing:** Latencia determinista de 16.6ms (60 Hz) y 8.3ms (120 Hz) sin judder, desacoplo de física fija mediante acumulador temporal, cero copias de memoria en la frontera JNI y cero asignaciones dinámicas en el hot path.

---

## 1. Diagrama Arquitectónico Integral (Edición EX - Builds 40 y 41)

```
===================================================================================================
                       CAPA DE APLICACIÓN: KOTLIN + JETPACK COMPOSE
===================================================================================================
  [Panel Modular QoL]        [Gestor Partidas/Cloud]        [HUD Táctil Adaptativo Zero-Alloc]
           │                           │                                    │
           │                           │                        Choreographer Frame Callback
           │                           │                                    │
           ▼                           ▼                                    ▼
===================================================================================================
                  FRONTERA JNI DE ULTRA-BAJA LATENCIA (SIN CRUCE POR FRAME)
===================================================================================================
  [AHardwareBuffer Zero-Copy]  ◄── Compartición Directa de GPU ──► [EGLImageKHR / VkImage]
  [DirectByteBuffer / Atomic]  ◄── Canal Atómico Lock-Free SPSC ──► [Bitmask Flags (0ms Input)]
  [@CriticalNative / @FastNative] ◄── Reducción de Costo de Transición JNI de ~30ns a ~2ns
  [PushLocalFrame / PopLocalFrame] ◄── Erradicación de JNI Local Reference Leaks (Límite 512)
===================================================================================================
                                              │
                                   Llamadas a Nivel de OS
                                              ▼
===================================================================================================
                          MOTOR NATIVO RUST: OpenGMK CORE (libgm8emulator.so)
===================================================================================================
  +---------------------------------------------------------------------------------------------+
  | 1. CORE SCHEDULING & AFINIDAD CPU                                                           |
  |    - pthread_setaffinity_np() fijado en núcleos Cortex-X / Cortex-A7x (Big/Prime Cores)     |
  |    - Mitigación de migración de hilos en DynamIQ y aislamiento térmico                      |
  +---------------------------------------------------------------------------------------------+
  | 2. GESTOR TEMPORAL Y FRAME PACING (FIX YOUR TIMESTEP)                                       |
  |    - Acumulador de delta time para física determinista de GM8 a 60 FPS fijos               |
  |    - Interpolación lineal de renderizado (LERP) a 90Hz, 120Hz y 144Hz sin micro-stutter    |
  +---------------------------------------------------------------------------------------------+
  | 3. SISTEMA DE COLISIONES O(1) & ESPACIAL                                                    |
  |    - Spatial Hash Grid con Morton Codes (Z-Order 64-bit)                                    |
  |    - Chunking espacial en memoria para suprimir el overhead de instance_deactivate_region  |
  |    - SIMD ARM NEON bitmasks para pruebas precisas píxel a píxel                             |
  +---------------------------------------------------------------------------------------------+
  | 4. PIPELINE DE RENDERIZADO 2D TIPO LIBGDX SPRITEBATCH                                       |
  |    - Búfer estático de índices precomputados: [0, 1, 2, 2, 3, 0] para N quads               |
  |    - Persistent Mapped Ring Buffers (glBufferStorageEXT) con triple buffering en vuelo      |
  |    - Qualcomm Adreno FlexRender™: Forzar Binning Mode y suprimir GMEM Unresolves            |
  |    - Preservación de Early-Z en Mali: Erradicación de discard() dinámico en shaders         |
  +---------------------------------------------------------------------------------------------+
  | 5. VM & EJECUCIÓN GML DE ALTO RENDIMIENTO (RUST RUNTIME)                                    |
  |    - Data-Oriented Design (DoD): SoA (Struct of Arrays) para instancias activas            |
  |    - String Interning & Symbol Cache (matklad style): comparaciones O(1) de identificadores |
  |    - NaN-Boxing / Shorts-Boxing (value-lang & craftspider): 8 bytes uniformes por Value     |
  |    - Reordenamiento físico de opcodes (Stefan Marr) y macro-fusión (superinstrucciones)     |
  |    - Eliminación de Bounds Checks mediante asserciones hoisteadas pre-bucle                 |
  +---------------------------------------------------------------------------------------------+
  | 6. SUBSISTEMA DE AUDIO DE ULTRA-BAJA LATENCIA                                               |
  |    - AAudio Stream en modo EXCLUSIVE + LOW_LATENCY (Búfer de 1 burst)                       |
  |    - Real-Time SPSC Ring Buffer lock-free (crate rtrb) entre hilo emulador y callback audio  |
  +---------------------------------------------------------------------------------------------+
```

---

## 2. Gestor Temporal y Frame Pacing: "Fix Your Timestep" (Fiedler Pattern)

*Referencia fundamental:* [Fix Your Timestep! (Glenn Fiedler / Gaffer on Games)](https://gafferongames.com/post/fix_your_timestep/).

### 2.1. El Dilema de Refresco en Pantallas Modernas (60Hz vs. 120Hz/144Hz)
El runtime original de GameMaker 8 calcula toda su lógica en función de una velocidad de sala fija (`room_speed = 60` o `30`). Si un emulador en Android corre el bucle de juego directamente sincronizado al VSYNC de una pantalla de 120Hz o 144Hz:
- O el juego corre al **doble de velocidad** (física acelerada).
- O se produce **judder / micro-stutter** si se saltan cuadros arbitrariamente.

### 2.2. Implementación del Acumulador Temporal con Interpolación LERP en Rust
El estado de simulación se actualiza a pasos discretos constantes ($dt = 1/60\,\text{s}$), mientras que el renderizado interpola las posiciones de los quads según el remanente en el acumulador ($\alpha$):

```rust
pub struct TimeStepController {
    fixed_dt: f64,       // 1.0 / 60.0 (16.666 ms)
    accumulator: f64,
    current_time: f64,
}

impl TimeStepController {
    pub fn new(target_fps: f64) -> Self {
        Self {
            fixed_dt: 1.0 / target_fps,
            accumulator: 0.0,
            current_time: 0.0,
        }
    }

    /// Llamado en cada pulso de AChoreographer (ej. a 120 Hz: frame_time = 0.00833s)
    pub fn advance_frame<F, R>(&mut self, frame_time_seconds: f64, mut tick_step: F, mut render_lerp: R)
    where
        F: FnMut(),
        R: FnMut(f32),
    {
        // Limitar frame_time para evitar "espiral de la muerte" tras cuelgues del OS
        let frame_time = frame_time_seconds.min(0.25);
        self.accumulator += frame_time;

        // Consumir el acumulador en pasos fijos deterministas de 60Hz
        while self.accumulator >= self.fixed_dt {
            tick_step(); // Step, Alarms, Collisions de GM8
            self.accumulator -= self.fixed_dt;
        }

        // Factor alfa de interpolación entre el estado anterior y el actual [0.0, 1.0]
        let alpha = (self.accumulator / self.fixed_dt) as f32;
        render_lerp(alpha); // Dibujar en pantalla a 120 FPS fluidos
    }
}
```

* **Interpolación de Posición de Instancias (LERP):**
  $$\text{render\_x} = \text{prev\_x} \cdot (1 - \alpha) + \text{curr\_x} \cdot \alpha$$
  $$\text{render\_y} = \text{prev\_y} \cdot (1 - \alpha) + \text{curr\_y} \cdot \alpha$$
* **Resultado:** Pokémon Reloaded mantiene su física y lógica de batallas intactas a 60 FPS exactos, pero la cámara y los movimientos se perciben a **120 FPS hiperfluidos** sin saltos en pantallas OLED de 120Hz.

---

## 3. Pipeline de Renderizado 2D Tipo LibGDX `SpriteBatch`

*Referencia fundamental:* [LibGDX SpriteBatch.java (Badlogic Games)](https://github.com/libgdx/libgdx/blob/master/gdx/src/com/badlogic/gdx/graphics/g2d/SpriteBatch.java).

### 3.1. Arquitectura de Quads con Búfer Estático de Índices
Para evitar recalcular y subir índices en cada cuadro, se precomputa un único `IndexBuffer` estático (`GL_STATIC_DRAW`) que indexa quads en parejas de triángulos con el patrón `[0, 1, 2, 2, 3, 0]`:

```rust
pub const MAX_QUADS_PER_BATCH: usize = 10920; // 10920 * 6 indices = 65520 (límite de u16)
pub const INDICES_PER_QUAD: usize = 6;
pub const VERTICES_PER_QUAD: usize = 4;

pub fn generate_static_quad_indices() -> Vec<u16> {
    let mut indices = Vec::with_capacity(MAX_QUADS_PER_BATCH * INDICES_PER_QUAD);
    for i in 0..MAX_QUADS_PER_BATCH as u16 {
        let base = i * 4;
        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);
        indices.push(base + 2);
        indices.push(base + 3);
        indices.push(base);
    }
    indices
}
```

### 3.2. Reglas de Volcado de Lote (*Flush Conditions*)
El batch acumula vértices contiguos en memoria de frame (`&mut [f32]`). El volcado (`flush()`) hacia OpenGL ES se dispara estrictamente si:
1. `current_quads == MAX_QUADS_PER_BATCH`.
2. Ocurre un cambio de textura (`active_texture != new_texture`).
3. Ocurre un cambio de shader (`active_shader != new_shader`).
4. Ocurre un cambio de modo de mezcla (`active_blend != new_blend`).

### 3.3. Empaquetado Compacto de Vértices (20 Bytes por Vértice)
```
[x: f32 (4B)] [y: f32 (4B)] [u: f32 (4B)] [v: f32 (4B)] [color: u32 ABGR (4B)] = 20 Bytes
1 Quad = 4 Vértices = 80 Bytes en total.
```
La alineación de 20 bytes satisface las reglas de stride de Qualcomm Adreno y ARM Mali sin rellenado (*padding*) innecesario.

---

## 4. Optimizaciones de VM y Bytecode (Stefan Marr, Bob Nystrom, Danny van Kooten)

*Referencias técnicas:*
- [Squeezing a Little More Performance Out of Bytecode Interpreters (Stefan Marr et al., SAC '23)](https://stefan-marr.de/2023/06/squeezing-a-little-more-performance-out-of-bytecode-interpreters/)
- [Game Programming Patterns: Bytecode (Bob Nystrom)](https://gameprogrammingpatterns.com/bytecode.html)
- [Rewriting an Interpreter in Rust (Danny van Kooten)](https://dannyvankooten.com/blog/2022/rewriting-interpreter-rust/)
- [Rust Optimization Guidelines (Joseph Fransham)](https://gist.github.com/jFransham/369a86eff00e5f280ed25121454acec1)

### 4.1. Reordenamiento Físico de Opcodes y Localidad L1i (Stefan Marr)
La investigación de Stefan Marr demuestra que organizar los casos de un despachador de bytecode según la frecuencia de ejecución y la similitud de microarquitectura acelera la ejecución de **un 7% a un 23%**:
1. **Hot Opcodes (Caché L1i caliente):**
   - Lecturas/escrituras de variables locales (`POP_LOCAL`, `PUSH_LOCAL`).
   - Operaciones aritméticas enteras y flotantes (`ADD`, `SUB`, `MUL`, `DIV`).
   - Saltos condicionales (`BRANCH_IF_FALSE`).
   - Deben agruparse físicamente juntos en el código de Rust para residir en las mismas líneas de caché L1 de instrucciones (L1i de 32 KB).
2. **Cold Opcodes (Aislados con `#[cold]`):**
   - Creación y destrucción de instancias (`instance_create`, `instance_destroy`).
   - Transiciones de sala (`room_goto`).
   - Funciones I/O y diálogos.
   - Forzar a LLVM a segregar estas ramas en bloques fríos de memoria lejana para no ensuciar el Branch Target Buffer (BTB).

### 4.2. Macro-Fusión de Opcodes (Superinstrucciones)
En el bytecode de GameMaker 8, secuencias repetitivas comunes pueden fusionarse en una única instrucción en tiempo de carga:
```
Patrón Tradicional (3 Dispatches):
  1. PUSH_VAR "hp"
  2. PUSH_INT 0
  3. CMP_LE_JUMP label_fainted

Superinstrucción Fusionada (1 Dispatch):
  -> JUMP_IF_VAR_LE_ZERO "hp", label_fainted
```
Reduce la sobrecarga de despacho (*dispatch overhead*) a un tercio en los bucles de daño de Pokémon.

### 4.3. Supresión de Bounds Checks mediante Pre-Condition Hoisting (Fransham)
El compilador `rustc` inserta comprobaciones de límites (`panic_bounds_check`) en cada indexación de vector.
* **Técnica:** Colocar un `assert!` del límite total antes de entrar al bucle permite a LLVM deducir que los accesos interiores son válidos, vectorizando automáticamente el bucle y eliminando saltos de pánico:
  ```rust
  pub fn evaluate_instance_batch(instances: &mut [Instance], len: usize) {
      assert!(instances.len() >= len); // LLVM hoist: elimina todos los bounds checks subsiguientes
      for i in 0..len {
          instances[i].x += instances[i].hspeed;
          instances[i].y += instances[i].vspeed;
      }
  }
  ```

---

## 5. Endurecimiento de la Frontera JNI (Google NDK Guidelines)

*Referencia fundamental:* [Android NDK JNI Tips (Google)](https://developer.android.com/ndk/guides/jni-tips) y [rust-mobile/ndk](https://github.com/rust-mobile/ndk).

### 5.1. `@CriticalNative` y `@FastNative`
En Android Runtime (ART), las llamadas JNI ordinarias incurren en un costo de transición de 30 a 100 ns por guardar registros, verificar transiciones de estado de hilos y registrar referencias locales.
* **`@FastNative`:** Para métodos que no suspenden el hilo de ejecución ni realizan llamadas complejas de vuelta a Java. Reduce el overhead a $\approx 10\,\text{ns}$.
* **`@CriticalNative`:** Para métodos estáticos nativos que solo reciben y devuelven tipos primitivos (`int`, `float`, `long`, `boolean`) y **no reciben punteros `JNIEnv` ni `jclass`**.
  ```kotlin
  // En Kotlin:
  @CriticalNative
  external fun nativeSetInputMask(mask: Int)
  ```
  ```rust
  // En Rust:
  #[no_mangle]
  pub extern "C" fn Java_com_klisee_reloaded_NativeBridge_nativeSetInputMask(mask: i32) {
      INPUT_BUS.store(mask as u32, Ordering::Relaxed);
  }
  ```
* **Costo de Transición:** Reducido a **$\approx 2.5\,\text{ns}$**, comparable a una llamada de función en C directo.

### 5.2. Erradicación del Desbordamiento de la Tabla de Referencias Locales
Android limita estrictamente la tabla de referencias JNI locales a **512 slots**. Crear referencias temporales en bucles intensivos (`FindClass`, `NewStringUTF`) causa el fallo fatal:
`JNI ERROR (app bug): local reference table overflow (max=512)`.

* **Regla de Oro en Rust NDK:**
  1. Envolver bloques con `env.push_local_frame(16)` y `env.pop_local_frame()`.
  2. Cachear `jclass`, `jmethodID` y `jfieldID` en `JNI_OnLoad` en variables atómicas globales permanentes (`NewGlobalRef`). Jamás invocar `FindClass` dentro del game loop.

---

## 6. Cuellos de Botella de Pokémon Reloaded y Soluciones en Rust

### 6.1. La Trampa de `instance_deactivate_region`
En GameMaker 8, para optimizar mapas grandes se suele invocar `instance_deactivate_region` e `instance_activate_region` alrededor de la cámara.
* **Problema Comunitario Comprobado:** Ejecutar esta función cada cuadro obliga al motor a iterar por todas las listas de instancias de la sala, produciendo picos de CPU brutales (*lag spikes*) y desestabilizando el frame pacing.
* **Solución en el Motor Nativo Rust:**
  - **Chunking Espacial Caching:** En lugar de iterar por arrays lineales de miles de instancias, el motor mantiene una cuadrícula espacial de chunks ($256 \times 256$ píxeles).
  - Solo los chunks que cruzan el bounding box de la vista expandida (con un padding de 128 px para evitar pop-in) se consideran activos para el bucle de eventos (`Step`, `Collision`, `Draw`).
  - No se destruyen ni recrean estructuras en heap: el estado se conmuta modificando una máscara de bits `active_mask: BitVec`.

### 6.2. Estructuras de Datos (`ds_grid`, `ds_list`) vs. Memoria Contigua
* **El Problema en GM8:** Las estructuras `ds_*` son recursos dinámicos asignados en el heap de forma individual, generando fragmentación y punteros dispersos (*pointer chasing*).
* **Optimización en OpenGMK:**
  - Mapear cada `ds_grid` a un vector plano unidimensional contiguo en memoria (`Box<[Value]>`), donde la posición $(x, y)$ se indexa branchless:
    $$\text{Index} = y \cdot \text{width} + x$$
  - Aprovechar la contigüidad para que las lecturas en bucle de mapas de colisiones caigan directamente en las líneas de caché L1 de la CPU ARM.

### 6.3. Representación de Variables Dinámicas con NaN-Boxing
GameMaker 8 es un entorno dinámico débilmente tipado donde una variable puede ser un número real, una cadena o un ID de instancia.

* **El Problema del Tagged Enum:** Un `enum Value { Real(f64), String(StringId), Instance(u32), Bool(bool) }` ocupa 32 bytes debido al discriminante y al alineamiento.
* **NaN-Boxing (8 Bytes Totales):** El estándar IEEE 754 de 64 bits (`f64`) contiene millones de combinaciones de bits que representan `NaN` (Not-a-Number). El exponente contiene todos los bits en 1 (`0x7FF8000000000000`), dejando 51 bits de carga útil (*payload*).
* **Layout de Memoria:**
  ```
  [1 bit Signo] [11 bits Exponente = 1] [1 bit Quiet NaN = 1] [4 bits Tag] [48 bits Payload / Pointer]
  ```
* **Ventajas en OpenGMK:**
  - Cada variable ocupa **exactamente 8 bytes**.
  - Duplica la cantidad de variables que caben simultáneamente en la caché L1d de la CPU.
  - Los números reales de punto flotante (`f64`) se leen y operan sin conversiones adicionales.

---

## 7. Arquitectura Interna de GameMaker 8 y Reversa de Ejecutables

*Referencias técnicas:*
- [UndertaleModTool (UnderminersTeam)](https://github.com/UnderminersTeam/UndertaleModTool)
- [RetroReversing Game Maker Architecture](https://www.retroreversing.com/game-maker)
- [GM8Decompiler & OpenGMK Rust Core](https://github.com/OpenGMK/GM8Decompiler)
- [OpenGM C++ Engine Reimplementation](https://github.com/misternebula/OpenGM)
- [TCRF GameMaker Legacy Technical Notes](https://tcrf.net/Help:Contents/Finding_Content/Game_Engines/GameMaker_Legacy)

### 7.1. Estructura de Chunks del Ejecutable GM8
Los ejecutables de GameMaker 8 (`.exe`) concatenan el runner compilado en Delphi con un paquete de recursos cifrados/comprimidos al final del archivo. La estructura de chunks contiene:
- **`FORM` / `GEN8`**: Metadatos generales, versión del engine (8.0 o 8.1), título y configuraciones.
- **`SPRT`**: Sprites empaquetados con máscaras de colisión precisas (`bitmaps` transparentes).
- **`BGND`**: Fondos y tilesets de mapa.
- **`OBJT`**: Definiciones de objetos, eventos enlazados y acciones.
- **`ROOM`**: Disposición inicial de instancias de salas, vistas (*views*) y fondos.
- **`CODE` / `SCPT`**: Scripts y eventos en GML (AST o bytecode precompilado).
- **`STRG`**: Tabla global de cadenas de texto.

### 7.2. Ciclo de Ejecución de Eventos en GameMaker 8
En cada iteración del bucle principal, el runner de GameMaker ejecuta los eventos de forma estrictamente secuencial y determinista:
1. **Begin Step Event:** Actualización previa de temporizadores y variables de entrada.
2. **Alarm Events:** Disparo de alarmas (`alarm[0..11]`).
3. **Keyboard / Mouse / Input Events:** Actualización de controles del jugador.
4. **Step Event:** Lógica central de Pokémon, NPCs, inteligencia artificial y proyectiles.
5. **End Step Event:** Resolución final de posiciones y sincronización de vistas (*view_xview*, *view_yview*).
6. **Collision Events:** Comprobación de pares de colisiones espaciales.
7. **Draw Event:** Despacho de gráficos (fondos, tiles, instancias en orden de profundidad, HUD).

OpenGMK replica esta secuencia en Rust pero reemplaza los bucles lineales de Delphi por **tablas SoA e índices espaciales hash**.

---

## 8. Checklist de Implementación Inmediata

- [ ] **Controlador Temporal (Fix Your Timestep):** Implementar `TimeStepController` para desacoplar el tick de física fija (60 Hz) del frame pacing de pantalla (120 Hz) con interpolación de quads.
- [ ] **SpriteBatch LibGDX-Style:** Integrar el búfer estático de índices `[0, 1, 2, 2, 3, 0]` y vertex buffer compacto de 20 bytes por vértice.
- [ ] **Anotaciones JNI `@CriticalNative`:** Marcar funciones de entrada y audio en Kotlin como `@CriticalNative` para reducir la latencia de transición a 2.5 ns.
- [ ] **Superinstrucciones en VM de GML:** Fusionar pares de instrucciones frecuentes (ej. `PUSH_VAR` + `CMP_ZERO`) en opcodes únicos.
- [ ] **Reordenamiento de Opcodes (Stefan Marr):** Reorganizar el `match` principal del intérprete para concentrar los opcodes aritméticos y de salto en bloques contiguos de L1i.
- [ ] **Data-Oriented Instances (SoA):** Migrar el array de instancias en `OpenGMK` a vectores continuos alineados (`x`, `y`, `hspeed`, `vspeed`).
- [ ] **String Interning para Variables GML:** Implementar `FastInterner` en la resolución de nombres de variables para sustituir comparaciones de cadenas por comprobaciones de enteros `Symbol(u32)`.
- [ ] **NaN-Boxing en `Value` Type:** Sustituir la estructura actual de valores dinámicos por un tipo `Value(u64)` compacto de 8 bytes.
- [ ] **Pinning CPU Big.LITTLE:** Forzar afinidad de hilos en `libgm8emulator.so` hacia los núcleos de mayor frecuencia mediante `sched_setaffinity`.
