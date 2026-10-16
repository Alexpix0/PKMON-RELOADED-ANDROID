package com.klisee.reloaded

import java.io.File

/**
 * Escribe el archivo de configuración de vídeo, rendimiento y opciones de juego que lee el motor OpenGMK al arrancar.
 *
 * Parámetros soportados:
 * - `scaling`: "aspect", "full", "pixel_perfect"
 * - `show_fps`: 1 / 0
 * - `bezel`: "oled", "retro_slate", "reloaded_blue"
 * - `turbo_speed`: multiplicador numérico (1.5, 2.0, 4.0, 6.0, 8.0)
 * - `frame_skip`: "0", "1", "2", "auto"
 * - `render_scale`: "0.75", "1.0", "1.25", "1.5"
 * - `filter`: "nearest", "sharp", "crt", "fxaa"
 * - `weather_opt`: "normal", "optimized", "minimal"
 * - `backend`: "opengl", "vulkan"
 * - `force_60fps`: 1 / 0
 * - Opciones de Juego (Ajustes -> Juego):
 *   - berry_tracker, egg_hatch_timer, key_items_radial, auto_run, dialog_tap_advance, repel_reminder,
 *     shiny_beacon, puzzle_pause, fishing_haptic, quick_save, save_reminder
 */
fun writeVideoFile(
    filesDir: File,
    scaling: String,
    showFps: Boolean = false,
    bezel: String = "oled",
    turboSpeed: Float = 2.0f,
    frameSkip: String = "0",
    renderScale: String = "1.0",
    filter: String = "sharp",
    weatherOpt: String = "normal",
    backend: String = "opengl",
    force60Fps: Boolean = false,
    bgmVolume: Int = 100,
    sfxVolume: Int = 100,
    autofire: Boolean = true,
    dynamicDpad: Boolean = false,
    berryTracker: Boolean = false,
    eggHatchTimer: Boolean = false,
    keyItemsRadial: Boolean = false,
    autoRun: Boolean = false,
    dialogTapAdvance: Boolean = false,
    repelReminder: Boolean = false,
    shinyBeacon: Boolean = false,
    puzzlePause: Boolean = false,
    fishingHaptic: Boolean = false,
    quickSave: Boolean = false,
    saveReminder: Boolean = false
) {
    val text = buildString {
        appendLine("# Escrito por Ajustes (v0.7.0). El motor OpenGMK lo lee al arrancar.")
        appendLine("scaling=$scaling")
        appendLine("show_fps=${if (showFps) 1 else 0}")
        appendLine("bezel=$bezel")
        appendLine("turbo_speed=$turboSpeed")
        appendLine("frame_skip=$frameSkip")
        appendLine("render_scale=$renderScale")
        appendLine("filter=$filter")
        appendLine("weather_opt=$weatherOpt")
        appendLine("backend=$backend")
        appendLine("force_60fps=${if (force60Fps) 1 else 0}")
        appendLine("bgm_volume=$bgmVolume")
        appendLine("sfx_volume=$sfxVolume")
        appendLine("autofire=${if (autofire) 1 else 0}")
        appendLine("dynamic_dpad=${if (dynamicDpad) 1 else 0}")
        // Opciones de Ajustes -> Juego
        appendLine("berry_tracker=${if (berryTracker) 1 else 0}")
        appendLine("egg_hatch_timer=${if (eggHatchTimer) 1 else 0}")
        appendLine("key_items_radial=${if (keyItemsRadial) 1 else 0}")
        appendLine("auto_run=${if (autoRun) 1 else 0}")
        appendLine("dialog_tap_advance=${if (dialogTapAdvance) 1 else 0}")
        appendLine("repel_reminder=${if (repelReminder) 1 else 0}")
        appendLine("shiny_beacon=${if (shinyBeacon) 1 else 0}")
        appendLine("puzzle_pause=${if (puzzlePause) 1 else 0}")
        appendLine("fishing_haptic=${if (fishingHaptic) 1 else 0}")
        appendLine("quick_save=${if (quickSave) 1 else 0}")
        appendLine("save_reminder=${if (saveReminder) 1 else 0}")
    }
    runCatching { File(filesDir, "video.conf").writeText(text) }
}

fun writeVideoFile(filesDir: File, scaling: String, showFps: Boolean = false, bezel: String = "oled") {
    writeVideoFile(filesDir, scaling, showFps, bezel, 2.0f, "0", "1.0", "sharp", "normal")
}

fun writeVideoFile(filesDir: File, scaling: String, showFps: Boolean = false) {
    writeVideoFile(filesDir, scaling, showFps, "oled", 2.0f, "0", "1.0", "sharp", "normal")
}

fun writeVideoFile(filesDir: File, keepAspect: Boolean, showFps: Boolean = false) {
    writeVideoFile(filesDir, if (keepAspect) "aspect" else "full", showFps)
}
