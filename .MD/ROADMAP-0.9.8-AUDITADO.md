# ROADMAP TÉCNICO v0.9.8 (BUILD 39) — V3 AUDITADA (ESTÁTICA)
**Proyecto:** Pokémon Reloaded Android (OpenGMK Native Engine + Android NDK Wrapper)
**Versión objetivo:** `v0.9.8` (`versionCode = 39`, `versionName = "0.9.8"`)
**Meta:** combates y ciudades al `room_speed` real del juego sin caídas, más cierre de 3 bugs de integración.

---

## 0. ALCANCE Y ESTADO DE VERIFICACIÓN (LEER PRIMERO)

La V2 original se redactó sin acceso al repositorio. Esta V3 se contrastó con el paquete `PARA QUE REVISES CLAUDE.7z` recibido el **2026-10-22**: inspección estática de archivos Rust/Android, configuración de build y notas de pantalla negra. El archivo no incluye metadatos `.git`, por lo que no se pudo fijar commit, comparar diffs ni determinar si todos los archivos están versionados.

**Límite de verificación:** no se ejecutaron `cargo check`, `cargo test`, build Gradle ni pruebas en Android: el entorno de auditoría no dispone de `rustc`/`cargo`, Gradle o `gradlew`, ni dispositivo conectado. Los hallazgos de código llevan rutas y líneas aproximadas del paquete; los defectos que dependen de driver/dispositivo se marcan como pendientes de reproducción. Ningún objetivo de FPS se considera demostrado.

Regla de esta V3: **ninguna optimización entra al código sin una medición previa (Fase 0), y ninguna se da por buena sin repetir la medición y pasar los gates de regresión.**
> **Resumen de auditoría (prioridad):** antes de distribuir el build, resolver la exposición de material de firma y la versión Android que aún declara `38 / 9.7`; después, corregir el falso supuesto de que el canal de snapshots ya separa lógica/render, conectar o desactivar esa ruta, cachear las variables de entorno leídas por frame y verificar la discrepancia `GL_LINEAR`/PowerVR. Los hallazgos y las soluciones quedan detallados en §11.


---

## 1. AUDITORÍA CRÍTICA DE LA V1

| # | Debilidad en V1 | Por qué importa | Corrección en V2 |
|---|---|---|---|
| 1 | Sin Fase 0 de medición. Los "hallazgos forenses" se presentan como evidencia pero no hay trazas, números ni capturas. | Se puede optimizar lo equivocado. Con lógica y render en hilos separados, 35 ms puede ser el coste de **GML (hilo lógica)** y no del render. | Fase 0: `simpleperf` + Perfetto + contadores GPU; presupuesto de frame por hilo. |
| 2 | §1.1 atribuye 5-12 ms al mutex. | Un mutex sin contención cuesta decenas de ns. Esperar milisegundos implica que el lock se mantiene **durante una copia/clonado grande del snapshot**. La causa es la copia, no el candado. | §3.1: eliminar la copia (triple buffer + buffers prealocados). Quitar el mutex sin quitar la copia no arregla nada. |
| 3 | §1.2 habla de "syscalls" por `env::var`. | `getenv` en Bionic es un escaneo de `environ` en memoria, sin syscall. Cuesta ~µs; 10 llamadas/frame son ruido (<0,05 ms). | Se mantiene la limpieza (barata y correcta) pero **se retira de los criterios de aceptación como causa del cuello**. |
| 4 | §2.1 mezcla "Ring Buffer SPSC" con "triple búfer". Son estructuras distintas. | Un anillo conserva *todos* los frames (cola); un triple buffer conserva *el último* (estado). Para snapshots de render se quiere el segundo. | §3.1 especifica el triple buffer con intercambio de índices. |
| 5 | §2.3 propone agrupar todas las partículas `bm_normal` y luego `bm_add`. | **Rompe el orden de dibujado** (GM dibuja por `depth` y orden de creación). Alpha normal no es conmutativo: cambia el resultado visual. | §3.6: solo se fusionan *tramos consecutivos* del mismo blend; solo el aditivo contra aditivo puede reordenarse. |
| 6 | §2.4 "intrínsecos NEON / fast-math" para `lengthdir_*`, `point_direction`. | `sin/cos` escalar no se vectoriza con NEON útilmente; `fast-math` no existe en Rust estable y rompería la compatibilidad numérica con GM8 (grados, redondeos) y la determinabilidad de replays. | Eliminado. Se conserva la semántica exacta; se optimiza el despacho, no la matemática. |
| 7 | §2.4 propone enchufar el JIT ARM64 sin análisis de riesgo. | W^X, icache, BTI/PAC, OEM kernels, `SIGSEGV` irrecuperable. Además GML es dinámico: sin especialización de tipos el JIT gana poco frente a un VM bien hecho. | §3.7: JIT queda **fuera del release**, tras flag, tras el VM plano y con tests diferenciales. |
| 8 | Meta fija "60 FPS". | Si `room_speed` del juego es 30, el objetivo real es sostener 30 lógicas/s sin picos; presentar a 60 Hz es otra cuestión (pacing). Lo observado (26-31 FPS caminando) sugiere que **conviene confirmar `room_speed`**. | §6: la meta se expresa contra `room_speed` y contra P95/P99 de frametime. |
| 9 | Ciudades: no se aborda el orden por `depth` ni los tiles. | Es el cuello principal fuera de combate. | §3.3 y §3.4. |
| 10 | `Vulkan Pipeline Alignment` en 1 día; JIT en 2 días; 8 días en total. | Planificación irreal; sin puertas de calidad. | §7: fases con *gates* medibles, sin fechas rígidas. |
| 11 | No hay plan de pruebas (concurrencia, equivalencia semántica GML, regresión visual). | "Sin margen de fallo" exige tests. | §8: `loom`, tests diferenciales, comparación de capturas, asignador contador. |
| 12 | Los 3 bugs de integración no figuran. | Bloquean release. | §5. |
| 13 | `no_std` aparece en el enunciado pero el código usa `std::sync::Mutex`/`std::env`. | Inconsistencia: el motor es `std` sobre NDK. | Se asume `std`. Los diseños aquí usan solo `core::sync::atomic` + `std::cell`. |
| 14 | Faltan frame pacing y térmica. | En Mali/Adreno el stutter y el throttling dominan la percepción. | §3.8: Swappy/`eglPresentationTimeANDROID`, ADPF `PerformanceHintManager`. |

---

## 2. FASE 0 — MEDICIÓN (OBLIGATORIA ANTES DE PROGRAMAR)

Objetivo: saber **qué hilo, qué función y qué estado de GPU** consumen el frame.

1. **Instrumentación propia (barata, siempre disponible):** `Instant` por sección, volcado cada 120 frames a logcat tag `RLD_PERF`:
   `gml_step`, `collision`, `snapshot_build`, `sort`, `submit`, `gl_draw`, `swap`, y `wait_for_render` / `wait_for_logic`.
2. **CPU:** `simpleperf record -g -p <pid> --duration 20` en un combate 2v2 con clima y otro en ciudad; reporte por hilo (`--sort comm,symbol`).
3. **GPU:**
   - Snapdragon: Snapdragon Profiler (draw calls, GMEM load/store, texture bandwidth).
   - Mali: Arm Streamline / Performance Advisor (tile-buffer, bytes de lectura/escritura externa).
   - Genérico: `adb shell dumpsys gfxinfo <pkg> framestats`, Perfetto (`gpu.counters`, `frame timeline`).
