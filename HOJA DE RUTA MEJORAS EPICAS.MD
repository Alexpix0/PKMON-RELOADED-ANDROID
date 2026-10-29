Compendio Maestro Absoluto de Optimización: Arquitectura de Motor OpenGMK (Android NDK)
Esta es la recopilación íntegra y exhaustiva de todas las técnicas, patrones de memoria, directivas de compilador y reingenierías de bajo nivel extraídas de los documentos técnicos. No se omite ningún componente; cada intervención está estructurada desde las fases inmediatas hasta el rediseño nuclear del runtime.   
MD
+ 1

FASE 1: Pipeline Gráfico Primario, Reducción de Vértices y Afinidad CPU (Versión v1.4.2)
Enfoque: Erradicar el 59.2% de ancho de banda redundante hacia la GPU y blindar la asignación de hardware contra el planificador móvil.   
MD

1.1. Static Quad Index Buffer (EBO) en OpenGL ES
Diagnóstico: El emulador emite cada sprite y tile cargando 6 vértices independientes por quad mediante llamadas a glDrawArrays.   
MD

Implementación en opengl.rs y render.rs:

Crear un GL_ELEMENT_ARRAY_BUFFER estático e inmutable (GL_STATIC_DRAW) con capacidad para 10.920 quads 
× 6 índices = 65.520 índices en formato u16.   
MD

Patrón por quad 
i: [4*i, 4*i + 1, 4*i + 2, 4*i + 2, 4*i + 3, 4*i].   
MD

Empujar únicamente 4 vértices por quad a la cola del VBO.   
MD

Ejecutar el despacho gráfico mediante glDrawElements(gl::TRIANGLES, (quad_count * 6) as i32, gl::UNSIGNED_SHORT, std::ptr::null()).   
MD

1.2. Compact Vertex Layout (72 Bytes 
→ 44 Bytes)
Diagnóstico: El struct Vertex arrastra atributos tridimensionales inútiles (normal: [f32; 3], 12 B) y color en cuatro floats decimales (blend: [f32; 4], 16 B).   
MD

Implementación en render.rs, opengl.rs y vertex.glsl:

Suprimir el vector normal.   
MD

Sustituir blend: [f32; 4] por un entero empaquetado de 4 bytes blend: u32 (formato ABGR / RGBA8888).   
MD

En setup_vertex_attributes: vincular loc_blend usando GL_UNSIGNED_BYTE, normalized = GL_TRUE, tamaño 4.   
MD

Layout resultante de 44 bytes alineado:

pos: [f32; 3] (12B)+interp: [f32; 2] (8B)+tex_coord: [f32; 2] (8B)+atlas_xywh: [f32; 4] (16B)+blend: u32 (4B)
Cada quad dibujado desciende de 432 bytes a 176 bytes de transferencia hacia la GPU (59.2% de ahorro en bus de memoria).   
MD

1.3. Thread Pinning a Núcleos Big/Prime (sched_setaffinity)
Diagnóstico: El planificador DynamIQ de Android migra el hilo de emulación hacia núcleos de bajo consumo Cortex-A53/A55 para ahorrar energía, produciendo caídas térmicas súbitas de 60 a 30 FPS.   
MD
+ 1

Implementación en android_main.rs:

Detectar la topología del SoC vía /sys/devices/system/cpu/.   
MD
+ 1

Invocar sched_setaffinity o pthread_setaffinity_np sobre el hilo del runner para confinarlo estrictamente a los núcleos cpu4–cpu7 (clúster Gold/Prime Cortex-A7x o Cortex-X).   
MD
+ 1

1.4. Bounds Check Hoisting en Bucle de Tiles (draw.rs)
Diagnóstico: LLVM inserta ramas condicionales de pánico (panic_bounds_check) en cada iteración de visible_tiles[i], impidiendo la auto-vectorización por hardware.   
MD

Implementación: Anteponer assert!(self.draw_scratchpad.visible_tiles.len() >= visible_count) antes de la iteración para que el compilador emita instrucciones vectoriales ARM NEON libres de saltos condicionales.   
MD

FASE 2: Latencia JNI Cero, Desactivación de Instancias O(1) y Telemetría Nativa (Versión v1.4.3)
Enfoque: Eliminar pausas del recolector de basura en controles y neutralizar el script de desactivación de GameMaker.   
MD

