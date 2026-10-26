# 🗺️ Hoja de Ruta — Pokémon Reloaded Android (v0.5.0 Milestone Definitivo)

> **Documento de Planificación y Arquitectura de la Versión Definitiva**  
> **Proyecto:** Pokémon Reloaded para Android  
> **Versión Objetivo:** 0.5.0  
> **Autores:** Alexpix0 / Armando  
> **Stack tecnológico:** Rust (OpenGMK Engine) + JIT Machine Code (Cranelift ARM64) + Kotlin (Jetpack Compose Launcher) — *Stack nativo de ultra-bajo consumo*

---

## 🎯 Objetivo General de la v0.5.0
Llevar el rendimiento y la personalización de **Pokémon Reloaded en Android** a su cota más alta en la historia del port mediante una arquitectura limpia y organizada:
1. **Toda la configuración técnica se centraliza en el Launcher** dentro de sus respectivas categorías de ajustes, manteniendo la **experiencia in-game 100% limpia e inmersiva** (solo la partida, su rueda táctil y sus controles).
2. **Gran Apartado de "Ajustes de Vídeo y Rendimiento"** con multiplicadores de velocidad turbo (**x1.5, x2, x4, x6, x8**), Frame Skip inteligente, escala de resolución y shaders.
3. **Revolución del Rendimiento Nativo:** Introducción de compilación **JIT a código máquina ARM64 (Cranelift)** para rutinas pesadas de combate, **Texture Atlasing** para reducir drásticamente las llamadas de dibujo (Draw Calls), **SIMD ARM NEON** y **Zero-Copy Framebuffer (`AHardwareBuffer`)**.
4. **Ergonomía de Controles Total:** Autofire individual, D-Pad dinámico bajo el pulgar, soporte plug-and-play de gamepads físicos y gestos táctiles.
5. **Acústica de Precisión:** Separación independiente de música (BGM) y efectos (SFX), calibración de latencia para auriculares Bluetooth y mezcla mono.
6. **Sistema y Protección:** Playtime tracker, diagnóstico térmico y actualizador OTA integrado.

---

## 📱 Hardware de Referencia (Baseline): MediaTek Helio G25 + GPUs de Gama Baja
* **Dispositivo de prueba:** Infinix Hot 20i (MediaTek Helio G25: 8x Cortex-A53, GPU PowerVR GE8320, eMMC 5.1).
* **GPUs de gama de entrada / antiguas compatibles:** PowerVR GE8320, Mali-400 / Mali-T720 / Mali-G52, Adreno 306 / Adreno 505.
* **Criterio de éxito:** Alcanzar **60 FPS rocosos** con tiempo de cuadro constante (16.6ms) incluso en combates dobles con clima adverso y múltiples proyectiles en pantalla.

---

## 📺 1. Apartado de "Ajustes de Vídeo y Rendimiento" (Ubicación: Launcher > Ajustes > Vídeo)

> **Ubicación en la UI:** Menú grande y dedicado dentro de la pantalla de Ajustes del Launcher. Todo se configura antes de lanzar la partida.

### 1.1. Selector de Multiplicador Turbo / Fast-Forward (x2, x4, x6, x8)
* **Qué es:** Selector deslizable/selector múltiple para definir la velocidad que aplica el botón rápido de turbo (`>>`):
  * **x1.5 (Acelerado suave):** Combates dinámicos con precisión total de esquiva.
  * **x2.0 (Doble velocidad):** Estándar ideal para crianza y rutas.
  * **x4.0 (Cuádruple velocidad):** Farmeo intensivo de niveles y eclosión de huevos.
  * **x6.0 y x8.0 (Ultra Turbo):** Avance instantáneo de conversaciones largas y viajes interurbanos.
* **Técnica:** Desactivación controlada del limitador de ticks en el motor Rust multiplicando los ciclos de GameMaker sin desincronizar la máquina de estados.

### 1.2. Frame Skip Inteligente y Adaptativo
* **Qué es:** Salta el renderizado visual de 1, 2 o 3 fotogramas cuando la GPU experimente caídas por debajo de 60 FPS, manteniendo la física y la lógica al 100% de velocidad.
  * **Modos:** *Desactivado*, *1 Frame*, *2 Frames*, *Automático Adaptativo* (se activa únicamente cuando el hardware lo necesita).
* **Impacto:** Elimina por completo el "efecto cámara lenta" en dispositivos de gama baja.