4. **Contadores por frame a registrar:** nº de `glDrawElements`, nº de `glBindTexture`/cambios de atlas, nº de `glBindFramebuffer`, nº de `glBlendFunc`, bytes subidos a VBO, nº de asignaciones (ver §8.4).
5. **Confirmar `room_speed`** y el tiempo máximo de frame que realmente exige el juego.

**Gate Fase 0:** tabla con presupuesto por hilo y top-10 de funciones en cada escenario. Las prioridades de las fases siguientes se **reordenan según esa tabla** (el orden de §7 es el esperado, no un dogma).

---

## 3. ARQUITECTURA V2

```
HILO LÓGICA (GML)                              HILO RENDER (GLES 3.0 / Vulkan)
┌────────────────────────────┐                 ┌─────────────────────────────────────────┐
│ VM plano + fast-path campos│  TripleBuffer   │ RenderConfig cacheado (sin env::var)    │
│ Spatial hash broad-phase   │ ─────────────►  │ Capas de tiles pre-baked (VBO estático) │
│ Emite DrawCmd POD ordenados│  (swap índices) │ Fusión depth: estático+dinámico         │
│ Partículas SoA prealocado  │                 │ Atlas Texture-Array (1 bind, N páginas) │
└──────────┬─────────────────┘                 │ Texto: atlas de glifos + caché de quads │
           │ ControlChannel (SPSC anillo)      │ FBO cache + invalidate (TBDR)           │
           └─ BakeTiles / TileDirty / Resize ─►└─────────────────────────────────────────┘
```

Dos canales, con propósitos distintos:
- **FrameChannel (triple buffer):** *estado* por frame; gana el último.
- **ControlChannel (anillo SPSC acotado):** *eventos* que no pueden perderse (bake de tiles, dirty de capas, resize, cambio de config, carga de atlas).

### 3.1 Canal de snapshots: Triple Buffer lock-free (Zero-Copy / Zero-Alloc)

**Veredicto:** un anillo SPSC simple **no basta** para este caso. Si el productor es más rápido, el anillo se llena y hay que bloquear o descartar con lógica extra; si el consumidor es más rápido, hay que clonar o recorrer elementos. El triple buffer con intercambio de **índices** resuelve ambos: el productor nunca espera, el consumidor siempre ve el último frame completo, y no se copia ni un byte (se intercambian 3 slots prealocados).

Nota: no se hace `std::mem::swap` sobre los `Vec` (exigiría acceso exclusivo a ambos y volvería a necesitar un candado). Se intercambian **índices atómicos**; cada hilo es dueño exclusivo de un slot en cada instante.

```rust
// render/frame_channel.rs
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

const IDX_MASK: u8 = 0b0011;
const DIRTY: u8 = 0b0100;

#[repr(align(64))] // evita false sharing entre slots y entre atómicos
struct Slot<T>(UnsafeCell<T>);

#[repr(align(64))]
struct PaddedAtomic(AtomicU8);

struct Shared<T> {
    slots: [Slot<T>; 3],
    middle: PaddedAtomic, // índice del slot intermedio | DIRTY si hay frame nuevo
}
// SAFETY: cada slot lo toca como máximo un hilo a la vez (protocolo de índices).
unsafe impl<T: Send> Sync for Shared<T> {}

pub struct FrameProducer<T> { shared: Arc<Shared<T>>, back: u8 }
pub struct FrameConsumer<T> { shared: Arc<Shared<T>>, front: u8 }

pub fn frame_channel<T: Send>(make: impl Fn() -> T) -> (FrameProducer<T>, FrameConsumer<T>) {
    let shared = Arc::new(Shared {
        slots: [Slot(UnsafeCell::new(make())), Slot(UnsafeCell::new(make())), Slot(UnsafeCell::new(make()))],
        middle: PaddedAtomic(AtomicU8::new(1)),
    });
    (FrameProducer { shared: shared.clone(), back: 0 }, FrameConsumer { shared, front: 2 })
}

impl<T> FrameProducer<T> {
    /// Slot exclusivo del productor. Contiene el frame de hace 2 publicaciones:
    /// el llamador DEBE hacer .clear() de sus Vec (conserva capacidad -> zero-alloc).
    #[inline] pub fn back_mut(&mut self) -> &mut T {
        unsafe { &mut *self.shared.slots[self.back as usize].0.get() }
    }
    /// Publica el slot de escritura y recupera el que estaba en medio. Nunca bloquea.
    #[inline] pub fn publish(&mut self) {
        let prev = self.shared.middle.0.swap(self.back | DIRTY, Ordering::AcqRel);
        self.back = prev & IDX_MASK;
    }
}

impl<T> FrameConsumer<T> {
    /// true si había frame nuevo (el consumidor ahora lo posee).
    #[inline] pub fn try_acquire(&mut self) -> bool {
        if self.shared.middle.0.load(Ordering::Relaxed) & DIRTY == 0 { return false; }
        let prev = self.shared.middle.0.swap(self.front, Ordering::AcqRel); // sin DIRTY
        self.front = prev & IDX_MASK;
        true
    }
    #[inline] pub fn front(&self) -> &T {
        unsafe { &*self.shared.slots[self.front as usize].0.get() }
    }
}
```

**Snapshot: solo datos POD y arenas prealocadas (nada de `Vec<Vec<_>>`, `String`, `Rc`, `Box<dyn>`):**

```rust
// render/snapshot.rs
#[repr(C)] #[derive(Copy, Clone)]
pub struct DrawCmd {
    pub sort_key: u64,        // (depth_biased:u32 << 32) | seq:u32  -> orden GM8 estable
    pub kind: CmdKind,        // Sprite, Rect, Line, Text, Tilemap(static), Particle, SurfaceBlit, Raw
    pub blend: BlendMode,     // normal/add/sub/max
    pub tex: TexRef,          // (atlas_layer:u8, flags) o SideTexture(id)
    pub a: [f32; 8],          // payload: x,y,xs,ys,ang,u0,v0,u1,v1 ... según kind
    pub color: u32,           // RGBA8 premultiplicado según convención actual [VERIFICAR]
    pub extra: u32,           // índice a text_arena / tile_layer_id / surface_id
}

pub struct FrameSnapshot {
    pub frame_id: u64,
    pub view: ViewState,              // cámaras/puertos (Copy)
    pub cmds: Vec<DrawCmd>,           // with_capacity(32_768)
    pub text_arena: Vec<u8>,          // UTF-8 concatenado; DrawCmd.extra = offset|len
    pub static_layers_visible: Vec<u16>, // ids de capas bake visibles este frame
}
impl FrameSnapshot {
    pub fn with_capacity() -> Self { /* prealoca una vez */ unimplemented!() }
    #[inline] pub fn begin(&mut self, frame_id: u64) {
        self.frame_id = frame_id;
        self.cmds.clear(); self.text_arena.clear(); self.static_layers_visible.clear();
    }
}
```

**Reglas:**
- El hilo lógica ordena `cmds` **in situ** con radix sort estable sobre `sort_key` usando un `Vec<u32>` de índices prealocado (no `sort_by` con asignación).
- Si `cmds.len() == cmds.capacity()` → `debug_assert!` + contador `snapshot_overflow` (se descarta el comando en release; nunca se realoca).
- Con triple buffer el render puede repetir un frame (si va más rápido que la lógica) o saltarse uno (si va más lento): es el comportamiento deseado. Si el render necesita interpolar, que use `prev_x/prev_y` en el payload (opcional; no activar por defecto).

