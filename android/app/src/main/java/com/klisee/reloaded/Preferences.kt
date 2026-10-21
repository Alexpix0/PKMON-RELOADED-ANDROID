package com.klisee.reloaded

import android.content.Context
import androidx.core.util.AtomicFile
import java.io.File
import java.io.IOException

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

    /** How opaque the control artwork is drawn. 1 is solid; lower lets the game show through. */
    var touchOpacity: Float
        get() = try {
            prefs.getFloat("touchOpacity", 0.7f)
        } catch (_: ClassCastException) {
            prefs.getInt("touchOpacity", 70) / 100f
        }
        set(value) {
            prefs.edit().putFloat("touchOpacity", value).apply()
        }

    /** One-shot flag: the classic overlay goes back to its cruceta. */
    var touchPadMigration: Boolean
        get() = prefs.getBoolean("touchPadMigration", false)
        set(value) {
            prefs.edit().putBoolean("touchPadMigration", value).apply()
        }

    /** Which overlay the game draws: the new one, the classic one, or none. */
    var touchStyle: TouchOverlayStyle
        get() = TouchOverlayStyle.from(prefs.getString("touchStyle", null))
        set(value) {
            prefs.edit().putString("touchStyle", value.id).apply()
        }

    /** Whether the arrow keys are driven by the joystick or the D-pad. */
    var moveMode: MoveMode
        get() = MoveMode.from(prefs.getString("moveMode", null))
        set(value) {
            prefs.edit().putString("moveMode", value.id).apply()
        }

    /** The overall size of the new overlay's controls. */
    var touchScale: Float
        get() = try {
            prefs.getFloat("touchScale", 1f)
        } catch (_: ClassCastException) {
            prefs.getInt("touchScale", 100) / 100f
        }
        set(value) {
            prefs.edit().putFloat("touchScale", value).apply()
        }

    var touchMarginX: Int
        get() = prefs.getInt("touchMarginX", 0)
        set(value) {
            prefs.edit().putInt("touchMarginX", value).apply()
        }

    var touchMarginY: Int
        get() = prefs.getInt("touchMarginY", 0)
        set(value) {
            prefs.edit().putInt("touchMarginY", value).apply()
        }

    var touchHaptics: Boolean
        get() = prefs.getBoolean("touchHaptics", true)
        set(value) {
            prefs.edit().putBoolean("touchHaptics", value).apply()
        }

    var touchDeadzone: Int
        get() = prefs.getInt("touchDeadzone", 35)
        set(value) {
            prefs.edit().putInt("touchDeadzone", value).apply()
        }

    var showNumKeys: Boolean
        get() = prefs.getBoolean("showNumKeys", true)
        set(value) {
            prefs.edit().putBoolean("showNumKeys", value).apply()
        }

    /** Where the player moved and resized each control, as `name=cx,cy,scale`. Empty means defaults. */
    var touchLayout: Map<String, Placement>
        get() = prefs.getStringSet("touchLayout", emptySet()).orEmpty().mapNotNull { entry ->
            val parts = entry.split('=')
            if (parts.size != 2) return@mapNotNull null
            val numbers = parts[1].split(',').mapNotNull { it.toFloatOrNull() }
            if (numbers.size < 2) return@mapNotNull null
            parts[0] to Placement(numbers[0], numbers[1], numbers.getOrElse(2) { 1f })
        }.toMap()
        set(value) {
            prefs.edit()
                .putStringSet("touchLayout", value.map { "${it.key}=${it.value.cx},${it.value.cy},${it.value.scale}" }.toSet())
                .apply()
        }

    /** Mirrors the current pad controls into the file the emulator reads at startup. */
    fun writeControls(filesDir: File, padConnected: Boolean) =
        writeControlsFile(filesDir, controlBindings, stickEnabled, padConnected)

    /** Mirrors the touch overlay into the file the emulator reads at startup. */
    fun writeTouch(filesDir: File) = writeTouchFile(filesDir, touchStyle, moveMode, touchScale, touchOpacity, touchLayout)

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

    /** Graphics backend: "opengl" or "vulkan" */
    var graphicsBackend: String
        get() = prefs.getString("graphicsBackend", "opengl") ?: "opengl"
        set(value) {
            prefs.edit().putString("graphicsBackend", value).apply()
        }

    /** Force unlock 60 FPS in 30 FPS capped rooms and menus */
    var force60Fps: Boolean
        get() = prefs.getBoolean("force60Fps", false)
        set(value) {
            prefs.edit().putBoolean("force60Fps", value).apply()
        }

    /** Background music (BGM) volume percentage: 0 to 100 */
    var bgmVolume: Int
        get() = prefs.getInt("bgmVolume", 100)
        set(value) {
            prefs.edit().putInt("bgmVolume", value).apply()
        }

    /** Sound effects (SFX) volume percentage: 0 to 100 */
    var sfxVolume: Int
        get() = prefs.getInt("sfxVolume", 100)
        set(value) {
            prefs.edit().putInt("sfxVolume", value).apply()
        }

    /** Autofire: continuous rapid press on holding action buttons (Z/X/dialogo) */
    var autofireEnabled: Boolean
        get() = prefs.getBoolean("autofireEnabled", true)
        set(value) {
            prefs.edit().putBoolean("autofireEnabled", value).apply()
        }

    /** Dynamic Floating D-Pad: joystick anchors where thumb touches */
    var dynamicDpad: Boolean
        get() = prefs.getBoolean("dynamicDpad", false)
        set(value) {
            prefs.edit().putBoolean("dynamicDpad", value).apply()
        }

    // ------------------------------------------------------------------------------------------
    // Ajustes -> Juego (Comodidad Externa y Opcional)

    /** Reloj de Cosecha de Bayas: aviso al completar 10 minutos de juego activo */
    var berryTracker: Boolean
        get() = prefs.getBoolean("berryTracker", false)
        set(value) {
            prefs.edit().putBoolean("berryTracker", value).apply()
        }

    /** Cronómetro de Movimiento para Huevos: mide segundos pedaleando o corriendo */
    var eggHatchTimer: Boolean
        get() = prefs.getBoolean("eggHatchTimer", false)
        set(value) {
            prefs.edit().putBoolean("eggHatchTimer", value).apply()
        }

    /** Rueda radial de 4 ranuras para objetos clave favoritos */
    var keyItemsRadial: Boolean
        get() = prefs.getBoolean("keyItemsRadial", false)
        set(value) {
            prefs.edit().putBoolean("keyItemsRadial", value).apply()
        }

    /** Token de Acceso Personal (PAT) de GitHub para repositorios privados */
    var githubToken: String
        get() = prefs.getString("githubToken", "") ?: ""
        set(value) {
            prefs.edit().putString("githubToken", value.trim()).apply()
        }

    /** Modo Auto-Run / Correr Permanente */
    var autoRun: Boolean
        get() = prefs.getBoolean("autoRun", false)
        set(value) {
            prefs.edit().putBoolean("autoRun", value).apply()
        }

    /** Avance de Diálogos por Toque en Pantalla */
    var dialogTapAdvance: Boolean
        get() = prefs.getBoolean("dialogTapAdvance", false)
        set(value) {
            prefs.edit().putBoolean("dialogTapAdvance", value).apply()
        }

    /** Auto-Recordatorio de Repelente agotado */
    var repelReminder: Boolean
        get() = prefs.getBoolean("repelReminder", false)
        set(value) {
            prefs.edit().putBoolean("repelReminder", value).apply()
        }

    /** Muestra en Discord que se está jugando y en qué ruta. */
    var discordEnabled: Boolean
        get() = prefs.getBoolean("discordEnabled", false)
        set(value) {
            prefs.edit().putBoolean("discordEnabled", value).apply()
        }

    var discordWarningDismissed: Boolean
        get() = prefs.getBoolean("discordWarningDismissed", false)
        set(value) {
            prefs.edit().putBoolean("discordWarningDismissed", value).apply()
        }

    var trainerNameLimit: Int
        get() = prefs.getInt("trainerNameLimit", 0)
        set(value) {
            prefs.edit().putInt("trainerNameLimit", value).apply()
        }

    /** Alerta Visual Perimetral de Pokémon Variocolor (Shiny Beacon) */
    var shinyBeacon: Boolean
        get() = prefs.getBoolean("shinyBeacon", false)
        set(value) {
            prefs.edit().putBoolean("shinyBeacon", value).apply()
        }

    /** Pausa temporal de encuentros salvajes en salas de puzles */
    var puzzlePause: Boolean
        get() = prefs.getBoolean("puzzlePause", false)
        set(value) {
            prefs.edit().putBoolean("puzzlePause", value).apply()
        }

    /** Asistente Háptico de Picada de Pesca (vibración en el teléfono) */
    var fishingHaptic: Boolean
        get() = prefs.getBoolean("fishingHaptic", false)
        set(value) {
            prefs.edit().putBoolean("fishingHaptic", value).apply()
        }

    /** Guardado Rápido Externo en 1 Toque (Quick Save / Quick Load) */
    var quickSave: Boolean
        get() = prefs.getBoolean("quickSave", false)
        set(value) {
            prefs.edit().putBoolean("quickSave", value).apply()
        }

    /** Recordatorio Periódico de Guardado Preventivo */
    var saveReminder: Boolean
        get() = prefs.getBoolean("saveReminder", false)
        set(value) {
            prefs.edit().putBoolean("saveReminder", value).apply()
        }

    // ------------------------------------------------------------------------------------------
    // Ajustes v0.9.0 ("El Salto Generacional y Magia Negra")

    /** Perfil Combate Extremo Mali/Infinix (0.85x scale, CAS 60%, 40 particles, Alpha-Test Discard) */
    var extremeCombat: Boolean
        get() = prefs.getBoolean("extremeCombat", false)
        set(value) {
            val editor = prefs.edit().putBoolean("extremeCombat", value)
            if (value) {
                editor.putBoolean("drsEnabled", false)
                editor.putInt("fsrMode", 0)
                editor.putString("renderScale", "0.85")
            }
            editor.apply()
        }

    /** Alpha-Test Early-Z FPK para GPUs ARM Mali */
    var maliEarlyZ: Boolean
        get() = prefs.getBoolean("maliEarlyZ", false)
        set(value) {
            prefs.edit().putBoolean("maliEarlyZ", value).apply()
        }

    /** Despertador de Frecuencia GPU (Bypass downclock MediaTek) */
    var gpuWakeup: Boolean
        get() = prefs.getBoolean("gpuWakeup", false)
        set(value) {
            prefs.edit().putBoolean("gpuWakeup", value).apply()
        }

    /** Salto Rápido de Intro de Batalla (Smart Pre-Roll Skip) */
    var smartPrerollSkip: Boolean
        get() = prefs.getBoolean("smartPrerollSkip", false)
        set(value) {
            prefs.edit().putBoolean("smartPrerollSkip", value).apply()
        }

    /** Refresco Adaptativo LTPO / VRR (10 Hz a 120 Hz) */
    var adaptiveLtpo: Boolean
        get() = prefs.getBoolean("adaptiveLtpo", false)
        set(value) {
            prefs.edit().putBoolean("adaptiveLtpo", value).apply()
        }

    /** Modo Ventana Flotante / Picture-in-Picture (PiP) */
    var pipMode: Boolean
        get() = prefs.getBoolean("pipMode", false)
        set(value) {
            prefs.edit().putBoolean("pipMode", value).apply()
        }

    /** Notificación Dinámica en Segundo Plano (Live Activity) */
    var liveActivity: Boolean
        get() = prefs.getBoolean("liveActivity", false)
        set(value) {
            prefs.edit().putBoolean("liveActivity", value).apply()
        }

    /** Partición de Instancias Activas vs Estáticas */
    var activeStaticPartition: Boolean
        get() = prefs.getBoolean("activeStaticPartition", false)
        set(value) {
            prefs.edit().putBoolean("activeStaticPartition", value).apply()
        }

    /** Mega-Atlas Virtual y Fusión Zero-Draw-Call */
    var megaAtlas: Boolean
        get() = prefs.getBoolean("megaAtlas", false)
        set(value) {
            prefs.edit().putBoolean("megaAtlas", value).apply()
        }

    /** Perfil AMD FidelityFX FSR 1.0 (0=Off, 1=Ultra Calidad, 2=Equilibrado, 3=Rendimiento) */
    var fsrMode: Int
        get() = prefs.getInt("fsrMode", 0)
        set(value) {
            val editor = prefs.edit().putInt("fsrMode", value)
            if (value > 0) {
                editor.putBoolean("extremeCombat", false)
                editor.putBoolean("drsEnabled", false)
            }
            editor.apply()
        }

    /** Resolución Dinámica DRS con Bucle PID Continuo */
    var drsEnabled: Boolean
        get() = prefs.getBoolean("drsEnabled", false)
        set(value) {
            val editor = prefs.edit().putBoolean("drsEnabled", value)
            if (value) {
                editor.putBoolean("extremeCombat", false)
                editor.putInt("fsrMode", 0)
            }
            editor.apply()
        }

    /** Super-Resolución Neuronal xBRZ 4x/6x HD */
    var xbrzEnabled: Boolean
        get() = prefs.getBoolean("xbrzEnabled", false)
        set(value) {
            prefs.edit().putBoolean("xbrzEnabled", value).apply()
        }

    /** Iluminación 2D Dinámica y Sombras por Ataques */
    var lighting2d: Boolean
        get() = prefs.getBoolean("lighting2d", false)
        set(value) {
            prefs.edit().putBoolean("lighting2d", value).apply()
        }

    /** Audio Espacial 3D Binaural HRTF */
    var audio3d: Boolean
        get() = prefs.getBoolean("audio3d", false)
        set(value) {
            prefs.edit().putBoolean("audio3d", value).apply()
        }

    /** Música Reactiva de Combate (Stems de Tensión) */
    var reactiveStems: Boolean
        get() = prefs.getBoolean("reactiveStems", false)
        set(value) {
            prefs.edit().putBoolean("reactiveStems", value).apply()
        }

    /** Preservación de Tono en Modo Turbo (WSOLA) */
    var wsola: Boolean
        get() = prefs.getBoolean("wsola", false)
        set(value) {
            prefs.edit().putBoolean("wsola", value).apply()
        }

    /** Filtro de Accesibilidad Cromática / Daltonismo (0=Off, 1=Protanopia, 2=Deuteranopia, 3=Tritanopia) */
    var colorblindMode: Int
        get() = prefs.getInt("colorblindMode", 0)
        set(value) {
            prefs.edit().putInt("colorblindMode", value).apply()
        }

    /** Rueda Táctica Radial de Combate (Doom/GTA) */
    var radialMenu: Boolean
        get() = prefs.getBoolean("radialMenu", false)
        set(value) {
            prefs.edit().putBoolean("radialMenu", value).apply()
        }

    /** Gestos Multitáctiles Inteligentes (Rewind, Zoom, Turbo) */
    var gestures: Boolean
        get() = prefs.getBoolean("gestures", false)
        set(value) {
            prefs.edit().putBoolean("gestures", value).apply()
        }

    /** Disposición Ergonómica a Una Mano (Arco para Pulgar) */
    var oneHanded: Boolean
        get() = prefs.getBoolean("oneHanded", false)
        set(value) {
            prefs.edit().putBoolean("oneHanded", value).apply()
        }

    /** Grabador de Repeticiones Deterministas (< 30 KB) */
    var replayBlackbox: Boolean
        get() = prefs.getBoolean("replayBlackbox", false)
        set(value) {
            prefs.edit().putBoolean("replayBlackbox", value).apply()
        }

    /** Sistema de Logros Retro con Banners Animados */
    var achievements: Boolean
        get() = prefs.getBoolean("achievements", false)
        set(value) {
            prefs.edit().putBoolean("achievements", value).apply()
        }

    /** Compilador JIT Tier-2 Nativo ARM64 */
    var jitArm64: Boolean
        get() = prefs.getBoolean("jitArm64", true)
        set(value) {
            prefs.edit().putBoolean("jitArm64", value).apply()
        }

    /** Afinidad Big.LITTLE de CPU */
    var bigLittleAffinity: Boolean
        get() = prefs.getBoolean("bigLittleAffinity", true)
        set(value) {
            prefs.edit().putBoolean("bigLittleAffinity", value).apply()
        }

    /** Estilo de Interfaz del Emulador ("classic", "glassmorphic") */
    var uiStyle: String
        get() = prefs.getString("uiStyle", "glassmorphic") ?: "glassmorphic"
        set(value) {
            prefs.edit().putString("uiStyle", value).apply()
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
        weatherOpt = weatherOpt,
        backend = graphicsBackend,
        force60Fps = force60Fps,
        bgmVolume = bgmVolume,
        sfxVolume = sfxVolume,
        autofire = autofireEnabled,
        dynamicDpad = dynamicDpad,
        berryTracker = berryTracker,
        eggHatchTimer = eggHatchTimer,
        keyItemsRadial = keyItemsRadial,
        autoRun = autoRun,
        dialogTapAdvance = dialogTapAdvance,
        repelReminder = repelReminder,
        shinyBeacon = shinyBeacon,
        puzzlePause = puzzlePause,
        fishingHaptic = fishingHaptic,
        quickSave = quickSave,
        saveReminder = saveReminder,
        extremeCombat = extremeCombat,
        maliEarlyZ = maliEarlyZ,
        gpuWakeup = gpuWakeup,
        smartPrerollSkip = smartPrerollSkip,
        adaptiveLtpo = adaptiveLtpo,
        pipMode = pipMode,
        liveActivity = liveActivity,
        activeStaticPartition = activeStaticPartition,
        megaAtlas = megaAtlas,
        fsrMode = fsrMode,
        drsEnabled = drsEnabled,
        xbrzEnabled = xbrzEnabled,
        lighting2d = lighting2d,
        audio3d = audio3d,
        reactiveStems = reactiveStems,
        wsola = wsola,
        colorblindMode = colorblindMode,
        radialMenu = radialMenu,
        gestures = gestures,
        floatingDpad = dynamicDpad,
        oneHanded = oneHanded,
        replayBlackbox = replayBlackbox,
        achievements = achievements,
        jitArm64 = jitArm64,
        bigLittleAffinity = bigLittleAffinity
    )

    /**
     * Aplica un perfil de rendimiento rápido de 1 toque:
     * - "eco": Límite a 30 FPS, escala 0.75x, clima mínimo, ahorro de batería.
     * - "balanced": 60 FPS fijos, escala 1.0x, filtro nítido, clima optimizado.
     * - "extreme": 60 FPS, Vulkan activo, turbo a 3.0x, escala 1.0x nativa.
     */
    fun applyPerformanceProfile(profile: String) {
        when (profile.lowercase()) {
            "eco", "ahorro" -> {
                frameSkip = "1"
                renderScale = "0.75"
                weatherOpt = "minimal"
                force60Fps = false
                turboMultiplier = 1.5f
                batterySaver = true
            }
            "balanced", "equilibrado" -> {
                frameSkip = "auto"
                renderScale = "1.0"
                weatherOpt = "optimized"
                videoFilter = "sharp"
                force60Fps = false
                turboMultiplier = 2.0f
                batterySaver = false
            }
            "extreme", "extremo" -> {
                frameSkip = "0"
                renderScale = "1.0"
                weatherOpt = "normal"
                videoFilter = "sharp"
                graphicsBackend = "vulkan"
                force60Fps = true
                turboMultiplier = 3.0f
                batterySaver = false
            }
            "extreme_combat", "infinix", "combate_extremo" -> {
                frameSkip = "0"
                renderScale = "0.85"
                weatherOpt = "minimal"
                videoFilter = "sharp"
                force60Fps = true
                extremeCombat = true
                maliEarlyZ = true
                gpuWakeup = true
                fsrMode = 0
                drsEnabled = false
                activeStaticPartition = true
                batterySaver = false
            }
        }
    }


    // --- Controles dinámicos (edición Controles) ---------------------------------------------

    /** Opacity of the classic overlay, as a percentage (5..100). */
    var touchOpacityPercent: Int
        get() = kotlin.math.round(touchOpacity * 100f).toInt()
        set(value) {
            touchOpacity = value.coerceIn(5, 100) / 100f
        }

    var activeUserProfile: Int
        get() = prefs.getInt("activeUserProfile", 0).coerceIn(0, 3)
        set(value) {
            prefs.edit().putInt("activeUserProfile", value.coerceIn(0, 3)).apply()
        }

    fun touchLayouts(userProfile: Int = activeUserProfile): Map<String, TouchPlacement> {
        val key = if (userProfile == 0) "touchLayoutV1" else "touchLayoutV1_p${userProfile + 1}"
        val saved = prefs.getString(key, null)
        if (saved != null) {
            return TouchLayouts.parse(saved)
        }
        return TouchLayouts.parse(prefs.getString("touchLayoutV1", TouchLayouts.HEADER).orEmpty())
    }

    /** Called on an IO dispatcher by Save and exit; a failed save is reported to the editor. */
    fun saveTouchLayouts(values: Map<String, TouchPlacement>, filesDir: File, userProfile: Int = activeUserProfile) {
        val text = TouchLayouts.encode(values)
        val key = if (userProfile == 0) "touchLayoutV1" else "touchLayoutV1_p${userProfile + 1}"
        if (!prefs.edit().putString(key, text).commit()) {
            throw IOException("No se pudo guardar la configuración. Inténtalo nuevamente.")
        }
        writeTouchLayoutText(filesDir, text, userProfile)
    }

    fun writeTouchLayouts(filesDir: File) = runCatching {
        val current = activeUserProfile
        writeTouchLayoutText(filesDir, TouchLayouts.encode(touchLayouts(current)), current)
        for (p in 0..3) {
            val key = if (p == 0) "touchLayoutV1" else "touchLayoutV1_p${p + 1}"
            val text = prefs.getString(key, null) ?: TouchLayouts.encode(touchLayouts(0))
            writeAtomicText(File(filesDir, "touch-layout-p${p + 1}.conf"), text)
        }
    }

    var touchAppearance: TouchAppearance
        get() = TouchAppearance.parse(prefs.getString("touchAppearanceV1", "").orEmpty())
        set(value) {
            prefs.edit().putString("touchAppearanceV1", value.encode()).apply()
        }

    fun writeTouchAppearance(filesDir: File) = runCatching {
        writeAtomicText(File(filesDir, "touch-appearance.conf"), touchAppearance.encode())
    }

    val touchSprites: Map<String, String>
        get() = TouchSprites.parse(prefs.getString("touchSpritesV1", "{}").orEmpty())

    fun saveTouchSprites(values: Map<String, String>) {
        if (!prefs.edit().putString("touchSpritesV1", TouchSprites.encode(values)).commit()) {
            throw IOException("No se pudieron guardar los sprites.")
        }
    }

    fun saveTouchConfiguration(
        layouts: Map<String, TouchPlacement>,
        appearance: TouchAppearance,
        opacity: Int,
        sprites: Map<String, String>,
    ) {
        if (!prefs.edit().putString("touchLayoutV1", TouchLayouts.encode(layouts))
                .putString("touchAppearanceV1", appearance.encode())
                .putFloat("touchOpacity", opacity.coerceIn(5, 100) / 100f)
                .putString("touchSpritesV1", TouchSprites.encode(sprites)).commit()
        ) {
            throw IOException("No se pudo guardar el paquete de controles.")
        }
    }

    private fun writeTouchLayoutText(filesDir: File, text: String, userProfile: Int = 0) {
        writeAtomicText(File(filesDir, "touch-layout.conf"), text)
        writeAtomicText(File(filesDir, "touch-layout-p${userProfile + 1}.conf"), text)
    }

    private fun writeAtomicText(target: File, text: String) {
        val file = AtomicFile(target)
        val stream = file.startWrite()
        try {
            stream.write(text.toByteArray(Charsets.UTF_8))
            file.finishWrite(stream)
        } catch (error: Exception) {
            file.failWrite(stream)
            throw error
        }
    }
}