### 1.3. Escala de Resolución Interna (Render Scale)
* **Opciones:**
  * **0.75x (Modo Rendimiento / GPUs antiguas):** Reduce la tasa de relleno requerida en GPUs PowerVR o Mali viejas. Aumenta de **5 a 15 FPS** o ahorra hasta un 30% de batería.
  * **1.0x (Nativo Pixel-Perfect):** Resolución 100% original con fidelidad total.
  * **1.25x / 1.5x (Super-Sampled Sharp):** Mayor definición para pantallas OLED 1080p y 2K.

### 1.4. Filtros Gráficos y Shaders
* **Pixel-Perfect (Nearest Neighbor):** Píxeles retro definidos y sin desenfoques.
* **Sharp Bilinear:** Suavizado inteligente que evita la distorsión de píxeles en pantallas panorámicas alargadas (20:9).
* **CRT Retro Scanlines (Ligero):** Simula las líneas de barrido de pantallas clásicas sin penalizar la GPU.
* **Anti-Aliasing FXAA / SMAA Ligero:** Suaviza los bordes dentados de sprites diagonales por post-procesado con consumo mínimo.

### 1.5. Modos de Formato de Pantalla
* **4:3 Centrado (Nativo):** Relación de aspecto original con bandas laterales limpias o Bezel Art.
* **16:9 Estirado Inteligente No Lineal:** Mantiene la proporción 1:1 en el centro de la pantalla (donde está el protagonista) y estira suavemente los extremos laterales para llenar pantallas anchas sin deformar al personaje.
* **Recorte Panorámico (Crop-to-Fill):** Llena toda la pantalla eliminando completamente barras negras.

### 1.6. Perfiles de Color y Saturación
* **OLED Vibrante:** Incrementa la saturación y el contraste para paneles AMOLED modernos.
* **Retro GBA Cálido:** Aplica una tonalidad cálida nostálgica clásica de consolas portátiles.
* **Estándar Neutro:** Fidelidad de color original del juego en PC.

### 1.7. Control de Tasa de Refresco & V-Sync
* **Límites seleccionables:**
  * **30 FPS (Ahorro Extremo de Batería).**
  * **60 FPS (Nativo fluido de Pokémon Reloaded).**
  * **90 Hz / 120 Hz:** Para pantallas modernas de alta tasa de refresco con interpolación de suavidad.
* **Triple Buffering On/Off:** Evita el desgarro de imagen (*tearing*) sin añadir latencia táctil.

### 1.8. Reducción y Simplificación de Clima y Partículas
* Selector de densidad para lluvia, nieve, niebla y tormenta de arena:
  * **Completo:** Idéntico a PC.
  * **Optimizado:** Reduce las partículas a la mitad manteniendo la atmósfera visual.
  * **Mínimo:** Tinte de pantalla ambiental que elimina el 90% de la carga gráfica en combates dobles.

### 1.9. Precarga Asíncrona de Shaders y Caché en Disco (Shader Cache Disk)
* Compila los shaders en un hilo secundario durante la pantalla de inicio y guarda los binarios compilados en disco.
* **Impacto:** Elimina el micro-congelamiento la primera vez que se lanza un ataque en combate e inicia el juego en menos de 1 segundo.

---

## ⚡ 2. Rendimiento Nativo, Arquitectura y Nuevo Lenguaje JIT (Motor Rust / C++)

> **Ubicación en la UI:** Optimización interna transparente del motor nativo con conmutadores de diagnóstico en *Ajustes > Rendimiento*.

### 2.1. Compilador JIT de GML a Código Máquina ARM64 (con `Cranelift`)
* **Problema:** Interpretar scripts de GameMaker en bucle consume ciclos excesivos de CPU evaluando variables dinámicas e instrucciones una por una.
* **Solución técnica:** Integrar **Cranelift** (el generador de código nativo de alto rendimiento en Rust) para compilar en caliente (*Just-In-Time*) las rutinas más pesadas de combate, proyectiles e inteligencia artificial directamente a **instrucciones de máquina nativas ARM64**.
* **Impacto esperado:** Ganancia de **2x a 4x en fotogramas mínimos** durante batallas masivas.

### 2.2. Dynamic Texture Atlasing en RAM (Reducción Masiva de Draw Calls)
* **Problema:** En batalla cada sprite individual realiza una llamada `glDrawArrays`, generando más de 400 llamadas por fotograma y saturando el driver OpenGL.
* **Solución técnica:** Empaquetar dinámicamente en RAM los sprites más utilizados en un único Atlas de 2048x2048.
* **Impacto esperado:** Reducción de 400+ llamadas de dibujo a **menos de 40 por frame**. Ganancia de **5 a 12 FPS** en zonas densas.

