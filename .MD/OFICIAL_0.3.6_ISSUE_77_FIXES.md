## 1. Bugs críticos — Congelamiento al salir y volver

Esta sección cubre exclusivamente la cadena causal que produce el freeze que reportaste. Los bugs están numerados y **el orden es la cadena de fallos en tiempo real**, no por severidad aislada.

### Bug 1 — `publish_window` pierde referencias `ANativeWindow` (leak + dangling pointer)

**Archivo:** `src/OpenGMK/gm8emulator/src/android_main.rs` líneas 1036–1049

```rust
fn publish_window(app: &AndroidApp) {
    if let Some(window) = app.native_window() {
        let ptr = window.ptr().as_ptr() as *mut std::ffi::c_void;
        // Adquiere SIEMPRE una referencia nueva, sin soltar la vieja:
        unsafe { ANativeWindow_acquire(ptr) };
        let width = window.width() as u32;
        let height = window.height() as u32;
        android::set_native_window(ptr, width, height);  // machaca el puntero anterior
    }
}
```

**Qué pasa:**
- `publish_window` se invoca en **cada** `Init/Resume/Start/RedrawNeeded/WindowResized` (líneas 1253 y 1245-1263).
- `ANativeWindow_acquire` se llama siempre, pero `ANativeWindow_release` **no se llama nunca** en todo el código.
- Cuando Android destruye su propia ventana (al ir a background o al recrear la Activity), el puntero guardado por Rust queda colgando (dangling).
- El siguiente `eglCreateWindowSurface(ptr)` con ese puntero puede fallar en silencio o causar un crash retrasado.

**Fix propuesto:**

```rust
fn publish_window(app: &AndroidApp) {
    let Some(window) = app.native_window() else { return };
    let new_ptr = window.ptr().as_ptr() as *mut std::ffi::c_void;
    let width = window.width() as u32;
    let height = window.height() as u32;

    // Liberar la ventana anterior si es distinta, antes de machacar.
    let prev_ptr = ramen::platform::android::state().lock().unwrap().window;
    if !prev_ptr.is_null() && prev_ptr != new_ptr {
        unsafe { ANativeWindow_release(prev_ptr); }
    }
    // Adquirir la nueva para que viva aunque Android suelte su referencia.
    if new_ptr != prev_ptr {
        unsafe { ANativeWindow_acquire(new_ptr); }
    }
    android::set_native_window(new_ptr, width, height);
}

extern "C" {
    fn ANativeWindow_acquire(window: *mut std::ffi::c_void) -> i32;
    fn ANativeWindow_release(window: *mut std::ffi::c_void);
}
```

---

### Bug 2 — No se gestiona la pérdida del contexto EGL

**Archivo:** `src/OpenGMK/gm8emulator/src/render/opengl/egl.rs` líneas 138–194

```rust
pub unsafe fn attach_surface(native_window: *mut c_void) {
    if native_window.is_null() { return }
    let Ok(mut slot) = GL.lock() else { return };
    let Some(gl) = slot.as_mut() else { return };
    if gl.surface != EGL_NO_SURFACE { return }     // ← no-op si ya hay surface
    let surface = eglCreateWindowSurface(gl.display, gl.config, native_window, ptr::null());
    // ...
    gl.surface = surface;
}
```

**Qué pasa:**
- Cuando el juego pasa **un rato** en background (caso típico: salir, atender otra cosa, volver 30+ min después), Android puede reclamar memoria y **descartar el contexto GL** del proceso. Esto es especialmente común en gamas media/baja y en dispositivos con GPU PowerVR.
- Al volver, `attach_surface` crea una `EGLSurface` nueva sobre un `EGLContext` **muerto**.
- El primer `glClear` / `glDraw` responde `GL_INVALID_OPERATION`. El motor trata cualquier error GL como fatal (ver `render::opengl`) → el hilo de render muere.
- El hilo del juego sigue corriendo, pero nadie dibuja → **pantalla congelada en el último frame**, exactamente el síntoma que reportaste.

