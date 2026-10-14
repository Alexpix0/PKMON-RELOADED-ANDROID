package com.klisee.reloaded

import java.io.File

/**
 * Writes the video mode the emulator reads when it starts.
 *
 * Supported scaling modes:
 * - `aspect`: classic 4:3 shape with pillarbox bands.
 * - `full`: stretched to fill the full screen.
 * - `pixel_perfect`: integer scaling with razor-sharp pixel edges for OLED/HD/2K screens.
 */
fun writeVideoFile(filesDir: File, scaling: String, showFps: Boolean = false) {
    val text = buildString {
        appendLine("# Escrito por Ajustes. El emulador lo lee al arrancar.")
        appendLine("scaling=$scaling")
        appendLine("show_fps=${if (showFps) 1 else 0}")
    }
    runCatching { File(filesDir, "video.conf").writeText(text) }
}

fun writeVideoFile(filesDir: File, keepAspect: Boolean, showFps: Boolean = false) {
    writeVideoFile(filesDir, if (keepAspect) "aspect" else "full", showFps)
}
