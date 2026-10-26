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
import androidx.compose.material.icons.filled.Speed
import androidx.compose.material.icons.filled.Visibility
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.klisee.reloaded.Preferences

/**
 * Pantalla de configuración de tasa de refresco e interpolación (v1.1.7).
 *
 * 30 FPS es la simulación canónica de GM8. Las tasas superiores interpolan
 * el movimiento de instancias y cámara desacoplados en el renderizador a 60/120 Hz.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FpsScreen(prefs: Preferences, onBack: () -> Unit) {
    val context = LocalContext.current
    var renderMode by remember { mutableStateOf(prefs.renderMode) }
    var showFps by remember { mutableStateOf(prefs.showFps) }
    val detectedHz = remember { prefs.detectedRefreshRate() }
    val supports60 = detectedHz >= 59.5f
    val supports120 = detectedHz >= 119.5f

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Tasa de Refresco e Interpolación") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Volver")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .padding(padding)
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(20.dp),
        ) {
            Spacer(Modifier.height(4.dp))

            SwitchRow(
                icon = Icons.Filled.Visibility,
                title = "Mostrar Contador de FPS",
                subtitle = "Superpone un contador numérico en tiempo real en la parte superior del juego",
                checked = showFps,
                items = 1,
                index = 0,
                onCheckedChange = {
                    showFps = it
                    prefs.showFps = it
                    prefs.writeVideo(context.filesDir)
                },
            )

            Text(
                "Configura la tasa de refresco visual. El motor lógico mantiene estrictamente los 30 ticks/s canónicos de GameMaker 8 para preservar la sincronización de combate e IA, interpolando los gráficos a la tasa seleccionada.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            SettingsGroup("Modos de Presentación") {
                ChoiceRow(
                    icon = Icons.Filled.Speed,
                    title = "30 FPS (Canónico GM8)",
                    subtitle = "Comportamiento clásico de GameMaker 8: 1 dibujo por paso de simulación. Tasa original histórica de Pokémon Reloaded.",
                    selected = renderMode == "native",
                    items = 4,
                    index = 0,
                    onSelect = {
                        renderMode = "native"
                        prefs.renderMode = "native"
                        prefs.writeVideo(context.filesDir)
                    },
                )
                ChoiceRow(
                    icon = Icons.Filled.Speed,
                    title = "60 FPS (Interpolado Suave)",
                    subtitle = "Interpola movimiento de cámara y Pokémon a 60 FPS mediante cálculo subframe en el shader, manteniendo la simulación a 30 Hz.",
                    selected = renderMode == "60",
                    items = 4,
                    index = 1,
                    enabled = supports60,
                    onSelect = {
                        renderMode = "60"
                        prefs.renderMode = "60"
                        prefs.writeVideo(context.filesDir)
                    },
                )
                ChoiceRow(
                    icon = Icons.Filled.Speed,
                    title = "120 FPS (Paneles de Alta Tasa)",
                    subtitle = "Fluidez cinematográfica ultra suave para pantallas de 120 Hz. Requiere panel compatible con tasa de refresco de 120 Hz o superior.",
                    selected = renderMode == "120",
                    items = 4,
                    index = 2,
                    enabled = supports120,
                    onSelect = {
                        renderMode = "120"
                        prefs.renderMode = "120"
                        prefs.writeVideo(context.filesDir)
                    },
                )
                ChoiceRow(
                    icon = Icons.Filled.Speed,
                    title = "Sin Límite (Uncapped)",
                    subtitle = "Dibuja a la máxima tasa que la GPU pueda producir sin bloqueo artificial de tiempo. Ideal para benchmarking y pantallas extremas.",
                    selected = renderMode == "uncapped",
                    items = 4,
                    index = 3,
                    enabled = supports120,
                    onSelect = {
                        renderMode = "uncapped"
                        prefs.renderMode = "uncapped"
                        prefs.writeVideo(context.filesDir)
                    },
                )
            }

            Spacer(Modifier.height(8.dp))
        }
    }
}