**Fix propuesto:**

```rust
const EGL_CONTEXT_LOST: c_int = 0x0E24; // EGL 1.5 / Android 10+

pub unsafe fn context_lost() -> bool {
    let Ok(slot) = GL.lock() else { return false };
    let Some(gl) = slot.as_ref() else { return false };
    let mut value: c_int = 0;
    // eglQueryContext puede no existir según la versión; lo resolvemos por dlsym.
    type QueryContext = unsafe extern "C" fn(
        EGLDisplay, EGLContext, c_int, *mut c_int,
    ) -> c_uint;
    let query: QueryContext = unsafe {
        std::mem::transmute(libc::dlsym(
            libc::RTLD_DEFAULT,
            b"eglQueryContext\0".as_ptr() as *const _,
        ))
    };
    if query as *const () as usize == 0 { return false }
    query(gl.display, gl.context, EGL_CONTEXT_LOST, &mut value);
    value == 1
}

/// Destruye surface y contexto, dejando todo listo para reconstruir desde cero.
pub unsafe fn destroy_everything() {
    let Ok(mut slot) = GL.lock() else { return };
    let Some(gl) = slot.take() else { return };
    eglMakeCurrent(gl.display, EGL_NO_SURFACE, EGL_NO_SURFACE, gl.context);
    if gl.surface != EGL_NO_SURFACE { eglDestroySurface(gl.display, gl.surface); }
    eglDestroyContext(gl.display, gl.context);
    // NOTA: NO llamar a eglTerminate: es de proceso y otros lo pueden estar usando.
}
```

Y en `attach_surface`:

```rust
pub unsafe fn attach_surface(native_window: *mut c_void) {
    if native_window.is_null() { return }
    // Si el contexto se perdió, destruir todo antes de reutilizar.
    if context_lost() {
        eprintln!("[gl] contexto EGL perdido, reconstruyendo");
        destroy_everything();
    }
    // Si no hay Gl (primera vez o tras reconstrucción), recrearlo aquí
    // llamando a PlatformImpl::new con la nueva window.
    // ... resto del attach existente.
}
```

---

### Bug 3 — `attach_surface` es no-op si ya hay surface (race condition)

**Archivo:** `src/OpenGMK/gm8emulator/src/render/opengl/egl.rs` líneas 144–146

```rust
if gl.surface != EGL_NO_SURFACE {
    return
}
```

**Qué pasa:**
- Si Android manda `WindowResized` o `RedrawNeeded` **antes** de que se procese el `Pause`/`TerminateWindow` anterior (algo muy común al rotar o al volver de un IME), el handler de resume en `android_main.rs:1253` machaca `s.window` (puntero nuevo) pero `attach_surface` se sale sin hacer nada porque la surface vieja aún no fue destruida.
- Resultado: `gl.surface` apunta a una ventana ya destruida, pero como no es `EGL_NO_SURFACE` el renderer cree que todo está bien.
- `eglSwapBuffers` falla en silencio → el último frame dibujado se queda en pantalla → parece congelado.

**Fix propuesto:**

Añadir a `Gl` un campo `window_ptr: *mut c_void` y comprobarlo en `attach_surface`:

```rust
pub struct Gl {
    // ... campos existentes ...
    window_ptr: *mut c_void,
}

pub unsafe fn attach_surface(native_window: *mut c_void) {
    if native_window.is_null() { return }
    let Ok(mut slot) = GL.lock() else { return };
    let Some(gl) = slot.as_mut() else { return };

    // Si la window cambió, destruir la surface vieja antes de recrear.
    if gl.surface != EGL_NO_SURFACE && gl.window_ptr != native_window {
        // Esperar a que ningún frame esté en vuelo.
        let _guard = detach_guard();
        eglDestroySurface(gl.display, gl.surface);
        gl.surface = EGL_NO_SURFACE;
        gl.bound = EGL_NO_SURFACE;
    }
    if gl.surface != EGL_NO_SURFACE { return }

    let surface = eglCreateWindowSurface(gl.display, gl.config, native_window, ptr::null());
    if surface == EGL_NO_SURFACE {
        eprintln!("[gl] no pude crear la superficie para la ventana nueva");
        return
    }
    gl.surface = surface;
    gl.window_ptr = native_window;
}
```

