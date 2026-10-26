# ROADMAP TÉCNICO Y ARQUITECTURA: POKÉMON RELOADED ANDROID v0.9.4 EX

**Versión:** `v0.9.4 EX`  
**Versión:** `v0.9.4 EX`  
**Código de Versión (`versionCode`):** `35`  
**Estado:** `COMPLETADO Y PUBLICADO EN GITHUB RELEASES (100% SUCCESS)`  
**Fecha de Publicación:** 20 de Octubre de 2026  
**Objetivo Primordial:** Reemplazo estético radical de la Rueda Radial táctica por un diseño biselado idéntico a los botones del HUD con iconos procedurales/pixel-art y 8 acciones críticas de partida, auditoría e integración nativa del ZIP de Pokémon Reloaded TLB (1.9.2.2 Full) con perfiles conmutables de usuario (P1, P2, P3, P4), monitor de FPS apilado vertical y adaptativo, actualizador directo integrado con GitHub Releases API, optimización Vulkan/OpenGL de latencia cero, sincronización con el ciclo de vida 0.3.6 (resiliencia EGL, AAudio, ANativeWindow, Audio Focus) y blindaje 60 FPS en Infinix Hot 20i (Helio G25 / PowerVR GE8320).

---

## 🎨 FASE 1: Rediseño Visual Total de la Rueda Táctica Radial + 8 Acciones Críticas [COMPLETADA]
### Diagnóstico de la Rueda Anterior (`image_17e618.jpg`)
* La rueda anterior usaba cuñas angulares con textos bitmap rasterizados que se escalaban al alto total de la ventana, provocando desbordamiento y solapamiento severo ("VELOCIDAD", "MARCO", "VOLUMEN", "FPS HUD", "GUARDAR", "REINICIAR").
* El estilo visual de cuña plana negra rompía con la estética premium de los botones Reloaded (cajas biseladas, gradientes oscuros, bordes oscuros de 2px y acentos cian).
* Opciones como "Marco" o "Reiniciar" eran redundantes o riesgosas en pleno combate/aventura.

### Especificación Arquitectónica del Anillo Táctico de Cajas Biseladas
1. **Topología de Nodos:** 8 tarjetas rectangulares biseladas equidistantes dispuestas radialmente alrededor del centro de activación, más un botón central de cierre y estado.
2. **Estilo Reloaded Oficial:**
   * Fondo: `#2B2D31` (Gris Grafito Oscuro Reloaded).
   * Borde Exterior: `2px` sólido `#151618`.
   * Bisel Superior/Interior: `#4E525A` (Efecto relieve 3D).
   * Hover / Selección táctil: Resplandor cian `#00E5FF` con borde luminoso y elevación de brillo.
3. **8 Opciones Críticas de Media Partida:**
   * **Sector 1 — `TURBO`:** Icono procedural Doble Rayo / Fast-Forward (`>>`) en oro y cian. Conmuta `1x -> 2x -> 4x`.
   * **Sector 2 — `GUARDAR`:** Icono procedural Disquete Retro / Estrella en verde esmeralda (`#2ECC71`). Dispara Quick Save instantáneo en un toque.
   * **Sector 3 — `CARGAR`:** Icono procedural Reloj de Arena / Carpeta con Retorno en azul cielo (`#3498DB`). Carga el último Quick Save.
   * **Sector 4 — `TIPOS`:** Icono procedural Tabla Elemental / Lupa Pokédex (Fuego, Agua, Planta). Alterna el visor de debilidades de tipo en combate.
   * **Sector 5 — `PERFIL`:** Icono procedural Mando / Selector P1 / P2 / P3 / P4. Conmuta instantáneamente el perfil de controles en partida.
   * **Sector 6 — `SONIDO`:** Icono procedural Altavoz / Nota Musical con indicador de silencio mute (`#E74C3C`). Alterna silencio rápido.
   * **Sector 7 — `FPS`:** Icono procedural Velocímetro / Gráfico de Barras en cian. Alterna el HUD de rendimiento apilado.
   * **Sector 8 — `CAPTURA`:** Icono procedural Cámara Fotográfica Pixel-Art. Captura el frame limpio del juego sin overlay.
   * **Centro — `CERRAR`:** Botón circular compacto con icono `X` e indicador de estado activo (`1X / 2X / P1`).
4. **Tipografía:** Renderizado de una única palabra corta con `scale` calculado dinámicamente según el ancho de la tarjeta biselada, garantizando cero desbordamiento y legibilidad absoluta.

---

