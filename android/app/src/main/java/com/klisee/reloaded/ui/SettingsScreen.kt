package com.klisee.reloaded.ui

import java.io.File
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Article
import androidx.compose.material.icons.filled.AspectRatio
import androidx.compose.material.icons.filled.BrightnessAuto
import androidx.compose.material.icons.filled.CalendarMonth
import androidx.compose.material.icons.filled.CameraAlt
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.DarkMode
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Fullscreen
import androidx.compose.material.icons.filled.HealthAndSafety
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.LightMode
import androidx.compose.material.icons.filled.Memory
import androidx.compose.material.icons.filled.Nightlight
import androidx.compose.material.icons.filled.Palette
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Restore
import androidx.compose.material.icons.filled.Save
import androidx.compose.material.icons.filled.Shield
import androidx.compose.material.icons.filled.Speed
import androidx.compose.material.icons.filled.SportsEsports
import androidx.compose.material.icons.filled.Storage
import androidx.compose.material.icons.filled.SystemUpdate
import androidx.compose.material.icons.filled.VideogameAsset
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
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
import androidx.compose.ui.Alignment
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
    var videoScaling by remember { mutableStateOf(prefs.videoScaling) }
    var videoBezel by remember { mutableStateOf(prefs.videoBezel) }
    var showFps by remember { mutableStateOf(prefs.showFps) }
    var batterySaver by remember { mutableStateOf(prefs.batterySaver) }
    var currentTheme by remember { mutableStateOf(prefs.themeMode) }
    var persistentShield by remember { mutableStateOf(prefs.persistentShield) }
    var checking by remember { mutableStateOf(false) }

    // Snapshots state
    var showSnapshotsDialog by remember { mutableStateOf(false) }
    var snapshotsList by remember { mutableStateOf<List<GameRepository.SnapshotItem>>(emptyList()) }
    var newSnapshotSlot by remember { mutableStateOf(1) }
    var newSnapshotLabel by remember { mutableStateOf("Pre-Gimnasio") }
    var restoreSnapshotTarget by remember { mutableStateOf<GameRepository.SnapshotItem?>(null) }
    var deleteSnapshotTarget by remember { mutableStateOf<GameRepository.SnapshotItem?>(null) }

    // Health check state
    var showHealthDialog by remember { mutableStateOf(false) }
    var runningHealthCheck by remember { mutableStateOf(false) }
    var healthReport by remember { mutableStateOf<GameRepository.HealthReport?>(null) }

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

    restoreSnapshotTarget?.let { snap ->
        AlertDialog(
            onDismissRequest = { restoreSnapshotTarget = null },
            icon = { Icon(Icons.Filled.Warning, contentDescription = null) },
            title = { Text("¿Restaurar Punto de Retorno?") },
            text = {
                Text("Esto restaurará la Partida ${snap.slot} con los datos de '${snap.label}' del ${formatDate(snap.timestamp)}. Tu partida actual en esa ranura se sobrescribirá. ¿Deseas continuar?")
            },
            confirmButton = {
                TextButton(onClick = {
                    val target = snap
                    restoreSnapshotTarget = null
                    scope.launch {
                        val result = repo.restoreSnapshot(target.file)
                        if (result.isSuccess) {
                            snackbar.showSnackbar("Punto de retorno restaurado con éxito en Slot ${result.getOrNull() ?: target.slot}.")
                        } else {
                            snackbar.showSnackbar("Error al restaurar: ${result.exceptionOrNull()?.message}")
                        }
                    }
                }) { Text("Restaurar") }
            },
            dismissButton = { TextButton(onClick = { restoreSnapshotTarget = null }) { Text("Cancelar") } },
        )
    }

    deleteSnapshotTarget?.let { snap ->
        AlertDialog(
            onDismissRequest = { deleteSnapshotTarget = null },
            icon = { Icon(Icons.Filled.Delete, contentDescription = null) },
            title = { Text("¿Eliminar Punto de Retorno?") },
            text = {
                Text("¿Deseas eliminar el snapshot '${snap.label}' (${formatDate(snap.timestamp)})?")
            },
            confirmButton = {
                TextButton(onClick = {
                    val target = snap
                    deleteSnapshotTarget = null
                    scope.launch {
                        val ok = repo.deleteSnapshot(target.file)
                        snapshotsList = repo.listSnapshots()
                        if (ok) {
                            snackbar.showSnackbar("Snapshot eliminado.")
                        }
                    }
                }) { Text("Eliminar") }
            },
            dismissButton = { TextButton(onClick = { deleteSnapshotTarget = null }) { Text("Cancelar") } },
        )
    }

    if (showSnapshotsDialog) {
        AlertDialog(
            onDismissRequest = { showSnapshotsDialog = false },
            icon = { Icon(Icons.Filled.CameraAlt, contentDescription = null) },
            title = { Text("Puntos de Retorno (Snapshots)") },
            text = {
                Column(
                    modifier = Modifier.verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    Text(
                        "Crea copias de seguridad instantáneas antes de gimnasios o combates difíciles. Se guardan tanto internamente como en la carpeta pública Documents.",
                        style = MaterialTheme.typography.bodySmall,
                    )
                    Card(
                        modifier = Modifier.fillMaxWidth(),
                        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHighest)
                    ) {
                        Column(
                            modifier = Modifier.padding(12.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Text("Nuevo Snapshot", style = MaterialTheme.typography.titleSmall)
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.spacedBy(8.dp),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Text("Ranura:", style = MaterialTheme.typography.bodyMedium)
                                (1..3).forEach { slotNum ->
                                    if (newSnapshotSlot == slotNum) {
                                        Button(
                                            onClick = { newSnapshotSlot = slotNum },
                                            modifier = Modifier.weight(1f)
                                        ) {
                                            Text("Slot $slotNum")
                                        }
                                    } else {
                                        OutlinedButton(
                                            onClick = { newSnapshotSlot = slotNum },
                                            modifier = Modifier.weight(1f)
                                        ) {
                                            Text("Slot $slotNum")
                                        }
                                    }
                                }
                            }
                            OutlinedTextField(
                                value = newSnapshotLabel,
                                onValueChange = { newSnapshotLabel = it },
                                label = { Text("Etiqueta / Motivo") },
                                singleLine = true,
                                modifier = Modifier.fillMaxWidth()
                            )
                            Button(
                                onClick = {
                                    scope.launch {
                                        val created = repo.createSnapshot(newSnapshotSlot, newSnapshotLabel.trim().ifEmpty { "Snapshot" })
                                        snapshotsList = repo.listSnapshots()
                                        if (created != null) {
                                            snackbar.showSnackbar("Punto de retorno guardado (${created.name})")
                                        } else {
                                            snackbar.showSnackbar("No se pudo crear el snapshot. Comprueba que el juego esté instalado.")
                                        }
                                    }
                                },
                                modifier = Modifier.fillMaxWidth(),
                                enabled = installed != null
                            ) {
                                Icon(Icons.Filled.CameraAlt, contentDescription = null, modifier = Modifier.size(18.dp))
                                Spacer(Modifier.size(8.dp))
                                Text("Guardar Snapshot Ahora")
                            }
                        }
                    }

                    Text("Historial de Snapshots (${snapshotsList.size})", style = MaterialTheme.typography.titleSmall)
                    if (snapshotsList.isEmpty()) {
                        Text(
                            "No hay puntos de retorno guardados aún.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.outline
                        )
                    } else {
                        snapshotsList.forEach { snap ->
                            Card(
                                modifier = Modifier.fillMaxWidth(),
                                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHighest)
                            ) {
                                Row(
                                    modifier = Modifier.padding(12.dp).fillMaxWidth(),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.SpaceBetween
                                ) {
                                    Column(modifier = Modifier.weight(1f)) {
                                        Text(
                                            "${snap.label} (Slot ${snap.slot})",
                                            style = MaterialTheme.typography.titleSmall
                                        )
                                        Text(
                                            "${formatDate(snap.timestamp)} • ${formatBytes(snap.sizeBytes)}",
                                            style = MaterialTheme.typography.bodySmall
                                        )
                                    }
                                    IconButton(onClick = { restoreSnapshotTarget = snap }) {
                                        Icon(Icons.Filled.Restore, contentDescription = "Restaurar")
                                    }
                                    IconButton(onClick = { deleteSnapshotTarget = snap }) {
                                        Icon(Icons.Filled.Delete, contentDescription = "Eliminar")
                                    }
                                }
                            }
                        }
                    }
                }
            },
            confirmButton = {
                TextButton(onClick = { showSnapshotsDialog = false }) { Text("Cerrar") }
            }
        )
    }

    if (showHealthDialog) {
        AlertDialog(
            onDismissRequest = { if (!runningHealthCheck) showHealthDialog = false },
            icon = { Icon(Icons.Filled.HealthAndSafety, contentDescription = null) },
            title = { Text("Diagnóstico de Integridad") },
            text = {
                Column(
                    modifier = Modifier.verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(10.dp)
                ) {
                    if (runningHealthCheck) {
                        Column(
                            horizontalAlignment = Alignment.CenterHorizontally,
                            modifier = Modifier.fillMaxWidth().padding(16.dp)
                        ) {
                            CircularProgressIndicator()
                            Spacer(Modifier.height(12.dp))
                            Text(
                                "Escaneando 34.388 archivos, sprites de batalla y pistas MIDI...",
                                style = MaterialTheme.typography.bodyMedium
                            )
                        }
                    } else if (healthReport != null) {
                        val report = healthReport!!
                        Card(
                            modifier = Modifier.fillMaxWidth(),
                            colors = CardDefaults.cardColors(
                                containerColor = when (report.level) {
                                    GameRepository.HealthLevel.EXCELLENT -> MaterialTheme.colorScheme.primaryContainer
                                    GameRepository.HealthLevel.GOOD -> MaterialTheme.colorScheme.secondaryContainer
                                    GameRepository.HealthLevel.WARNING -> MaterialTheme.colorScheme.tertiaryContainer
                                    GameRepository.HealthLevel.CRITICAL -> MaterialTheme.colorScheme.errorContainer
                                }
                            )
                        ) {
                            Row(
                                modifier = Modifier.padding(12.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(8.dp)
                            ) {
                                Icon(
                                    when (report.level) {
                                        GameRepository.HealthLevel.EXCELLENT, GameRepository.HealthLevel.GOOD -> Icons.Filled.CheckCircle
                                        GameRepository.HealthLevel.WARNING -> Icons.Filled.Warning
                                        GameRepository.HealthLevel.CRITICAL -> Icons.Filled.Warning
                                    },
                                    contentDescription = null
                                )
                                Text(
                                    text = when (report.level) {
                                        GameRepository.HealthLevel.EXCELLENT -> "ESTADO EXCELENTE"
                                        GameRepository.HealthLevel.GOOD -> "ESTADO ÓPTIMO"
                                        GameRepository.HealthLevel.WARNING -> "ADVERTENCIA: Faltan recursos"
                                        GameRepository.HealthLevel.CRITICAL -> "ERROR CRÍTICO"
                                    },
                                    style = MaterialTheme.typography.titleSmall
                                )
                            }
                        }
                        Text(report.summary, style = MaterialTheme.typography.bodyMedium)
                        Spacer(Modifier.height(4.dp))
                        val exeOk = !report.missingCoreFiles.any { it.contains(".exe") }
                        Text("• Archivos totales analizados: ${report.totalFiles} (${formatBytes(report.totalSizeBytes)})", style = MaterialTheme.typography.bodySmall)
                        Text("• Sprites de combate: ${report.battleSpritesCount} encontrados", style = MaterialTheme.typography.bodySmall)
                        Text("• Pistas MIDI de sonido: ${report.midisCount} encontradas", style = MaterialTheme.typography.bodySmall)
                        Text("• Ejecutable (.exe): ${if (exeOk) "Presente y válido ✓" else "Falta ejecutable ✗"}", style = MaterialTheme.typography.bodySmall)
                        if (report.zeroByteFiles > 0) {
                            Text("• Archivos vacíos/corruptos (0 bytes): ${report.zeroByteFiles}", color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
                        }
                        if (report.missingCoreFiles.isNotEmpty()) {
                            Text("• Recursos críticos faltantes: ${report.missingCoreFiles.joinToString()}", color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
                        }
                    } else {
                        Text(
                            "Comprueba la integridad de los más de 34.000 archivos instalados (sprites de combate, efectos de sonido, MIDIs y ejecutables) para verificar que la instalación está libre de fallos y lista para jugar.",
                            style = MaterialTheme.typography.bodyMedium
                        )
                    }
                }
            },
            confirmButton = {
                TextButton(
                    enabled = !runningHealthCheck && installed != null,
                    onClick = {
                        runningHealthCheck = true
                        scope.launch {
                            val rep = repo.runHealthCheck()
                            healthReport = rep
                            runningHealthCheck = false
                        }
                    }
                ) {
                    Text(if (healthReport == null) "Iniciar Diagnóstico" else "Volver a analizar")
                }
            },
            dismissButton = {
                TextButton(
                    enabled = !runningHealthCheck,
                    onClick = { showHealthDialog = false }
                ) {
                    Text("Cerrar")
                }
            }
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
                    subtitle = "Aspecto clásico original con barras laterales",
                    selected = videoScaling == "aspect",
                    items = 5,
                    index = 0,
                    onSelect = {
                        videoScaling = "aspect"
                        prefs.videoScaling = "aspect"
                        prefs.writeVideo(context.filesDir)
                    },
                )
                ChoiceRow(
                    icon = Icons.Filled.Fullscreen,
                    title = "Rellenar la pantalla",
                    subtitle = "La imagen cubre toda la pantalla del dispositivo",
                    selected = videoScaling == "full",
                    items = 5,
                    index = 1,
                    onSelect = {
                        videoScaling = "full"
                        prefs.videoScaling = "full"
                        prefs.writeVideo(context.filesDir)
                    },
                )
                ChoiceRow(
                    icon = Icons.Filled.AspectRatio,
                    title = "Escalado Pixel-Perfect (Nítido / Sharp)",
                    subtitle = "Escalado entero exacto para bordes 100% nítidos en pantallas HD/FHD/OLED",
                    selected = videoScaling == "pixel_perfect",
                    items = 5,
                    index = 2,
                    onSelect = {
                        videoScaling = "pixel_perfect"
                        prefs.videoScaling = "pixel_perfect"
                        prefs.writeVideo(context.filesDir)
                    },
                )
                SwitchRow(
                    icon = Icons.Filled.Speed,
                    title = "Contador de FPS en pantalla",
                    subtitle = "Muestra la tasa de cuadros por segundo en tiempo real durante la partida",
                    checked = showFps,
                    items = 5,
                    index = 3,
                    onCheckedChange = {
                        showFps = it
                        prefs.showFps = it
                        prefs.writeVideo(context.filesDir)
                    },
                )
                SwitchRow(
                    icon = Icons.Filled.LightMode,
                    title = "Mantener pantalla encendida",
                    subtitle = "Evita que el móvil se bloquee mientras juegas",
                    checked = keepScreenOn,
                    items = 5,
                    index = 4,
                    onCheckedChange = {
                        keepScreenOn = it
                        prefs.keepScreenOn = it
                    },
                )
            }

            SettingsGroup("Bordes en 4:3 (Bezel Art)") {
                ChoiceRow(
                    icon = Icons.Filled.Nightlight,
                    title = "Negro AMOLED (Puro)",
                    subtitle = "Franjas negras absolutas (#000000) para máximo ahorro de batería OLED",
                    selected = videoBezel == "oled",
                    items = 3,
                    index = 0,
                    onSelect = {
                        videoBezel = "oled"
                        prefs.videoBezel = "oled"
                        prefs.writeVideo(context.filesDir)
                    },
                )
                ChoiceRow(
                    icon = Icons.Filled.AspectRatio,
                    title = "Consola Retro (Grafito)",
                    subtitle = "Marco de consola portátil texturizado con acentos pizarra sobrios",
                    selected = videoBezel == "retro_slate",
                    items = 3,
                    index = 1,
                    onSelect = {
                        videoBezel = "retro_slate"
                        prefs.videoBezel = "retro_slate"
                        prefs.writeVideo(context.filesDir)
                    },
                )
                ChoiceRow(
                    icon = Icons.Filled.Palette,
                    title = "Reloaded Azul (Neón)",
                    subtitle = "Franjas azul marino profundo con líneas decorativas cian neón",
                    selected = videoBezel == "reloaded_blue",
                    items = 3,
                    index = 2,
                    onSelect = {
                        videoBezel = "reloaded_blue"
                        prefs.videoBezel = "reloaded_blue"
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
                    items = 4,
                    index = 0,
                    enabled = installed != null,
                    onClick = onMySaves,
                )
                SwitchRow(
                    icon = Icons.Filled.Shield,
                    title = "Escudo Scoped Storage Shield",
                    subtitle = "Réplica en Documents/PokemonReloadedSaves/ para no perder partidas si desinstalas el APK",
                    checked = persistentShield,
                    items = 4,
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
                NavRow(
                    icon = Icons.Filled.CameraAlt,
                    title = "Puntos de Retorno (Instant Snapshots)",
                    subtitle = "Crea copias de seguridad instantáneas antes de eventos clave o combates difíciles",
                    items = 4,
                    index = 2,
                    enabled = installed != null,
                    onClick = {
                        scope.launch { snapshotsList = repo.listSnapshots() }
                        showSnapshotsDialog = true
                    },
                )
                NavRow(
                    icon = Icons.Filled.HealthAndSafety,
                    title = "Diagnóstico de Integridad del Juego",
                    subtitle = "Verifica los más de 34.000 archivos, sprites de combate, MIDIs y ejecutables",
                    items = 4,
                    index = 3,
                    enabled = installed != null,
                    onClick = {
                        showHealthDialog = true
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