2.1. Spatial Chunking (
256×256 px) contra instance_deactivate_region
Diagnóstico: Pokémon Reloaded ejecuta instance_deactivate_region alrededor del jugador en cada paso, forzando un escaneo lineal 
O(N) sobre miles de entidades de la sala que satura la CPU.   
MD

Implementación en instancelist.rs y room.rs:

Segmentar el plano de la sala en celdas espaciales de 
256×256 píxeles.   
MD

Mantener un mapa de bits contiguo (active_mask: BitVec).   
MD

Activar o desactivar en 
O(1) las entidades que caen dentro de los chunks que intersectan el frustum expandido de la cámara (+128 px de padding contra pop-in), erradicando las iteraciones sobre instancias lejanas.   
MD

2.2. Entrada Táctil Ultrarrápida con @CriticalNative (Frontera JNI)
Diagnóstico: Cada evento de pantalla táctil cruza la frontera de la JVM incurriendo en 30 a 100 ns de overhead y arriesgando el desbordamiento de la tabla local de referencias (límite de 512 slots).   
MD

Implementación en Kotlin y Rust:

Anotar en Kotlin: @CriticalNative external fun nativeSetInputMask(mask: Int).   
MD

En Rust: recibir el entero mask: i32 directamente sin punteros JNIEnv ni jclass.   
MD

Almacenar en una variable atómica global lock-free (INPUT_BUS: AtomicU32) con ordenamiento Ordering::Relaxed.   
MD

Transición reducida a 2.5 ns a velocidad de C directo.   
MD

2.3. Erradicación de Fugas de Referencias Locales en JNI
Directiva: Envolver todo bloque auxiliar de interoperabilidad JNI en Rust con env.push_local_frame(16) y env.pop_local_frame().   
MD

Cachear identificadores jclass, jmethodID y jfieldID durante JNI_OnLoad en variables globales permanentes (NewGlobalRef); prohibir llamadas a FindClass en el bucle principal.   
MD

2.4. Telemetría Nativa con Guards RAII de ATrace
Diagnóstico: Imposibilidad de medir el costo exacto de Ciudad Naos o colisiones en microsegundos sin recompilar con flags pesados.   
MD

Implementación:

Enlazar las primitivas nativas <android/trace.h> (ATrace_beginSection y ATrace_endSection) envueltas en un guard RAII ProfileScope en Rust.   
MD

Instrumentar las rutinas: OpenGMK::DrawViews, OpenGMK::TileCullingAABB, OpenGMK::CollisionPass y OpenGMK::StepEvents.   
MD

Proyecta trazas precisas en ui.perfetto.dev con costo nulo cuando la traza está inactiva.   
MD

FASE 3: Sincronización Gráfica TBDR, Vértices LibGDX y Colisiones NEON (Versión v1.5.0)
Enfoque: Eliminar pausas del driver gráfico móvil, compactar vértices al límite teórico y procesar colisiones por hardware.   
MD

3.1. VBO Ping-Pong (Double/Triple Buffering con glBufferSubData)
Diagnóstico: En GPUs móviles basadas en tiles (Qualcomm Adreno, ARM Mali, PowerVR), llamar a glBufferData(GL_STREAM_DRAW) en cada vaciado puede forzar bloqueos de sincronización si la GPU todavía dibuja el fotograma anterior.   
MD

Implementación: Rotar entre dos o tres VBOs contiguos. Mientras la GPU procesa el búfer 
A, la CPU escribe los vértices del siguiente fotograma en el búfer 
B mediante glBufferSubData, erradicando pausas del driver.   
MD
+ 1

3.2. Preservación de Early-Z en Shaders de GPU Móvil
Diagnóstico: La instrucción discard; en el fragment shader destruye la prueba temprana de profundidad (Early-Z rejection), forzando a chips Adreno y Mali a ejecutar sombreado sobre píxeles que terminarán tapados.   
MD

Implementación: Reestructurar los shaders 2D para garantizar que la profundidad se resuelva en la fase previa sin interrumpir la cola de rasterizado por tiles.   
MD

3.3. Empaquetado Compacto de Vértices Estilo LibGDX (20 Bytes por Vértice)
Implementación: Comprimir el layout de vértices para sprites 2D al mínimo matemático:
   
MD