**Verificación obligatoria:** test `loom` del protocolo de índices (ver §8.1) + test de estrés 10⁸ publicaciones con checksum monótono.

### 3.2 `RenderConfig` sin `env::var` en el bucle caliente

```rust
// render/config.rs
pub struct RenderConfig {
    pub filter_mode: AtomicU8,    // 0 nearest, 1 linear
    pub render_scale_q8: AtomicU16, // escala*256
    pub bezel_active: AtomicBool,
    pub vsync_mode: AtomicU8,
    pub backend_requested: AtomicU8,
    pub generation: AtomicU32,    // +1 en cada cambio
}
pub static CONFIG: RenderConfig = RenderConfig::new(); // const fn
```
- Se rellena una vez en arranque y desde `SettingsBridge` (JNI). El render lee con `Relaxed` y compara `generation` al inicio del frame para reconstruir estado (viewport, filtros) **solo** si cambió.
- Se mantiene la lectura de `OPENGMK_*` únicamente en `RenderConfig::init_from_env()`, una vez.
- Verificación: `rg "env::var" src/OpenGMK/**/render*` debe devolver únicamente `init_from_env`.

### 3.3 Capas de tiles estáticas pre-baked (ciudades)

**Corrección al planteamiento:** lo que rompe el batch en una ciudad no son las capas de tiles (suelen estar en pocos `depth` fijos) sino los **cambios de textura entre instancias** intercaladas por `depth = -y`. Aun así, enviar y ordenar miles de tiles por frame es un coste puro: se elimina.

**Semántica GM8 a respetar [VERIFICAR contra el comportamiento actual de OpenGMK]:**
- Los tiles se pueden añadir/borrar/modificar en runtime (`tile_add`, `tile_delete`, `tile_set_*`, `tile_layer_*`) → "estático" significa *sin cambios desde el último bake*, no *inmutable*.
- Un tile pertenece a una capa por `depth`; el orden entre tiles del mismo `depth` es el de creación. **No se pueden reordenar tiles de distinta textura dentro de una capa si se solapan**: se generan *runs* consecutivos.
- Orden de empate tile vs. instancia en el mismo `depth` → confirmar en OpenGMK y replicarlo en el criterio de `sort_key` (`seq`).

```rust
// render/tile_bake.rs
pub struct TileDesc { pub bg_atlas_layer: u8, pub src: [f32;4], pub x: f32, pub y: f32,
                      pub w: f32, pub h: f32, pub color: u32, pub order: u32 }

pub struct TileRun { pub atlas_layer: u8, pub first_index: u32, pub index_count: u32 }

pub struct TileLayerBake {
    pub depth: i32,
    pub vbo_offset: u32,       // en vértices, dentro del VBO estático
    pub runs: Vec<TileRun>,    // preservan orden original
    pub bbox: [f32; 4],        // para cull grueso por vista
    pub dirty: bool,
    pub visible: bool,
}

pub struct StaticTileStore { pub vbo: GLuint, pub ibo: GLuint, pub layers: Vec<TileLayerBake> }

impl StaticTileStore {
    pub fn bake_layer(&mut self, gl: &Gl, depth: i32, tiles: &[TileDesc]); // 1 vez, en el hilo render
    pub fn mark_dirty(&mut self, depth: i32);                               // desde ControlChannel
    pub fn rebuild_dirty(&mut self, gl: &Gl);                               // al inicio del frame
}
```

**Flujo:**
1. `room_start` → hilo lógica envía por **ControlChannel** `Control::BakeTiles { layers: Box<[TileLayerInput]> }` (una sola vez, fuera del frame caliente).
2. `tile_*` que modifique una capa → `Control::TileDirty(depth)` + reenvío de esa capa. Coste O(N de la capa) una vez; nunca por frame.
3. Cada frame el hilo lógica solo añade `static_layers_visible.push(layer_id)` (no tiles).
4. VBO/IBO estáticos con `GL_STATIC_DRAW`; índices `u16` por tramos (≤65 535 vértices por run) o `u32` si hay soporte.
5. **Cull opcional por chunks (fase 3b):** dividir cada capa en rejillas de 512 px solo si se comprueba que no hay solape dependiente del orden entre chunks; si no, se deja una sola unidad.

**Fusión por `depth` en el render (merge de dos secuencias ordenadas):**
```rust
pub enum DrawItem<'a> { Dynamic(&'a [DrawCmd]), StaticLayer(u16) }
fn build_draw_list(snap: &FrameSnapshot, store: &StaticTileStore, out: &mut Vec<DrawItem>);
// Recorre cmds (ya ordenados) y static_layers_visible (ordenadas por depth) y emite
// segmentos alternados. Nº de segmentos = (capas estáticas visibles) + 1 en el peor caso.
```
**Optimización extra:** capas estáticas adyacentes (sin ninguna instancia dinámica con `depth` entre ambas) se funden en un único rango en tiempo de composición.

### 3.4 Batching multi-textura: `GL_TEXTURE_2D_ARRAY` (no arreglo de samplers)

**Corrección técnica clave:** en GLSL ES 3.00 **no se puede indexar un array de `sampler2D` con un valor por-vértice/por-fragmento** (solo expresiones constantes; ES 3.2 exige uniformidad dinámica). Un "uber-shader con 8-16 samplers" seleccionado por atributo es inválido o cae a cadenas de `if` costosas en Mali. La solución portable es **`sampler2DArray`**: una única textura, un bind, la página va en la coordenada `z`.

```glsl
#version 300 es
precision mediump float; precision mediump sampler2DArray;
uniform sampler2DArray u_atlas;
in vec2 v_uv; in vec4 v_col; flat in int v_layer;
out vec4 o_col;
void main(){ o_col = texture(u_atlas, vec3(v_uv, float(v_layer))) * v_col; }
```
Vértice (20 B): `pos: [f32;2]`, `uv: [u16;2]` (normalizado), `color: [u8;4]`, `layer: u8` + padding, usando `glVertexAttribIPointer` para `layer`.

```rust
pub struct AtlasArray {
    pub tex: GLuint,            // GL_TEXTURE_2D_ARRAY, RGBA8, sin mipmaps
    pub page_size: u16,         // 2048 por defecto
    pub layers_used: u8, pub layers_cap: u8,   // cap según presupuesto de VRAM
    pub side_textures: Vec<SideTex>,           // fondos > page_size, backgrounds con tiling, surfaces
}
impl AtlasArray {
    pub fn alloc_rect(&mut self, w: u16, h: u16) -> Option<AtlasRect>; // empaquetador shelf/maxrects
    pub fn upload(&mut self, gl: &Gl, rect: AtlasRect, rgba: &[u8]);
}
```
**Restricciones que deben quedar codificadas:**
- Presupuesto de memoria: 2048² RGBA8 ≈ 16 MiB por capa. En gama baja limitar a ~8 capas (128 MiB) y medir; usar páginas de 1024 si la VRAM compartida es crítica.
- Bordes: padding de 1-2 px con extrusión de bordes en cada sprite (sin sangrado con filtro lineal). El flag "interpolate" de GM8 se respeta usando `nearest` global del array o dos arrays (nearest/linear) **[VERIFICAR cómo lo resuelve hoy el renderer]**.
- Texturas con wrap/tiling (backgrounds repetidos) y surfaces **no** entran en el array: batch propio.
- El orden por `depth` no se altera: el batching solo fusiona comandos **consecutivos** compatibles (mismo blend, mismo shader, mismo array). Con el array, "cambiar de sprite_index" ya no rompe el batch, que es el objetivo.
- Vulkan: mismo concepto (imagen 2D array + `layer` en vértice); evita depender de *descriptor indexing*, no garantizado en gama baja.

