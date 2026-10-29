# Subsistema Oficial de Renderizado de Sombras y Primitivas Vectoriales

Archivos extraídos directamente de la versión oficial de referencia:
`SOURCE CODE DEL OFICIAL/pokemon-reloaded-android-0.4.1/src/OpenGMK/gm8emulator/src/`

---

## 📂 Archivos y Funciones Específicas

### 1. `render.rs`
- **Ruta de origen:** `src/OpenGMK/gm8emulator/src/render.rs`
- **Función en el subsistema:**
  - Define la estructura de vértices de la GPU: `Vertex { pos, interp, tex_coord, blend, atlas_xywh, normal }`.
  - Implementa `PrimitiveBuilder` y su método `push_vertex(...)` con valor base `interp: [0.0, 0.0]`.
  - Establece la convención para geometrías vectoriales no texturizadas (`tex_coord: [-1.0, -1.0]` o flat-color).
  - Declara el trait `RendererTrait` con las firmas de `set_draw_offset(x, y)` y `set_interp_amount(amount)`.

### 2. `opengl.rs`
- **Ruta de origen:** `src/OpenGMK/gm8emulator/src/render/opengl.rs`
- **Función en el subsistema:**
  - Implementa `draw_ellipse(...)`, que descompone el óvalo de la sombra en un abanico de triángulos (`TriFan`) o tira de líneas (`LineStrip`) según `self.circle_precision` (por defecto 24-32 segmentos).
  - Gestiona `ShapeBuilder::new(outline, self.white_pixel, alpha, self.depth)`.
  - En `push_primitive(builder)`, asigna de manera uniforme y masiva el vector de interpolación activo a **todos los vértices** de la figura:
    ```rust
    if self.interp_amount != 0.0 {
        let offset = self.draw_offset;
        for vertex in &mut self.vertex_queue[start..] {
            vertex.interp = offset;
        }
    }
    ```
  - Mantiene `self.draw_offset` sincronizado con la instancia que despacha el evento `draw`.

### 3. `draw.rs`
- **Ruta de origen:** `src/OpenGMK/gm8emulator/src/game/draw.rs`
- **Función en el subsistema:**
  - Gestiona la fase de renderizado de instancias `draw_instance(game, idx)`.
  - Calcula el vector delta de posición del frame lógico previo:
    `dx = instance.xprevious - instance.x;`
    `dy = instance.yprevious - instance.y;`
  - Inyecta `set_draw_offset(dx, dy)` al renderizador **antes** de invocar el evento de dibujo (`ev::DRAW`) de la instancia.
  - Al terminar el evento de la instancia, restablece estrictamente `set_draw_offset(0.0, 0.0)`.
  - Calcula la interpolación de cámara `prev_views` -> `room.views` con `interp_alpha`.

### 4. `vertex.glsl`
- **Ruta de origen:** `src/OpenGMK/gm8emulator/src/render/glsl/vertex.glsl`
- **Función en el subsistema:**
  - Vertex Shader responsable del desplazamiento continuo de subpíxel:
    ```glsl
    world_pos.xy += interp * interp_amount;
    ```
  - Aplica la traslación temporal a las sombras y sprites de forma simultánea en la GPU según la fracción temporal `interp_amount`.

### 5. `fragment.glsl`
- **Ruta de origen:** `src/OpenGMK/gm8emulator/src/render/glsl/fragment.glsl`
- **Función en el subsistema:**
  - Fragment Shader de rasterización.
  - Renderiza el pixel blanco (`white_pixel`) modulado por el color de sombra (`frag_blend * frag_blend_flat`) y alfa del juego.
  - Evita muestras degeneradas en texturas sobre geometrías no mapeadas.

### 6. `kernel.rs`
- **Ruta de origen:** `src/OpenGMK/gm8emulator/src/gml/kernel.rs`
- **Función en el subsistema:**
  - Enlaces de la máquina virtual GML: `draw_ellipse`, `draw_ellipse_color`, `draw_circle`.
  - Transforma las coordenadas de caja delimitadora GML `(x1, y1, x2, y2)` al centro `(xcenter, ycenter)` y radios `(rad_x, rad_y)` requeridos por el backend de OpenGL.

### 7. `game.rs`
- **Ruta de origen:** `src/OpenGMK/gm8emulator/src/game.rs`
- **Función en el subsistema:**
  - Bucle principal desacoplado (`run` / `frame` / `render_only`).
  - En frames interpolados intermedios (`> 30 FPS`), calcula la fracción temporal acumulada:
    `self.interp_alpha = (accumulator.as_secs_f64() / duration.as_secs_f64()).clamp(0.0, 1.0);`
  - Configura `interp_draw = true` e invoca `draw()`, entregando al renderizador el factor `(1.0 - interp_alpha)`.

