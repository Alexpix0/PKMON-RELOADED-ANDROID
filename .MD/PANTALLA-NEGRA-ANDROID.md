# Pantalla negra en Android (GPU PowerVR) — hallazgos y bloqueos

Documento de trabajo. El contexto completo, los archivos de diagnóstico y los diffs de la sesión
quedan **en local**, fuera de git, en `game/pantalla-negra-android/` (la carpeta `game/` está ignorada
por `.gitignore`).

## Síntoma

En un móvil con **PowerVR Rogue GE8320** (MediaTek MT6765): el juego **arranca con audio y responde a
los toques, pero la pantalla se queda negra**. En PC (NVIDIA) y en otro Android (Mali) se ve bien.

## Conclusión y Causa Raíz

En esta GPU (PowerVR Rogue GE8320) la arquitectura es **TBDR (Tile-Based Deferred Renderer)**: las llamadas
`glDrawArrays` no procesan la geometría de inmediato, sino que se encolan para la fase de rasterizado diferido.

Las causas de que la pantalla se quedara negra y los arreglos aplicados:

1. **Función inexistente en GLES: `glGetTexLevelParameteriv` devolvía ancho y alto = 0:**
   En OpenGL ES (en ninguna versión 2.0/3.0/3.2) existe `glGetTexLevelParameteriv`. En `egl.rs` se sustituía
   por un no-op (`no_op_fn`), por lo que `fb_width` y `fb_height` quedaban en **0x0**. Al llamar a
   `setup_frame`, `reset_target` y `present`:
   - El Viewport y Scissor se fijaban a **(0, 0, 0, 0)**, recortando el 100% de la geometría generada.
   - `glBlitFramebuffer` intentaba copiar un rectángulo de **0x0** píxeles hacia la pantalla.
   - En PC (NVIDIA) y Mali funcionaba porque los drivers exportan ese símbolo de escritorio, pero PowerVR
     en el Helio G25 es un driver GLES estricto donde no existe.
   -> **Solución:** `Framebuffer` ahora almacena `width` y `height` directamente en CPU y se leen de ahí.
2. **Vaciado de memoria con `BufferData(..., NULL, STREAM_DRAW)` tras `DrawArrays`:**
   En `draw_buffer`, se invocaba inmediatamente una invalidación de búfer con `ptr::null()`. En arquitecturas
   TBDR como PowerVR, la etapa de tiling/vértices se ejecuta en diferido; al anular el búfer de inmediato, el
   procesador de vértices encontraba datos vacíos/descartados y no producía fragmentos.
   -> **Solución:** Eliminada la invalidación con `ptr::null()`.
3. **Pérdida de enlace de VAO / VBO / Programa:**
   `draw_buffer` no aseguraba que el VAO y el VBO del motor estuvieran enlazados activamente, asumiendo que el
   estado global se mantenía intacto.
   -> **Solución:** Se almacenan `program`, `vao` y `commands_vbo` en `RendererImpl` y se enlazan explícitamente
   antes de cada dibujo.
4. **Falta de `#[repr(C)]` en `Vertex`:**
   `struct Vertex` en `render.rs` tenía layout de Rust por defecto, lo que permitía reordenamiento no conforme
   con los atributos de vértice en GPU.
   -> **Solución:** Se añadió `#[repr(C)]`.
5. **Desajuste del bloque `std140` en `RenderState`:**
   `fog_colour` (vec4) requiere alineación a 16 bytes (offset 576 en GLSL vs 564 en Rust). Además, Rust ponía
   `depth_test` en el offset de `alpha_test`.
   -> **Solución:** Añadido `_fog_pad: [u32; 3]` (12 bytes) antes de `fog_colour` para totalizar exactamente 592
   bytes de bloque `std140`, y separado `alpha_test` (uniforme) de `depth_test` (estado de pipeline).
6. **Restricción de `glBlitFramebuffer` en GLES 3.0 al invertir eje Y:**
   En GLES 3.0, el blit con inversión de coordenadas (srcY0 > srcY1) y `GL_LINEAR` genera `GL_INVALID_OPERATION`.
   -> **Solución:** En Android se garantiza `gl::NEAREST`.
7. **Optimización de arranque (85–90 s -> <1 s):**
   `push_atlases` subía cada sprite individualmente con miles de llamadas `TexSubImage2D`.
   -> **Solución:** Se compone el atlas completo en memoria RAM y se transfiere en una sola llamada por textura.