### 3.5 Texto y surfaces en el HUD de combate

**Estado a verificar [VERIFICAR]:** las fuentes de GM8 vienen como glifos pre-rasterizados en el `.gmk`/`data.win`; OpenGMK ya los sube a atlas. Por tanto el trabajo es:

1. **Glifos en el `AtlasArray`** (misma textura que los sprites) → el texto no rompe batch.
2. **Generación de quads sin asignar:** `draw_text` escribe directamente `DrawCmd` + vértices en arenas del snapshot (no `String`, no `Vec` temporales).
3. **Caché de geometría de texto** (solo si Fase 0 lo justifica; con unas decenas de strings el coste de generar quads es del orden de microsegundos):
```rust
pub struct TextKey { font: u16, hash: u64, halign: u8, valign: u8 } // hash de (str, sep, w, scale)
pub struct TextEntry { quads: Range<u32>, last_used: u32 }
pub struct TextCache { map: FxHashMap<TextKey, TextEntry>, arena: Vec<GlyphQuad>, frame: u32 }
// Posición: quads relativos al origen + offset (x,y) por comando (uniform o atributo de instancia).
// "Dirty-flag por string" no existe en GML (draw_text es inmediato): la clave es el contenido.
// Evicción LRU por frame, tope de tamaño fijo.
```
4. **Surfaces / FBO (lo que de verdad duele en TBDR):**
   - `FboCache` por `surface_id`; **nunca** crear/destruir FBO por frame.
   - Estado `bound_fbo: GLuint` cacheado → `bind_fbo()` se salta si no cambia.
   - Al terminar de escribir un FBO: `glInvalidateFramebuffer` de depth/stencil (y de color si se va a sobrescribir entero) para evitar *store* a memoria externa. Sin attachment de depth/stencil en surfaces 2D.
   - `glClear` solo cuando se necesita; evitar `glReadPixels`/`surface_getpixel` en el frame (provocan *flush* y bloqueo de pipeline) → si el juego lo usa, resolver en CPU con espejo o diferir.
   - Garantía de fin de frame: `debug_assert!(bound_fbo == 0)` antes de `swap`; si un script deja un `surface_set_target` abierto, se fuerza `surface_reset_target` y se loguea (esto también es candidato al Bug 2).
   - Orden de pases: minimizar idas y vueltas FBO↔pantalla (cada una fuerza *resolve* del tile en Mali/Adreno).
5. **Barras de vida:** dibujar con `Rect` en el mismo batch (no por surface); si el juego las compone en surface, cachearla y redibujar solo cuando cambie el valor.

### 3.6 Partículas y modos de mezcla (versión correcta)
- `ParticleSystem` en SoA prealocado en `room_start` (`x[], y[], vx[], vy[], life[], alpha[], type[]`) con capacidad máxima configurable; sin `Vec::push` en caliente (`[VERIFICAR]` el tamaño máximo real).
- Culling en masa vectorizable por el compilador (bucles simples sobre slices contiguos), sin `unsafe` salvo evidencia en perfil.
- **Batching por blend sin romper orden:** se fusionan solo tramos consecutivos del mismo `BlendMode`. Reordenar está permitido **únicamente** entre comandos aditivos contiguos (conmutativos). Si el sistema de partículas tiene su propio `depth`, ya queda delimitado por el orden global.
- `glBlendFunc` cacheado (`current_blend`) → la llamada solo ocurre al cambiar.
- VBO de streaming: anillo de 3 regiones (`glMapBufferRange` con `MAP_WRITE_BIT|MAP_INVALIDATE_RANGE_BIT|MAP_UNSYNCHRONIZED_BIT` + fences, o *orphaning* con `glBufferData(NULL)` si el driver lo penaliza). Validar en Mali y Adreno; `glBufferSubData` a un anillo suele bastar.

### 3.7 Motor GML: fast-path O(1) + Bytecode plano, JIT como experimento

**Estado a verificar [VERIFICAR]:** en OpenGMK los nombres ya se resuelven a IDs en compilación y los campos built-in suelen ir por enum; el coste real suele estar en (a) recorrido recursivo del árbol, (b) campos de usuario en `HashMap`, (c) clonado de `Value::Str`. Se mide antes de decidir.

**Orden de ejecución (cada paso se puede revertir con feature flag):**

**Paso A — Fast-path de campos (bajo riesgo, se hace primero, también en el árbol actual):**
```rust
#[repr(u8)] pub enum HotField { X, Y, XPrev, YPrev, Speed, Direction, Hspeed, Vspeed,
                                 SpriteIndex, ImageIndex, ImageSpeed, ImageXScale, ImageYScale,
                                 ImageAngle, ImageAlpha, Depth, Visible, Solid, ObjectIndex, Id }
pub struct Instance { pub hot: HotBlock /* Cell<f64>/Cell<i32> en struct contiguo */, /* ... */ pub user: UserSlots }
// Acceso: instance.hot.get(HotField::X)  -> match por enum, sin hash.
```
- Campos de usuario: `UserSlots` con **inline cache por sitio de acceso** `{ object_index: i32, slot: u16 }`. Hit: índice directo. Miss: búsqueda en el mapa del objeto, actualiza el IC. Mantiene la semántica dinámica (`other.hp`, `with`, herencia).
- Arrays GML (`a[i]`, `a[i,j]`): almacenamiento `Vec<Value>` con redimensionado explícito, sin hash por elemento.

**Paso B — Linear Bytecode VM (comportamiento idéntico, sin recursión):**
```rust
pub struct Chunk { pub code: Vec<u32>, pub consts: Vec<Value>, pub ics: Vec<InlineCache>, pub lines: Vec<u32> }
// Instrucción 32 bit, VM de registros: op:8 | a:8 | b:8 | c:8   (operandos anchos -> palabra siguiente)
#[repr(u8)] pub enum Op { LoadK, Move,
    GetHot, SetHot, GetOtherHot, SetOtherHot,       // campos built-in
    GetUser, SetUser,                                // con IC
    GetGlobal, SetGlobal, GetArr, SetArr,
    Add, Sub, Mul, Div, Mod, IDiv, Neg, Not, Lt, Le, Eq, Ne, Ge, Gt, And, Or,
    Jmp, JmpIf, JmpIfNot,
    CallBuiltin, CallScript, Ret,
    WithBegin, WithNext, WithEnd, Exit }
pub struct Vm { regs: Vec<Value> /* prealocado, con límite de profundidad */, frames: Vec<Frame> }
impl Vm { pub fn run(&mut self, game: &mut Game, chunk: &Chunk, self_id: InstanceId, other: InstanceId) -> Result<Value, GmlError>; }
```
- Compilación `Tree -> Chunk` **por script/evento al cargar la room** (o perezosa en primer uso), cacheada. Bucle de despacho `loop { match op }` (Rust lo compila a jump table).
- `Value`: `Real(f64) | Str(Rc<str>/interned)`; evitar clonar cadenas en operaciones de comparación (`Rc::ptr_eq` primero).
- **Equivalencia:** flag `gml_bytecode` (por defecto OFF hasta pasar tests diferenciales). Modo `gml_verify`: ejecutar árbol y VM sobre una copia y comparar estado (hash de instancias/globales) por step en CI.