x: f32 (4B)+y: f32 (4B)+u: f32 (4B)+v: f32 (4B)+color: u32 ABGR (4B)=20 Bytes
Un quad completo pasa a pesar 80 bytes en total (
4×20 B), encajando en solo 1.25 líneas de caché L1D (64 bytes).   
MD

3.4. Reglas de Volcado de Lote (Flush Conditions)
Implementación en el batcher: Disparar flush() hacia OpenGL ES únicamente bajo cuatro condiciones estrictas:   
MD

Contador de quads alcanza el límite de capacidad del buffer (current_quads == 10920).   
MD

Cambio de textura activa (active_texture != new_texture).   
MD

Cambio de programa shader (active_shader != new_shader).   
MD

Cambio de modo de mezcla (active_blend != new_blend).   
MD

3.5. Qualcomm Adreno FlexRender™ y Supresión de GMEM Unresolves
Directiva: Configurar el framebuffer para forzar el modo de binning directo en GPUs Adreno y evitar que el driver realice volcados intermedios de memoria GMEM a DRAM del sistema.   
MD

3.6. Colisiones Precisas por Píxel con Bitmasking de 64 Bits y ARM NEON
Diagnóstico: La comprobación de máscaras de colisión precisa mediante bucles anidados en 
(x,y) píxel por píxel destruye la predicción de saltos de la CPU.   
MD

Implementación:

Empaquetar las máscaras de transparencia en arrays continuos de enteros [u64; N] (1 bit = 1 píxel).   
MD

Evaluar filas completas de 64 píxeles simultáneos mediante operaciones lógicas bitwise directas: word_a & (word_b << shift_x) en 1 solo ciclo de CPU.   
MD

Vectorizar la mezcla de máscaras mediante instrucciones ARM NEON (vld1q_u8, vcgtq_u8, vbslq_u8).   
MD

3.7. Compatibilidad Nativa con Páginas de 16 KB (Android 15+)
Implementación en .cargo/config.toml:

Ini, TOML
[target.aarch64-linux-android]
rustflags = [
    "-C", "link-arg=-Wl,-z,max-page-size=16384",
    "-C", "link-arg=-Wl,-z,common-page-size=16384",
]
Cuadruplica la cobertura del Translation Lookaside Buffer (TLB) y reduce los fallos de traducción de memoria virtual entre un 4% y 12% en lecturas de RAM emulada.

FASE 4: Frame Pacing Determinista y Carga de Paquetes Zero-Copy (Versión v1.6.0)
Enfoque: Cero micro-stutter en pantallas de alta tasa de refresco (90/120/144 Hz) y arranque instantáneo.   
MD

4.1. Controlador Temporal "Fix Your Timestep" (Glenn Fiedler Pattern)
Diagnóstico: Enlazar GameMaker 8 al VSYNC de 120 Hz o 144 Hz duplica la velocidad física o genera judder por cuadros descartados arbitrariamente.   
MD

Implementación de TimeStepController en Rust:

Acumulador temporal alimentado por frame_time de pantalla con clamp a 0.25 s para evitar la espiral de la muerte.   
MD

La lógica, alarmas, step y colisiones avanzan a pasos discretos constantes de 60 Hz fijos (
dt=1/60 s≈16.666 ms).   
MD

El renderizado interpola las posiciones de los quads según el remanente en el acumulador (
α=acumulador/dt) mediante LERP continuo:
   
MD

render_x=prev_x⋅(1.0−α)+curr_x⋅α
render_y=prev_y⋅(1.0−α)+curr_y⋅α
Física determinista idéntica a la original a 60 FPS con movimiento a 120 FPS fluidos en paneles OLED.   
MD

4.2. Spatial Hash Grid con Morton Codes (Z-Order 64-Bit)
Implementación: Sustituir listas planas de instancias por una cuadrícula de hash espacial donde las coordenadas de las entidades se entrelazan en códigos de Morton de 64 bits para mapear proximidad bidimensional a contigüidad unidimensional en memoria, acelerando búsquedas de rango espacial.   
MD

4.3. Carga de Assets con Memory-Mapped I/O (memmap2)
Diagnóstico: Leer el ejecutable de GameMaker (game.data de más de 150 MB) mediante std::fs::read satura el heap y retrasa el inicio de la app.

Implementación: Mapear el archivo en el espacio de memoria virtual con mmap en modo lectura compartida.

Apertura del juego en 0.1 milisegundos, paginación bajo demanda (demand paging) y ahorro de más de 150 MB de memoria RAM física.

