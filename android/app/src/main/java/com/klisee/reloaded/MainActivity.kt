package com.klisee.reloaded

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import com.klisee.reloaded.ui.App
import com.klisee.reloaded.ui.ReloadedTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val repo = GameRepository(applicationContext)
        val prefs = Preferences(applicationContext)
        setContent {
            ReloadedTheme { App(repo, prefs) }
        }
    }
}