**Paso C — JIT ARM64 (`jit_arm64.rs`): NO entra en 0.9.8 como camino por defecto.**

| Riesgo | Detalle | Mitigación obligatoria |
|---|---|---|
| Memoria ejecutable | `mmap(PROT_EXEC)` anónimo suele estar permitido a apps, pero SELinux/OEM/kernels endurecidos pueden denegarlo; ejecutar desde archivos escribibles del app **está prohibido** desde Android 10 | Probar en arranque: crear página, escribir `ret`, `mprotect` a RX, llamar. Si falla → VM plano, sin crash |
| W^X | Alternar RW→RX con `mprotect` por función compilada es caro y fragmenta | Pool de código: 1 región grande, escribir en RW, un `mprotect` a RX por lote; o doble mapeo (`memfd`) RW y RX |
| icache | ARM64 necesita invalidar I-cache tras escribir código; con big.LITTLE hay núcleos con distintas líneas de caché | Usar `__builtin___clear_cache` / `libc::__clear_cache` sobre el rango; nunca instrucciones manuales con una línea fija |
| BTI/PAC/MTE | Páginas con `PROT_BTI` exigen `bti c`; PAC/MTE pueden alterar punteros | Mapear sin `PROT_BTI` o emitir `bti c` en cada entrada |
| `SIGSEGV`/`SIGILL` | Un bug del JIT es irrecuperable en la práctica (no se debe «recuperar» desde un signal handler) | Flag `gml_jit` OFF por defecto; *kill-switch* persistente: si el proceso murió con JIT activo, el próximo arranque lo desactiva |
| Beneficio real | GML es dinámico; sin especialización de tipos el JIT copia la lógica del VM | Solo evaluar tras Pasos A-B y perfil; empezar por trazas de bucles aritméticos puros |

**Criterio para activar JIT en una versión futura:** ≥25 % de mejora medida sobre el VM plano en combate + 7 días de telemetría sin cierres atribuibles + tests diferenciales verdes.

### 3.8 Pacing y térmica
- **Frame pacing:** integrar Android Frame Pacing (Swappy) o `eglPresentationTimeANDROID`; fijar objetivo de swap según `room_speed` y refresco del panel (30/60/90/120 Hz).
- **ADPF:** `PerformanceHintManager` con `reportActualWorkDuration` para ambos hilos (API 31+); sin `sched_setaffinity` manual.
- Hilos con nombre (`rld-logic`, `rld-render`) para Perfetto.

---

## 4. COLISIONES (verificación, no suposición)
- Instrumentar `place_meeting`/`instance_place`/`collision_*` con contador de instancias examinadas por llamada. Si el P95 de instancias examinadas por llamada supera ~16, entonces investigar rutas que evitan `spatial_hash.rs` **[VERIFICAR]** (cambios de máscara/capa, instancias `solid` re-insertadas cada frame).
- Re-inserción en el hash solo si la caja cambió (comparar `bbox` previo).

---

## 5. BUGS CRÍTICOS DE INTEGRACIÓN

### Bug 1 — Vulkan seleccionado, motor inicia en GL

**Hipótesis ordenadas por probabilidad:**
1. **La `Surface` ya está conectada a EGL.** Si Android crea la vista con `GLSurfaceView`/EGL en Java, la `ANativeWindow` queda ocupada por EGL y `vkCreateAndroidSurfaceKHR` falla (`VK_ERROR_NATIVE_WINDOW_IN_USE_KHR`) → el motor cae a GL en silencio. **Una misma `Surface` no puede usar GL y Vulkan.**
2. La opción se guarda pero la nativa lee la config **antes** de que llegue (orden de inicialización) o lee `OPENGMK_RENDERER`/default.
3. Falla de `vkCreateInstance`/dispositivo sin extensiones requeridas y el fallback no informa.
4. El HUD muestra una etiqueta fija.

**Diagnóstico (30 min):** `adb logcat | grep -E "RLD|vulkan|VK_|EGL"`; añadir log `requested=... actual=... reason=...`; `adb shell dumpsys SurfaceFlinger` para ver quién tiene la ventana.

**Fix:**
```rust
// render/backend.rs
#[repr(u8)] pub enum BackendKind { Gles = 0, Vulkan = 1 }
pub struct BackendInit { pub requested: BackendKind, pub actual: BackendKind, pub fallback_reason: Option<String> }

pub fn init_renderer(win: NativeWindow, requested: BackendKind, allow_fallback: bool)
    -> Result<(Box<dyn Renderer>, BackendInit), InitError>;
// Vulkan: instancia -> surface -> dispositivo -> swapchain. Si falla y allow_fallback=false => Err (se muestra al usuario).
```
```rust
// jni.rs  (devuelve backend REAL; negativo = código de error)
#[no_mangle] pub extern "system" fn Java_<pkg>_NativeBridge_nativeStart(
    env: JNIEnv, _cls: JClass, surface: JObject, backend: jint, /* ... */) -> jint;
#[no_mangle] pub extern "system" fn Java_<pkg>_NativeBridge_nativeBackendInfo(env: JNIEnv, _cls: JClass) -> jstring;
```
Android:
- Usar `SurfaceView` plano (callbacks de `SurfaceHolder`), **sin** crear contexto EGL en Java, y pasar la `Surface` a nativo.
- Leer el backend de `SharedPreferences` **antes** de `surfaceCreated` y pasarlo en `nativeStart`.
- Al cambiar de backend en Ajustes: pedir reinicio de la Activity del juego (recomendado: `android:process=":game"` y finalizar el proceso) para que la `ANativeWindow` esté libre.
- El HUD muestra `nativeBackendInfo()` (real, no la selección). Si hubo fallback: `Toast` con el motivo; no fallback silencioso.
- **Aceptación:** Vulkan seleccionado + dispositivo compatible → HUD `VK`; dispositivo incompatible → mensaje explícito y arranque en GL.

### Bug 2 — Pantalla negra en el menú de controles del juego

**Hipótesis ordenadas:**
1. **Función GM8 bloqueante** (`keyboard_wait`, `io_handle`, `get_integer`, `show_menu`, `screen_redraw` síncrono) ejecutada en el hilo lógica: el bucle no publica snapshots ni procesa eventos Android → el render muestra el último frame o negro tras reconfigurar.
2. **`surface_set_target` sin `surface_reset_target`** en el script del menú → el present dibuja con otro FBO enlazado.
3. **Ruta de `.ini`** (`ini_open`) fuera de un directorio escribible → error → `show_error`/abort que no se muestra.
4. Funciones no implementadas que devuelven error y el script entra en bucle de espera.

**Diagnóstico:**
- Tracer de GML con *ring buffer* de las últimas 200 llamadas builtin; volcar en logcat si no se publica un frame en 500 ms (watchdog en el hilo lógica).
- Registrar qué builtin llevaba bloqueado el hilo.
- `glGetError` + `glCheckFramebufferStatus` tras cada pase en debug.

