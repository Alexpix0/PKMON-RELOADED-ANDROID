# PROFILING_AND_SYS_TRACE_GUIDE.md: Guía de Auditoría de Rendimiento en Android NDK

**Objetivo:** Guía práctica para perfilar `libgm8emulator.so`, medir la sincronización JNI, detectar pérdidas de memoria nativa y verificar el *thread pinning* en arquitecturas big.LITTLE / DynamIQ.

---

## 1. Herramientas del Ecosistema de Profiling en Android

| Herramienta | Nivel de Inspección | Propósito Principal en el Port |
| :--- | :--- | :--- |
| **Android Studio CPU Profiler** | Hilos JVM & NDK | Detección de pausas por GC y llamadas JNI excesivas |
| **Perfetto (System Trace)** | Kernel / Scheduling | Visualizar migración de hilos entre núcleos LITTLE y BIG/PRIME |
| **Simpleperf (NDK)** | Contadores de Hardware ARM | Medir fallos de caché L1d/L2, BTB misses y saturación de pipeline |
| **Native Memory Profiler** | Asignaciones C/Rust | Erradicación de memory leaks en el boundary JNI |

---

## 2. Perfetto: Auditoría de Migración de Hilos (big.LITTLE)

Para verificar si el pinning de núcleos (`pthread_setaffinity_np` / `sched_setaffinity`) está funcionando:

1. **Captura de Traza desde la Terminal:**
   ```bash
   adb shell perfetto \
     -c - --txt \
     -o /data/misc/perfetto-traces/trace.perfetto-trace <<EOF
   buffers: {
       size_kb: 65536
       fill_policy: RING_BUFFER
   }
   data_sources: {
       config {
           name: "linux.ftrace"
           ftrace_config {
               ftrace_events: "sched/sched_switch"
               ftrace_events: "sched/sched_wakeup"
               ftrace_events: "power/cpu_frequency"
           }
       }
   }
   duration_ms: 10000
   EOF
   ```
2. **Descarga y Análisis:**
   ```bash
   adb pull /data/misc/perfetto-traces/trace.perfetto-trace .
   ```
   Abrir en [ui.perfetto.dev](https://ui.perfetto.dev).
3. **Puntos Críticos a Auditar:**
   - **Hilo `gm8_runner`:** Debe permanecer confinado exclusivamente a `cpu4`, `cpu5`, `cpu6` o `cpu7` (núcleos Gold/Prime).
   - Si se observan saltos hacia `cpu0`–`cpu3` (núcleos Silver/LITTLE), la afinidad fue ignorada o sobrescrita por el kernel.

---

## 3. Simpleperf: Medición de Fallos de Caché en Rust (`libgm8emulator.so`)

Simpleperf permite inspeccionar los contadores de rendimiento de la CPU ARM Cortex directamente:

1. **Grabar Eventos de Fallo de Caché y Predicción de Saltos:**
   ```bash
   python $ANDROID_NDK_HOME/simpleperf/app_profiler.py \
     -p com.klisee.reloaded \
     -e cache-misses,branch-misses,instructions,cpu-cycles \
     --duration 10
   ```
2. **Generar Flamegraph de Código Nativo:**
   ```bash
   python $ANDROID_NDK_HOME/simpleperf/report_to_url.py
   ```
3. **Métricas Clave:**
   - **`cache-misses / instructions`:** Si supera el 2%, indica dispersión de memoria (revisar si falta aplicar SoA en instancias de GameMaker).
   - **`branch-misses / instructions`:** Si supera el 3%, el despachador de opcodes o las comprobaciones de colisión están penalizando la predicción branchless.

---

## 4. Auditoría de Frontera JNI (Zero-Leak Check)

Para evitar desbordamientos de referencias locales en JNI (`JNI local reference table overflow`):

1. **Activar CheckJNI en el Dispositivo:**
   ```bash
   adb shell setprop debug.checkjni 1
   ```
2. **Buenas Prácticas Obligatorias en C/Rust:**
   - Limitar el alcance con `env.with_local_frame(16, || { ... })`.
   - No cruzar la frontera JNI para eventos táctiles repetitivos; usar el buffer atómico `INPUT_BUS` documentado en [`ENGINE_OPTIMIZATIONS_RESEARCH.md`](file:///c:/Users/ADV/Desktop/pokemon-reloaded-android-main/OPTIMIZAR%20RUST/ENGINE_OPTIMIZATIONS_RESEARCH.md).

