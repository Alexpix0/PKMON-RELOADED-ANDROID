# ANDROID UI/UX, QOL & KOTLIN PERFORMANCE PLAYBOOK

> **Enfoque:** Rendimiento de capa Android (Kotlin/Java), latencia táctil ultra-baja, ergonomía del overlay de control, frame pacing, gestión de energía, feedback háptico y Quality-of-Life (QoL) para Pokémon Reloaded Android.

---

## 1. Matriz de Ingesta de Enlaces y Recursos

| ID | Fecha | Categoría | Fuente / Título | Enlace | Estado | Impacto Esperado |
|---|---|---|---|---|---|---|
| **KOT-001** | 2026-10-30 | Frame Pacing | *Android Frame Pacing Library (Swappy)* | https://developer.android.com/games/sdk/frame-pacing?hl=es-419 | **ORO PURO** | Sincronización sub-milisegundo con Choreographer VSYNC en 60/90/120Hz |
| **KOT-002** | 2026-10-30 | Frame Pacing | *AOSP Graphics: Frame Pacing Architecture* | https://source.android.com/docs/core/graphics/frame-pacing | **ORO** | Eliminación de encolamiento de buffers en `SurfaceFlinger` y judder |
| **KOT-003** | 2026-10-30 | UX / Aspect Ratio | *20:9 Aspect Ratio Display & Game Resolution Guide* | https://www.reddit.com/r/Gamefusion/comments/1o29bu9/resolution_chart_for_209_aspect_ratio_phones/ | **ORO UX** | Aprovechamiento de bordes laterales (pillarbox) para HUD táctil en pantallas modernas |
| KOT-004 | 2026-10-30 | Android Games | *Android Official Game Optimization Tips* | https://developer.android.com/games/optimize/optimization-tips | Incorporado | Buenas prácticas de SurfaceView, thread affinity y throttling térmico |

---

## 2. HALLAZGOS CRÍTICOS: "EL ORO" EN LA CAPA ANDROID / KOTLIN

### 🌟 ORO K-1: Biblioteca de Frame Pacing de Android (Swappy)
* **El Problema del Stutter a 60 FPS en Pantallas de 90Hz / 120Hz:**
  - Pokémon Reloaded corre internamente a 60 ticks por segundo.
  - La mayoría de teléfonos Android modernos corren su pantalla a **90Hz, 120Hz o frecuencia variable**.
  - Si un frame de Rust/GLES se retrasa apenas 0.5 milisegundos en llamar a `eglSwapBuffers()`, `SurfaceFlinger` pierde el hardware VSYNC y retrasa el frame hasta el *siguiente* refresco. Esto produce una caída instantánea a 30 FPS perceptibles (*frame drop / micro-stutter*).
* **La Solución Técnica (Swappy NDK / C++ / Rust FFI):**
  - La biblioteca oficial **Swappy** se engancha directamente al `Choreographer` de Android a través de un canal de sincronización por hardware.
  - En lugar de bloquear la GPU con `eglSwapInterval(1)`, Swappy calcula el tiempo exacto de presentación de cada fotograma para que caiga en el pulso preciso del VSYNC (por ejemplo, exactamente 1 de cada 2 pulsos en un panel de 120Hz).
  - **Eliminación del BufferQueue Lag:** Evita que se acumulen 2 o 3 fotogramas en la cola del compositor, **reduciendo la latencia de entrada de los botones en hasta 30-50 milisegundos**.

---

### 🌟 ORO K-2: Diseño Ergonómico de Controles en Pantallas 20:9 (Pillarboxing Táctil)
* **El Problema:** Pokémon Reloaded fue diseñado originalmente con una relación de aspecto clásica **4:3**. Los teléfonos modernos tienen pantallas panorámicas ultra-anchas (**20:9** o **19.5:9**).
* **La Trampa Común:** Estirar la imagen (deforma los píxeles) o poner los botones táctiles encima de la acción del juego (los pulgares tapan al personaje, Pokémon o menú de batalla).
* **Solución de Oro (HUD en Pillarbox):**
  - Centrar el viewport 4:3 con escalado de enteros píxel-perfecto.
  - Utilizar el espacio sobrante de los laterales (aproximadamente un 30% del ancho total de la pantalla) para colocar la cruceta direccional (DPad) a la izquierda y los botones (A, B, Z, X, Start) a la derecha, **completamente fuera del área visible del juego**.
  - Ventajas:
    1. Cero obstrucción visual de la pantalla de batalla o el mapa.
    2. Ergonomía idéntica a una consola portátil física (estilo Nintendo Switch / Game Boy Advance).
    3. No se requiere renderizar texturas transparentes sobre el framebuffer del juego.

---

## 3. Auditoría de Enlaces Proporcionados: Oro vs Ruido

| Enlace / Documento | Veredicto | Motivo Técnico |
|---|---|---|
| `developer.android.com/.../frame-pacing` | **ORO PURO** | Swappy: sincronización exacta del tick a 60/90/120Hz sin queue lag. |
| `source.android.com/.../frame-pacing` | **ORO PURO** | Detalla el funcionamiento de BufferQueue y SurfaceFlinger en el kernel. |
| `reddit.com/.../resolution_chart_for_209` | **ORO UX** | Mapeo de resoluciones y márgenes para ubicar controles fuera de 4:3. |
| `developer.android.com/.../optimization-tips` | **ALTO VALOR** | Guía de SurfaceView vs TextureView y thread affinity. |
| `bugnet.io/blog/tips-for-improving-frame-pacing` | **MEDIO VALOR** | Principios generales de frame time consistency. |
| `impactotic.co/.../relacion-de-aspecto` | *RUIDO* | Artículo divulgativo de revista de tecnología para público no técnico. |
| `newhavendisplay.com/.../choosing-the-right-display` | *RUIDO* | Blog comercial para compra de módulos LCD industriales. |
| `pcbstore.com.bd/glossary/frame-pacing` | *RUIDO* | Diccionario breve de términos de una tienda online. |
| `mathworks.com/.../turbo-coding` | *RUIDO* | Algoritmos de corrección de errores en telecomunicaciones (Matlab). |
| `packages.debian.org/.../rust` | *RUIDO* | Lista de paquetes binarios de Debian sin información de optimización. |
| `github.com/jiegec/awesome-stars/...` | *RUIDO* | Bookmark personal de GitHub sin relación con el motor. |
| `superrendersfarm.com/.../gpu-rendering-errors` | *RUIDO* | Soporte técnico para granjas de render de 3ds Max/Blender. |
