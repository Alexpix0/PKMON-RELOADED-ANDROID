# 🗺️ Hoja de Ruta — Pokémon Reloaded Android (v0.4.0 EX)

> **Documento de Planificación y Arquitectura**  
> **Proyecto:** Pokémon Reloaded para Android (Edición EX)  
> **Autores:** Alexpix0 / Armando  
> **Stack tecnológico:** Rust (OpenGMK Engine) + Kotlin (Jetpack Compose Launcher) — *Stack puro sin lenguajes externos*

---

## 🎯 Objetivo General
Hacer de la versión **0.4.0 EX** la entrega más fluida, cómoda y visualmente personalizable del port, incorporando:
1. **Rendimiento profundo sin perder calidad visual** (Poda de VRAM sin pérdida y escalado Pixel-Perfect).
2. **Editor de controles in-game** (arrastrar y soltar con escalado individual por pellizco).
3. **Contador de FPS en tiempo real** para auditar el rendimiento exacto en pantalla.
4. **Reorganización modular de la pantalla de Ajustes** dividida por categorías lógicas.
5. **Tema Claro, Tema Oscuro y Negro Puro AMOLED**.
6. **Importación ultra rápida del ZIP de 700MB** (reduciendo el tiempo de 3-5 minutos a 30-45 segundos).
7. **Modo Fast-Forward (Acelerador 1.5x / 2.0x)** en fase de pruebas.
8. **Soporte de packs de música/SoundFonts y sprites (`.gif`) externos** + reporte de fallos en 1 toque.

---

## ⚡ 1. Rendimiento Gráfico Sin Pérdida y Eficiencia de Batería (Motor Rust)

### 1.1. Poda de VRAM Sin Pérdida (Lossless VRAM Trimming) y Escalado Pixel-Perfect
* **Garantía de calidad visual:** **Cero pérdida de calidad gráfica ni compresión destructiva.** Los sprites conservan el 100% de su resolución nativa original.
* **Mecanismo técnico:**
  * En lugar de mantener en la memoria de la GPU texturas de mapas o especies de rutas que ya no están visibles, el recolector de texturas en `opengl.rs` descarga de la VRAM lo inactivo y mantiene activos únicamente los elementos en pantalla.
  * **Escalado Pixel-Perfect / Sharp Bilinear:** Ajuste matemático en pantalla completa que preserva los píxeles cuadrados con bordes limpios, viéndose aún más nítido y definido en pantallas 1080p y 2K OLED que en el monitor de un PC.
* **Impacto:** Reduce el uso de memoria RAM/VRAM en más del 40%, previniendo cierres inesperados por falta de memoria (*OOM Killer*) y manteniendo 60 FPS estables.

### 1.2. JIT / Optimización de Ejecución de Bytecode GML (Cranelift)
* **Propuesta técnica:** Integrar **Cranelift** (generador de código JIT ligero de Rust) para compilar en tiempo de ejecución rutinas críticas de colisiones, físicas de combate masivo e IA a código máquina nativo ARM64.
* **Impacto:** Multiplica por 2.5x a 4x los FPS mínimos y previene calentamiento térmico en sesiones largas.

### 1.3. Síntesis de Audio Lock-Free de Latencia Cero
* **Propuesta técnica:** Desacoplar por completo el sintetizador SoundFont (`rustysynth` / `aaudio`) del bucle gráfico mediante un búfer circular lock-free (`rtrb`).
* **Impacto:** El audio nunca tartamudea ni se entrecorta, incluso ante cargas pesadas de procesamiento gráfico.

### 1.4. Congelamiento Inteligente en Segundo Plano (Standby 0% Consumo)
* **Propuesta técnica:** Conectar el ciclo de vida de Android (`onPause` / `onStop`) con el bucle nativo en Rust para pausar los timers y suspender el hilo (`futex / epoll_wait`).
* **Impacto:** 0% de drenaje de batería mientras la pantalla esté apagada o el juego minimizado.

---

## 🎮 2. Interfaz, Controles y Ajustes Modulares (Kotlin + Rust)

### 2.1. Editor Visual Flotante In-Game (Arrastrar y Soltar)
* **Objetivo:** Permitir ajustar cada botón directamente sobre la partida en vivo sin usar sliders estáticos.
* **Características:**
  * **Arrastre libre:** Mover con el dedo cualquier botón (`Z`, `X`, `C`, `V`, `SEL`, `OK`, `ESC`, `Diálogo` y `Joystick`).
  * **Pinch-to-zoom (Pellizco táctil):** Redimensionar cada botón de manera independiente (por ejemplo, hacer el botón de Ataque más grande que el de Diálogo).
  * **Opacidad en tiempo real:** Ver la transparencia sobre el mapa del juego mientras se ajusta.
  * **Guardado instantáneo:** Se guarda en `controls.conf` de inmediato sin reiniciar la partida.

### 2.2. Pantalla de Configuración Modular Dividida por Categorías
Reorganización integral de la pantalla de Ajustes en pestañas/secciones limpias y ordenadas:
* 🎮 **Controles:** Sensibilidad del joystick virtual, zona muerta (deadzone), márgenes, opacidad y botón directo al *Editor In-Game*.
* 📺 **Vídeo y Pantalla:** Aspect ratio original (4:3) o pantalla completa, escalado pixel-perfect, mantener pantalla encendida y **Contador de FPS en pantalla**.
* 🎵 **Audio:** Selector de SoundFonts (`.sf2`), volumen maestro, balance para altavoces móviles y auriculares.
* ⚡ **Rendimiento y Batería:** Modo ahorro térmico, standby inteligente 0% y optimizador de VRAM.
* 🎨 **Apariencia:** Selector de Tema Claro / Oscuro / Negro Puro AMOLED.
* 🛠️ **Sistema y Partidas:** Importador ultra-rápido, exportador de respaldos en `.zip`, clonador de ranuras y visualizador de registros.

