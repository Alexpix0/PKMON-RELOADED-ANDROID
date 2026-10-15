package com.klisee.reloaded

import java.io.File

/**
 * Escribe el archivo de configuración de vídeo y rendimiento que lee el motor OpenGMK al arrancar.
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
    weatherOpt: String = "normal"
) {
    val text = buildString {
        appendLine("# Escrito por Ajustes (v0.5.0). El motor OpenGMK lo lee al arrancar.")
        appendLine("scaling=$scaling")
        appendLine("show_fps=${if (showFps) 1 else 0}")
        appendLine("bezel=$bezel")
        appendLine("turbo_speed=$turboSpeed")
        appendLine("frame_skip=$frameSkip")
        appendLine("render_scale=$renderScale")
        appendLine("filter=$filter")
        appendLine("weather_opt=$weatherOpt")
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
    writeVideoFile(filesDir, if (keepAspect) "aspect" else "full", showFps, "oled", 2.0f, "0", "1.0", "sharp", "normal")
}
