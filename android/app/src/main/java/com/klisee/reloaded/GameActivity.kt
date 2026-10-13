package com.klisee.reloaded

import android.app.NativeActivity
import android.os.Bundle
import android.view.WindowManager
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat

/**
 * The emulator activity: a plain `NativeActivity` (that is what loads `libgm8emulator.so`),
 * subclassed to hide the system bars and to honour the launcher's screen setting.
 *
 * Hiding has to be done here rather than from Rust: the Context that `android-activity` publishes
 * for JNI is the *Application*, which has no `getWindow()`, so the native attempt always failed
 * with a NoSuchMethodError and the navigation bar stayed on screen.
 */
class GameActivity : NativeActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if (Preferences(applicationContext).keepScreenOn) {
            window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        }
        hideSystemBars()
    }

    // The bars come back on some system events, so re-hide them whenever we regain focus.
    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) hideSystemBars()
    }

    private fun hideSystemBars() {
        WindowCompat.setDecorFitsSystemWindows(window, false)
        WindowInsetsControllerCompat(window, window.decorView).apply {
            hide(WindowInsetsCompat.Type.systemBars())
            systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        }
    }
}
