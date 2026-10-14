# 🗺️ Hoja de Ruta — Pokémon Reloaded Android (v0.4.0 EX)

> **Documento de Planificación y Arquitectura**  
> **Proyecto:** Pokémon Reloaded para Android (Edición EX)  
> **Autores:** Alexpix0 / Armando  
> **Stack tecnológico:** Rust (OpenGMK Engine) + Kotlin (Jetpack Compose Launcher)

---

## 🎯 Objetivo General
Llevar a **Pokémon Reloaded Android** al estándar definitivo de rendimiento, usabilidad y personalización en la versión **0.4.0 EX**, manteniendo el stack puro en **Rust + Kotlin** (sin añadir lenguajes innecesarios por ahora) y abordando las 4 áreas clave solicitadas:
1. **Rendimiento profundo y autonomía térmica.**
2. **Editor de controles in-game con arrastrar y soltar.**
3. **Modo Fast-Forward (Acelerador experimental 1.5x / 2.0x).**
4. **Packs modulares de música/SoundFonts y sprites (`.gif`) + Telemetría de errores local.**

---

## ⚡ 1. Rendimiento y Eficiencia de Batería (Motor Rust)

### 1.1. JIT / Optimización de Ejecución de Bytecode GML (Cranelift)
* **Problema actual:** OpenGMK interpreta el bytecode de GameMaker 8 instrucción por instrucción en un bucle central (`interpreter loop`). En batallas complejas con múltiples proyectiles, clima y partículas, esto satura un solo núcleo de CPU.
* **Propuesta técnica:**
  * Integrar **Cranelift** (backend de compilación JIT en Rust de bajo peso).
  * Compilar dinámicamente las rutinas críticas de colisiones, físicas de combate e inteligencia artificial a código máquina ARM64 en tiempo de ejecución.
  * Mantener el intérprete estándar como respaldo (fallback) para funciones infrecuentes.
* **Impacto:** Incremento de 2.5x a 4x en fotogramas por segundo mínimos (1% low FPS) y reducción de temperatura en sesiones prolongadas.

### 1.2. Síntesis de Audio Lock-Free de Cero Latencia
* **Problema actual:** Si la lógica del juego tiene una caída de FPS al cargar una ruta o instanciar efectos, el renderizado de audio MIDI (`rustysynth` / `aaudio`) puede sufrir micro-tartamudeos o chasquidos audibles.
* **Propuesta técnica:**
  * Desvincular por completo el sintetizador de sonido del hilo del juego mediante un búfer circular lock-free (`rtrb`).
  * El hilo de audio de AAudio procesa a prioridad de tiempo real sin adquirir mutexes compartidos con la CPU gráfica.
* **Impacto:** Audio 100% fluido y cristalino sin importar lo pesado de la escena en pantalla.

### 1.3. Congelamiento Inteligente en Segundo Plano (Standby 0% Consumo)
* **Problema actual:** Cuando el usuario bloquea la pantalla o cambia de app sin cerrar el juego, el bucle nativo puede continuar consumiendo ciclos innecesarios.
* **Propuesta técnica:**
  * Enlace JNI directo con el ciclo de vida de Android (`onPause` / `onStop` en `GameActivity.kt`).
  * Poner en reposo el hilo de Rust mediante variables de condición (`futex / epoll_wait`), deteniendo el reloj de ticks.
* **Impacto:** 0% de drenaje de batería cuando la app está minimizada.

---

## 🎮 2. Interfaz y Controles: Editor Flotante In-Game (Kotlin + Rust)