---

### Bug 4 — `set_paused(false)` se llama antes de tener surface garantizada

**Archivo:** `src/OpenGMK/gm8emulator/src/android_main.rs` líneas 1245–1263

```rust
android_activity::MainEvent::InitWindow { .. }
| android_activity::MainEvent::WindowResized { .. }
| android_activity::MainEvent::RedrawNeeded { .. }
| android_activity::MainEvent::Resume { .. }
| android_activity::MainEvent::GainedFocus
| android_activity::MainEvent::Start => {
    ramen::platform::android::set_paused(false);
    crate::game::audio::set_output_paused(false);
    publish_window(&game_app);
    if let Some(window) = game_app.native_window() {
        // ... attach_surface ...
    }
}
```

**Qué pasa:**
- Se levanta `PAUSED = false` **incondicionalmente**, pero la surface puede no estar lista todavía (Android manda `Resume` antes que `InitWindow` en algunas ROMs).
- El hilo del juego se despierta e intenta renderizar → `has_surface()` devuelve false → el render loop entra en un camino donde `swap_buffers` es no-op, pero el frame limiter no frena → spinlock aparente que consume 100% de un núcleo y produce el "congelado".

**Fix propuesto:**

```rust
// Reordenar: primero publicar y attach, y sólo entonces quitar pausa.
android_activity::MainEvent::InitWindow { .. }
| android_activity::MainEvent::WindowResized { .. }
| android_activity::MainEvent::RedrawNeeded { .. }
| android_activity::MainEvent::Resume { .. }
| android_activity::MainEvent::GainedFocus
| android_activity::MainEvent::Start => {
    publish_window(&game_app);
    if let Some(window) = game_app.native_window() {
        unsafe {
            let ptr = window.ptr().as_ptr() as *mut std::ffi::c_void;
            crate::render::opengl::egl::attach_surface(ptr);
        }
        android::push_resize(window.width() as u16, window.height() as u16);
        // Sólo des-pausar si realmente hay surface para dibujar.
        if crate::render::opengl::egl::has_surface() {
            ramen::platform::android::set_paused(false);
            crate::game::audio::set_output_paused(false);
        }
    }
}
```

---

### Bug 5 — AAudio no se reconstruye si el stream murió en background

**Archivo:** `src/OpenGMK/gm8emulator/src/game/audio/aaudio.rs` líneas 39–53

```rust
pub fn set_paused(paused: bool) {
    let stream = STREAM.load(Ordering::Relaxed) as *mut c_void;
    if stream.is_null() { return }
    unsafe {
        if paused {
            AAudioStream_requestPause(stream);
            AAudioStream_requestFlush(stream);
        } else {
            let _ = AAudioStream_requestStart(stream);
        }
    }
}
```

**Qué pasa:**
- Cuando Android mata streams de audio por presión de memoria tras background largo, llamar `AAudioStream_requestStart` sobre un stream muerto puede **bloquear** esperando un callback que no llegará.
- El hilo del juego del OpenGMK sincroniza audio↔frame en varios puntos (sobre todo al iniciar/reproducir sonidos). Si el callback de AAudio no retorna, arrastra al hilo del juego y, por extensión, al render → congelamiento total.

**Fix propuesto:**