### 2.3. Desacoplamiento de Hilos (Logic Thread & Render Thread Independientes)
* **Solución técnica:** Separar el bucle de eventos y físicas de GameMaker del hilo de renderizado OpenGL/EGL mediante un búfer circular de comandos sin bloqueo (`ringbuffer`).
* **Impacto esperado:** Si la GPU tiene un tirón, la respuesta a los botones táctiles y la lógica no sufren retraso.

### 2.4. Aceleración Vectorial SIMD ARM NEON en Físicas
* **Solución técnica:** Reescribir las funciones críticas de cálculo de distancias euclidianas, trayectorias de ataques y cajas de colisión (*AABB / Hitboxes*) utilizando instrucciones intrínsecas ARM NEON (`std::arch::aarch64`).
* **Impacto esperado:** Cálculo de físicas hasta **3x más rápido** en CPU.

### 2.5. Zero-Copy Framebuffer Blitting (`AHardwareBuffer` Nativo)
* **Solución técnica:** Enlazar el framebuffer del motor directamente con el compositor del sistema Android (*SurfaceFlinger*) vía `AHardwareBuffer`.
* **Impacto:** Elimina copias redundantes en memoria RAM, liberando ancho de banda del bus de memoria.

### 2.6. Arena Allocator de Memoria (`bumpalo` en Rust)
* **Solución técnica:** Reemplazar las asignaciones dinámicas dispersas por un asignador en bloque para instancias temporales de GameMaker.
* **Impacto:** 0 fragmentación de memoria RAM y prevención total de pausas del recolector de memoria.

### 2.7. Compresión de Texturas en VRAM (ASTC / ETC2)
* Carga de sprites comprimidos por hardware en la GPU.
* **Impacto:** Reduce el peso de texturas en VRAM de 150MB a menos de 40MB, ideal para dispositivos con 2GB o 3GB de RAM.

### 2.8. Backend Gráfico Vulkan (Vía SPIR-V / `ash`) como Alternativa Opcional
* Para dispositivos modernos con Android 10+, ofrecer la opción de renderizar mediante Vulkan para reducir al mínimo el overhead del driver en comparación con OpenGL ES.

---

## 🎮 3. Apartado de "Ajustes de Controles y Entrada" (Ubicación: Launcher > Ajustes > Controles)

> **Ubicación en la UI:** Configuración principal en el Launcher y aplicación transparente sobre el HUD táctil in-game.

### 3.1. Autofire / Disparo Automático Individual por Botón
* **Qué es:** Opción para que al mantener presionado cualquier botón (por ejemplo `Z` o `X`), el emulador emita una ráfaga continua configurable (10, 15 o 20 pulsaciones por segundo).
* **Utilidad:** Pasar diálogos instantáneamente y combatir con comodidad sin castigar la pantalla ni cansar los dedos.

### 3.2. D-Pad Flotante Dinámico (Smart Thumb Joystick)
* **Qué es:** El joystick táctil no permanece fijo en una esquina; aparece instantáneamente donde el usuario apoye el pulgar izquierdo en la mitad izquierda de la pantalla.
* **Utilidad:** Ergonomía adaptativa que se acomoda automáticamente a la postura de la mano.

### 3.3. Retroalimentación Háptica (Vibración Táctil)
* **Qué es:** Vibración táctil regulable (Desactivada, Suave, Media, Fuerte) al pulsar botones táctiles y vibración inmersiva en eventos clave (recibir un golpe crítico o al capturar un Pokémon).

### 3.4. Soporte Integral de Mandos Físicos (Bluetooth y USB-C OTG)
* **Soporte plug-and-play:** Detección de mandos Xbox, PlayStation (PS4/PS5), Switch Pro y controladores telescópicos (GameSir, Kishi, Backbone).
* **Remapeo visual:** Asignador intuitivo de botones físicos.
* **Auto-ocultado:** Al detectar pulsaciones de un gamepad físico, los controles táctiles de la pantalla se desvanecen automáticamente dejando la imagen 100% limpia.

### 3.5. Gestos Táctiles Rápidos (Swipe Gestures)
* Atajos con dedos sobre la pantalla del juego:
  * Deslizar 2 dedos hacia arriba: Pausar el juego.
  * Deslizar hacia la derecha: Activar / desactivar velocidad Turbo.
  * Tocar con 3 dedos: Silenciar sonido al instante.

### 3.6. Combo Buttons (Botón Combinado Configurable)
* Botón táctil adicional en el HUD que ejecuta dos teclas a la vez (por ejemplo `Z + X`), muy útil para ciertas confirmaciones o maniobras especiales.

### 3.7. Ajuste de Zona Muerta (Deadzone) y Curva de Sensibilidad
* Calibración del recorrido del joystick táctil para ajustarse a pantallas con muestreo táctil de 120Hz o 240Hz.

