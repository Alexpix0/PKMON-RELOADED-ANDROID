package com.klisee.reloaded

import android.content.Context
import java.io.File

/**
 * The launcher's own settings. The game keeps its separate options screen for in-game things; this
 * only holds what the launcher itself can honour.
 */
class Preferences(context: Context) {
    private val prefs = context.getSharedPreferences("launcher", Context.MODE_PRIVATE)

    /** Keep the screen awake while the game is running, so long battles don't lock the phone. */
    var keepScreenOn: Boolean
        get() = prefs.getBoolean("keepScreenOn", true)
        set(value) {
            prefs.edit().putBoolean("keepScreenOn", value).apply()
        }

    /** Whether the first-run setup has been completed. */
    var onboardingDone: Boolean
        get() = prefs.getBoolean("onboardingDone", false)
        set(value) {
            prefs.edit().putBoolean("onboardingDone", value).apply()
        }

    /** Pad buttons the player has rebound, as `ACTION=keyCode`. Absent means the built-in layout. */
    var controlBindings: Map<ControlAction, Int>
        get() = prefs.getStringSet("controlBindings", emptySet()).orEmpty().mapNotNull { entry ->
            val parts = entry.split('=')
            if (parts.size != 2) return@mapNotNull null
            val action = ControlAction.entries.firstOrNull { it.name == parts[0] } ?: return@mapNotNull null
            val code = parts[1].toIntOrNull() ?: return@mapNotNull null
            action to code
        }.toMap()
        set(value) {
            prefs.edit().putStringSet("controlBindings", value.map { "${it.key.name}=${it.value}" }.toSet()).apply()
        }

    /** Whether a pad's sticks move the player. */
    var stickEnabled: Boolean
        get() = prefs.getBoolean("stickEnabled", true)
        set(value) {
            prefs.edit().putBoolean("stickEnabled", value).apply()
        }

    /** On-screen controls opacity in percentage (20 to 100, default 70). */
    var touchOpacity: Int
        get() = prefs.getInt("touchOpacity", 70)
        set(value) {
            prefs.edit().putInt("touchOpacity", value).apply()
        }

    /** On-screen controls scale in percentage (60 to 140, default 100). */
    var touchScale: Int
        get() = prefs.getInt("touchScale", 100)
        set(value) {
            prefs.edit().putInt("touchScale", value).apply()
        }

    /** On-screen controls horizontal margin / offset (0 to 15%, default 0). */
    var touchMarginX: Int
        get() = prefs.getInt("touchMarginX", 0)
        set(value) {
            prefs.edit().putInt("touchMarginX", value).apply()
        }

    /** On-screen controls vertical margin / offset (0 to 15%, default 0). */
    var touchMarginY: Int
        get() = prefs.getInt("touchMarginY", 0)
        set(value) {
            prefs.edit().putInt("touchMarginY", value).apply()
        }

    /** Whether haptic feedback (vibration) is triggered on touch button presses. */
    var touchHaptics: Boolean
        get() = prefs.getBoolean("touchHaptics", true)
        set(value) {
            prefs.edit().putBoolean("touchHaptics", value).apply()
        }

    /** Joystick deadzone in percentage (10 to 60, default 35). */
    var touchDeadzone: Int
        get() = prefs.getInt("touchDeadzone", 35)
        set(value) {
            prefs.edit().putInt("touchDeadzone", value).apply()
        }

    /** Whether to show the top 1-6 party/shortcut buttons on screen. */
    var showNumKeys: Boolean
        get() = prefs.getBoolean("showNumKeys", true)
        set(value) {
            prefs.edit().putBoolean("showNumKeys", value).apply()
        }

    /** Mirrors the current controls into the file the emulator reads at startup. */
    fun writeControls(filesDir: File, padConnected: Boolean) =
        writeControlsFile(filesDir, controlBindings, stickEnabled, padConnected, touchOpacity, touchScale, touchMarginX, touchMarginY, touchHaptics, touchDeadzone, showNumKeys)

