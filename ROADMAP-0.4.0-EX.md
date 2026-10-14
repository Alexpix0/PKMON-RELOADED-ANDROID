# 🗺️ Hoja de Ruta — Pokémon Reloaded Android (v0.4.0 EX)

> **Documento de Planificación y Arquitectura**  
> **Proyecto:** Pokémon Reloaded para Android (Edición EX)  
> **Autores:** Alexpix0 / Armando  
> **Stack tecnológico:** Rust (OpenGMK Engine) + Kotlin (Jetpack Compose Launcher) — *Stack nativo puro*

---

## 🎯 Objetivo General
Transformar la versión **0.4.0 EX** en el salto generacional definitivo de **Pokémon Reloaded en Android**, convirtiendo el port en una experiencia idéntica a la de una consola portátil moderna:
1. **Rendimiento y fluidez sin concesiones visuales** (Poda de VRAM sin pérdida, índice en RAM para los 26,240 sprites de combate y escalado Pixel-Perfect).
2. **Editor de controles in-game** con arrastrar y soltar, redimensionado libre por pellizco y opacidad en vivo.
3. **Contador de FPS y Frame Time en pantalla** para auditoría en tiempo real.
4. **Reorganización modular de Ajustes** por categorías (Controles, Vídeo, Audio, Rendimiento, Apariencia, Sistema).
5. **Tema Claro, Tema Oscuro y Negro Puro AMOLED**.
6. **Importación ultra rápida del ZIP de 700MB** (reduciendo el tiempo de 3-5 minutos a 30-45 segundos con streaming Zero-Copy).
7. **Modo Fast-Forward con botón de turbo en pantalla (1-toque)** y atajo de silenciado rápido.
8. **Seguridad total de partidas:** Escudo Scoped Storage contra desinstalaciones, Snapshot pre-gimnasio y auto-diagnóstico de integridad de archivos.
9. **Soporte de packs de música/SoundFonts y sprites (`.gif`) en `custom_assets/`** + reporte de fallos en 1 toque.

---

## 📱 Dispositivo y Hardware de Referencia (Baseline): MediaTek Helio G25 (Infinix Hot 20i)

> **Regla de oro del proyecto:** Todo el código y las optimizaciones de la versión **0.4.0 EX** están diseñados y calibrados teniendo como objetivo que el juego funcione de forma impecable y a **60 FPS rocosos** en el hardware del **Infinix Hot 20i (MediaTek Helio G25)**:
> * **CPU (8 núcleos Cortex-A53 a 2.0 GHz / 1.5 GHz):** Código nativo Rust sin operaciones pesadas de punteros ni saltos de rama innecesarios para evitar estrangulamiento térmico (*thermal throttling*) y mantener el consumo de batería al mínimo.
> * **GPU (PowerVR GE8320):** Renderizado por lotes (Batch Quads), mínima alternancia de texturas y shaders GLSL planos sin bifurcaciones pesadas para no saturar la tasa de relleno de la GPU PowerVR.
> * **Almacenamiento (eMMC 5.1):** Descompresión en streaming Zero-Copy e índice en memoria RAM para evitar que las bajas tasas de lectura/escritura aleatoria 4K de la memoria eMMC causen tirones.
> * **Resultado garantizado:** Si el juego va fluido, frío y sin tirones en el Helio G25, **irá con holgura extrema, máxima respuesta y absoluta perfección en cualquier móvil de gama media o alta**.

---

## ⚡ 1. Rendimiento Gráfico Sin Pérdida y Eficiencia de Batería (Motor Rust)

### 1.1. Índice en Memoria para Combates (Fast Sprite Loader para los 26,240 `.gmspr`)
* **Problema:** En `data/pkmnbattle/` hay 26,240 archivos `.gmspr`. Leerlos del almacenamiento flash uno a uno al iniciar un combate causa micro-congelamientos (*stutter*).
* **Solución técnica:** Construir un índice en memoria RAM durante el inicio que mapea offsets y descriptores de cada sprite.
* **Impacto:** Los combates contra líderes, Alto Mando y Pokémon salvajes inician al instante a 60 FPS estables.

### 1.2. Poda de VRAM Sin Pérdida (Lossless VRAM Trimming) y Escalado Pixel-Perfect
* **Garantía de calidad:** **Cero pérdida de calidad gráfica ni compresión destructiva.** Los sprites conservan el 100% de su resolución nativa.
* **Mecanismo:** Descarga de la GPU las texturas de mapas anteriores que no estén en pantalla. Incorpora **escalado Pixel-Perfect (Sharp Bilinear)** para que en pantallas 1080p y 2K OLED se vea con bordes definidos y nítidos, superando la nitidez del PC original.
* **Impacto:** Ahorra más del 40% de RAM/VRAM y previene cierres por falta de memoria (*OOM Killer*).

### 1.3. Pre-Buffer de Audio y Modo Low-RAM para MIDIs
* **Mecanismo:** Carga en memoria únicamente las muestras del SoundFont (`.sf2`) que la canción actual necesita, liberando bancos de instrumentos no utilizados.
* **Impacto:** Ahorra entre 50MB y 100MB de RAM en todo momento, ideal para teléfonos de 2GB o 3GB.

