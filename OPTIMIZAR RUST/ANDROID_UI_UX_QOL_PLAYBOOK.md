# ANDROID UI/UX, QOL & KOTLIN PERFORMANCE PLAYBOOK

> **Enfoque:** Rendimiento de capa Android (Kotlin/Java), latencia táctil ultra-baja, ergonomía del overlay de control, frame pacing, gestión de energía, feedback háptico y Quality-of-Life (QoL) para Pokémon Reloaded Android.

---

## 1. Matriz de Ingesta de Enlaces y Recursos

| ID | Fecha | Categoría | Fuente / Título | Enlace | Estado | Impacto Esperado |
|---|---|---|---|---|---|---|
| KOT-001 | 2026-10-30 | Touch Latency | *Por auditar* | - | Pendiente | Reducción de latencia táctil en `MotionEvent` |

---

## 2. Ejes de Investigación Crítica (UI/UX, QoL y Kotlin)

### A. Latencia Táctil y Eventos de Entrada (Touch Input Pipeline)
* **Unbuffered Input:** `View.requestUnbufferedDispatch(MotionEvent)` para saltar el buffer de vsync del compositor de Android y recibir toques a la tasa del digitalizador (120Hz - 240Hz).
* **Zero-Alloc en `onTouchEvent`:** Reutilización de estructuras de toques sin instanciar objetos por evento (`MotionEvent.getHistoricalX/Y`, pool de pointers).
* **Native InputQueue:** Pasar eventos táctiles directamente a Rust/C++ mediante JNI directo o AInputQueue sin cruzar el framework de vistas de Java.

### B. Frame Pacing y Tasa de Refresco Variable (VRR / 90Hz / 120Hz)
* **Android Frame Pacing (Swappy):** Sincronización entre el tick de GM8 y el VSYNC de Android para erradicar micro-stuttering.
* **Modos de Pantalla:** Configuración de `preferredDisplayModeId` y `Window.setFrameRate` para forzar 60fps constantes o sincronizar con paneles de 120Hz.
* **SurfaceView vs TextureView:** Uso de `SurfaceView` con hardware-compositor overlay para ahorrar hasta un 20% de consumo de GPU frente a `TextureView`.

### C. UX & Ergonomía del Virtual Pad / Touch Overlay
* **Multi-touch sin ghosting:** Algoritmos de discriminación de toques simultáneos (DPad deslizante + Botones A/B/Z/X).
* **Snap-to-Edge y Deadzones:** Zonas muertas configurables en DPad/Stick analógico para evitar toques fantasma al mover el pulgar.
* **Modo Ambidiestro / Layouts Dinámicos:** Perfiles de botones personalizables por pantalla y persistencia en SharedPreferences sin bloqueo de I/O en UI thread.

### D. Háptica Inmersiva (Advanced Haptic Feedback)
* **`VibrationEffect.createWaveform`:** Feedback háptico diferenciado (click seco al pulsar, release sutil, vibración continua de baja amplitud en eventos de batalla).
* **Pre-carga de Efectos:** `VibratorManager` y `Vibrator.areAllPrimitivesSupported` para evitar latencia de inicialización del motor háptico.

### E. Integración de Sistema y QoL de Emulación
* **Display Cutout / Notch / Letterbox:** Gestión precisa de `WindowInsetsControllerCompat` y `layoutInDisplayCutoutMode` para aprovechar pantallas 20:9 colocando los controles táctiles en los bordes negros (pillarbox) sin tapar el área de juego 4:3.
* **Gestión de Saves (SAF & Cache):** I/O asíncrono con corrutinas de Kotlin (`Dispatchers.IO`) y canales no bloqueantes para copias de seguridad de partidas guardadas sin congelar la UI.
* **Discord RPC Throttling:** Evitar drenaje de batería y rate limits actualizando el estado de Discord únicamente en transiciones de sala (`room_goto`) o batallas, no en cada frame.

---

## 3. Síntesis y Acciones Derivadas
*(Espacio reservado para análisis de enlaces aportados: Diagnóstico -> Código Kotlin/JNI -> Benchmark de Latencia)*