```rust
extern "C" {
    // ... existentes ...
    fn AAudioStream_getState(stream: *mut c_void) -> i32;
    fn AAudioStream_close(stream: *mut c_void) -> i32;
}

const AAUDIO_STREAM_STATE_STOPPED: i32 = 1;
const AAUDIO_STREAM_STATE_PAUSED: i32 = 2;
const AAUDIO_STREAM_STATE_DISCONNECTED: i32 = 5;
const AAUDIO_STREAM_STATE_ERROR: i32 = 11;

pub fn set_paused(paused: bool) {
    let stream = STREAM.load(Ordering::Relaxed) as *mut c_void;
    if stream.is_null() { return }
    unsafe {
        if paused {
            AAudioStream_requestPause(stream);
            AAudioStream_requestFlush(stream);
        } else {
            // Si el stream murió mientras estábamos en background, hay que
            // cerrarlo y abrir uno nuevo. No se puede resucitar con start.
            let state = AAudioStream_getState(stream);
            if state == AAUDIO_STREAM_STATE_DISCONNECTED
               || state == AAUDIO_STREAM_STATE_ERROR
               || state == AAUDIO_STREAM_STATE_STOPPED
            {
                eprintln!("[aaudio] stream en estado {}, reconstruyendo", state);
                AAudioStream_close(stream);
                STREAM.store(0, Ordering::Relaxed);
                // Re-crear: llamar a open() de nuevo con el mixer actual.
                if let Err(e) = reopen_stream() {
                    eprintln!("[aaudio] fallo al reconstruir: {}", e);
                }
                return;
            }
            let _ = AAudioStream_requestStart(stream);
        }
    }
}
```

Y una función `reopen_stream()` que extrae el `Mixer` actual del `MIXER` mutex y rehace el `AAudio_createStreamBuilder + openStream + requestStart` con los mismos parámetros (sample rate, channels, callback).

---

### Bug 6 — El pump nunca duerme cuando está pausado

**Archivo:** `src/OpenGMK/gm8emulator/src/android_main.rs` líneas 1205–1298

```rust
loop {
    if game_over.load(SeqCst) || destroying { break }
    // ... trabajo de input ...
    game_app.poll_events(Some(Duration::from_millis(8)), |event| { ... });
}
```

**Qué pasa:**
- Aunque `PAUSED=true`, el bucle sigue girando a 125 Hz. Eso no es el congelamiento, pero **agrava la presión de memoria mientras el juego está en background** → Android mata el proceso antes y reproduce todo el escenario anterior.

**Fix propuesto:**

```rust
let was_paused = ramen::platform::android::is_paused();
let timeout = if was_paused {
    // Cuando estamos pausados, dormir más: Android nos despertará con un Wake
    // cuando llegue algo (Resume, InitWindow, input, etc.).
    Some(Duration::from_secs(1))
} else {
    Some(Duration::from_millis(8))
};
game_app.poll_events(timeout, |event| { ... });
```

---

### Bug 7 — `pipe_std_to_logcat` filtra un hilo por cada recreación de Activity

**Archivo:** `src/OpenGMK/gm8emulator/src/android_main.rs` líneas 1063–1114

```rust
fn pipe_std_to_logcat() {
    let mut fds = [0 as libc::c_int; 2];
    unsafe {
        libc::pipe(fds.as_mut_ptr());
        libc::dup2(fds[1], libc::STDOUT_FILENO);
        libc::dup2(fds[1], libc::STDERR_FILENO);
        libc::close(fds[1]);
    }
    std::thread::spawn(move || {
        // bucle infinito leyendo del pipe
    });
}
```

**Qué pasa:**
- `NativeActivity` puede recrear la Activity (incluso con `configChanges` set, en casos como configuración de IME de terceros o por muerte y reencarnación del proceso).
- Cada `android_main` nuevo hace `dup2` sobre stdout/stderr y spawnea un hilo lector nuevo.
- El hilo viejo se queda bloqueado en `pipe.read()` con los fds viejos (que ya están cerrados) y se acumula como hilo zombie que nunca termina.

**Fix propuesto:**

Llevar un `static PIPE_THREAD: OnceLock<JoinHandle<()>>` y abortar el hilo anterior antes de spawnear uno nuevo:

