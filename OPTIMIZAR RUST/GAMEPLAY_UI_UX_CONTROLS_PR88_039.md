# ARQUITECTURA TÉCNICA E INTEGRACIÓN: CONTROLES TÁCTILES, UI/UX Y HÁPTICA (PR #88 & PR #2)

> **Fuentes Analizadas:**
> - [Klisee/pokemon-reloaded-android PR #88](https://github.com/Klisee/pokemon-reloaded-android/pull/88)
> - [lionelmendozaparsons020-lab/pokemon-reloaded-android PR #2](https://github.com/lionelmendozaparsons020-lab/pokemon-reloaded-android/pull/2)
> - Rama base: `0.3.7` -> `0.3.9-controles`

---

## 1. Resumen Ejecutivo de la Solución

Esta línea de desarrollo resuelve los 4 problemas críticos de jugabilidad y UX táctil en Pokémon Reloaded Android:
1. **Softlock y navegación en menús:** Erradica bloqueos en PC, Mochila y pantalla de Datos/Movimientos inyectando navegación con joystick y atajos `Z`/`X`.
2. **Caja Pokémon 100% táctil:** Elimina la necesidad de botones flotantes para mover Pokémon; implementa *tap-to-pickup*, *free-drag* y *tap-to-place*, reservando el clic secundario (menú de datos) a una pulsación fija de 1.0 segundo.
3. **Joystick analógico a 8 direcciones sin traqueteo (*chatter*):** Histéresis radial (activación al 10%, liberación al 7%) e histéresis angular (5°) con resorte visual amortiguado (*critically damped spring*).
4. **Háptica directa en NativeActivity:** Soluciona el fallo silencioso de `performHapticFeedback()` en `NativeActivity` disparando pulsos directos de `VibrationEffect` con coalescencia de 25 ms.
5. **Botón 8 y Estilo Moderno V2:** Expansión del mapa de teclas con tecla 8, rueda SEL de 7 centros y tercer tema visual persistente.

---

## 2. Desglose de Módulos Técnicos (Rust & Kotlin)

### MÓDULO A: `src/OpenGMK/gm8emulator/src/box_touch.rs` (Caja Pokémon y PC)
* **Objetivo:** Control del cursor del ratón virtual sin emular pulsación continua en pantalla.
* **Constantes Clave:**
  - `LEFT = 0`, `RIGHT = 2` (mapeo del backend ramen).
  - `HOLD = 1000 ms` (tiempo estacionario para clic secundario / datos).
  - `HOVER = 45 ms`, `CLICK = 70 ms`.
  - `slop = (height * 0.015).clamp(8.0, 24.0)` (zona muerta de arrastre proporcional a la altura de la pantalla).
* **Máquina de Estados (`Phase`):**
  ```rust
  enum Phase {
      Waiting,
      LeftPending { due: Instant, point: Point, finish: bool },
      LeftDown { at: Instant, point: Point, finish: bool },
      Hovering { last: Point, tap_places: bool },
      RightDown(Instant),
      Consumed,
  }
  ```
* **Lógica Operativa:**
  - **Recoger Pokémon:** Un toque corto emite `LeftDown` seguido de `LeftUp` tras 70 ms. El Pokémon queda "levantado".
  - **Mover:** Mientras `carrying == true`, los eventos táctiles generan `Event::Move(Point)`. No hay botón izquierdo sostenido (`hovering`).
  - **Soltar:** Un nuevo toque sobre una casilla válida o vacía emite otro clic izquierdo rápido para depositarlo.
  - **Menú de Opciones (Clic Secundario):** Si el usuario apoya el dedo sobre un Pokémon en casilla y no se desplaza más allá del umbral `slop` durante 1.0 segundo, se emite un `RightDown` para abrir el menú emergente (Datos, Soltar, Objeto). Arrastrar cancela inmediatamente el timer de clic secundario.

---

### MÓDULO B: `src/OpenGMK/gm8emulator/src/touch_stick.rs` (Joystick y Resorte Visual)
* **Algoritmo de 8 Direcciones con Histéresis Doble:**
  ```rust
  pub const ACTIVATE: f32 = 0.10; // 10% del radio
  const RELEASE: f32 = 0.07;       // 7% del radio
  const ANGLE_HYSTERESIS: f32 = 5.0 * PI / 180.0; // 5 grados
  ```
* **Prevención de Traqueteo Diagonal:**
  - Para pasar de ortogonal (ej. Derecha) a diagonal (Arriba-Derecha), el ángulo debe cruzar el sector de $45^\circ$ más un margen de histéresis de $5^\circ$. Esto evita que el personaje cambie erráticamente de dirección con vibraciones microscópicas del pulgar.
* **Física del Resorte Visual (`StickSpring`):**
  - Desacopla la lógica de entrada de la animación gráfica:
  $$\text{impulso} = v + \text{speed} \cdot (\text{pos} - \text{goal})$$
  $$\text{pos}_{\text{next}} = \text{goal} + ((\text{pos} - \text{goal}) + \text{impulso} \cdot dt) \cdot e^{-\text{speed} \cdot dt}$$
  - Movimiento críticamente amortiguado (`speed = 25.0` en reposo, `45.0` en tracción). Cero sobre-oscilaciones (*overshoot*).

---

### MÓDULO C: `android/.../TouchFeedback.kt` (Háptica de Baja Latencia)
* **Diagnóstico del Bug:** `NativeActivity` en Android no utiliza la jerarquía estándar de `View` interactiva; `view.performHapticFeedback()` es descartado en silencio por el decorador de ventana en muchos fabricantes (Xiaomi, Samsung, Motorola).
* **Solución de PR #2:**
  - Uso directo del servicio del sistema `Vibrator` / `VibrationEffect`.
  - **API 30+ (Android 11+):**
    ```kotlin
    val effect = VibrationEffect.startComposition()
        .addPrimitive(VibrationEffect.Composition.PRIMITIVE_CLICK, appearance.strength.scale)
        .compose()
    vibrator.vibrate(effect, VibrationAttributes.Builder().setUsage(VibrationAttributes.USAGE_TOUCH).build())
    ```
  - **Fallback Legacy (Android 8 - 10):**
    ```kotlin
    val duration = (14f + 10f * strength).toLong()
    val amplitude = (255f * strength).toInt().coerceIn(1, 255)
    val effect = VibrationEffect.createOneShot(duration, amplitude)
    ```
  - **Coalescencia Anti-Stall:** `if (now - lastPulse < 25) return false` evita saturar la cola del actuador háptico con multitouch rápido.

---

### MÓDULO D: Botón 8 y Ruleta SEL de 7 Centros
* **Integración en Layouts (`TouchLayouts.kt` & `touch_controls.rs`):**
  - Tecla física GameMaker mapeada a `key 8` (código de función configurable en opciones).
  - Coordenadas predeterminadas y visibilidad diferenciada entre modo Exploración y modo Combate.
  - La ruleta del menú `SEL` incluye 7 puntos de anclaje angular para selección rápida con deslizamiento radial.

---

### MÓDULO E: Estilo "Moderno V2" (`TouchStyle.MODERN_V2`)
* **Extensión de Compose y SharedPreferences:**
  - ID de serialización: `"modern_v2"`.
  - Mantiene renderizado basado en sprites pero con bordes vectoriales, palanca translúcida y sombras dinámicas.
  - Previsualización nativa en `TouchAppearanceScreen.kt`, `TouchEditorScreen.kt` y `TouchSettingsScreen.kt`.

---

## 3. Plan de Integración en Nuestro Código

| Paso | Archivo Destino | Acción Requerida |
|---|---|---|
| **1** | `src/OpenGMK/gm8emulator/src/box_touch.rs` | Incorporar módulo de máquina de estados para caja/PC. |
| **2** | `src/OpenGMK/gm8emulator/src/touch_stick.rs` | Integrar cálculo de 8 direcciones con histéresis y `StickSpring`. |
| **3** | `src/OpenGMK/gm8emulator/src/touch_controls.rs` | Enlazar `box_touch` y `touch_stick` con los eventos de `android_main.rs`. |
| **4** | `android/app/src/main/AndroidManifest.xml` | Declarar `<uses-permission android:name="android.permission.VIBRATE" />`. |
| **5** | `android/.../TouchFeedback.kt` | Sustituir llamada a decorador por pulsos directos de `Vibrator` con coalescencia de 25 ms. |
| **6** | `android/.../TouchAppearance.kt` | Registrar enum `TouchStyle.MODERN_V2` en parseadores y migraciones. |
| **7** | `verification/touch_tests.rs` | Añadir suite de pruebas de regresión táctil (`cargo test`). |

