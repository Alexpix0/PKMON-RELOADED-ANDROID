package com.klisee.reloaded

import android.app.NativeActivity
import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.os.Build
import android.os.Bundle
import android.text.InputType
import android.util.Log
import android.view.KeyEvent
import android.view.View
import android.view.ViewGroup
import android.view.WindowManager
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputMethodManager
import android.widget.EditText
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat

private const val TAG = "GameActivity"

/**
 * The emulator activity: a plain `NativeActivity` (that is what loads `libgm8emulator.so`),
 * subclassed to hide the system bars, support display cutout and honour the launcher's screen setting.
 */
class GameActivity : NativeActivity() {
    private val touchFeedback by lazy { TouchFeedback(this) }
    private val touchPreferences by lazy { Preferences(this) }
    private val audioManager by lazy { getSystemService(Context.AUDIO_SERVICE) as AudioManager }
    private var focusRequest: AudioFocusRequest? = null

    /**
     * Another app took the audio focus (a call, a notification, a video): the game pauses its output
     * instead of talking over it, and resumes when the focus comes back.
     */
    private val focusListener = AudioManager.OnAudioFocusChangeListener { change ->
        when (change) {
            AudioManager.AUDIOFOCUS_LOSS, AudioManager.AUDIOFOCUS_LOSS_TRANSIENT -> nativeAudioFocus(true)
            AudioManager.AUDIOFOCUS_GAIN -> nativeAudioFocus(false)
        }
    }

    override fun onResume() {
        super.onResume()
        // Ask once the window is actually on screen: some Android versions reject a focus request
        // made during onResume, and a denied request must never take the game down with it.
        window.decorView.post { requestAudioFocus() }
    }