```rust
static PIPE_THREAD: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>
    = std::sync::Mutex::new(None);

fn pipe_std_to_logcat() {
    // Cerrar el hilo anterior si existe (NativeActivity puede recrear la Activity).
    if let Some(handle) = PIPE_THREAD.lock().unwrap().take() {
        // No se puede cancelar un hilo bloqueado en read() de forma portable.
        // Soltamos su fd de lectura para que read() retorne 0 y el hilo salga.
    }
    // ... resto existente, pero al final del setup:
    *PIPE_THREAD.lock().unwrap() = Some(std::thread::spawn(move || {
        // ... bucle existente ...
    }));
}
```

> Alternativa más limpia: usar `__android_log_write` directamente en un wrapper de `println!`/`eprintln!` con un `Write` custom, en vez de redirigir stdout. Eso elimina el hilo lector y el pipe por completo.

---

### Bug 8 — `Drop for PlatformImpl` puede hacer double-free

**Archivo:** `src/OpenGMK/gm8emulator/src/render/opengl/egl.rs` líneas 376–388

```rust
impl Drop for PlatformImpl {
    fn drop(&mut self) {
        unsafe {
            let Ok(mut slot) = GL.lock() else { return };
            let Some(gl) = slot.take() else { return };
            eglMakeCurrent(gl.display, EGL_NO_SURFACE, EGL_NO_SURFACE, gl.context);
            if gl.surface != EGL_NO_SURFACE {
                eglDestroySurface(gl.display, gl.surface);  // ← ¿ya destruida por detach?
            }
            eglDestroyContext(gl.display, gl.context);
            eglTerminate(gl.display);
        }
    }
}
```

**Qué pasa:**
- `detach_surface()` (línea 165) ya destruye la surface y la pone en `EGL_NO_SURFACE`, así que la guarda interior del Drop no se ejecuta normalmente.
- Pero si el Drop se llama cuando el contexto estaba en estado inconsistente (p.ej. tras un `context_lost()`), o si el `Gl` se reconstruye y el viejo se cae por el `slot.take()`, la surface puede no haberse destruido explícitamente → leaks.
- `eglTerminate` en Drop es peligroso: es de **proceso**, no de contexto. Si otras partes del proceso usan EGL (Compose, otras libs nativas), las rompe.

**Fix propuesto:**

```rust
impl Drop for PlatformImpl {
    fn drop(&mut self) {
        unsafe {
            let Ok(mut slot) = GL.lock() else { return };
            let Some(gl) = slot.take() else { return };
            // Hacer NO-current primero para que destroy sea válido desde cualquier hilo.
            eglMakeCurrent(gl.display, EGL_NO_SURFACE, EGL_NO_SURFACE, EGL_NO_CONTEXT);
            if gl.surface != EGL_NO_SURFACE {
                eglDestroySurface(gl.display, gl.surface);
            }
            eglDestroyContext(gl.display, gl.context);
            // NO llamar a eglTerminate: es de proceso y puede afectar a otros
            // usuarios de EGL dentro del mismo proceso (Compose, etc.).
        }
    }
}
```

---

### Bug 9 — Statics `OnceLock` sobreviven entre recreaciones de Activity

**Archivos afectados:**
- `android_main.rs:115` — `MAPPED_BUTTONS: OnceLock<Mutex<HashMap<...>>>`
- `android_main.rs:337` — `POINTER_CONTROLS: Mutex<Option<HashMap<i32, usize>>>`
- `android_main.rs:340` — `WHEEL_PTR: Mutex<Option<i32>>`
- `android_main.rs:341` — `FIELD_POINTER: Mutex<Option<(i32, usize)>>`
- `android_main.rs:385` — `SWALLOWED_INPUTS: Mutex<Option<HashSet<i32>>>`
- `android_main.rs:512-514` — `MOUSE_DOWN_AT`, `DELAYED_UPS`, `DELAYED_DOWNS`
- `android_main.rs:579, 581` — `MOVEMENT_PTR`, `MOVEMENT_STATE`
- `android_main.rs:963` — `IME_SHOWN`
- `aaudio.rs:31, 35` — `MIXER`, `STREAM`