### 2.1. Editor Visual Arrastrar y Soltar (Drag & Drop In-Game)
* **Objetivo:** Descartar la necesidad de ajustar sliders a ciegas en menús separados y permitir que el jugador ajuste cada botón exactamente donde caen sus dedos.
* **Características:**
  * **Modo Edición Rápido:** Acceso desde el menú de pausa o botón flotante semitransparente.
  * **Arrastrar libremente:** Mover individualmente cualquier control (`Z`, `X`, `C`, `V`, `SEL`, `OK`, `ESC`, `Diálogo` y `Joystick`).
  * **Pinch-to-zoom (Pellizco táctil):** Escalar el tamaño de botones de manera individual (por ejemplo, hacer el botón de Ataque más grande que el de Diálogo).
  * **Ajuste de opacidad en vivo:** Ver la transparencia sobre el gameplay real.
  * **Guardado directo:** Actualización instantánea en `controls.conf` sin tener que reiniciar la partida.

---

## ⏩ 3. Calidad de Vida (QoL): Modo Fast-Forward (Experimental)

### 3.1. Acelerador de Juego (1.5x / 2.0x)
* **Objetivo:** Agilizar el grindeo de niveles, exploración de rutas largas o cinemáticas repetitivas.
* **Diseño y Pruebas requeridas:**
  * **Multiplicador dinámico de ticks:** Multiplicar el delta-time de GameMaker en Rust preservando la física y las colisiones.
  * **Sincronización de sonido:** Opciones de mutear o mantener el pitch musical sin distorsión tonal durante la aceleración.
  * **Activación dual:** Modo "mantener pulsado" para ráfagas cortas o "conmutar (toggle)" con un toque.
  * **Interruptor de seguridad:** Desactivación automática al entrar en eventos críticos para evitar saltos de scripts.

---

## 📦 4. Escalabilidad: Mods de Assets y Telemetría Local

### 4.1. Packs de Música y Sprites (`.gif`) Externos
* **Objetivo:** Permitir personalizar el juego sin alterar el archivo `.zip` de 700MB ni requerir reinstalaciones.
* **Estructura de carpetas en almacenamiento accesible:**
  ```text
  Android/data/com.klisee.reloaded/files/custom_assets/
  ├── soundfont/
  │   └── soundfont.sf2         (SoundFont personalizado / alternativo)
  ├── bgm/
  │   └── ruta1.ogg             (Música o remixes de reemplazo)
  └── sprites/
      ├── pkmn_front/           (Sprites .gif frontales alternativos)
      └── pkmn_back/            (Sprites .gif traseros)
  ```
* **Lógica del motor:** En cada llamada de carga (`load_sprite`, `load_audio`), Rust busca primero en `custom_assets/`. Si existe el archivo, lo prioriza; de lo contrario, lee el asset por defecto del juego.

### 4.2. Crash Dump & Telemetría Local (Reporte en 1 Toque)
* **Objetivo:** Diagnóstico instantáneo de cierres inesperados sin necesidad de pedir logs complejos por terminal.
* **Funcionamiento:**
  * Interceptor de traps/panics en Rust y captura de excepciones en Kotlin.
  * Si ocurre un error, se abre un diálogo modal con:
    * Nombre del script de GML que causó el fallo.
    * Línea del script y valores de las variables locales en ese instante.
    * Botón *"Copiar reporte"* que genera un texto formateado listo para pegar en GitHub Issues o Discord.

---

## 📋 Fases de Ejecución Propuestas

| Fase | Bloque | Entregables | Estado |
| :---: | :--- | :--- | :---: |
| **Fase 1** | Controles & Standby | • Editor flotante in-game con arrastrar y soltar botones.<br>• Congelamiento del loop en segundo plano (0% standby). | ⏳ Planificado |
| **Fase 2** | Audio & Fast-Forward | • Búfer de audio lock-free desacoplado del frame loop.<br>• Botón y motor de Fast-Forward (1.5x / 2.0x) en pruebas. | ⏳ Planificado |
| **Fase 3** | Mods de Assets & Crash Dump | • Sistema de carga prioritaria para `custom_assets/` (música y gifs).<br>• Reporte de errores modal en 1 solo toque con script GML. | ⏳ Planificado |
| **Fase 4** | Optimización Profunda | • Compilación JIT de rutinas críticas con Cranelift. | ⏳ Planificado |
