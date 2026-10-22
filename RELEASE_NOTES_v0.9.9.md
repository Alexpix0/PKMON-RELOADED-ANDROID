# Pokémon Reloaded Android — Versión 0.9.9 (Build 40)

### 🚀 Nuevas Funcionalidades y Mejoras (Respecto a v0.9.8)
- **Cruceta Táctil Octogonal (8-Way D-Pad):** Soporte nativo para movimiento diagonal fluido en combates en tiempo real mediante cálculo trigonométrico por sectores de 45° en el D-Pad táctil.
- **Botones Virtuales para Puzzles y Utilidades:**
  - Nuevos controles `⟲` (`Q`) y `⟳` (`E`) para girar las piezas líticas en las Ruinas Alfa y Ruinas Omega.
  - Nuevas teclas rápidas en pantalla: `H` (visualizar reloj/hora en partida), `F` (flauta), `F1` (ayuda) y `F5` (guardado rápido).
- **Soporte de Controles en Cajas de Pokémon y Menús (Modo Híbrido):** Habilitada la disponibilidad y personalización completa de botones de acción (`Z`, `X`, `C`, `A`, `OK`, `ESC` y `Joystick`) dentro de la caja de almacenamiento del PC del Centro Pokémon.
- **Localización Completa al Inglés:** Creado el catálogo de recursos `values-en/strings.xml` para que el selector de perfiles táctiles, modos de control y opciones se presenten en inglés según el idioma del sistema.
- **Guía Rápida de Onboarding en el Editor de Controles:** Tarjeta explicativa visual integrada en el editor táctil para orientar al jugador sobre cómo añadir, mover o redimensionar cualquier botón.

### 🛠️ Correcciones Críticas de Errores (Respecto a v0.9.8)
- **Eliminación del Freeze en Último Frame por Worker Huérfano (Issue #96):**
  - Implementada bandera atómica `SHUTDOWN` con terminación sincronizada del bucle de emulación.
  - El callback nativo de AAudio se detiene inmediatamente con `AAUDIO_CALLBACK_RESULT_STOP` evitando desbordamientos de audio y carreras con el mixer al recrear la actividad.
  - Watchdog de ciclo de vida con timeout de 2000 ms para garantizar que el proceso se reinicia con estado gráfico EGL limpio.
- **Supresión de Entradas Fantasma ("Sticky Input" - Issue #106):** Purga automática de toques sostenidos (`flush_held_inputs`) durante transiciones de combate o debilitamiento de Pokémon.
- **Corrección de Entrada en Teclado Android (IME - Issue #98):** Ajuste de flags del anchor nativo a `TYPE_TEXT_VARIATION_VISIBLE_PASSWORD` para evitar interferencias de predictores de texto y garantizar que las letras se escriban en los diálogos de GameMaker.
- **Estabilidad de Capas y Profundidad (Z-Index / Surf - Issue #99):** Desempate determinista por ID de instancia en el algoritmo de ordenamiento de dibujo 2D, eliminando artefactos visuales donde los Pokémon en Surf o efectos de mapa parpadeaban bajo el suelo.