**Fix nativo:**
```rust
pub enum WaitState { None, KeyPress { deadline: Option<Instant> }, Redraw, Dialog(DialogId) }
// keyboard_wait / get_* / show_* => no bloquean: dejan WaitState y la VM devuelve "Yield".
// El bucle principal sigue: procesa eventos, publica snapshot, hace present; reanuda al resolver.
impl Game { fn pump_frame_while_waiting(&mut self) -> WaitResult; }
```
- `screen_redraw()` / `screen_refresh()`: `publish()` + espera **acotada** (≤ 100 ms, `fence`) al ack del render; nunca espera infinita.
- Mapear el toque sobre el overlay a eventos de teclado virtuales que completen `keyboard_wait`/`keyboard_lastkey`.
- `ini_open`/rutas relativas: remapear a `getFilesDir()/gamedata/` en la capa de ficheros del runner; crear directorios; devolver error GML (no abortar).
- Protección: antes de `present`, si `bound_fbo != 0` ⇒ `surface_reset_target` + log.
- **Aceptación:** abrir/cerrar el menú de controles 20 veces seguidas sin pantalla negra, remapear una tecla y que persista tras reiniciar.

### Bug 3 — UI/UX Android

**3a. Controles de ratón ocultos de fábrica.**
- Añadir `defaultVisible: Boolean` al modelo del control y `userAdded: Boolean`.
- Catálogo del editor: los controles `LMB`, `RMB`, `Scroll` quedan con `defaultVisible = false`; en el layout por defecto se crean con `visibility = View.GONE` (o `visible=false` en el estado Compose) y solo aparecen si el usuario los añade desde el catálogo (`userAdded = true`).
- **Migración:** `layoutVersion` 2. En la migración, ocultar los controles de ratón **solo si `userAdded == false`** (no tocar los que el usuario añadió).
- Aceptación: instalación limpia y actualización desde 0.9.7 no muestran LMB/RMB/Scroll salvo los añadidos a mano.

**3b. Barra de navegación que tapa opciones en Ajustes.**
- Views:
```kotlin
ViewCompat.setOnApplyWindowInsetsListener(binding.settingsRoot) { v, insets ->
    val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout())
    v.updatePadding(bottom = bars.bottom)
    insets
}
binding.settingsScroll.clipToPadding = false
```
- Compose: `Modifier.windowInsetsPadding(WindowInsets.safeDrawing)` en el contenedor y `contentPadding = PaddingValues(bottom = navBarHeight + 16.dp)` en `LazyColumn`.
- Si la "barra flotante" es una barra de navegación propia de la app: sumar su altura medida (`onSizeChanged`) al padding inferior del scroll.
- `enableEdgeToEdge()` coherente en todas las pantallas de Ajustes.
- Aceptación: la última opción de cada pantalla de Ajustes es completamente visible y pulsable en 3 resoluciones y con navegación por gestos y por 3 botones.

---

## 6. CRITERIOS DE ACEPTACIÓN

Se expresan contra `room_speed` del juego (`R`), no contra una cifra fija. Dispositivos mínimos: Helio G85 / Snapdragon 680 y un gama media-alta.

| Escenario | v0.9.7 | Meta v0.9.8 | Métrica |
|---|---|---|---|
| Combate 1v1 con clima | 25-32 FPS | ≥ R sostenido | P95 frametime ≤ 1/R + 1 ms; P99 sin picos > 2/R |
| Combate 2v2 alta densidad | 20-26 FPS | ≥ R (mín. 55 si R=60) | Sin picos de VSync perdidos |
| Ciudad densa (tiles + NPCs) | 26-35 FPS | ≥ R sostenido | `draw calls/frame` ≤ 40; `bytes subidos a VBO/frame` < 64 KiB en tiles |
| Contención del canal de render | 5-12 ms esperas (a confirmar con Fase 0) | 0 esperas del hilo lógica | `wait_for_render` = 0; 0 bloqueos |
| Asignaciones por frame en caliente | a medir | 0 | Contador de allocator en builds de test |
| `glBindFramebuffer` por frame (combate) | a medir | ≤ 2 | Contadores GL |
| Bugs 1-3 | presentes | corregidos | Casos de aceptación de §5 |
| Equivalencia GML | — | 100 % | Test diferencial §8.2 sin divergencias |

---

## 7. PLAN FASEADO CON PUERTAS DE CALIDAD

> Las duraciones son estimaciones; lo que manda son los *gates*.

**Fase 0 — Medición (primero).** Gate: tabla de presupuesto por hilo + top-10 funciones + `room_speed` confirmado.

**Fase 1 — Canal y config (riesgo bajo).**
- [ ] `frame_channel.rs` (triple buffer) + `ControlChannel` (anillo acotado).
- [ ] `FrameSnapshot` POD prealocado; eliminar clon bajo candado.
- [ ] `RenderConfig` atómico; `env::var` solo en `init_from_env`.
- Gate: tests `loom` + estrés verdes; `wait_for_render == 0`; `cargo check --manifest-path src/OpenGMK/Cargo.toml` y `cargo test`.

**Fase 2 — Bugs de integración (desbloquean release).**
- [ ] Bug 1, Bug 2 (con watchdog y `WaitState`), Bug 3.
- Gate: casos de aceptación de §5 en ≥2 dispositivos reales.

**Fase 3 — Render: ciudades y texto.**
- [ ] `AtlasArray` + shader `sampler2DArray` + side textures.
- [ ] `StaticTileStore` + fusión por depth.
- [ ] Glifos en atlas; `FboCache`/`bound_fbo`/`glInvalidateFramebuffer`.
- [ ] Batching de partículas por tramos consecutivos y blend cacheado.
- Gate: comparación de capturas píxel a píxel (tolerancia 0 en normal, ≤1 LSB en alpha) en 20 escenas; `draw calls/frame` dentro de meta.

**Fase 4 — GML.**
- [ ] Paso A (hot fields + IC de campos de usuario) — gate: tests diferenciales.
- [ ] Paso B (bytecode, flag OFF por defecto) — gate: `gml_verify` sin divergencias en 3 partidas completas de replay y mejora medida; si cumple, ON por defecto.
- [ ] Paso C (JIT): solo diseño de *kill-switch* y detección de capacidad; **sin activar**.

**Fase 5 — Validación de dispositivos.** Gama baja (Helio G85/P35, Snapdragon 680), media-alta; térmica a 15 min de partida continua; pacing (Swappy/ADPF).

**Fase 6 — Release.** Notas de versión, `versionCode = 39`, plan de rollback (flags `gml_bytecode`, `tile_bake`, `atlas_array`, `text_cache` con *kill-switch* remoto o por ajuste de depuración).

---

## 8. ESTRATEGIA DE PRUEBAS

1. **Concurrencia:** `loom` sobre `frame_channel` (modelo reducido de 3 slots, producer/consumer, comprobar que nunca se solapan accesos a un slot y que el consumidor siempre ve un frame completo). Estrés en dispositivo real (ARM débilmente ordenado) con checksum.
2. **GML diferencial:** correr árbol vs bytecode con la misma semilla/entrada; hash del estado (instancias, globales, RNG) cada step; fallar ante la primera divergencia.
3. **Regresión visual:** captura por frame en escenas fijas (combate, Carmín, Silph, Bosque Verde) comparadas con la referencia de 0.9.7.
4. **Allocator contador** (global allocator de prueba): `assert_eq!(allocs_in_hot_frame, 0)` tras el calentamiento.
5. **Fuzz de tiles/surfaces:** secuencias aleatorias de `tile_add/delete/layer_depth` y `surface_set/reset_target` verificando que el FBO final es 0 y que las capas dirty se reconstruyen.

