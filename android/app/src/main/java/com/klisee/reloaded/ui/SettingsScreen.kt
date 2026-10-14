package com.klisee.reloaded.ui

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Article
import androidx.compose.material.icons.filled.AspectRatio
import androidx.compose.material.icons.filled.BrightnessAuto
import androidx.compose.material.icons.filled.CalendarMonth
import androidx.compose.material.icons.filled.DarkMode
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Fullscreen
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.LightMode
import androidx.compose.material.icons.filled.Memory
import androidx.compose.material.icons.filled.Nightlight
import androidx.compose.material.icons.filled.Save
import androidx.compose.material.icons.filled.Shield
import androidx.compose.material.icons.filled.Speed
import androidx.compose.material.icons.filled.SportsEsports
import androidx.compose.material.icons.filled.Storage
import androidx.compose.material.icons.filled.SystemUpdate
import androidx.compose.material.icons.filled.VideogameAsset
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.klisee.reloaded.GameRepository
import com.klisee.reloaded.Preferences
import com.klisee.reloaded.Update
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    repo: GameRepository,
    prefs: Preferences,
    revision: Int = 0,
    onLog: () -> Unit,
    onAbout: () -> Unit,
    onControls: () -> Unit,
    onMySaves: () -> Unit,
    onUpdateFound: (Update.Release) -> Unit,
    onThemeChange: (String) -> Unit = {},
    bottomBar: @Composable () -> Unit = {},
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val snackbar = remember { SnackbarHostState() }
    var status by remember(revision) { mutableStateOf(repo.status()) }
    var keepScreenOn by remember { mutableStateOf(prefs.keepScreenOn) }
    var keepAspect by remember { mutableStateOf(prefs.keepAspect) }
    var showFps by remember { mutableStateOf(prefs.showFps) }
    var batterySaver by remember { mutableStateOf(prefs.batterySaver) }
    var currentTheme by remember { mutableStateOf(prefs.themeMode) }
    var persistentShield by remember { mutableStateOf(prefs.persistentShield) }
    var checking by remember { mutableStateOf(false) }
    val appVersion = remember {
        runCatching { context.packageManager.getPackageInfo(context.packageName, 0).versionName }.getOrNull().orEmpty()
    }
    var confirmingDelete by remember { mutableStateOf(false) }
    val installed = status as? GameRepository.Status.Ready

    if (confirmingDelete) {
        AlertDialog(
            onDismissRequest = { confirmingDelete = false },
            title = { Text("¿Borrar el juego?") },
            text = { Text("Se borrará la copia extraída (${formatBytes(installed?.sizeBytes ?: 0)}). Podrás volver a descargarlo o importarlo. Si Scoped Storage Shield está activo, tus partidas seguirán a salvo en Documents/PokemonReloadedSaves/.") },
            confirmButton = {
                TextButton(onClick = {
                    confirmingDelete = false
                    scope.launch {
                        repo.deleteGame()
                        status = repo.status()
                    }
                }) { Text("Borrar") }
            },
            dismissButton = { TextButton(onClick = { confirmingDelete = false }) { Text("Cancelar") } },
        )
    }

    Scaffold(
        topBar = { TopAppBar(title = { Text("Ajustes") }) },
        bottomBar = bottomBar,
        // Lifted clear of the floating bar, which otherwise sits on top of the message.
        snackbarHost = { SnackbarHost(snackbar, modifier = Modifier.padding(bottom = BottomNavSpace)) },
        contentWindowInsets = WindowInsets(0.dp),
    ) { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState()).padding(horizontal = 24.dp),
            verticalArrangement = Arrangement.spacedBy(24.dp),
        ) {
            Spacer(Modifier.height(4.dp))

            SettingsGroup("Mando y Controles") {
                NavRow(
                    icon = Icons.Filled.SportsEsports,
                    title = "Configuración de Controles",
                    subtitle = "Personaliza botones en pantalla, opacidad, escala y mandos físicos",
                    items = 1,
                    index = 0,
                    onClick = onControls,
                )
            }

            SettingsGroup("Vídeo y Pantalla") {
                ChoiceRow(
                    icon = Icons.Filled.AspectRatio,
                    title = "Mantener proporción (4:3)",
                    subtitle = "Aspecto clásico sin distorsión, con barras laterales",
                    selected = keepAspect,
                    items = 4,
                    index = 0,
                    onSelect = {
                        keepAspect = true
                        prefs.keepAspect = true
                        prefs.writeVideo(context.filesDir)
                    },
                )
                ChoiceRow(
                    icon = Icons.Filled.Fullscreen,
                    title = "Rellenar la pantalla",
                    subtitle = "La imagen cubre toda la pantalla del dispositivo",
                    selected = !keepAspect,
                    items = 4,
                    index = 1,
                    onSelect = {
                        keepAspect = false
                        prefs.keepAspect = false
                        prefs.writeVideo(context.filesDir)
                    },
                )
                SwitchRow(
                    icon = Icons.Filled.LightMode,
                    title = "Mantener pantalla encendida",
                    subtitle = "Evita que el móvil se bloquee mientras juegas",
                    checked = keepScreenOn,
                    items = 4,
                    index = 2,
                    onCheckedChange = {
                        keepScreenOn = it
                        prefs.keepScreenOn = it
                    },
                )
                SwitchRow(
                    icon = Icons.Filled.Speed,
                    title = "Contador de FPS en pantalla",
                    subtitle = "Muestra la tasa de cuadros por segundo en tiempo real durante la partida",
                    checked = showFps,
                    items = 4,
                    index = 3,
                    onCheckedChange = {
                        showFps = it
                        prefs.showFps = it
                        prefs.writeVideo(context.filesDir)
                    },
                )
            }

            SettingsGroup("Apariencia") {
                ChoiceRow(
                    icon = Icons.Filled.BrightnessAuto,
                    title = "Automático (Sistema)",
                    subtitle = "Sigue el modo claro u oscuro configurado en Android",
                    selected = currentTheme == "system",
                    items = 4,
                    index = 0,
                    onSelect = {
                        currentTheme = "system"
                        prefs.themeMode = "system"
                        onThemeChange("system")
                    },
                )
                ChoiceRow(
                    icon = Icons.Filled.LightMode,
                    title = "Tema Claro",
                    subtitle = "Interfaz clara y luminosa",
                    selected = currentTheme == "light",
                    items = 4,
                    index = 1,
                    onSelect = {
                        currentTheme = "light"
                        prefs.themeMode = "light"
                        onThemeChange("light")
                    },
                )
                ChoiceRow(
                    icon = Icons.Filled.DarkMode,
                    title = "Tema Oscuro",
                    subtitle = "Interfaz oscura relajante para la vista",
                    selected = currentTheme == "dark",
                    items = 4,
                    index = 2,
                    onSelect = {
                        currentTheme = "dark"
                        prefs.themeMode = "dark"
                        onThemeChange("dark")
                    },
                )
                ChoiceRow(
                    icon = Icons.Filled.Nightlight,
                    title = "Negro Puro (AMOLED)",
                    subtitle = "Fondo negro absoluto (#000000) para ahorrar batería en pantallas OLED",
                    selected = currentTheme == "amoled",
                    items = 4,
                    index = 3,
                    onSelect = {
                        currentTheme = "amoled"
                        prefs.themeMode = "amoled"
                        onThemeChange("amoled")
                    },
                )
            }

            SettingsGroup("Rendimiento y Batería") {
                SwitchRow(
                    icon = Icons.Filled.Memory,
                    title = "Modo Ahorro de Energía",
                    subtitle = "Optimiza consumo térmico y batería para sesiones largas",
                    checked = batterySaver,
                    items = 1,
                    index = 0,
                    onCheckedChange = {
                        batterySaver = it
                        prefs.batterySaver = it
                    },
                )
            }

            SettingsGroup("Seguridad y Partidas") {
                NavRow(
                    icon = Icons.Filled.Save,
                    title = "Mis Partidas Guardadas",
                    subtitle = if (installed == null) {
                        "El juego no está instalado"
                    } else {
                        "Consulta, exporta o gestiona tus archivos de guardado"
                    },
                    items = 2,
                    index = 0,
                    enabled = installed != null,
                    onClick = onMySaves,
                )
                SwitchRow(
                    icon = Icons.Filled.Shield,
                    title = "Escudo Scoped Storage Shield",
                    subtitle = "Réplica en Documents/PokemonReloadedSaves/ para no perder partidas si desinstalas el APK",
                    checked = persistentShield,
                    items = 2,
                    index = 1,
                    onCheckedChange = { enabled ->
                        persistentShield = enabled
                        prefs.persistentShield = enabled
                        if (enabled) {
                            scope.launch {
                                repo.autoBackupSaves()
                                snackbar.showSnackbar("Escudo activado: partidas replicadas en Documents/PokemonReloadedSaves")
                            }
                        }
                    },
                )
            }

            SettingsGroup("Juego y Almacenamiento") {
                if (installed == null) {
                    InfoRow(
                        icon = Icons.Filled.Info,
                        title = "Sin juego instalado",
                        value = "Descárgalo o impórtalo desde la pantalla principal",
                        items = 1,
                        index = 0,
                    )
                } else {
                    InfoRow(Icons.Filled.VideogameAsset, "Versión del juego", installed.version ?: "1.9.2.2 Full", items = 4, index = 0)
                    InfoRow(Icons.Filled.Storage, "Espacio ocupado", formatBytes(installed.sizeBytes), items = 4, index = 1)
                    InfoRow(Icons.Filled.CalendarMonth, "Instalado el", formatDate(installed.installedAt), items = 4, index = 2)
                    NavRow(
                        icon = Icons.Filled.Delete,
                        title = "Borrar el juego",
                        subtitle = "Libera ${formatBytes(installed.sizeBytes)} del almacenamiento",
                        items = 4,
                        index = 3,
                        onClick = { confirmingDelete = true },
                    )
                }
            }

            SettingsGroup("Actualizaciones y Ayuda") {
                NavRow(
                    icon = Icons.Filled.SystemUpdate,
                    title = "Versión de la app: $appVersion",
                    subtitle = "Comprueba si hay actualizaciones disponibles",
                    items = 3,
                    index = 0,
                    enabled = !checking,
                    onClick = {
                        checking = true
                        scope.launch {
                            val found = Update.check(appVersion).getOrNull()
                            checking = false
                            if (found != null) {
                                onUpdateFound(found)
                            } else {
                                snackbar.showSnackbar("Ya tienes la última versión ($appVersion).")
                            }
                        }
                    },
                )
                NavRow(
                    icon = Icons.Filled.Article,
                    title = "Logs técnicos",
                    subtitle = "En caso de un error, comparte los logs para diagnosticar el problema",
                    items = 3,
                    index = 1,
                    onClick = onLog,
                )
                NavRow(
                    icon = Icons.Filled.Info,
                    title = "Acerca de",
                    subtitle = "Versión, créditos y aviso de Pokémon Reloaded Android",
                    items = 3,
                    index = 2,
                    onClick = onAbout,
                )
            }

            Spacer(Modifier.height(BottomNavSpace))
        }
    }
}