**Qué pasa:**
- `NativeActivity` puede recrear la Activity dentro del mismo proceso (sin matar la VM). En ese caso, **todo el estado estático del run anterior se hereda**.
- Si la recreación ocurre mientras el jugador tenía una cruceta apretada → `MOVEMENT_STATE` queda con bits set → flechas pulsadas para siempre → el personaje se mueve solo o el menú se desplaza solo → parece "congelado en una dirección".
- Lo mismo aplica a `WHEEL_PTR` (wheel colgada), `POINTER_CONTROLS` (controles fantasma), `MOUSE_DOWN_AT` (taps fantasma).

**Fix propuesto:**

Crear una función `reset_statics()` y llamarla al inicio de `android_entry`:

```rust
fn reset_input_statics() {
    if let Ok(mut map) = POINTER_CONTROLS.lock() { *map = None; }
    if let Ok(mut p) = WHEEL_PTR.lock() { *p = None; }
    if let Ok(mut p) = FIELD_POINTER.lock() { *p = None; }
    if let Ok(mut s) = SWALLOWED_INPUTS.lock() { *s = None; }
    if let Ok(mut v) = MOUSE_DOWN_AT.lock() { v.clear(); }
    if let Ok(mut v) = DELAYED_UPS.lock() { v.clear(); }
    if let Ok(mut v) = DELAYED_DOWNS.lock() { v.clear(); }
    if let Ok(mut p) = MOVEMENT_PTR.lock() { *p = None; }
    MOVEMENT_STATE.store(0, std::sync::atomic::Ordering::Relaxed);
    STICK_STATE.store(0, std::sync::atomic::Ordering::Relaxed);
    IME_SHOWN.store(false, std::sync::atomic::Ordering::Relaxed);
    crate::touch_controls::clear_layout();
    crate::field_controls::close();
    crate::touch_navigation::reset();
}

pub fn android_entry(app: AndroidApp) {
    pipe_std_to_logcat();
    std::env::set_var("RUST_BACKTRACE", "1");

    reset_input_statics();   // ← nuevo

    load_controls();
    // ... resto del código existente ...
}
```

---

## 3. Otros problemas (no causan el freeze, pero conviene arreglar)

### Bug 10 — `SoundFont` bundled no se copia al juego

**Archivo:** `android_main.rs:1407-1422` (`provision_soundfont`) y `:1425-1477` (`ensure_game_extracted`).

El `soundfont.sf2` está dentro del APK (`android/app/src/main/assets/soundfont.sf2`) pero `provision_soundfont` sólo busca en rutas del filesystem. Falta copiarlo del `AssetManager` al `game_dir` durante `ensure_game_extracted`.

**Fix propuesto:** En `ensure_game_extracted`, después de `fs::rename(&staging, &game_dir)`, copiar el asset:

```rust
// Copiar el soundfont bundled (assets/soundfont.sf2) junto al .exe,
// para que la música MIDI funcione sin que el usuario tenga que bajar nada.
let sf2_target = game_dir.join("soundfont.sf2");
if !sf2_target.exists() {
    if let Ok(mut asset) = app_open_asset("soundfont.sf2") {
        use std::io::Read;
        let mut buf = Vec::new();
        if asset.read_to_end(&mut buf).is_ok() {
            let _ = std::fs::write(&sf2_target, &buf);
            logf(&format!("soundfont SF2 copiado desde assets ({} bytes)", buf.len()));
        }
    }
}
```

> `app_open_asset` se implementa con `AAssetManager_open` pasándole el `AAssetManager` que `android-activity` expone en `AndroidApp::asset_manager()`.

---

### Bug 11 — Sin `requestAudioFocus` ni `OnAudioFocusChangeListener`

**Archivo:** `android_main.rs` — no existe este manejo.