## 📦 FASE 2: Auditoría del ZIP Pokémon Reloaded TLB y Sistema Multi-Perfil de Usuario [COMPLETADA]
### Auditoría Nativa de `Proyecto Reloaded The Last Beta 1.9.2.2 Full.zip`
* Análisis del set completo de controles en GameMaker 8 / PC:
  * Teclas de acción principales: Z (Confirmar/Ataque 1), X (Cancelar/Correr/Ataque 2), C (Bicicleta/Ataque 3), V (Caña/Ataque 4), Espacio (Equipo Pokémon), Enter (Menú Pausa/OK).
  * Teclas de combate secundarias: B (Relevo), M (Objeto Rápido), S (Megaevolución/Dinamax), E (Huir/Escapar), A (Ataque Especial), N (Movimiento Z).
  * Teclas de menú y atajos rápidos: O (Mochila), P (PokéNav), T (Frente Batalla), L (Registro), Números 1-9 (Atajos directos de equipo / Safari), F1-F12 (Funciones y trucos del juego).
* **Multi-Perfil de Usuario (`P1`, `P2`, `P3`, `P4`):**
  * Independiente de los perfiles contextuales (`Exploración`, `Combate`, etc.), el usuario dispone de 4 perfiles maestros de asignación.
  * Selector en el editor "Personalizar controles" para cambiar entre Perfil 1, 2, 3 y 4 con opción de "Copiar de Perfil 1" o "Restablecer valores Reloaded TLB".
  * Conmutación en caliente desde el Sector 5 de la nueva Rueda Radial.

---

## 📊 FASE 3: Monitor de FPS y Frametime Apilado en el Borde [COMPLETADA]
* Layout de 3 líneas apiladas verticalmente:
  1. `60.0 FPS` + Badge `GL` o `VK`
  2. `16.6 ms`
  3. Sparkline de 30 muestras históricas
* Posicionamiento adaptativo:
  * Modo 4:3 con bandas negras: Ubicado exactamente en la franja derecha exterior (`right_bar_start + 8.0px`), protegiendo el área activa de juego.
  * Modo Pantalla Completa: Pegado al borde superior derecho (`margin_right = 4.0px`, `margin_top = 6.0px`).
* Implementación en OpenGL ES y Vulkan con `Zero-Heap Allocations` (`[u8; 16]` stack buffer).

---

## 🔄 FASE 4: Buscador e Instalador de Actualizaciones Directo desde GitHub [COMPLETADA]
* **API de GitHub Releases:**
  * Endpoint: `https://api.github.com/repos/Alexpix0/PKMON-RELOADED-ANDROID/releases/latest`.
  * Detección semántica de nuevas versiones comparando `tag_name` vs `AppInfo.VERSION_NAME` (`0.9.4`).
* **UI en Ajustes y "Acerca de":**
  * Tarjeta interactiva destacada en `SettingsScreen.kt` y `AboutScreen.kt`: "🔄 Buscar actualizaciones de versión (GitHub)".
  * Diálogo con changelog oficial formateado y botón "Descargar e Instalar APK ahora".
  * Descargador con barra de progreso en vivo, guardado en caché de descargas y lanzamiento seguro de `ACTION_VIEW` con `FileProvider` (`com.klisee.reloaded.fileprovider`).

---

## ⚡ FASE 5: Optimización Extrema Vulkan, OpenGL ES y Ciclo de Vida 0.3.6 (Infinix Hot 20i) [COMPLETADA]
* **Vulkan Swapchain:** `VK_SURFACE_TRANSFORM_IDENTITY_BIT_KHR` para omitir rotación de SurfaceFlinger en Android.
* **Frames-in-Flight:** `MAX_FRAMES_IN_FLIGHT = 2` con sincronización por fences y semáforos binarios.
* **Zero-Copy Host Memory:** Ring-buffer persistente de 4 MB mapeado con coherencia de host (`HOST_VISIBLE | HOST_COHERENT`).
* **Protección Helio G25 / PowerVR Rogue GE8320:**
  * Eliminación de sincronizaciones pesadas en hot loop.
  * Descarte seguro en shaders para GPUs que no admiten float de 32 bits de alta precisión en fragment shaders.
* **Ciclo de Vida del Motor (Upstream 0.3.6):**
  * Recuperación y reinicio seguro ante pérdida de contexto EGL (`EGL_CONTEXT_LOST`).
  * Destrucción ordenada de superficies EGL viejas ante entrega de nuevas ventanas y liberación limpia de `ANativeWindow` (`PUBLISHED_WINDOW`).
  * Reanudación condicional con `has_surface()` para evitar fotogramas ciegos.
  * Reconexión dinámica de AAudio con buffer lock-free de < 3 ms ante suspensiones del sistema.
  * Integración con `AudioManager.OnAudioFocusChangeListener` para pausar ante llamadas o notificaciones externas.

---

## 🚀 FASE 6: Versionado, CI y Despliegue Oficial [COMPLETADA]
* `versionCode = 35`, `versionName = "0.9.4 EX"`, `FULL_VERSION_STRING = "0.9.4 EX"`.
* Verificación 100% limpia en GitHub Actions CI (Run #`39450248791` y #`39450221924`).
* Tags `v0.9.4` y `0.9.4` actualizados en `origin` y `fork`.
* **Publicación confirmada en GitHub Releases:** `app-release.apk` (60.54 MB).

