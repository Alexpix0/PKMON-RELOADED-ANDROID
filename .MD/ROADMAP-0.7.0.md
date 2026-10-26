# 🌋 ROADMAP v0.7.0: MOTOR VULKAN PURO (`ash`), ENSAMBLADOR AArch64 & JIT CRANELIFT, UPSTREAM v0.3.0 & AJUSTES "JUEGO"

---

## 📑 ÍNDICE GENERAL DEL PROYECTO

1. [Principio Fundamental de Emulación Estricta y Configuración Opcional](#-1-principio-fundamental-de-emulación-estricta-y-configuración-opcional)
2. [Arquitectura del Motor Gráfico: Vulkan Nativo Puro (`ash`) & Dual Backend](#-2-arquitectura-del-motor-gráfico-vulkan-nativo-puro-ash--dual-backend)
3. [Núcleo de Ultra-Rendimiento: Ensamblador ARM64 Puro & JIT Cranelift](#-3-núcleo-de-ultra-rendimiento-ensamblador-arm64-puro--jit-cranelift)
4. [Sincronización Total con Upstream v0.3.0 (Klisee)](#-4-sincronización-total-con-upstream-v030-klisee)
5. [Nuevo Apartado de Ajustes: "Juego" (Comodidad Externa y Opcional)](#-5-nuevo-apartado-de-ajustes-juego-comodidad-externa-y-opcional)
6. [Los 11 Pilares Técnicos y de Experiencia de Usuario](#-6-los-11-pilares-técnicos-y-de-experiencia-de-usuario)
   * [Pilar I: Micro-Arquitectura y Memoria en Rust del Motor OpenGMK](#-pilar-i-micro-arquitectura-y-memoria-en-rust-del-motor-opengmk)
   * [Pilar II: Gráficos de Próxima Generación, Bindless y 120 FPS](#-pilar-ii-gráficos-de-próxima-generación-bindless-y-120-fps)
   * [Pilar III: Controles, Mandos Físicos y Latencia Ultra-Baja](#-pilar-iii-controles-mandos-físicos-y-latencia-ultra-baja)
   * [Pilar IV: Experiencia de la App Android, Dashboard y Multitarea](#-pilar-iv-experiencia-de-la-app-android-dashboard-y-multitarea)
   * [Pilar V: Suite Multimedia, Grabación y Repetición Instantánea](#-pilar-v-suite-multimedia-grabación-y-repetición-instantánea)
   * [Pilar VI: Acústica Móvil, Audio DSP y Sonido Espacial 3D](#-pilar-vi-acústica-móvil-audio-dsp-y-sonido-espacial-3d)
   * [Pilar VII: Eficiencia Energética, Batería y Control Térmico](#-pilar-vii-eficiencia-energética-batería-y-control-térmico)
   * [Pilar VIII: Modo Ultra Low-End para Dispositivos de Entrada (2 GB)](#-pilar-viii-modo-ultra-low-end-para-dispositivos-de-entrada-2-gb)
   * [Pilar IX: Cuaderno de Aventura, Notas y Libreta del Entrenador](#-pilar-ix-cuaderno-de-aventura-notas-y-libreta-del-entrenador)
   * [Pilar X: Seguridad de Partidas, Nube y Transferencia Offline](#-pilar-x-seguridad-de-partidas-nube-y-transferencia-offline)
   * [Pilar XI: Adaptabilidad a Formatos, Pantallas Plegables y Accesibilidad](#-pilar-xi-adaptabilidad-a-formatos-pantallas-plegables-y-accesibilidad)
7. [Registro Oficial de Exclusiones Definitivas (Lista Negra)](#-7-registro-oficial-de-exclusiones-definitivas-lista-negra)
8. [Cronograma de Ejecución por Fases (Plan de Desarrollo)](#-8-cronograma-de-ejecución-por-fases-plan-de-desarrollo)
9. [Resumen Consolidado de Entregables](#-9-resumen-consolidado-de-entregables)

---

## ⚖️ 1. Principio Fundamental de Emulación Estricta y Configuración Opcional

> ### 🛑 REGLA INMUTABLE DEL PROYECTO
> **OpenGMK es un emulador en Rust que ejecuta el juego original de GameMaker 8 (`pokemon-reloaded.exe`). NO somos un ROM-hack, NO modificamos el código fuente de Pokémon Reloaded ni alteramos las reglas creadas por F3R.**
>
> 1. **Cero invasión del juego base:** No se hackean tablas internas, no se tocan las cajas del PC, no se alteran mecánicas de combate, ni se modifican los sistemas de crianza o guardería.
> 2. **100% Externo al juego:** Toda mejora de comodidad o visual vive estrictamente en la capa externa: el overlay táctil de Android, los temporizadores del emulador en Rust, los controladores de hardware o los shaders de la GPU.
> 3. **Nada viene impuesto de fábrica:** Ninguna función de comodidad viene activa por defecto obligatoriamente. Se agrupan como **interruptores opcionales (Switch ON/OFF)** dentro del Launcher en el nuevo apartado de Ajustes llamado **"Juego"**. Si el jugador los apaga, el juego corre en su estado 100% puro y original.

---

## 🏛️ 2. Arquitectura del Motor Gráfico: Vulkan Nativo Puro (`ash`) & Dual Backend

El núcleo visual de la versión `v0.7.0` incorpora un backend nativo en **Vulkan** mediante enlaces directos de bajo nivel de Rust (`ash`), interactuando de forma limpia con `libvulkan.so` de Android NDK y coexistiendo con el motor **OpenGL ES** actual mediante detección y conmutación automática de respaldo (*auto-fallback*):

```
src/OpenGMK/gm8emulator/src/render/
├── mod.rs                  <-- Selector: enum GraphicsBackend { OpenGl, Vulkan }
├── opengl.rs               <-- Backend OpenGL ES actual (mantenido como reserva y fallback universal)
└── vulkan/                 <-- NUEVO MÓDULO NATIVO VULKAN PURO (ash)
    ├── mod.rs              <-- Implementación de RendererTrait para Vulkan
    ├── context.rs          <-- Carga dinámica de libvulkan.so, VkInstance, VkPhysicalDevice, VkDevice y VkQueue
    ├── surface.rs          <-- Extensión VK_KHR_android_surface vinculada al ANativeWindow de Android
    ├── swapchain.rs        <-- Gestión de VkSwapchainKHR (Mailbox/Fifo, recreación segura en giros y pausas)
    ├── memory.rs           <-- Asignador GPU contiguo (Staging Buffers, memoria de vértices e índices)
    ├── textures.rs         <-- Subida de atlas de GameMaker a VkImage, descriptores y samplers
    ├── pipeline.rs         <-- VkPipelineLayout, estados de mezcla (Blend Modes) y pipelines gráficos 2D
    ├── render_pass.rs      <-- Render Pass principal y gestión de superficies de renderizado offscreen
    ├── command.rs          <-- VkCommandPool y buffers de comandos preasignados
    └── shaders/            <-- Binarios SPIR-V precompilados (.spv) de ultra-baja latencia
        ├── sprite.vert.spv / sprite.frag.spv
        └── shape.vert.spv  / shape.frag.spv
```

### 🎛️ Selector Dual en los Ajustes del Launcher:
* **OpenGL ES (Estable / Universal):** Compatibilidad 100% garantizada en cualquier dispositivo histórico, chips antiguos PowerVR o arquitecturas Mali-T.
* **Vulkan Nativo (`ash`):** Máximo rendimiento, envío directo de Command Buffers sin hilo de contexto EGL, shaders binarios SPIR-V y aprovechamiento del renderizado por baldosas (GMEM) en Snapdragon (Adreno) y MediaTek (Mali).
* **Resiliencia Automática (Auto-Fallback):** Si el dispositivo no cuenta con controladores Vulkan compatibles o falla la inicialización de la superficie, OpenGMK conmuta de inmediato a OpenGL ES sin cierres forzados ni pantallas negras.

---

## ⚡ 3. Núcleo de Ultra-Rendimiento: Ensamblador ARM64 Puro & JIT Cranelift

Representa el salto tecnológico más potente en la historia de la emulación de Pokémon Reloaded, combinando código máquina nativo y paralelismo masivo:

1. **🦾 Bucle del Intérprete en Ensamblador Puro ARM64 (`AArch64 Assembly`):**
   * Archivo: `src/OpenGMK/gm8emulator/src/gml/asm/dispatch_aarch64.s`
   * Bucle central de despacho de instrucciones (`interpreter_dispatch_loop`) escrito a mano en ensamblador AArch64.
   * **Asignación de registros físicos dedicados en silicio:**
     * `x19`: Puntero permanente a la pila de evaluación GML.
     * `x20`: Puntero base a la tabla de saltos directos de opcodes.
     * `x21`: Puntero de instrucción (`PC` - Program Counter).
   * **Resultado:** Elimina por completo los fallos de predicción de saltos (*branch misprediction*) del procesador ARM móvil, despachando instrucciones a velocidad máxima de hardware.
2. **🧬 Compilador Dinámico JIT en Tiempo Real (Cranelift en Rust):**
   * Módulo: `src/OpenGMK/gm8emulator/src/gml/jit/`
   * Los bucles matemáticos pesados de daño, cálculo de IA y colisiones se compilan al vuelo a código máquina ARM64 en memoria ejecutable (`PROT_READ | PROT_EXEC`).
   * **Resultado:** Aceleración de cómputo entre **3x y 5x**. Cero caídas de FPS en combates masivos o con clima y partículas simultáneas.
3. **🌌 Texturas Bindless y 1 Single Draw Call por Frame (`VK_EXT_descriptor_indexing`):**
   * Todos los atlas de sprites del juego se vinculan en un megagrupo indexado de descriptores en la GPU.
   * **Resultado:** El emulador dibuja todo el escenario, los Pokémon, proyectiles, interfaz y efectos en **1 sola llamada de dibujo (Single Draw Call)** a la GPU por frame.
4. **🧈 Extrapolación e Interpolación de Movimiento a 120 FPS Reales:**
   * Un interpolador matemático en Rust analiza los vectores de movimiento `(x, y, hspeed, vspeed)` de las instancias visibles y calcula posiciones intermedias entre ticks de físicas de 60 Hz.
   * **Resultado:** Fluidez absoluta a 90 Hz o 120 Hz en pantallas AMOLED sin acelerar la velocidad del juego.
5. **⚡ Mapeo de Memoria Virtual Zero-Copy (`mmap`):**
   * Proyección directa de los archivos del juego en el espacio de memoria virtual con la llamada de Linux `mmap`.
   * **Resultado:** Cero copias de datos en RAM. Arranque en menos de 1 segundo y transiciones de puertas y salas instantáneas (0 ms).
6. **🎯 Escalado Espacial AMD FSR 1.0 Mobile en GPU Compute Shader:**
   * Algoritmo de reconstrucción espacial *AMD FidelityFX Super Resolution* como Compute Shader en Vulkan.
   * **Resultado:** Imagen nítida en pantallas 1080p y 2K con reconstrucción de bordes a una fracción del consumo de batería de filtros pesados como xBRZ.

---

## 🔄 4. Sincronización Total con Upstream v0.3.0 (Klisee)

A partir del código fuente oficial provisto (`pokemon-reloaded-android-0.3.0.zip`), se incorporan y armonizan los cuatro grandes bloques oficiales:

1. **🎮 Intro y Selección de Personaje Adaptada al Táctil:**
   * Contexto `CHARACTER` en `touch_controls.rs`.
   * Detección dinámica de cajas delimitadoras (`PICKER_BOXES`) para seleccionar personaje tocando directamente sobre su sprite.
   * Flechas contextuales dinámicas a los lados (`flechaderecha.png`, `flechaizquierda.png`) y botones de confirmación (`eleccion_ok.png`, `eleccion_atras.png`).
2. **⌨️ Teclado Nativo de Android (Input Method Framework - IME):**
   * Anclaje transparente `imeAnchor` de 1x1 píxel en `GameActivity.kt` con conexión `ForwardingInputConnection`.
   * Apertura automática del teclado de Android al escribir motes, nombres en PC, nombre de entrenador o trucos (`truco_ctrl_p.png`).
   * Detección reactiva de altura (`IME_INSET`) mediante `WindowInsetsCompat.Type.ime()`, elevando la ventana para que el texto editado nunca quede oculto.
   * Auto-ocultación del overlay táctil mientras el teclado virtual esté desplegado.
3. **🛡️ Corrección Crítica del Crash por "Desobediencia por Nivel":**
   * En `gml/runtime.rs`: Al consultar variables de instancias desvinculadas o destruidas (`Target::Single(None)`), se retorna de forma segura `Ok(Default::default())` (valor 0, comportamiento fiel a GameMaker 8), eliminando el crash `UninitializedVariable` en combates (ej. sala 191).
4. **🧭 Restauración del Botón de Pokébola en la Zona Safari:**
   * En `touch_controls.rs`: Nuevo contexto `SAFARI` que garantiza la presencia del botón `B` (asignado a lanzar Pokébolas en la Zona Safari) junto al joystick y botones de mochila y menú.

---

## ⚙️ 5. Nuevo Apartado de Ajustes: "Juego" (Comodidad Externa y Opcional)

Todas estas opciones son **100% externas al juego**, funcionan mediante el emulador y el overlay, y se presentan en el Launcher bajo la sección **Ajustes ➔ Juego** con interruptores individuales (Switch ON/OFF):

1. **🍇 Reloj de Cosecha de Bayas (Berry Harvest Tracker):**
   * *Mecánica del juego:* En Pokémon Reloaded las bayas se regeneran cada **10 minutos de tiempo activo de juego**.
   * *En el emulador:* OpenGMK acumula los 10 minutos de tiempo activo de juego y despliega un aviso HUD no invasivo: *"🌱 ¡Ciclo de bayas completado (10 min)!"*.
2. **🚴 Cronómetro de Movimiento para Huevos (Egg Hatch Movement Timer):**
   * *Mecánica del juego:* Reloaded no cuenta pasos por cuadrícula, sino el **tiempo y fotogramas en que el personaje se mantiene en movimiento activo** (pedaleando o corriendo).
   * *En el emulador:* El emulador mide los segundos acumulados mientras mantienes accionado el D-Pad/Joystick para saber el avance exacto de eclosión.
3. **🧭 Rueda Radial de 4 Ranuras para Objetos Clave Favoritos:**
   * Atajos táctiles directos en la rueda radial para usar la Bici, Zahorí, Cañas u otros objetos frecuentes sin navegar manualmente por la mochila.
4. **🏃 Modo Auto-Run / Correr Permanente:**
   * Interruptor en el overlay para mantener virtualmente presionado el botón de carrera y evitar la fatiga del pulgar.
5. **💬 Avance de Diálogos por Toque en Pantalla Completa:**
   * Tocar cualquier punto de la mitad inferior de la pantalla envía la tecla de confirmación (`Z`/Enter) para pasar textos cómodamente como en una novela visual.
6. **🎒 Auto-Recordatorio de Repelente:**
   * Al agotarse el repelente, el overlay muestra un botón directo de 1 toque para reabrir la mochila y reaplicarlo al instante.
7. **✨ Alerta Visual Perimetral de Pokémon Variocolor (Shiny Beacon):**
   * Inspección pasiva del buffer de entidades. Si aparece un variocolor en el combate, el emulador emite un sutil destello dorado en el borde de la pantalla de Android para no huir por error.
8. **🧩 Pausa Temporal de Encuentros en Salas de Puzles:**
   * Opción en la rueda radial para silenciar temporalmente los encuentros salvajes mientras se resuelven puzles de fuerza o hielo.
9. **🎣 Asistente Háptico de Picada de Pesca:**
   * Pulso de vibración en el motor del teléfono en el instante exacto en que aparece el signo de exclamación al pescar.
10. **💾 Guardado Rápido Externo en 1 Toque (Quick Save / Quick Load):**
    * Atajo flotante o en la rueda radial que vuelca una copia instantánea del estado de la partida en el almacenamiento de la app.
11. **⏰ Recordatorio de Guardado Preventivo:**
    * Aviso periódico configurable (ej. cada 30 o 45 minutos) para recordar guardar la partida antes de batallas importantes.

---

## 🚀 6. Los 11 Pilares Técnicos y de Experiencia de Usuario

### ⚙️ Pilar I: Micro-Arquitectura y Memoria en Rust del Motor OpenGMK
1. **Asignador de Memoria de Arena por Habitación (Room Arena Allocator):** La memoria de entidades de cada sala se aloja en un bloque contiguo y se libera en 1 sola operación atómica al cambiar de sala, eliminando la fragmentación de RAM.
2. **Tablas Trigonométricas Precalculadas (Fast Sin/Cos Lookup):** Tablas de búsqueda en caché de CPU para proyectiles y trayectorias en combates en tiempo real, ahorrando ciclos matemáticos pesados.
3. **Piscina de Objetos de Memoria (Object Pooling):** Reutilización de estructuras contiguas para proyectiles y partículas en lugar de llamar a asignaciones dinámicas repetitivas.
4. **Aceleración SIMD NEON para Colisiones Bounding-Box:** Evaluación paralela de 4 colisiones en una sola instrucción vectorial de 128 bits en CPUs ARM64.
5. **Carga Asíncrona de Salas y Purgado LRU de Texturas:** Precarga de la siguiente habitación en segundo plano durante fundidos a negro y descarga de texturas inactivas de la GPU para mantener el uso de RAM por debajo de 200 MB.

### 🎨 Pilar II: Gráficos de Próxima Generación, Bindless y 120 FPS
1. **Shaders Post-Procesado Retro:** Filtros visuales opcionales en GPU (CRT Trinitron, scanlines, matriz LCD y colores Game Boy Color).
2. **Escalado de Alta Definición xBRZ 2x/4x:** Suavizado de curvas pixel-art para pantallas 1080p, 2K y tablets.
3. **Biseles Temáticos Dinámicos (Dynamic Bezel Art):** Fondos decorativos elegantes para rellenar las franjas negras laterales en teléfonos modernos 20:9.
4. **Modo Oscuro Puro AMOLED (`#000000`):** Máximo contraste y ahorro de energía en todas las pantallas del Launcher y ajustes.
5. **Sincronización Adaptativa de Cuadros:** Eliminación total del Screen Tearing vinculando la entrega de fotogramas con el compositor `SurfaceFlinger` de Android.

### 🕹️ Pilar III: Controles, Mandos Físicos y Latencia Ultra-Baja
1. **Soporte Plug & Play para Mandos Físicos:** Detección instantánea de mandos USB-C y Bluetooth (Xbox, PlayStation, Razer Kishi, GameSir) con auto-ocultación total de botones táctiles.
2. **Muestreo Táctil a Frecuencia Extrema (Touch Polling a 240 Hz / 360 Hz):** Hilo NDK que lee eventos táctiles a la máxima tasa del digitalizador, reduciendo la latencia de respuesta a menos de 4 ms.
3. **Escudo Protector de Gestos del Sistema (Edge Gesture Shield):** Bloqueo temporal de gestos en los bordes de Android (volver atrás, cambiar de app) mientras los dedos operan sobre los controles.
4. **Botones Físicos de Volumen como Gatillos L y R:** Mapeo de teclas de volumen del teléfono como botones de hombro físicos.
5. **Vibración Háptica Progresiva y Alerta de Límite (Boundary Haptic Pop):** Pulso háptico diferenciado y aviso táctil cuando el joystick virtual alcanza la máxima inclinación.
6. **Soporte para Teclados Físicos y Stick Derecho como Ratón:** Mapeo completo de teclado USB/BT y uso del stick analógico derecho para mover el cursor en menús.

### 📱 Pilar IV: Experiencia de la App Android, Dashboard y Multitarea
1. **Dashboard de Estadísticas de Sesión y Tiempo Total:** Muestra horas jugadas totales, sesiones abiertas, hora de última partida y gráfica semanal de uso en el Launcher con estética Material You.
2. **Modo Ventana Flotante / Picture-in-Picture (PiP):** Minimizar el emulador a una ventana flotante interactiva para seguir viendo el juego mientras revisas una guía en Chrome o respondes mensajes.
3. **Galería Integrada de Capturas de Pantalla con Editor Rápido:** Pestaña en el Launcher donde se guardan automáticamente las capturas tomadas durante el juego con recorte, marcos temáticos y compartir directo.
4. **Cajón de Atajos Rápidos en el Icono de la App (Android App Shortcuts):** Mantener presionado el icono de la app en la pantalla de inicio para lanzar directamente: "Continuar Partida", "Ajustes de Juego" o "Configuración de Controles".
5. **Visor Ligero de Guías y Manuales Integrado (Companion Docs):** Visor en el launcher para consultar el manual oficial de Pokémon Reloaded o tablas de tipos sin salir de la app.

### 🎬 Pilar V: Suite Multimedia, Grabación y Repetición Instantánea
1. **Búfer de Repetición Instantánea "ShadowPlay" (Últimos 30 Segundos):** Grabar en un ring-buffer en RAM los últimos 30 segundos de gameplay a 60 FPS con codificación por hardware (`MediaCodec`) para exportar momentos épicos en MP4 al instante.
2. **Modo Foto con Congelado de Cuadro y Ocultación de HUD:** Atajo en la rueda radial que congela el frame del emulador y oculta instantáneamente todos los botones táctiles para tomar la captura limpia perfecta en resolución nativa.
3. **Generador de GIFs Animados de 5 Segundos:** Captura rápida de 5 segundos de animación comprimida directamente a un GIF animado ligero optimizado para compartir en redes.
4. **Grabador de Audio de Banda Sonora Limpia (Direct BGM Ripper):** Volcado directo del flujo de audio digital del sintetizador a archivos WAV o FLAC estéreo sin ruidos externos.
5. **Marca de Agua / Watermark Personalizable:** Opción en ajustes para colocar tu nombre de usuario o un logo PNG transparente en una esquina del render para proteger tus capturas y gameplays.

### 🎧 Pilar VI: Acústica Móvil, Audio DSP y Sonido Espacial 3D
1. **Silenciado Inmediato al Desconectar Auriculares:** Pausa o silenciado instantáneo del emulador si se desconectan los auriculares Bluetooth o cableados.
2. **Control de Volumen Independiente para Música y Efectos:** Deslizadores en el menú radial del emulador para nivelar la música de fondo y los efectos de sonido por separado.
3. **Efecto de Audio Espacial Virtual 3D en Auriculares:** Algoritmo HRTF ligero que procesa el sonido estéreo para dar sensación de dirección espacial a los ataques y pasos en combate.
4. **Ecualizador DSP de 3 Bandas Móvil y Potenciador de Gritos:** Filtro enfocado en resaltar graves en altavoces móviles y realzar los gritos de Pokémon en la banda media (1 kHz - 3 kHz).
5. **Compresor Nocturno (Night Sound Leveler) y Mute en Rueda Radial:** Nivelación de sonidos fuertes para jugar de noche sin sobresaltos y botón de silenciado rápido en 1 toque.

### 🔋 Pilar VII: Eficiencia Energética, Batería y Control Térmico
1. **Suspensión Limpia en Segundo Plano (Zero-Wake Lock):** Congelación total de hilos de cómputo y audio al minimizar la app (0% drenaje de batería en reposo).
2. **Tasa de Refresco Adaptativa según Estado:** 120 Hz en combates en tiempo real donde los reflejos son críticos, y 60 Hz al caminar por rutas o leer texto, ahorrando hasta un 35% de energía.
3. **Protección Térmica Preventiva (GPU Duty Cycle Throttling):** Detección de temperatura de batería superior a 41-43 °C con intercalado sutil de cuadros de presentación para enfriar el dispositivo sin alterar las físicas.
4. **Control de Brillo Rápido por Gesto:** Deslizar el dedo verticalmente por el borde izquierdo de la pantalla para regular el brillo del juego sin desplegar la barra de Android.
5. **Modo Lectura Nocturna (Night Screen Dimmer):** Filtro de software que reduce la luz azul y atenúa el brillo por debajo del mínimo de fábrica del teléfono.

### ⚡ Pilar VIII: Modo Ultra Low-End para Dispositivos de Entrada (2 GB)
1. **Perfil Ultra Low-End (1 Clic para Teléfonos de 2 GB de RAM):** Desactiva partículas secundarias y ajusta el renderizado a resolución 0.5x con escalado bilineal para asegurar 60 FPS estables en teléfonos muy modestos.
2. **Priorización de Proceso de CPU (`Thread.MAX_PRIORITY`):** El launcher solicita a Android la máxima prioridad de CPU para el hilo de emulación, eliminando micro-tirones causados por procesos del sistema.
3. **Supresión Dinámica de Fondos Estáticos:** Desactivar capas decorativas lejanas en ciudades pesadas cuando la tasa de cuadros caiga por debajo de 45 FPS.
4. **Modo Pantalla Monocromática para Ahorro Extremo:** Shader que convierte la imagen a blanco y negro con pantalla AMOLED cuando la batería esté al 5% para terminar un combate sin que el móvil se apague.
5. **Temporizador de Suspensión por Inactividad (Sleep Timer):** Si el juego queda desatendido durante más de 10 minutos sin toques en pantalla, el emulador realiza un guardado de seguridad y suspende el proceso para no drenar la batería.

### 📓 Pilar IX: Cuaderno de Aventura, Notas y Libreta del Entrenador
1. **Diario de Aventura Personalizable (Trainer Journal):** Libreta de notas en el launcher donde puedes anotar pistas de NPCs, zonas pendientes por explorar con surf/cascada o Pokémon avistados.
2. **Lista de Metas y Tareas Personales (Quest Checklist):** Pestaña donde el jugador puede escribir y marcar sus propios objetivos autogestionados (ej. "Entrenar a Lucario a nivel 50", "Conseguir la 5ta Medalla").
3. **Calculadora de Tipos y Debilidades Integrada (Type Matchup Companion):** Herramienta de consulta rápida en el launcher para revisar fortalezas y debilidades elementales de tipos simples y duales sin salir de la app.
4. **Contador de Combates y Victorias de la Sesión:** Registro de estadísticas externas que muestra cuántas batallas has disputado y ganado en tu sesión de juego.
5. **Visor de Mapas de Rutas en Alta Definición:** Galería interactiva con mapas generales de la región para orientarse y planificar rutas de viaje.

### 💾 Pilar X: Seguridad de Partidas, Nube y Transferencia Offline
1. **Transferencia Directa de Partida por Código QR / Wi-Fi Direct:** Pasar tu archivo de guardado de un teléfono a otro en 3 segundos mediante código QR o conexión local directa sin necesidad de internet.
2. **Respaldo Automático en Tarjeta MicroSD y Nube (Drive / WebDAV):** Clonación automática de seguridad en la tarjeta SD extraíble y sincronización opcional con Google Drive.
3. **Historial de Versiones de Guardado (Rollback Saves de 3 Ranuras):** Conservación automática en disco de los últimos 3 guardados anteriores para recuperarse ante cualquier error fortuito.
4. **Exportador a Formato de PC (.sav) e Importador desde Downloads:** Conversión en 1 clic para jugar indistintamente en Android y PC, con escaneo automático de la carpeta `Downloads`.
5. **Bloqueo de Partida con Contraseña Local:** Protección de tus ranuras de guardado con PIN o huella digital de Android para evitar que terceros borren tu progreso al prestar el móvil.

### 📱 Pilar XI: Adaptabilidad a Formatos, Pantallas Plegables y Accesibilidad
1. **Perfiles de Hardware para Snapdragon (Adreno) y MediaTek (Mali):** Ajuste de baldosas y parámetros internos según el chip gráfico detectado.
2. **Ajuste Automático de Notch y Agujero de Cámara (Safe Area Padding):** Desplazamiento inteligente de los botones del overlay para evitar ser cubiertos por cámaras frontales o esquinas redondeadas.
3. **Soporte para Teléfonos Plegables y Tablets (Foldable / Dual-Screen):** Disposición adaptativa con pantalla arriba y controles ergonómicos en la mitad inferior.
4. **Shaders de Corrección para Daltonismo en GPU (Protanopía, Deuteranopía y Tritanopía):** Recálculo de la matriz de color RGB para distinguir claramente tipos elementales y barras de salud.
5. **Lupa Dinámica de Lectura (Pinch-to-Zoom), Inversión para Zurdos y Modo Sticky Keys:** Herramientas completas para lectura cómoda y reducción de fatiga motriz.

---

## 🚫 7. Registro Oficial de Exclusiones Definitivas (Lista Negra)

Las siguientes funciones quedan **100% excluidas de forma definitiva** del desarrollo por vulnerar el límite de emulación, invadir la lógica interna de GameMaker 8 o por descarte explícito:

| Característica Rechazada | Motivo de Inviabilidad Técnica / Descarte |
|---|---|
| ❌ **TODO EL ÁMBITO DE ASISTENTES DE CAMPO (Dowsing, Clima, Safari, Sigilo, Baya en sala)** | Descartado explícitamente por el usuario. |
| ❌ **TODO EL ÁMBITO DE COMBATE (Lock-on, Heartbeat vida crítica, Cambio rápido, Dirección huida, Turbo turnos)** | Descartado explícitamente por el usuario. |
| ❌ **Atajo Rápido de Curación de Emergencia (Pociones/Revivir)** | Descartado explícitamente por el usuario. |
| ❌ **Selector Rápido de Caña de Pescar en Rueda Radial** | Descartado explícitamente por el usuario. |
| ❌ **Historial de Gastos en Tiendas Pokémon** | Descartado explícitamente por el usuario. |
| ❌ **Aviso Preventivo de Mochila al Límite** | Descartado explícitamente por el usuario. |
| ❌ **Recordador de Movimientos Portátil en 1 Toque** | Exige reprogramar el sistema de movimientos del juego original. |
| ❌ **Indicador de Felicidad y Amistad en el Resumen** | Modifica la interfaz gráfica interna y sprites de datos de GameMaker. |
| ❌ **Selección Múltiple y Liberación Masiva en Cajas del PC** | Altera la lógica interna del sistema de almacenamiento de F3R. |
| ❌ **Comparador Dinámico de Ataques** | Obliga a rediseñar menús nativos del juego. |
| ❌ **Filtro y Buscador Rápido en la Mochila** | Invasión del sistema de inventario interno de GameMaker. |
| ❌ **Indicador de Efecto Secundario en Combate** | Altera los cuadros de diálogo y el motor de batalla de Reloaded. |
| ❌ **Contador de PP Restantes en Botones del HUD** | Invasión de las mecánicas internas de combate de GameMaker. |
| ❌ **MOs Sin Necesidad de Esclavos de MO** | Modifica de raíz las reglas de viaje y campo establecidas por el creador. |
| ❌ **Iluminación Adaptativa en Cuevas Oscuras** | Altera el diseño de niveles y el mapa de luces original de las salas. |
| ❌ **Mini-Radar Translúcido en HUD** | Exige mapear y parsear las coordenadas de todas las salas de GameMaker. |
| ❌ **Detector de Compatibilidad en Guardería** | Invade los algoritmos de crianza y variables internas de guardería. |
| ❌ **Alerta Flotante de Huevo Listo en Guardería** | Invade las rutinas internas de la guardería de Pokémon Reloaded. |
| ❌ **Guía de Herencia Genética (Everstone / Destiny Knot)** | Altera el sistema genético interno de GameMaker 8. |
| ❌ **TODO EL ANTIGUO ÁMBITO 6: Modos de Desafío (Nuzlocke, Monotype, etc.)** | Requiere crear reglas de juego inexistentes en el código original. |
| ❌ **TODO EL ANTIGUO ÁMBITO 7: Conectividad Local P2P / Trade Inalámbrico** | GameMaker 8 no posee arquitectura de red para Pokémon Reloaded. |
| ❌ **TODO EL ANTIGUO ÁMBITO 8: Redes Sociales / Exportación Showdown / Card PNG** | Invade estructuras de datos de Pokémon y exportación externa compleja. |
| ❌ **TODO EL ANTIGUO ÁMBITO 11: Herramientas de Integridad y Rescate (Unstuck Tool)** | Manipula coordenadas de colisión internas del motor de F3R. |

---

## 📅 8. Cronograma de Ejecución por Fases (Plan de Desarrollo)

```
┌────────────────────────────────────────────────────────────────────────┐
│                        CRONOGRAMA DE EJECUCIÓN                         │
├─────────┬──────────────────────────────────────────────────────────────┤
│ FASE 1  │ Fusión y Sincronización Upstream v0.3.0 (Klisee)             │
│         │ • Intro táctil (PICKER_BOXES) y botones contextuales         │
│         │ • Teclado Android IME con anclaje transparente e IME_INSET   │
│         │ • Fix desobediencia por nivel y botón Pokébola Zona Safari   │
├─────────┼──────────────────────────────────────────────────────────────┤
│ FASE 2  │ Núcleo de Ultra-Rendimiento (Ensamblador ARM64 & Cranelift)  │
│         │ • dispatch_aarch64.s con registros dedicados x19, x20, x21   │
│         │ • Integración de Cranelift JIT para bucles matemáticos GML   │
│         │ • Mapeo de memoria virtual Zero-Copy (mmap)                  │
├─────────┼──────────────────────────────────────────────────────────────┤
│ FASE 3  │ Motor Gráfico Vulkan Puro (ash) & Texturas Bindless          │
│         │ • render/vulkan/ (context, surface, swapchain, pipeline)     │
│         │ • VK_EXT_descriptor_indexing para 1 Single Draw Call         │
│         │ • Extrapolación e interpolación a 120 FPS y AMD FSR 1.0      │
│         │ • Selector dual con fallback dinámico a OpenGL ES            │
├─────────┼──────────────────────────────────────────────────────────────┤
│ FASE 4  │ Apartado de Ajustes "Juego" en el Launcher (Kotlin Compose)  │
│         │ • Interruptores para Bayas (10 min) y Huevos (pedaleo)      │
│         │ • Rueda radial de 4 ranuras de objetos clave y Auto-Run      │
│         │ • Avance de diálogo por toque, Shiny Beacon y alerta repelente│
├─────────┼──────────────────────────────────────────────────────────────┤
│ FASE 5  │ Suite Android, Ergonomía, Multimedia y Pruebas CI/CD         │
│         │ • Picture-in-Picture (PiP), ShadowPlay y Touch Polling 240Hz │
│         │ • Compilación y verificación final de APK release en CI/CD   │
└─────────┴──────────────────────────────────────────────────────────────┘
```

---

## 🏆 9. Resumen Consolidado de Entregables

1. **⚡ Núcleo de CPU:** Bucle de intérprete en **Ensamblador Puro ARM64 (`AArch64`)** con registros fijos (`x19`, `x20`, `x21`), **Cranelift JIT** (3x-5x aceleración) y **Zero-Copy `mmap`** (arranque 0 ms).
2. **🌋 Motor Gráfico Dual:** **Vulkan Puro (`ash`)** con **Texturas Bindless** (1 Single Draw Call por frame), **120 FPS Smoothing**, **AMD FSR 1.0 Mobile Compute Shader**, y fallback automático a **OpenGL ES**.
3. **🔄 Upstream v0.3.0:** Intro táctil (`PICKER_BOXES`), teclado Android IME reactivo (`IME_INSET`), fix de desobediencia y botón de Zona Safari.
4. **🎮 Ajustes "Juego" (Launcher):** Reloj de bayas (10 min), cronómetro de pedaleo para huevos, rueda radial de 4 ranuras, auto-run, avance de diálogos por toque, shiny beacon y alerta de repelente.
5. **📱 Plataforma Móvil:** Modo ventana flotante (PiP), grabación ShadowPlay (30s), touch polling a 240/360 Hz, escaneo QR de partidas offline, perfiles para Snapdragon y MediaTek, y perfil Ultra Low-End para 2 GB de RAM.