---

## 9. REGISTRO DE RIESGOS

| Riesgo | Prob. | Impacto | Mitigación |
|---|---|---|---|
| La causa dominante resulta estar en GML y no en render | Media | Alto | Fase 0; prioridades reordenables |
| Memoria del texture array en gama baja | Media | Medio | Presupuesto por capas, páginas 1024, side textures |
| Divergencias semánticas del bytecode | Media | Alto | Flag OFF, `gml_verify`, replays |
| Orden de dibujado alterado por batching | Baja | Alto | Fusión solo de tramos consecutivos; capturas de regresión |
| Vulkan inestable en ciertos drivers | Media | Medio | Fallback explícito + aviso, GLES como backend por defecto |
| JIT inseguro en algunos OEM | Alta si se activa | Alto | No activar en 0.9.8; kill-switch |

---

## 10. CHECKLIST FINAL DE ENTREGA 0.9.8
- [ ] Fase 0 documentada con números.
- [ ] `frame_channel` + snapshot POD sin clones ni locks.
- [ ] `RenderConfig` sin `env::var` en caliente.
- [ ] Bugs 1-3 cerrados con evidencia.
- [ ] Tiles pre-baked + atlas array + texto en atlas + FBO cache.
- [ ] Hot fields + IC; bytecode tras flag con test diferencial.
- [ ] Métricas de §6 cumplidas en gama baja y media.
- [ ] Notas de versión y plan de rollback.

---

## 11. AUDITORÍA ESTÁTICA DEL PAQUETE ENTREGADO (2026-10-22)

### 11.1 Hallazgos confirmados y acciones

| ID | Prioridad | Hallazgo confirmado en el código entregado | Solución / gate de salida |
|---|---|---|---|
| AUD-01 | **P0 — seguridad de release** | El archivo comprimido incluye material relacionado con la firma (`android/release.keystore`, `android/keystore.properties` y `android/keystore/alex_private.keystore`). `android/app/build.gradle.kts` también contiene credenciales de respaldo incrustadas en texto plano. El paquete de auditoría no incluye `.git`, así que no afirmo que los archivos estén en el historial remoto; sí están incluidos en este paquete. | Retirar keystores y propiedades privadas de cualquier paquete compartido/release y del repositorio; dejar de usar contraseñas/alias de respaldo hardcodeados. El build de release debe fallar si faltan secretos externos seguros (CI/variables protegidas). Tratar la clave compartida fuera del equipo como potencialmente expuesta. **No generar una clave nueva a ciegas**: comprobar el mecanismo de distribución/firma actual y planificar continuidad de actualizaciones de instalaciones existentes antes de rotar. Gate: una compilación limpia firmada desde secretos inyectados y sin claves/contraseñas en el paquete. No incluir valores secretos en este documento. |
| AUD-02 | **P1 — bloqueante de release** | El roadmap apunta a `versionCode = 39`, `versionName = "0.9.8"`, pero `android/app/build.gradle.kts` aún declara `versionCode = 38`, `versionName = "9.7"`. Así el APK no representaría la versión objetivo indicada. | Cambiar a `versionCode = 39` y `versionName = "0.9.8"` al preparar el release; confirmar que el código supera el último build publicado y que el APK muestra esos valores. Gate: inspeccionar el manifest/APK final y probar instalación/actualización. |
| AUD-03 | **P1 — arquitectura/performance** | `src/OpenGMK/gm8emulator/src/render/snapshot.rs:60-95` denomina el canal lock-free, pero cada slot usa `Mutex<Option<RenderStateSnapshot>>`; `publish()` adquiere un lock y `read_latest()` clona el snapshot, incluido su `Vec`. No es un canal lock-free ni zero-copy. Además, la búsqueda global encuentra la definición de `read_latest()`, pero no una llamada a ella. | No parchear únicamente el mutex. Elegir expresamente una ruta: (A) eliminar/desactivar snapshots e interpolación en 0.9.8 si no se van a integrar, o (B) integrar el consumidor y sustituir la implementación por un triple buffer de slots prealocados con ownership probado. Añadir test `loom`, estrés productor/consumidor y comprobación de checksum antes de declarar completado §3.1. |
| AUD-04 | **P1 — arquitectura/performance** | En este snapshot, `Game::step` llama a `self.renderer.present(...)` en línea (`game.rs`, aprox. línea 2401). La creación del snapshot ocurre después de ese present (líneas 2411-2441). No se encontró consumidor de `SNAPSHOT_QUEUE` ni uso de `SnapshotInterpolator`. La preferencia `async_render_thread` se lee y escribe, pero `async_render_thread_enabled()` no tiene consumidores. | La separación lógica/render descrita en el diagrama todavía no está implementada para el renderer principal de esta copia. Antes de atribuirle beneficios al triple buffer, diseñar el arranque/parada del hilo, la propiedad del contexto/surface EGL/Vulkan, el resize/lifecycle de Android y el consumo real del snapshot. Si se pospone, desactivar u ocultar la opción y no afirmar que interpola a 60/90/120 FPS. Gate: traza de hilos que demuestre separación real y tests de pausa, cambio de surface, background/foreground y cierre. |
| AUD-05 | **P1 — hot path medible** | Hay lecturas de entorno dentro de rutas de render repetidas: `render/opengl.rs:2302` consulta `OPENGMK_BEZEL` al preparar el overlay; `render/opengl.rs:4473-4487` consulta `OPENGMK_FILTER` al presentar el frame. Esto contradice el objetivo “sin `env::var` en caliente”. No prueba por sí solo que explique las caídas de FPS. | Parsear la configuración una vez en `load_video`/inicialización y conservarla en un estado de render. Si se permiten cambios en caliente, actualizarlos explícitamente desde la configuración compartida, no consultando el entorno cada frame. Medir con contadores antes/después. Prioridad secundaria frente al cuello dominante que revele Fase 0. |
| AUD-06 | **P1 — backend engañoso** | `Renderer::new` intenta Vulkan y, si falla, registra un mensaje y hace fallback a OpenGL (`render.rs:652-675`). Sin embargo, Ajustes persiste `graphicsBackend = "vulkan"` y muestra “Backend Vulkan activado” (`SettingsScreen.kt:795-818`); no se encontró un puente de UI que muestre backend real y motivo del fallback. Por ello, la preferencia solicitada no garantiza que Vulkan esté ejecutándose. | Devolver/reportar el backend real y motivo de fallback desde el motor; mostrarlo en Ajustes/HUD y notificar el fallback. Separar “preferido” de “activo”. El reinicio debe quedar claro y comprobarse con el proceso de juego real. La hipótesis de conflicto con una `GLSurfaceView` Java no está respaldada por esta copia: `GameActivity` extiende `NativeActivity`; no atribuirlo a EGL ocupado sin logs que lo demuestren. Gate: logs `requested/actual/reason` y prueba tanto en dispositivo compatible como incompatible. |
| AUD-07 | **P1 — posible regresión PowerVR** | `PANTALLA-NEGRA-ANDROID.md` dice que en Android el blit se fuerza a `GL_NEAREST` debido a un problema con coordenadas Y invertidas. En la implementación actual, `render/opengl.rs:4462-4487` invierte el eje Y y puede pasar `GL_LINEAR` si `OPENGMK_FILTER` es `sharp`, `linear` o `fxaa`. La discrepancia entre nota y código está confirmada; que cause pantalla negra en esta versión requiere reproducción con el driver afectado. | Reconciliar nota y código: probar en PowerVR/Rogue GE8320 con cada filtro, revisar el `glGetError` justo después del blit y preservar el filtro visual. Si la combinación no es válida en el dispositivo, usar `GL_NEAREST` en esa ruta o rediseñar el blit sin coordenadas invertidas. Gate: arranque, gameplay y menú sin frame negro con filtros `sharp`, `linear`, `fxaa` y `nearest`; no dar el fix por probado sin dispositivo/registro. |
| AUD-08 | **P2 — controles táctiles** | LMB/RMB y `qty_plus`/`qty_minus` siguen en los catálogos nativo y Kotlin, pero tienen `contexts = 0` / perfiles vacíos. `TouchLayouts.controlsFor()` solo los muestra si existen en el layout guardado, por lo que el código parece haberlos excluido del layout predeterminado. Sin embargo, no se identificó una migración explícita de versión del layout. | No añadir `defaultVisible`/`userAdded` duplicados sin necesidad. Validar instalación limpia, actualización con layout existente y controles que el usuario añadió manualmente. Si falla el upgrade, añadir una migración idempotente que oculte solo los controles nunca añadidos por el usuario. Gate: LMB/RMB/+/- ocultos por defecto, visibles después de añadirlos, y preservados tras actualizar. |
| AUD-09 | **P2 — área segura de Ajustes** | Hay salvaguardas parciales: `SettingsScreen.kt` reserva `120.dp` al final del scroll, desplaza el snackbar y la barra flotante aplica `WindowInsets.safeDrawing`. Eso reduce el riesgo de solape, pero no demuestra que todas las subpantallas y resoluciones estén cubiertas. | Mantener el espacio de la barra y probar las últimas opciones de cada categoría en 3 resoluciones, navegación por gestos y 3 botones. Añadir insets específicos donde la prueba demuestre que falta espacio; no aplicar padding duplicado global a ciegas. |
| AUD-10 | **P2 — límites de esta auditoría** | No fue posible compilar ni ejecutar tests en este entorno: no están `rustc`/`cargo`, ni `gradle`/`gradlew`, ni hay dispositivo Android. Tampoco hay `.git` para identificar commit/diffs. | En la siguiente estación de desarrollo ejecutar primero `cargo check --manifest-path src/OpenGMK/Cargo.toml -p gm8emulator`, tests Rust disponibles y build Android de debug/release; guardar logs y APK exacto. Resolver errores antes de iniciar optimizaciones. |

