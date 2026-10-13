package com.klisee.reloaded

import java.io.File

/**
 * Writes the video mode the emulator reads when it starts.
 *
 * The game was drawn for a 4:3 shape, so `aspect` shows it the way it looked on a PC, with bands at
 * the sides, while `full` stretches it to fill the phone's screen.
 */
fun writeVideoFile(filesDir: File, keepAspect: Boolean) {
    val text = buildString {
        appendLine("# Escrito por Ajustes. El emulador lo lee al arrancar.")
        appendLine("scaling=${if (keepAspect) "aspect" else "full"}")
    }
    runCatching { File(filesDir, "video.conf").writeText(text) }
}