4.4. Empaquetado Nativo Zero-Copy (extractNativeLibs = false)
Implementación:

Definir useLegacyPackaging = false en build.gradle.kts y android:extractNativeLibs="false" en el AndroidManifest.

Alinear libgm8emulator.so a 16 KB en el contenedor APK mediante zipalign -p 16.

El linker de Android mapea la biblioteca nativa directamente desde el archivo APK a la memoria sin descomprimirla en el almacenamiento flash.

4.5. Estructuras de Datos GameMaker Planas Contiguas (ds_grid / ds_list)
Diagnóstico: Las estructuras dinámicas ds_* de GameMaker se alojan de forma dispersa en el heap, destruyendo la localidad espacial (pointer chasing).   
MD

Implementación: Mapear cada ds_grid a un vector plano unidimensional contiguo Box<[Value]> indexado sin saltos condicionales:
   
MD

I
ˊ
 ndice=y⋅width+x

Permite que las líneas de caché L1D precarguen filas completas de celdas de colisión.   
MD

FASE 5: Optimización Guiada por Perfiles y Pipeline de Audio Nativo (Versión v1.7.0)
Enfoque: Reorganización binaria guiada por silicio y audio de latencia ultra-baja sin contención de hilos.   
MD
+ 1

5.1. Profile-Guided Optimization (PGO) en Android NDK
Paso 1 (Compilación de Instrumentación): Compilar libgm8emulator.so con -Cprofile-generate=/data/local/tmp/pgo-data.

Paso 2 (Entrenamiento): Ejecutar 5 minutos de juego real en el teléfono recorriendo Ciudad Naos, rutas con clima y combates.

Paso 3 (Extracción y Fusión): Volcar los archivos de perfil con adb pull y unificarlos mediante llvm-profdata merge -output=merged.profdata.

Paso 4 (Compilación Final): Compilar en release inyectando -Cprofile-use=merged.profdata -Cllvm-args=-pgo-warn-missing.

LLVM reorganiza la distribución de bloques básicos en el binario, colocando las ramas calientes en bloques contiguos y eliminando fallos en la caché de instrucciones L1i.

5.2. Muestreo de Contadores de Hardware con simpleperf
Implementación: Medir directamente contadores de la CPU ARM Cortex:   
MD

Bash
python $ANDROID_NDK_HOME/simpleperf/app_profiler.py \
  -p com.klisee.reloaded \
  -e cache-misses,branch-misses,instructions,cpu-cycles \
  --duration 10
Umbrales de control: Mantener cache-misses / instructions por debajo del 2% y branch-misses / instructions por debajo del 3%.   
MD

5.3. Audio Nativo AAudio en Modo EXCLUSIVE y Anillo Lock-Free (rtrb)
Diagnóstico: El servidor de audio de Android (AudioFlinger) introduce entre 40 y 100 ms de retardo y bloqueos de mutex en el hilo de renderizado.   
MD

Implementación:

Abrir un stream directo de AAudio configurado con SharingMode::Exclusive y PerformanceMode::LowLatency conectado al DSP de sonido.   
MD

Comunicar el hilo del emulador con el callback de audio mediante un Single-Producer Single-Consumer (SPSC) Ring Buffer lock-free (crate rtrb) con tamaño potencia de dos.   
MD

Sincronización libre de mutex mediante barreras de memoria atómicas Acquire / Release.   
MD

Latencia de sonido reducida a < 10 milisegundos sin cortes auditivos.   
MD

5.4. Reloj Maestro Guiado por Audio (Eliminación Absoluta de Judder)
Implementación: Sincronizar el reloj del acumulador temporal con el drenaje del buffer de audio. Aplicar ajustes suaves de velocidad (
±0.5%) para absorber el desfase temporal entre el refresco de pantalla y la frecuencia de muestreo de audio, garantizando fluidez sin micro-tirones.   
MD
+ 1

FASE 6: Reingeniería Estructural de la Máquina Virtual GML (Versión v2.0.0 "Next-Gen")
Enfoque: Rediseño del intérprete, reducción del tamaño de variables a 8 bytes y diseño orientado a datos.   
MD

6.1. NaN-Boxing en Variables Dinámicas de GML (Value(u64))
Diagnóstico: El enum Value ocupa de 24 a 32 bytes por variable en memoria, saturando la caché L1D con discriminantes y padding.   
MD