### 3.8. Perfiles de Disposición de Controles
* Selector de esquemas guardados en el Launcher: *Perfil Estándar*, *Perfil 1 Mano*, *Perfil Tablet / Pantalla Grande*.

---

## 🎵 4. Apartado de "Ajustes de Audio y Sonido" (Ubicación: Launcher > Ajustes > Audio)

> **Ubicación en la UI:** Sección dedicada en Ajustes del Launcher.

### 4.1. Separación de Canales de Sonido (BGM vs SFX)
* Controles deslizantes independientes para Música de fondo (BGM) y Efectos de sonido (SFX).
* **Utilidad:** Permite jugar escuchando podcasts o música propia silenciando solo la BGM, sin perder los sonidos de los ataques.

### 4.2. Selector de Calidad de Interpolación de SoundFonts MIDI
* **Lineal (Rápida):** Mínimo consumo de CPU para teléfonos de gama baja.
* **Cúbica / Sinc (Alta Fidelidad):** Máxima pureza orquestal del SoundFont de Reloaded en terminales medios/altos.

### 4.3. Pipeline AAudio Exclusivo de Ultra-Baja Latencia
* Conexión directa a bajo nivel con el servidor de audio de Android (`Oboe/AAudio`), reduciendo el retraso de sonido a menos de 15ms.

### 4.4. Compensación de Retardo para Auriculares Bluetooth
* Slider de sincronización (-50ms a +200ms) para que los sonidos de batalla coincidan milimétricamente con la imagen al usar audífonos inalámbricos.

### 4.5. Modo Forzar Mono
* Mezcla ambos canales en uno solo para dispositivos móviles que cuentan con un único altavoz en la parte inferior, evitando perder notas o sonidos paneados.

### 4.6. Atenuación Suave al Minimizar (Audio Fade-Out)
* Al salir de la app o recibir una llamada, el volumen desciende de forma suave y elegante en lugar de cortarse bruscamente.

---

## 📱 5. Apartado de "Ajustes Generales y Sistema" (Ubicación: Launcher > Ajustes > General)

> **Ubicación en la UI:** Pantalla principal del Launcher y Ajustes Generales.

### 5.1. Contador de Horas de Juego (Playtime Tracker)
* Registro nativo en el Launcher con el tiempo total jugado (horas y minutos), sin alterar el archivo de guardado del juego.

### 5.2. Botón de Lanzamiento Rápido "Continuar Inmediato"
* Acceso directo en el Launcher que salta intros o pantallas de bienvenida y carga inmediatamente la partida guardada.

### 5.3. Widget de Diagnóstico Térmico y Batería
* Indicador visual en el Launcher que muestra el porcentaje de batería y la temperatura del dispositivo antes de iniciar una sesión.

### 5.4. Perfil Térmico y Battery Saver Inteligente
* Si el teléfono supera los 42°C o la batería baja del 15%, activa automáticamente Frame Skip o reduce la resolución interna a 0.75x para proteger el hardware y alargar la autonomía.

### 5.5. Comprobador de Actualizaciones OTA (GitHub Releases)
* Notificación discreta en el Launcher cuando haya una nueva versión del APK disponible en GitHub, con descarga e instalación en 1 toque.

### 5.6. Selector de Idioma del Launcher
* Alternar la interfaz del emulador entre Español e Inglés.

---

## 📋 Tabla Maestra de Implementaciones para la Versión 0.5.0