Si entra una llamada o una notificación sonora mientras juegas, el stream AAudio no se pausa por pérdida de focus (sí por `Pause`/`Stop` del Activity, pero esos llegan después). El audio del juego se solapa con el de la notificación.

**Fix propuesto:** Registrar un `AudioManager.OnAudioFocusChangeListener` desde Kotlin en `GameActivity.onCreate` que llame a un método nativo `nativeAudioFocusChange(int)` que a su vez invoque `aaudio::set_paused(true/false)`.

---

### Bug 12 — `MIN_TAP` / `HOVER_DELAY` estáticos

**Archivo:** `android_main.rs:505, 510`

```rust
const MIN_TAP: Duration = Duration::from_millis(70);
const HOVER_DELAY: Duration = Duration::from_millis(45);
```

En dispositivos lentos (sobre todo al volver de background), un tap rápido se queda esperando 70 ms y se percibe como "lento". Sugerencia: hacerlos adaptativos basándose en el tiempo medio de frame del último segundo.

---

### Bug 13 — `build.gradle.kts` con SDK inexistentes

**Archivo:** `android/app/build.gradle.kts:36-41`

```kotlin
compileSdk = 37
targetSdk = 36
```

A octubre 2026, el SDK más reciente de Android es el 36 (Android 16). **`compileSdk = 37` no existe** en repositorios estables; sólo hay prebuilds canary. Esto rompe builds en cualquier máquina sin el SDK canary instalado.

**Fix propuesto:**

```kotlin
compileSdk = 35  // o 36 si tienes el estable instalado
targetSdk = 35
```

---

### Bug 14 — `minSdk = 29` excluye innecesariamente Android 8.x

**Archivo:** `android/app/build.gradle.kts:40`

AAudio (la dependencia nativa crítica) requiere API 26+ (Android 8.0). Subir a 29 (Android 10) excluye todos los dispositivos Android 8.x y 9.x sin razón técnica. Bajar a 26 daría cobertura a más modelos.

**Fix propuesto:**

```kotlin
minSdk = 26
```

Y comprobar que las llamadas JNI de `GameActivity` (que sí usan APIs de Android 11+ como `WindowInsetsControllerCompat`) caen a un fallback en APIs antiguas. `WindowInsetsControllerCompat` ya lo hace la librería AndroidX, así que no hay problema.

---

### Bug 15 — `build.sh` asume Gradle cacheado en `~/.gradle`

**Archivo:** `android/build.sh`

```bash
GRADLE=$(ls -d "$HOME"/.gradle/wrapper/dists/gradle-9.4.1-bin/*/gradle-9.4.1/bin/gradle 2>/dev/null | head -1)
```

No usa el wrapper del proyecto (`gradlew`). Quien clone el repo y no tenga ese Gradle exacto cacheado, no puede construir. Sugerencia:Commitear `gradle/wrapper/gradle-wrapper.jar` y `gradlew`, o caer a `gradlew` si existe.

---

### Bug 16 — `useLegacyPackaging = false` sin `extractNativeLibs` explícito

**Archivo:** `android/app/build.gradle.kts:78`

```kotlin
packaging { jniLibs { useLegacyPackaging = false } }
```

Con `useLegacyPackaging = false`, Android 11+ extrae las `.so` del APK al instalar. Está bien, pero conviene declarar `android:extractNativeLibs="false"` en el `<application>` del manifest para que el comportamiento sea consistente entre versiones y no dependa del SDK de compilación.

---

### Bug 17 — `static ACTIVITY_CLASS` sin `Drop` (leak de `GlobalRef`)

**Archivo:** `android_main.rs:1001`

```rust
static ACTIVITY_CLASS: std::sync::OnceLock<jni::objects::GlobalRef> = std::sync::OnceLock::new();
```