Implementación: Empaquetar todo tipo de dato dinámico en un escalar u64 de exactamente 8 bytes bajo el estándar IEEE-754 Quiet-NaN:   
MD

[1b Signo] [11b Exponente = 1] [1b Quiet NaN = 1] [4b Tag] [48b Payload / Pointer]
Números reales (f64): almacenados de forma directa sin conversión.   
MD

Punteros a Heap / Strings: 48 bits de dirección de memoria virtual.   
MD

Identificadores de Instancia y Booleanos: codificados en el payload de 48 bits.   
MD

Cuadruplica la cantidad de variables que residen simultáneamente en la caché L1D.   
MD

6.2. String Interning y Symbol Cache (
O(1) Identificadores)
Diagnóstico: La resolución de nombres de variables e instancias dinámicas compara cadenas de texto byte a byte continuamente.   
MD

Implementación: Reemplazar nombres de cadenas por identificadores enteros Symbol(u32) mediante un internador estático (FastInterner / lasso). Las comparaciones de variables en tiempo de ejecución pasan de 
O(N) a una simple igualdad de enteros en 1 ciclo de reloj (
O(1)).   
MD
+ 1

6.3. Data-Oriented Design (SoA) para Instancias Activas
Diagnóstico: El struct Instance es monolítico (~400 bytes con decenas de campos fríos como alarmas o fricción), causando que un barrido de físicas cargue datos inútiles a las líneas de caché L1D.   
MD
+ 1

Implementación: Separar las propiedades dinámicas en arrays paralelos contiguos (xs: Vec<f32>, ys: Vec<f32>, hspeeds: Vec<f32>, vspeeds: Vec<f32>). LLVM auto-vectoriza el bucle de movimiento actualizando de a 4 instancias por ciclo de CPU mediante registros vectoriales ARM NEON (vaddq_f32).   
MD
+ 1

6.4. Reordenamiento Físico de Opcodes en L1i (Stefan Marr Pattern)
Implementación en el despachador de GML:

Agrupar físicamente los opcodes calientes (aritmética, lectura/escritura de locales, saltos) en el mismo bloque contiguo de código de 32 KB para maximizar los aciertos en la caché L1i.   
MD

Aislar los opcodes fríos (instance_create, instance_destroy, room_goto, diálogos) con anotaciones #[cold] #[inline(never)] para enviarlos a páginas de memoria lejanas.   
MD

6.5. Macro-Fusión de Opcodes (Superinstrucciones)
Implementación: Fusionar pares de instrucciones habituales generadas por el compilador de GM8 en un solo opcode durante la carga:   
MD

Patrón: PUSH_VAR "hp" + PUSH_INT 0 + CMP_LE_JUMP label 
→ JUMP_IF_VAR_LE_ZERO "hp", label.   
MD

Reduce la sobrecarga de saltos de despacho del intérprete a un tercio en los bucles de combate.   
MD

6.6. Despacho de Instrucciones Branchless mediante Look-Up Table (LUT)
Diagnóstico: Los bloques match de más de 200 ramas saturan el Branch Target Buffer (BTB) del procesador.

Implementación: Crear una tabla estática alineada a 64 bytes de 256 punteros a función:

Rust
#[repr(align(64))]
static OP_TABLE: [unsafe fn(&mut VMState); 256] = [ ... ];
El opcode u8 indexa directamente sin bifurcaciones condicionales: OP_TABLE[opcode as usize](state).

FASE 7: Rendimiento Máximo a Nivel de OS y Memoria (Versión v2.1.0 "Bare-Metal Tier")
Enfoque: Interacción directa con el hardware de pantalla y erradicación de asignaciones dinámicas.   
MD

7.1. Presentación Directa sobre ANativeWindow (Zero-Copy Framebuffer)
Diagnóstico: Presentar mediante Bitmap en Java o cruces intermedios de JNI ejecuta copias de memoria redundantes en el subsistema de ventanas.   
MD

Implementación:

Transferir la Surface de Android a Rust mediante el NDK.   
MD

Enlazar mediante ANativeWindow_fromSurface y bloquear el búfer de display con ANativeWindow_lock.   
MD

Escribir los píxeles directamente en la memoria mapeada de SurfaceFlinger y despachar con ANativeWindow_unlockAndPost.   
MD

Cero asignaciones intermedias en la JVM y transmisión directa al hardware de pantalla.   
MD