| Módulo / Categoría | Ubicación en la App | Característica Propuesta | Prioridad | Impacto en Rendimiento / Experiencia |
| :--- | :--- | :--- | :---: | :--- |
| **Vídeo / Rendimiento** | Launcher > Ajustes > Vídeo | **Turbo Multiplicador:** x1.5, x2, x4, x6, x8 | 🔴 Alta | Aceleración extrema para farmeo y diálogos. |
| **Vídeo / Rendimiento** | Launcher > Ajustes > Vídeo | **Frame Skip Inteligente / Adaptativo** | 🔴 Alta | Elimina la ralentización en teléfonos de gama baja. |
| **Vídeo / Rendimiento** | Launcher > Ajustes > Vídeo | **Escala de Resolución Interna (0.75x a 1.5x)** | 🟡 Media | Gana de 5 a 15 FPS en GPUs PowerVR y Mali antiguas. |
| **Vídeo / Rendimiento** | Launcher > Ajustes > Vídeo | **Filtros Gráficos (Sharp Bilinear, Nearest, CRT, FXAA)** | 🟢 Deseable | Estética visual prémium y nítida. |
| **Vídeo / Rendimiento** | Launcher > Ajustes > Vídeo | **Formato 16:9 Estirado Inteligente No Lineal** | 🟢 Deseable | Llena la pantalla sin deformar al personaje central. |
| **Vídeo / Rendimiento** | Launcher > Ajustes > Vídeo | **Precarga Asíncrona de Shaders & Disk Cache** | 🔴 Alta | Elimina micro-congelamientos y arranca en 1 segundo. |
| **Vídeo / Rendimiento** | Launcher > Ajustes > Vídeo | **Control de Refresco (30, 60, 90/120 Hz) & V-Sync** | 🟡 Media | Fluidez máxima o ahorro extremo de batería. |
| **Vídeo / Rendimiento** | Launcher > Ajustes > Vídeo | **Optimización de Clima y Partículas** | 🟡 Media | Alivia la carga de GPU en tormentas y combates dobles. |
| **Motor & Arquitectura**| Motor Rust / C++ Interno | **JIT Compiler a ARM64 Nativo (Cranelift)** | 🔴 Alta | Multiplica por 2x a 4x los FPS mínimos en combates. |
| **Motor & Arquitectura**| Motor Rust Interno | **Texture Atlasing Dinámico en RAM** | 🔴 Alta | Reduce llamadas de dibujo de >400 a <40 por frame. |
| **Motor & Arquitectura**| Motor Rust Interno | **Desacoplamiento Logic/Render (Multithread)** | 🔴 Alta | Respuesta táctil a 60 FPS sin importar carga gráfica. |
| **Motor & Arquitectura**| Motor Rust Interno | **Aceleración SIMD ARM NEON en colisiones** | 🟡 Media | Cálculo de proyectiles e IA 3x más rápido. |
| **Motor & Arquitectura**| Motor Rust Interno | **Zero-Copy Framebuffer (`AHardwareBuffer`)** | 🟡 Media | Ahorra ancho de banda de memoria RAM. |
| **Motor & Arquitectura**| Motor Rust Interno | **Arena Allocator de Memoria (`bumpalo`)** | 🟡 Media | Cero fragmentación de RAM y pausas de recolección. |
| **Controles** | Launcher > Ajustes > Controles | **Autofire Individual por Botón (Ráfaga Z/X)** | 🟡 Media | Pasar textos y combates sin cansar los dedos. |
| **Controles** | Launcher > Ajustes > Controles | **D-Pad Flotante Dinámico** | 🟢 Deseable | Comodidad ergonómica total con una mano. |
| **Controles** | Launcher > Ajustes > Controles | **Soporte Gamepads Físicos + Ocultar Táctil** | 🟡 Media | Experiencia de consola con mandos Bluetooth/USB. |
| **Controles** | Launcher > Ajustes > Controles | **Gestos Táctiles Rápidos (Swipe Gestures)** | 🟢 Deseable | Atajos con 2 y 3 dedos para pausar, turbo y mute. |
| **Controles** | Launcher > Ajustes > Controles | **Combo Buttons (Botón Combinado Z+X)** | 🟢 Deseable | Ejecutar dos acciones con un solo botón táctil. |
| **Audio** | Launcher > Ajustes > Audio | **Separación Independiente BGM y SFX** | 🟡 Media | Jugar con efectos mientras se escucha música propia. |
| **Audio** | Launcher > Ajustes > Audio | **Compensación de Latencia Bluetooth** | 🟡 Media | Sincronización exacta con auriculares inalámbricos. |
| **Audio** | Launcher > Ajustes > Audio | **Selector Calidad Interpolación SoundFonts** | 🟢 Deseable | Optimización de CPU en síntesis MIDI. |
| **Audio** | Launcher > Ajustes > Audio | **Modo Forzar Mono** | 🟢 Deseable | Mezcla perfecta para móviles con un solo altavoz. |
| **General / Sistema** | Launcher > Ajustes > General | **Contador de Horas Jugadas (Playtime)** | 🟢 Deseable | Registro de tiempo de juego en el Launcher. |
| **General / Sistema** | Launcher > Ajustes > General | **Lanzamiento Rápido "Continuar Inmediato"** | 🟢 Deseable | Entrada instantánea a la partida saltando avisos. |
| **General / Sistema** | Launcher > Ajustes > General | **Protección Térmica y Battery Saver Dinámico**| 🟢 Deseable | Evita calentamiento y estira la batería. |
| **General / Sistema** | Launcher > Ajustes > General | **Actualizador OTA Integrado (GitHub Releases)**| 🟢 Deseable | Actualizar el emulador en 1 toque. |
