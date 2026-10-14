package com.klisee.reloaded

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import com.klisee.reloaded.ui.App
import com.klisee.reloaded.ui.ReloadedTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val repo = GameRepository(applicationContext)
        val prefs = Preferences(applicationContext)
        setContent {
            var themeMode by remember { mutableStateOf(prefs.themeMode) }
            ReloadedTheme(themeMode = themeMode) {
                App(repo, prefs, onThemeChange = { themeMode = it })
            }
        }
    }
}