7.2. Compartición Directa de GPU con AHardwareBuffer
Implementación: Crear un AHardwareBuffer nativo compartido entre el hilo emulador y la GPU enlazado como textura mediante EGLImageKHR o VkImage. Elimina transferencias de texturas a través del bus del sistema.   
MD
+ 1

7.3. Arena Allocators por Cuadro (bumpalo)
Implementación: Asignar toda la memoria temporal del cuadro a partir de una Arena lineal preasignada en memoria contigua. La asignación consiste en un incremento de puntero de hardware (bump pointer) y al concluir el frame se vacía entera en 
O(1) sin llamar destructores individuales ni fragmentar el asignador nativo del sistema.

7.4. Niche Optimization y Manipulación Branchless
Optimización de nichos: Empaquetar identificadores opcionales de entidades con Option<NonZeroU32>, reduciendo el campo de 8 bytes a 4 bytes exactos sin coste de discriminante.

Bits branchless: Sustituir barridos de booleanos por bitsets u64 iterados con trailing_zeros, compilando directamente a instrucciones de hardware rbit + clz en 1 ciclo de reloj ARM64.

7.5. Configuración Definitiva del Toolchain en .cargo/config.toml
Ini, TOML
[target.aarch64-linux-android]
rustflags = [
    "-C", "target-feature=+neon,+fp-armv8,+dotprod,+fp16",
    "-C", "link-arg=-Wl,--gc-sections",
    "-C", "link-arg=-Wl,-Bsymbolic",
    "-C", "link-arg=-Wl,--no-undefined",
    "-C", "link-arg=-Wl,-z,max-page-size=16384",
    "-C", "link-arg=-Wl,-z,common-page-size=16384",
]
MATRIZ INTEGRAL DE VERSIONES Y TECNOLOGÍAS
Versión	Denominación	Tecnologías y Técnicas Clave	Impacto Principal
v1.4.1	Build 73	
Detección universal del jugador, homogeneidad TriFan de sombras, FxHashMap, cero clones en colisiones, poda AABB 20K tiles. 
MD
+ 1

Sombra fijada a 60/120 FPS; eliminación de micro-stutter. 
MD
+ 1

v1.4.2	Build 74	
Static Quad EBO [0,1,2,2,3,0], Vértice compacto de 44B, Thread Pinning a núcleos Big/Prime, Bounds Check Hoisting. 
MD
+ 1

-59.2% de tráfico a GPU; prevención de caídas térmicas. 
MD
+ 1

v1.4.3	Map & Input Polish	
Spatial Chunking (
256×256 px) con BitVec, @CriticalNative en JNI, telemetría nativa ATrace RAII. 
MD
+ 1

Erradicación de lag spikes en mapa extenso; latencia táctil de 2.5 ns. 
MD

v1.5.0	Hardware Sync Tier	
VBO Ping-Pong (glBufferSubData), Preservación Early-Z, Vértices compactos de 20B LibGDX, Colisiones Bitmask NEON, Páginas 16 KB. 
MD

Estabilidad gráfica sin bloqueos de driver TBDR; colisiones en 1 ciclo. 
MD

v1.6.0	Deterministic Time	
"Fix Your Timestep" (Fiedler LERP), Morton Codes, Carga memmap2, APK zero-copy extractNativeLibs=false, ds_grid planos. 
MD

120/144 Hz sin judder con física fija a 60 Hz; arranque en 0.1 ms. 
MD

v1.7.0	Empirical Profiling	
Profile-Guided Optimization (PGO NDK), Simpleperf counters, AAudio EXCLUSIVE con ring buffer SPSC lock-free, Audio-Driven Clock. 
MD
+ 1

Binario reordenado por hardware real; latencia de audio < 10 ms. 
MD
+ 1

v2.0.0	Next-Gen VM Core	
NaN-Boxing (Value(u64)), Symbol Interning 
O(1), SoA para entidades, Reordenamiento Marr L1i, Superinstrucciones, LUT dispatch. 
MD

Cuadruplicación de variables en L1D; vectorización NEON completa en VM. 
MD

v2.1.0	Bare-Metal OS Tier	
Presentación directa ANativeWindow, AHardwareBuffer sharing, Arena Allocators por frame (bumpalo), Niche-Filling optimization. 
MD

Eliminación total del recolector de basura; rendimiento nativo absoluto. 
MD