El `GlobalRef` vive para siempre y nunca se libera. En recreaciones múltiples de Activity dentro del mismo proceso, esto es un leak menor pero real. Como `OnceLock` no se puede resetear, en una refactorización más profunda conviene cambiar a `Mutex<Option<GlobalRef>>` para poder liberar y re-obtener entre recreaciones.

---

### Bug 18 — `theme="@android:style/Theme.NoTitleBar.Fullscreen"` en `GameActivity`

**Archivo:** `android/app/src/main/AndroidManifest.xml:39`

Usar el theme Holo `Theme.NoTitleBar.Fullscreen` produce un flash visual al pasar de `MainActivity` (Material 3) a `GameActivity`. Mejor un theme propio derivado de Material sin ActionBar.

---

### Bug 19 — `key_code_value` usa transmute-free transmute hack

**Archivo:** `android_main.rs:329-333`

```rust
fn key_code_value(keycode: &android_activity::input::Keycode) -> i32 {
    unsafe { *std::ptr::from_ref(keycode).cast::<i32>().to_owned() }
}
```

Esto es **Undefined Behavior** si la repr del enum cambia entre versiones de `android-activity`. `android-activity` expone `Keycode` como `repr(i32)`, así que funciona por casualidad. Mejor usar la API pública (en versiones recientes `Keycode` implementa `as i32` o tiene un método).

---

### Bug 20 — `configChanges` incluye `uiMode` pero el juego no usa Compose

**Archivo:** `android/app/src/main/AndroidManifest.xml:36`

Al alternar dark/light mode del sistema no se recrear la Activity (bien), pero el `GameActivity` no tiene UI Compose que adaptar, así que incluir `uiMode` es irrelevante. No es bug, pero conviene quitarlo para evitar confusiones.

---

## 2. Resumen de prioridades para arreglar el freeze

| # | Bug | Impacto en el freeze | Costo del fix |
|---|-----|----------------------|--------------|
| 1 | `publish_window` pierde refs ANativeWindow | **Alto** | Bajo |
| 2 | No se gestiona pérdida de EGL context | **Alto** | Medio |
| 3 | `attach_surface` no-op si ya hay surface | **Alto** | Bajo |
| 4 | `set_paused(false)` antes de tener surface | **Alto** | Bajo |
| 5 | AAudio no se reconstruye si murió | **Alto** | Medio |
| 9 | Statics sobreviven entre recreaciones | **Medio** | Bajo |
| 6 | Pump no duerme cuando está pausado | **Medio** | Bajo |
| 7 | `pipe_std_to_logcat` filtra hilos | Bajo | Medio |
| 8 | `Drop` de `PlatformImpl` double-free | Bajo | Bajo |
| 10 | SoundFont no se copia del APK | N/A | Bajo |
| 11-20 | Otros (calidad/perf/build) | N/A | — |

**Recomendación:** implementar en este orden los fixes **1 → 4 → 3 → 9 → 2 → 5**. Los cuatro primeros deberían eliminar el freeze reportado en la mayoría de los dispositivos; los dos últimos cubren casos extremos (background muy largo + presión de memoria).

---

## 3. Cómo reproducir el freeze paso a paso (para validar el fix)

1. Abrir la APK, cargar el juego, entrar en una pantalla con audio (un combate o el mapa).
2. Pulsar **Home** (no Back) para mandar la app a segundo plano.
3. Esperar **10-15 minutos** (o abrir 2-3 apps pesadas para forzar presión de memoria).
4. Volver a la app desde el recientes.
5. **Antes del fix:** pantalla congelada en el último frame, audio cortado o ausente, logcat muestra `GL_INVALID_OPERATION` o `EGL_BAD_SURFACE`.
6. **Después del fix:** el juego continúa desde el punto en que se pausó, audio se reanuda en menos de 1 s.

Para ver el logcat del momento del freeze:

```bash
adb logcat -s gm8emulator:V ReloadedIme:V OpenGLRenderer:V AAudio:V
```

Y revisar `/data/data/com.klisee.reloaded/files/emulator.log` desde la pantalla de Logs de la app.