### 2.3. Soporte Completo de Tema Claro, Tema Oscuro y Negro Puro AMOLED
* **Opciones disponibles:**
  * **Tema Claro:** Interfaz luminosa y moderna con contraste óptimo.
  * **Tema Oscuro:** Tonos grises oscuros elegantes para uso nocturno.
  * **Negro Puro (AMOLED):** Fondo negro absoluto (`#000000`) que apaga los píxeles de las pantallas OLED para un ahorro extra de batería.
  * **Seguir al sistema:** Conmutación automática según la configuración del sistema Android.

---

## ⏩ 3. Calidad de Vida (QoL) y Usabilidad

### 3.1. Contador de FPS y Frame Time en Tiempo Real
* **Qué es:** Un ajuste configurable en *Ajustes > Vídeo* que añade un overlay sutil y minimalista en la esquina superior de la pantalla.
* **Qué muestra:** Tasa de cuadros por segundo en tiempo real (ej. `60.0 FPS`) y tiempo de respuesta en milisegundos (ej. `16.6 ms`).
* **Utilidad:** Permite verificar con precisión matemática la fluidez del juego y el impacto de los modos de rendimiento.

### 3.2. Modo Fast-Forward / Acelerador de Juego (1.5x / 2.0x — Experimental)
* **Objetivo:** Acelerar el avance en crianza, entrenamiento o cinemáticas lentas.
* **Diseño:** Multiplicador dinámico del reloj de GameMaker con sincronización de audio (modo mute o tono ajustado) y activación configurable por pulsación sostenida o conmutación (*toggle*).

---

## 📦 4. Escalabilidad, Mods y Rendimiento de Instalación

### 4.1. Importación Ultra Rápida del ZIP del Juego (De 3-5 min a 30-45 s)
* **Problema actual:** Actualmente el importador copia el `.zip` de 700MB a la memoria interna (`import.zip`) y luego lo descomprime secuencialmente con un solo hilo y un búfer pequeño de 64KB, escribiendo casi 2GB en la memoria flash del teléfono.
* **Solución técnica implementada:**
  1. **Zero-Copy Streaming:** Descompresión directa desde el archivo seleccionado en Descargas vía `ParcelFileDescriptor`, eliminando la copia intermedia de 700MB.
  2. **Descompresión Multihilo en Paralelo:** Aprovechamiento de los 8 núcleos del procesador del móvil para descomprimir lotes de archivos concurrentemente con corrutinas de Kotlin.
  3. **Búfer de I/O de 1MB:** Reducción del 80% en los cambios de contexto del sistema operativo.
  4. **Caché de directorios creados:** Evita llamadas redundantes a `mkdirs()`.
* **Resultado:** La importación de Pokémon Reloaded se completa en un 70% a 80% menos de tiempo.

### 4.2. Packs Externos de Música y Sprites (`.gif`) en `custom_assets/`
* **Qué es:** El motor buscará prioritariamente en la carpeta `Android/data/com.klisee.reloaded/files/custom_assets/`:
  * SoundFonts (`soundfont.sf2`) alternativos o remixes de música (`.ogg`).
  * Sprites alternativos (`pkmn_front/`, `pkmn_back/`).
* Si el archivo no existe en la carpeta personalizada, el juego utiliza el original sin alterar el zip base.

### 4.3. Crash Dump & Telemetría Local (Reporte en 1 Toque)
* Si ocurre un cierre inesperado o error de GameMaker, el motor intercepta la excepción y muestra un diálogo modal con el script de GML y línea exacta del fallo, con un botón para copiar el reporte formateado en 1 toque.

---

## 📋 Fases de Ejecución Propuestas para v0.4.0 EX

| Fase | Bloque | Entregables Clave | Estado |
| :---: | :--- | :--- | :---: |
| **Fase 1** | **Instalador & Apariencia** | • Importador ultra-rápido de ZIP (Zero-Copy + Multihilo).<br>• Reorganización de Ajustes por categorías.<br>• Temas Claro, Oscuro y Negro Puro AMOLED. | ⏳ Siguiente |
| **Fase 2** | **Controles & QoL** | • Editor flotante in-game (arrastrar y soltar + pinch-to-zoom).<br>• Contador de FPS y Frame Time en pantalla.<br>• Standby inteligente 0% en segundo plano. | ⏳ Planificado |
| **Fase 3** | **Rendimiento & QoL** | • Poda de VRAM sin pérdida + Escalado Pixel-Perfect.<br>• Audio lock-free independiente.<br>• Modo Fast-Forward experimental (1.5x / 2.0x). | ⏳ Planificado |
| **Fase 4** | **Mods & Estabilidad** | • Soporte de packs en `custom_assets/` (música y gifs).<br>• Crash Dump modal en 1 toque. | ⏳ Planificado |
| **Fase 5** | **Optimización Profunda** | • JIT con Cranelift para rutinas críticas de combate. | ⏳ Planificado |