### 1.4. JIT / Optimización de Bytecode GML (Cranelift)
* **Propuesta:** Integrar **Cranelift** para compilar dinámicamente las rutinas pesadas de físicas, proyectiles e IA a código máquina nativo ARM64.
* **Impacto:** Incremento de 2.5x a 4x en fotogramas mínimos durante batallas con efectos de clima y partículas.

### 1.5. Síntesis de Audio Lock-Free de Latencia Cero
* **Propuesta:** Búfer circular sin bloqueo (`rtrb` + `aaudio`) que desvincula la música y los efectos de sonido de la carga de la GPU.
* **Impacto:** Audio sin chasquidos ni tartamudeos bajo ninguna circunstancia.

### 1.6. Congelamiento Inteligente en Segundo Plano (Standby 0% Consumo)
* **Propuesta:** Pausado instantáneo de timers y suspensión de hilos (`futex / epoll_wait`) al bloquear pantalla o minimizar la app.
* **Impacto:** 0% de consumo de batería mientras no se esté jugando.

### 1.7. Limpiador Automático de Basura Temporal y Caché de Partidas
* **Propuesta:** Rutina de mantenimiento que purga archivos `.tmp` y volcados residuales en la memoria interna.
* **Impacto:** La app se mantiene siempre limpia y con el menor peso en disco posible.

---

## 🎮 2. Interfaz, Controles y Experiencia Visual (Kotlin + Rust)

### 2.1. Editor Visual Flotante In-Game (Arrastrar y Soltar)
* **Características:**
  * **Arrastre libre:** Mover con el dedo cualquier botón (`Z`, `X`, `C`, `V`, `SEL`, `OK`, `ESC`, `Diálogo` y `Joystick`).
  * **Pinch-to-zoom (Pellizco táctil):** Redimensionar cada botón individualmente.
  * **Opacidad en tiempo real:** Ver la transparencia sobre el mapa del juego en vivo.
  * **Guardado instantáneo:** Persistencia inmediata en `controls.conf` sin reiniciar.

### 2.2. Botón Rápido de Turbo / Fast-Forward en Pantalla (Acceso 1-Toque)
* **Qué es:** Botón táctil discreto con icono `>>` en la esquina superior para conmutar al instante entre velocidad normal (1.0x) y turbo (1.5x / 2.0x).
* **Impacto:** Farmeo de experiencia, crianza y exploración ágiles sin entrar a menús de ajustes.

### 2.3. Bordes Temáticos para Modo 4:3 (Bezel Art)
* **Qué es:** Personalización de las bandas laterales en pantallas panorámicas (Negro Puro OLED, bordes estilo consola retro Game Boy, o arte oficial ilustrado de Reloaded).
* **Impacto:** Estética prémium de consola portátil.

### 2.4. Botón Táctil de Silenciado Rápido (Mute Toggle 1-Toque)
* **Qué es:** Atajo en pantalla para enmudecer el juego al instante con un toque sin manipular los botones físicos de volumen del móvil.

### 2.5. Pantalla de Configuración Modular por Categorías
Reorganización estructurada de la pantalla de Ajustes en secciones limpias:
* 🎮 **Controles:** Sensibilidad del joystick, zona muerta (deadzone), márgenes, opacidad y acceso al Editor In-Game.
* 📺 **Vídeo y Pantalla:** Proporción 4:3 / pantalla completa, marcos temáticos Bezel, escalado pixel-perfect, mantener pantalla encendida y **Contador de FPS**.
* 🎵 **Audio:** Selector de SoundFonts (`.sf2`), volumen maestro, balance para altavoces y botón de silenciado.
* ⚡ **Rendimiento y Batería:** Modo ahorro térmico, standby inteligente 0%, VRAM trimmer y limpiador de caché.
* 🎨 **Apariencia:** Selector de Tema Claro / Oscuro / Negro Puro AMOLED.
* 🛠️ **Sistema y Partidas:** Importador ultra-rápido, exportador de respaldos, Snapshot rápido, escudo Scoped Storage y verificador de salud.

### 2.6. Soporte Completo de Tema Claro, Oscuro y Negro Puro AMOLED
* **Opciones:** Tema Claro luminoso, Tema Oscuro elegante, Negro Puro AMOLED (apaga los píxeles de pantallas OLED para máximo ahorro de batería) y modo Automático según el sistema Android.

---

## ⏩ 3. Calidad de Vida (QoL) y Usabilidad

### 3.1. Contador de FPS y Frame Time en Tiempo Real
* **Qué es:** Overlay sutil en la esquina superior con cuadros por segundo exactos (`60.0 FPS`) y tiempo de respuesta (`16.6 ms`).
* **Utilidad:** Comprobación matemática en tiempo real de la estabilidad del motor.

