package com.klisee.reloaded

import java.io.File

/**
 * Escribe el archivo de configuración de vídeo, rendimiento, opciones de juego y v0.9.0
 * que lee el motor OpenGMK al arrancar.
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
    saveReminder: Boolean = false,
    // Nuevas Opciones v0.9.0 ("El Salto Generacional y Magia Negra")
    extremeCombat: Boolean = false,
    maliEarlyZ: Boolean = true,
    gpuWakeup: Boolean = true,
    smartPrerollSkip: Boolean = false,
    adaptiveLtpo: Boolean = false,
    pipMode: Boolean = false,
    liveActivity: Boolean = false,
    activeStaticPartition: Boolean = true,
    megaAtlas: Boolean = false,
    fsrMode: Int = 0,
    drsEnabled: Boolean = false,
    xbrzEnabled: Boolean = false,
    lighting2d: Boolean = false,
    audio3d: Boolean = false,
    reactiveStems: Boolean = false,
    wsola: Boolean = false,
    colorblindMode: Int = 0,
    radialMenu: Boolean = false,
    gestures: Boolean = false,
    floatingDpad: Boolean = false,
    oneHanded: Boolean = false,
    replayBlackbox: Boolean = false,
    achievements: Boolean = false,
    jitArm64: Boolean = true,
    bigLittleAffinity: Boolean = true
) {
    val text = buildString {
        appendLine("# Escrito por Ajustes (v0.9.0). El motor OpenGMK lo lee al arrancar.")
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
        // Opciones v0.9.0
        appendLine("extreme_combat=${if (extremeCombat) 1 else 0}")
        appendLine("mali_earlyz=${if (maliEarlyZ) 1 else 0}")
        appendLine("gpu_wakeup=${if (gpuWakeup) 1 else 0}")
        appendLine("smart_preroll_skip=${if (smartPrerollSkip) 1 else 0}")
        appendLine("adaptive_ltpo=${if (adaptiveLtpo) 1 else 0}")
        appendLine("pip_mode=${if (pipMode) 1 else 0}")
        appendLine("live_activity=${if (liveActivity) 1 else 0}")
        appendLine("active_static_partition=${if (activeStaticPartition) 1 else 0}")
        appendLine("mega_atlas=${if (megaAtlas) 1 else 0}")
        appendLine("fsr_mode=$fsrMode")
        appendLine("drs_enabled=${if (drsEnabled) 1 else 0}")
        appendLine("xbrz_enabled=${if (xbrzEnabled) 1 else 0}")
        appendLine("lighting2d=${if (lighting2d) 1 else 0}")
        appendLine("audio_3d=${if (audio3d) 1 else 0}")
        appendLine("reactive_stems=${if (reactiveStems) 1 else 0}")
        appendLine("wsola=${if (wsola) 1 else 0}")
        appendLine("colorblind_mode=$colorblindMode")
        appendLine("radial_menu=${if (radialMenu) 1 else 0}")
        appendLine("gestures=${if (gestures) 1 else 0}")
        appendLine("floating_dpad=${if (floatingDpad) 1 else 0}")
        appendLine("one_handed=${if (oneHanded) 1 else 0}")
        appendLine("replay_blackbox=${if (replayBlackbox) 1 else 0}")
        appendLine("achievements=${if (achievements) 1 else 0}")
        appendLine("jit_arm64=${if (jitArm64) 1 else 0}")
        appendLine("big_little_affinity=${if (bigLittleAffinity) 1 else 0}")
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