### 11.2 Estado real de componentes importantes de la V2

| Componente / promesa | Estado encontrado en el paquete auditado | Consecuencia para 0.9.8 |
|---|---|---|
| Frame snapshots/interpolación | Estructuras presentes, pero con mutex/clonación y sin lector conectado. | **No contar como render asíncrono ni mejora de FPS.** Integrar con tests o dejar fuera del release. |
| `RenderConfig` cacheado | No existe la abstracción prevista en §3.2; se leen `OPENGMK_BEZEL` y `OPENGMK_FILTER` desde funciones de render. | Implementar el cacheado de configuración como tarea real y medir su efecto. |
| `sampler2DArray` / `AtlasArray` | No se encontró la ruta `sampler2DArray`/`GL_TEXTURE_2D_ARRAY` descrita en §3.4; hay renderer OpenGL basado en atlas/texture ids existente. | Sigue siendo trabajo pendiente, no una optimización ya entregada. Introducirla solo después de capturas comparativas y medir memoria por GPU. |
| `StaticTileStore` / `TileLayerBake` | No se encontraron estas estructuras en el renderer actual. | Sigue pendiente; verificar antes el orden exacto de tiles/instancias por `depth` y el coste medido de ciudades. |
| VM GML lineal / `gml_bytecode` | No se encontró el `Chunk`/VM lineal indicado en §3.7 con ese identificador. | Tratar el bytecode del roadmap como arquitectura futura; no activar JIT por defecto. Primero medir el coste real del intérprete y construir tests diferenciales. |
| Fixes principales de pantalla negra | Sí aparecen en el código actual: dimensiones `width`/`height` guardadas en `Framebuffer`, `#[repr(C)]` para `Vertex`, padding `_fog_pad`, y re-enlace explícito de programa/VAO/VBO en `draw_buffer`. | Estos puntos parecen ya aplicados en esta copia; confirmar en PowerVR con prueba real y resolver AUD-07 antes de cerrar el bug. |
| Ajustes de backend | Configuración se escribe en `video.conf`; el nativo la lee al iniciar y selecciona Vulkan/GL. El fallback existe, pero el backend efectivo no se devuelve a la UI. | La selección está conectada, pero falta transparencia verificable del backend real y de fallbacks. |

### 11.3 Orden recomendado actualizado

1. **Gate R0 — Seguridad + identidad del build:** aislar/eliminar material de firma del paquete, retirar credenciales hardcodeadas y preparar `versionCode 39` / `versionName 0.9.8`.
2. **Gate R1 — Build reproducible:** compilar Rust y Android sin errores; registrar commit, configuración, ABI y artefacto. No empezar refactors sobre un build sin línea base.
3. **Gate R2 — Medición + arquitectura real:** confirmar `room_speed`, perfil por hilo y si se va a separar realmente lógica/render. No implementar el triple buffer como cambio aislado cuando no hay consumidor.
4. **Gate R3 — Correcciones de poco riesgo:** cachear `OPENGMK_FILTER`/`OPENGMK_BEZEL`; resolver y probar la discrepancia de blit PowerVR; exponer backend efectivo y fallback.
5. **Gate R4 — Render estructural:** atlas array, bake de tiles y FBO cache únicamente con baseline antes/después y tests de regresión visual; preservar orden de dibujado por `depth`.
6. **Gate R5 — Runtime GML:** fast-path/bytecode tras tests de equivalencia; JIT fuera del camino por defecto en 0.9.8.
7. **Gate R6 — Validación release:** regresión limpia/upgrade de controles, navegación de Ajustes, 15 minutos de térmica y pruebas en al menos un PowerVR y un Mali/Adreno cuando estén disponibles; adjuntar logs, FPS/frametime y APK firmado.

### 11.4 Evidencia mínima que debe acompañar el cierre

- `cargo check` y `cargo test` con salida guardada; build Android exitoso y metadata de versión verificada.
- Para rendimiento: `room_speed`, frametime P50/P95/P99 por escenario, hilos separados/no separados, draw calls, cambios de textura, bytes VBO y asignaciones por frame.
- Para Vulkan: backend solicitado, backend efectivo y motivo de fallback capturados en el dispositivo.
- Para pantalla negra: modelo/GPU/versión Android, filtro usado, `glGetError`, framebuffer completo y captura de pantalla en el arranque, menú y una escena jugable.
- Para controles/Insets: pruebas de instalación limpia y actualización conservando preferencias y elementos añadidos por el usuario.
- Para seguridad: el APK de release y las claves deben estar separados; el informe de auditoría no debe incluir contraseñas, tokens ni contenido de keystores.

