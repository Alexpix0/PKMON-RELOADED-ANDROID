package com.klisee.reloaded.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Brush
import androidx.compose.material.icons.filled.Favorite
import androidx.compose.material.icons.filled.Memory
import androidx.compose.material.icons.filled.Person
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

import androidx.compose.material.icons.filled.SystemUpdate
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import com.klisee.reloaded.AppInfo
import com.klisee.reloaded.Update
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AboutScreen(onBack: () -> Unit, onUpdateFound: (Update.Release) -> Unit = {}) {
    val snackbar = remember { SnackbarHostState() }
    val scope = rememberCoroutineScope()
    var checking by remember { mutableStateOf(false) }

    Scaffold(
        snackbarHost = { SnackbarHost(snackbar) },
        topBar = {
            TopAppBar(
                title = { Text("Acerca de") },
                navigationIcon = {
                    IconButton(onClick = onBack) { Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Volver") }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState()).padding(horizontal = 24.dp),
            verticalArrangement = Arrangement.spacedBy(24.dp),
        ) {
            Spacer(Modifier.height(4.dp))

            Text(
                text = "Pokemon Reloaded",
                style = MaterialTheme.typography.headlineMedium,
                modifier = Modifier.padding(horizontal = 4.dp),
            )
            Text(
                text = "Port no oficial para Android del fangame, hecho con OpenGMK. No emula Windows: lee los datos del juego y los ejecuta directamente.",
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(horizontal = 4.dp),
            )

            SettingsGroup("Quien lo hizo") {
                InfoRow(Icons.Filled.Person, "El juego", "Fer Productions", items = 4, index = 0)
                InfoRow(Icons.Filled.Memory, "El motor", "OpenGMK, reimplementacion en Rust", items = 4, index = 1)
                InfoRow(Icons.Filled.Favorite, "Port Android", "Comunidad, sin animo de lucro", items = 4, index = 2)
                InfoRow(Icons.Filled.Favorite, "Edicion EX", "v${AppInfo.VERSION_NAME} EX (Alexpix0 / Armando)", items = 4, index = 3)
            }

            SettingsGroup("Licencias y avisos") {
                InfoRow(Icons.Filled.Brush, "Tipografia", "Google Sans Flex, bajo licencia OFL", items = 2, index = 0)
                InfoRow(
                    icon = Icons.Filled.Memory,
                    title = "Marcas",
                    value = "Pokemon es de Nintendo, Game Freak y Creatures. Este proyecto no esta afiliado ni respaldado por ellos.",
                    items = 2,
                    index = 1,
                )
            }

            SettingsGroup("Versión y Actualizaciones") {
                val context = androidx.compose.ui.platform.LocalContext.current
                val version = remember(context) {
                    runCatching {
                        val info = context.packageManager.getPackageInfo(context.packageName, 0)
                        "${info.versionName} (${info.versionCode})"
                    }.getOrDefault("0.9.4 EX (33)")
                }
                InfoRow(Icons.Filled.Memory, "Launcher", version, items = 2, index = 0)
                NavRow(
                    icon = Icons.Filled.SystemUpdate,
                    title = "🔄 Buscar actualizaciones de versión (GitHub)",
                    subtitle = "Instalada: v${AppInfo.VERSION_NAME} (build 33) · Toca para comprobar GitHub Releases",
                    items = 2,
                    index = 1,
                    enabled = !checking,
                    onClick = {
                        checking = true
                        scope.launch {
                            val result = Update.check(AppInfo.VERSION_NAME)
                            checking = false
                            val found = result.getOrNull()
                            if (found != null) {
                                onUpdateFound(found)
                            } else {
                                snackbar.showSnackbar("¡Estás al día! Tienes la última versión instalada (v${AppInfo.VERSION_NAME} EX).")
                            }
                        }
                    },
                )
            }

            Spacer(Modifier.height(8.dp))
        }
    }
}