### 3.2. Copia de Seguridad Instantánea "Punto de Retorno" (Snapshot Pre-Gimnasio)
* **Qué es:** Botón en el menú de pausa que genera un punto de retorno seguro (*"Snapshot_Antes_De_Combate"*) antes de enfrentar a un líder, Alto Mando o legendario.
* **Impacto:** Seguridad para experimentar estrategias arriesgadas sin miedo a perder horas de progreso.

### 3.3. Visor de Tabla de Tipos y Guías Integrado (In-App Battle Guide)
* **Qué es:** Ventana modal flotante dentro del juego que abre la tabla de tipos (`data/image/Efectividades.png`) y listas de MTs/MOs sin salir de la app ni arriesgar que Android cierre el juego por falta de memoria.

### 3.4. Historial de Mensajes y Diálogos Recientes (Text Log)
* **Qué es:** Historial con los últimos 10 diálogos en pantalla para recuperar pistas o conversaciones que se hayan saltado por accidente al tocar rápido.

---

## 📦 4. Escalabilidad, Seguridad y Compatibilidad

### 4.1. Importación Ultra Rápida del ZIP del Juego (De 3-5 min a 30-45 s)
* **Tecnología:** Streaming Zero-Copy directo desde el archivo en Descargas (sin la copia intermedia de 700MB), descompresión paralela en los 8 núcleos del CPU con corrutinas de Kotlin, búfer de 1MB y caché de directorios.
* **Impacto:** Reduce el tiempo de instalación entre un 70% y 80%.

### 4.2. Escudo de Partidas contra Borrado de Android (Scoped Storage Shield)
* **Qué es:** Réplica de seguridad protegida fuera de la carpeta volátil `Android/data/`.
* **Impacto:** Las partidas sobreviven al 100% incluso si el usuario desinstala la aplicación o borra datos del almacenamiento.

### 4.3. Auto-Diagnóstico de Integridad de Archivos (Health Checker de los 34,388 archivos)
* **Qué es:** Verificador de suma y tamaño de los 34,388 archivos para detectar assets a 0 bytes o corruptos tras copias incompletas, ofreciendo reparación automática.

### 4.4. Soporte de Parches Ligeros (`.patch` / Actualizaciones Modulares)
* **Qué es:** Descarga y aplicación de parches de corrección de unos pocos megas sin requerir volver a descargar el zip de 700MB.

### 4.5. Packs Externos de Música y Sprites (`.gif`) en `custom_assets/`
* **Qué es:** El motor prioriza archivos de `custom_assets/` (música `.ogg`, SoundFonts `.sf2` y sprites `.gif`) sobre los originales sin tocar el zip base.

### 4.6. Crash Dump & Telemetría Local (Reporte en 1 Toque)
* Diálogo modal con script GML, línea y variables ante cualquier fallo, con botón de copiado en 1 toque.

---

## 📋 Fases de Ejecución Propuestas para v0.4.0 EX

| Fase | Bloque | Entregables Clave | Estado |
| :---: | :--- | :--- | :---: |
| **Fase 1** | **Instalador, Seguridad & UI Base** | • Importador ultra-rápido de ZIP (Zero-Copy + Multihilo).<br>• Escudo Scoped Storage para partidas.<br>• Reorganización de Ajustes por categorías.<br>• Temas Claro, Oscuro y Negro Puro AMOLED. | ✅ Completado |
| **Fase 2** | **Controles & Turbo Táctil** | • Editor flotante in-game (arrastrar y soltar + pinch-to-zoom).<br>• Botón rápido de Turbo (1-toque) y botón Mute.<br>• Contador de FPS y Frame Time en pantalla.<br>• Standby inteligente 0% en segundo plano. | ✅ Completado |
| **Fase 3** | **Rendimiento Gráfico & Batalla** | • Índice en RAM para los 26,240 sprites `.gmspr` de combate.<br>• Poda de VRAM sin pérdida + Escalado Pixel-Perfect.<br>• Audio lock-free independiente.<br>• Visor in-game de tabla de tipos (`Efectividades.png`). | ✅ Completado |
| **Fase 4** | **QoL Avanzado & Diagnóstico** | • Snapshot rápido pre-combate (Puntos de Retorno + Scoped Storage).<br>• Historial de textos recientes (In-Game Dialogue Text Log modal).<br>• Bordes temáticos Bezel Art en modo 4:3 (AMOLED, Retro Slate, Reloaded Blue).<br>• Auto-diagnóstico de integridad de archivos (Health Checker de 34,388 assets). | ✅ Completado |
| **Fase 5** | **Mods, Rueda Drastic & Optimización Profunda** | • Menú radial flotante (Rueda estilo Drastic DS) activado con flecha inferior central.<br>• Soporte transparente de mods externos en `custom_assets/` (música, SoundFonts y sprites .gif).<br>• Instalador modular de parches ligeros (.zip / .patch) sin redescargar el juego base.<br>• Crash Dump con captura de pánicos nativos, diagnóstico de sala y reporte en 1 toque.<br>• Nuevo icono 3D oficial vibrante para Android adaptativo. | ✅ Completado |