    private fun requestAudioFocus() {
        if (focusRequest != null) return
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            runCatching {
                val request = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
                    .setAudioAttributes(
                        AudioAttributes.Builder()
                            .setUsage(AudioAttributes.USAGE_GAME)
                            .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                            .build(),
                    )
                    .setOnAudioFocusChangeListener(focusListener)
                    .build()
                if (audioManager.requestAudioFocus(request) == AudioManager.AUDIOFOCUS_REQUEST_GRANTED) {
                    focusRequest = request
                }
            }
        }
    }

    override fun onPause() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            runCatching { focusRequest?.let { audioManager.abandonAudioFocusRequest(it) } }
        }
        focusRequest = null
        super.onPause()
    }

    private var imeAnchor: EditText? = null

    private val batteryReceiver = object : android.content.BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: android.content.Intent?) {
            if (intent?.action == android.content.Intent.ACTION_BATTERY_LOW) {
                emergencySaveFlush()
            }
        }
    }

    private fun emergencySaveFlush() {
        Thread {
            try {
                Log.w(TAG, "ACTION_BATTERY_LOW recibido: iniciando Emergency Save Flush...")
                val gameDir = java.io.File(filesDir, "game")
                val partidasDir = java.io.File(gameDir, "data/partidas")
                val backupEmergency = java.io.File(filesDir, "backups_emergency").apply { mkdirs() }

                val savFiles = mutableListOf<java.io.File>()
                if (partidasDir.isDirectory) {
                    partidasDir.listFiles()?.filter { it.isFile && it.name.endsWith(".sav", ignoreCase = true) }?.let {
                        savFiles.addAll(it)
                    }
                }
                val dataDir = java.io.File(gameDir, "data")
                if (dataDir.isDirectory) {
                    dataDir.listFiles()?.filter { it.isFile && it.name.endsWith(".sav", ignoreCase = true) }?.let {
                        savFiles.addAll(it)
                    }
                }

                for (sav in savFiles) {
                    try {
                        java.io.FileOutputStream(sav, true).use { fos ->
                            fos.fd.sync()
                        }
                        val dest = java.io.File(backupEmergency, sav.name)
                        sav.copyTo(dest, overwrite = true)
                        Log.i(TAG, "Emergency flush & mirror exitoso: ${sav.name} (${sav.length()} bytes)")
                    } catch (e: Throwable) {
                        Log.e(TAG, "Error al sincronizar ${sav.name}: ${e.message}")
                    }
                }
                Log.w(TAG, "Emergency Save Flush completado exitosamente.")
            } catch (e: Throwable) {
                Log.e(TAG, "Fallo en emergencySaveFlush: ${e.message}")
            }
        }.start()
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        // Draw behind the notch/camera hole. Without this the system insets the window on the short
        // edge and the game is left with a black band beside it instead of filling the screen.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            val attrs = window.attributes
            attrs.layoutInDisplayCutoutMode =
                WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
            window.attributes = attrs
        }
        super.onCreate(savedInstanceState)
        instance = this
        if (Preferences(applicationContext).keepScreenOn) {
            window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        }
        setVsyncEnabled(touchPreferences.vsyncEnabled)
        setAdaptiveFrameskip(touchPreferences.adaptiveFrameskip)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            try {
                val pm = getSystemService(Context.POWER_SERVICE) as? android.os.PowerManager
                if (pm?.isSustainedPerformanceModeSupported == true) {
                    window.setSustainedPerformanceMode(true)
                    Log.i(TAG, "Sustained performance mode activado para sesiones prolongadas")
                }
            } catch (e: Throwable) {
                Log.w(TAG, "No se pudo activar Sustained Performance Mode en este dispositivo: ${e.message}")
            }
        }
        try {
            registerReceiver(batteryReceiver, android.content.IntentFilter(android.content.Intent.ACTION_BATTERY_LOW))
        } catch (e: Throwable) {
            Log.w(TAG, "No se pudo registrar batteryReceiver: ${e.message}")
        }
        installImeAnchor()
        watchImeHeight()
        hideSystemBars()
    }

    override fun onDestroy() {
        try {
            unregisterReceiver(batteryReceiver)
        } catch (_: Throwable) {}
        if (instance === this) instance = null
        super.onDestroy()
        // Terminates the isolated :game process so no background native threads stay running
        // and in-memory save state is completely cleared.
        android.os.Process.killProcess(android.os.Process.myPid())
    }

    // The bars come back on some system events, so re-hide them whenever we regain focus.
    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) hideSystemBars()
    }

    private fun hideSystemBars() {
        WindowCompat.setDecorFitsSystemWindows(window, false)
        val controller = WindowInsetsControllerCompat(window, window.decorView)
        controller.systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        controller.hide(WindowInsetsCompat.Type.systemBars())
        window.addFlags(WindowManager.LayoutParams.FLAG_FULLSCREEN)
        window.decorView.postDelayed(
            { WindowInsetsControllerCompat(window, window.decorView).hide(WindowInsetsCompat.Type.systemBars()) },
            400,
        )
    }

    // ------------------------------------------------------------------------------------------
    // System keyboard. The game only asks for text through its own modal and has no Android view of
    // its own, so this tiny transparent view exists only to be the target the keyboard attaches to.
    // Nothing typed is kept in it: a custom InputConnection forwards each character to the engine,
    // which is the one that decides when to raise the keyboard (it sees the game's dialog open).

    private fun installImeAnchor() {
        val anchor = object : EditText(this) {
            override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection {
                outAttrs.inputType = InputType.TYPE_CLASS_TEXT or 
                    InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD or 
                    InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
                // No fullscreen/extract editor and no suggestion strip: in landscape the system
                // would otherwise cover the game with its own text box.
                outAttrs.imeOptions = EditorInfo.IME_ACTION_DONE or
                    EditorInfo.IME_FLAG_NO_EXTRACT_UI or
                    EditorInfo.IME_FLAG_NO_FULLSCREEN
                return ForwardingInputConnection(this)
            }
        }
        anchor.isFocusable = true
        anchor.isFocusableInTouchMode = true
        anchor.alpha = 0f
        addContentView(anchor, ViewGroup.LayoutParams(1, 1))
        imeAnchor = anchor
    }

    /** Sends what is typed to the engine instead of leaving it in the view. */
    private inner class ForwardingInputConnection(target: View) : BaseInputConnection(target, false) {
        override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
            if (!text.isNullOrEmpty()) nativeCommitText(text.toString())
            return true
        }

        override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
            repeat(beforeLength) { nativeBackspace() }
            return true
        }

        override fun sendKeyEvent(event: KeyEvent): Boolean {
            when (event.keyCode) {
                KeyEvent.KEYCODE_DEL -> {
                    nativeBackspace()
                    return true
                }
                KeyEvent.KEYCODE_ENTER -> {
                    nativeEnter()
                    return true
                }
            }
            return super.sendKeyEvent(event)
        }

        override fun performEditorAction(actionCode: Int): Boolean {
            nativeEnter()
            return true
        }
    }

    /**
     * Keeps the engine told of the system keyboard's height, so it can fit the game above it instead
     * of leaving what is being typed hidden under the keyboard.
     */
    private fun watchImeHeight() {
        ViewCompat.setOnApplyWindowInsetsListener(window.decorView) { _, insets ->
            reportImeHeight(insets.getInsets(WindowInsetsCompat.Type.ime()).bottom)
            insets
        }
    }

    private fun currentImeHeight(): Int =
        ViewCompat.getRootWindowInsets(window.decorView)
            ?.getInsets(WindowInsetsCompat.Type.ime())?.bottom ?: 0

    private fun reportImeHeight(bottom: Int) {
        Log.i(TAG, "altura del teclado = $bottom")
        nativeImeInset(bottom)
    }

    private fun applyImeVisible(visible: Boolean) {
        val anchor = imeAnchor ?: return
        val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
        Log.i(TAG, "teclado visible = $visible")
        if (visible) {
            anchor.requestFocus()
            imm.showSoftInput(anchor, 0)
            // The keyboard animates in, so its height is not final on the first frame; ask again once
            // it has settled. If it is still not up by then, the system ignored the first request
            // (it happens when nothing was touched), so ask once more.
            anchor.postDelayed({
                if (currentImeHeight() == 0) imm.showSoftInput(anchor, 0)
                reportImeHeight(currentImeHeight())
            }, 400)
        } else {
            imm.hideSoftInputFromWindow(anchor.windowToken, 0)
            anchor.clearFocus()
        }
    }

    external fun nativeCommitText(text: String)
    external fun nativeBackspace()
    external fun nativeEnter()
    external fun nativeImeInset(height: Int)
    external fun nativeAudioFocus(lost: Boolean)
    external fun nativeReloadLayout(profile: Int)
    external fun nativeBackendInfo(): String
    external fun nativeSetVsyncEnabled(enabled: Boolean)
    external fun nativeSetAdaptiveFrameskip(enabled: Boolean)

    companion object {
        init {
            // These are declared as `native` methods on this class, so the JVM has to find their
            // `Java_com_klisee_reloaded_GameActivity_*` symbols in the library. The emulator loads
            // the same one through the manifest, and this only links it to the app's class loader.
            System.loadLibrary("gm8emulator")
        }

        private var instance: GameActivity? = null

        /** Applies runtime VSync setting to native engine */
        @JvmStatic
        fun setVsyncEnabled(enabled: Boolean) {
            runCatching { NativeBridge.setVsyncEnabled(enabled) }
            runCatching { instance?.nativeSetVsyncEnabled(enabled) }
        }

        /** Applies runtime Adaptive Frameskip setting to native engine */
        @JvmStatic
        fun setAdaptiveFrameskip(enabled: Boolean) {
            runCatching { NativeBridge.setAdaptiveFrameskip(enabled) }
            runCatching { instance?.nativeSetAdaptiveFrameskip(enabled) }
        }

        /** Returns JSON string with native backend info (requested, actual, fallback, reason). */
        @JvmStatic
        fun getBackendInfo(): String {
            return runCatching { instance?.nativeBackendInfo() }.getOrNull() ?: "{}"
        }

        /** Notifies the running engine to reload control layouts from storage without restarting. */
        @JvmStatic
        fun reloadLayout(profile: Int = 0) {
            runCatching { instance?.nativeReloadLayout(profile) }
        }

        /** Called from the engine when a touch control is pressed, to make it vibrate. */
        @JvmStatic
        fun touchControlPressed() {
            val activity = instance ?: return
            activity.runOnUiThread {
                if (!activity.isFinishing && !activity.isDestroyed && activity.hasWindowFocus()) {
                    activity.touchFeedback.press(activity.window.decorView, activity.touchPreferences.touchAppearance)
                }
            }
        }

        /**
         * Called from the engine over JNI (it only holds the Application context, so it goes through
         * this static). The keyboard has to be raised from the UI thread.
         */
        @JvmStatic
        fun setImeVisible(visible: Boolean) {
            val activity = instance ?: return
            activity.runOnUiThread {
                if (!activity.isFinishing && !activity.isDestroyed) activity.applyImeVisible(visible)
            }
        }

        /**
         * Called from the engine to make it read the keyboard's height again. The height normally
         * reaches it on its own from the insets; this is there so a callback that never arrived
         * cannot leave the game pushed up with its controls hidden.
         */
        @JvmStatic
        fun refreshImeHeight() {
            val activity = instance ?: return
            activity.runOnUiThread {
                if (!activity.isFinishing && !activity.isDestroyed) {
                    activity.reportImeHeight(activity.currentImeHeight())
                }
            }
        }

        @JvmStatic
        fun showTouchHint() {
            val activity = instance ?: return
            activity.runOnUiThread {
                if (activity.isFinishing || activity.isDestroyed) return@runOnUiThread
                com.klisee.reloaded.ui.Strings.init(activity)
                com.klisee.reloaded.ui.Strings.update(Preferences(activity))
                android.widget.Toast.makeText(
                    activity,
                    com.klisee.reloaded.ui.Strings.tr("Toca una pieza y pulsa Girar para rotarla."),
                    android.widget.Toast.LENGTH_LONG,
                ).show()
            }
        }
    }
}