    /** Keep the shape the game was drawn for, with bands at the sides, instead of filling the screen. */
    var keepAspect: Boolean
        get() = prefs.getBoolean("keepAspect", false)
        set(value) {
            prefs.edit().putBoolean("keepAspect", value).apply()
        }

    /** Battery saver mode to keep CPU and GPU cool on prolonged gaming sessions. */
    var batterySaver: Boolean
        get() = prefs.getBoolean("batterySaver", false)
        set(value) {
            prefs.edit().putBoolean("batterySaver", value).apply()
        }

    /** Theme mode: "system", "light", "dark", "amoled" */
    var themeMode: String
        get() = prefs.getString("themeMode", "system") ?: "system"
        set(value) {
            prefs.edit().putString("themeMode", value).apply()
        }

    /** Real-time FPS counter overlay in-game */
    var showFps: Boolean
        get() = prefs.getBoolean("showFps", false)
        set(value) {
            prefs.edit().putBoolean("showFps", value).apply()
        }

    /** Scoped Storage Shield: whether to keep external persistent backups in Documents folder */
    var persistentShield: Boolean
        get() = prefs.getBoolean("persistentShield", true)
        set(value) {
            prefs.edit().putBoolean("persistentShield", value).apply()
        }

    /** Fast-forward speed multiplier (e.g. 1.5f, 2.0f, 4.0f, 6.0f, 8.0f) */
    var turboMultiplier: Float
        get() = prefs.getFloat("turboMultiplier", 2.0f)
        set(value) {
            prefs.edit().putFloat("turboMultiplier", value).apply()
        }

    /** Frame skip mode: "0" (off), "1", "2", "auto" */
    var frameSkip: String
        get() = prefs.getString("frameSkip", "0") ?: "0"
        set(value) {
            prefs.edit().putString("frameSkip", value).apply()
        }

    /** Render scale: "0.75", "1.0", "1.25", "1.5" */
    var renderScale: String
        get() = prefs.getString("renderScale", "1.0") ?: "1.0"
        set(value) {
            prefs.edit().putString("renderScale", value).apply()
        }

    /** Video filter / shader: "sharp", "nearest", "crt", "fxaa" */
    var videoFilter: String
        get() = prefs.getString("videoFilter", "sharp") ?: "sharp"
        set(value) {
            prefs.edit().putString("videoFilter", value).apply()
        }

    /** Weather & Particle optimization: "normal", "optimized", "minimal" */
    var weatherOpt: String
        get() = prefs.getString("weatherOpt", "normal") ?: "normal"
        set(value) {
            prefs.edit().putString("weatherOpt", value).apply()
        }

    /** Fast-forward speed multiplier in tenths: 15 for 1.5x, 20 for 2.0x (legacy compatibility) */
    var fastForwardSpeed: Int
        get() = prefs.getInt("fastForwardSpeed", (turboMultiplier * 10).toInt())
        set(value) {
            prefs.edit().putInt("fastForwardSpeed", value).apply()
            turboMultiplier = value / 10.0f
        }

    /** Video scaling mode: "aspect", "full", or "pixel_perfect". */
    var videoScaling: String
        get() = prefs.getString("videoScaling", if (keepAspect) "aspect" else "full") ?: "aspect"
        set(value) {
            prefs.edit().putString("videoScaling", value).apply()
            keepAspect = (value == "aspect" || value == "pixel_perfect")
        }

    /** Bezel art style for 4:3 mode: "oled", "retro_slate", "reloaded_blue" */
    var videoBezel: String
        get() = prefs.getString("videoBezel", "oled") ?: "oled"
        set(value) {
            prefs.edit().putString("videoBezel", value).apply()
        }

    /** Mirrors the video mode into the file the emulator reads at startup. */
    fun writeVideo(filesDir: File) = writeVideoFile(
        filesDir = filesDir,
        scaling = videoScaling,
        showFps = showFps,
        bezel = videoBezel,
        turboSpeed = turboMultiplier,
        frameSkip = frameSkip,
        renderScale = renderScale,
        filter = videoFilter,
        weatherOpt = weatherOpt
    )
}
