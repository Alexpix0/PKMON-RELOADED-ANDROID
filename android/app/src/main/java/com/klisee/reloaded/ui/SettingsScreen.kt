package com.klisee.reloaded.ui

import java.io.File
import androidx.activity.compose.BackHandler
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
import androidx.compose.material.icons.filled.BugReport
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.Extension
import androidx.compose.material.icons.filled.Storage
import androidx.compose.material.icons.filled.SystemUpdate
import androidx.compose.material.icons.filled.VideogameAsset
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material.icons.filled.Tune
import androidx.compose.material.icons.filled.Spa
import androidx.compose.material.icons.filled.DirectionsBike
import androidx.compose.material.icons.filled.AutoAwesome
import androidx.compose.material.icons.filled.TouchApp
import androidx.compose.material.icons.filled.DirectionsRun
import androidx.compose.material.icons.filled.Backpack
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.filled.HourglassEmpty
import androidx.compose.material.icons.filled.Vibration
import androidx.compose.material.icons.filled.Bookmark
import androidx.compose.material.icons.filled.Alarm
import android.content.ClipboardManager
import android.content.ClipData
import android.content.Context
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
    onTouchSettings: () -> Unit = {},
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
    var turboMultiplier by remember { mutableStateOf(prefs.turboMultiplier) }
    var frameSkip by remember { mutableStateOf(prefs.frameSkip) }
    var renderScale by remember { mutableStateOf(prefs.renderScale) }
    var videoFilter by remember { mutableStateOf(prefs.videoFilter) }
    var weatherOpt by remember { mutableStateOf(prefs.weatherOpt) }
    var batterySaver by remember { mutableStateOf(prefs.batterySaver) }
    var currentTheme by remember { mutableStateOf(prefs.themeMode) }
    var persistentShield by remember { mutableStateOf(prefs.persistentShield) }
    var graphicsBackend by remember { mutableStateOf(prefs.graphicsBackend) }
    var force60Fps by remember { mutableStateOf(prefs.force60Fps) }
    var bgmVolume by remember { mutableStateOf(prefs.bgmVolume) }
    var sfxVolume by remember { mutableStateOf(prefs.sfxVolume) }
    var autofireEnabled by remember { mutableStateOf(prefs.autofireEnabled) }
    var dynamicDpad by remember { mutableStateOf(prefs.dynamicDpad) }
    var berryTracker by remember { mutableStateOf(prefs.berryTracker) }
    var eggHatchTimer by remember { mutableStateOf(prefs.eggHatchTimer) }
    var keyItemsRadial by remember { mutableStateOf(prefs.keyItemsRadial) }
    var autoRun by remember { mutableStateOf(prefs.autoRun) }
    var dialogTapAdvance by remember { mutableStateOf(prefs.dialogTapAdvance) }
    var repelReminder by remember { mutableStateOf(prefs.repelReminder) }
    var shinyBeacon by remember { mutableStateOf(prefs.shinyBeacon) }
    var puzzlePause by remember { mutableStateOf(prefs.puzzlePause) }
    var fishingHaptic by remember { mutableStateOf(prefs.fishingHaptic) }
    var quickSave by remember { mutableStateOf(prefs.quickSave) }
    var saveReminder by remember { mutableStateOf(prefs.saveReminder) }
    var checking by remember { mutableStateOf(false) }
    var currentSubScreen by remember { mutableStateOf<String?>(null) }

    // v0.9.0 State Variables
    var extremeCombat by remember { mutableStateOf(prefs.extremeCombat) }
    var maliEarlyZ by remember { mutableStateOf(prefs.maliEarlyZ) }
    var gpuWakeup by remember { mutableStateOf(prefs.gpuWakeup) }
    var fsrMode by remember { mutableStateOf(prefs.fsrMode) }
    var drsEnabled by remember { mutableStateOf(prefs.drsEnabled) }
    var xbrzEnabled by remember { mutableStateOf(prefs.xbrzEnabled) }
    var lighting2d by remember { mutableStateOf(prefs.lighting2d) }
    var adaptiveLtpo by remember { mutableStateOf(prefs.adaptiveLtpo) }
    var colorblindMode by remember { mutableStateOf(prefs.colorblindMode) }
    var jitArm64 by remember { mutableStateOf(prefs.jitArm64) }
    var bigLittleAffinity by remember { mutableStateOf(prefs.bigLittleAffinity) }
    var megaAtlas by remember { mutableStateOf(prefs.megaAtlas) }
    var activeStaticPartition by remember { mutableStateOf(prefs.activeStaticPartition) }
    var radialMenu by remember { mutableStateOf(prefs.radialMenu) }
    var gestures by remember { mutableStateOf(prefs.gestures) }
    var oneHanded by remember { mutableStateOf(prefs.oneHanded) }
    var audio3d by remember { mutableStateOf(prefs.audio3d) }
    var reactiveStems by remember { mutableStateOf(prefs.reactiveStems) }
    var wsola by remember { mutableStateOf(prefs.wsola) }
    var pipMode by remember { mutableStateOf(prefs.pipMode) }
    var liveActivity by remember { mutableStateOf(prefs.liveActivity) }
    var smartPrerollSkip by remember { mutableStateOf(prefs.smartPrerollSkip) }
    var replayBlackbox by remember { mutableStateOf(prefs.replayBlackbox) }
    var achievements by remember { mutableStateOf(prefs.achievements) }
    var uiStyle by remember { mutableStateOf(prefs.uiStyle) }

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

    // Mod and Crash Dump state
    var installingMod by remember { mutableStateOf(false) }
    var showCrashDumpDialog by remember { mutableStateOf(false) }
    var currentCrashDump by remember { mutableStateOf(repo.getCrashDump()) }

    val modPickerLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocument()
    ) { uri ->
        if (uri != null) {
            installingMod = true
            scope.launch {
                val result = repo.applyPatchOrMod(uri)
                installingMod = false
                result.fold(
                    onSuccess = { count ->
                        snackbar.showSnackbar("¡Parche/Mod aplicado con éxito! ($count archivos actualizados)")
                    },
                    onFailure = { err ->
                        snackbar.showSnackbar("Error al instalar parche/mod: ${err.message}")
                    }
                )
            }
        }
    }

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
                                        created.fold(
                                            onSuccess = { snapFile -> snackbar.showSnackbar("Punto de retorno guardado (${snapFile.name})") },
                                            onFailure = { err -> snackbar.showSnackbar("No se pudo crear el snapshot: ${err.message}") }
                                        )
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

    if (showCrashDumpDialog) {
        val dumpText = currentCrashDump ?: "No hay reporte disponible."
        AlertDialog(
            onDismissRequest = { showCrashDumpDialog = false },
            icon = { Icon(Icons.Filled.BugReport, contentDescription = null) },
            title = { Text("Reporte de Fallo (Crash Dump)") },
            text = {
                Column(modifier = Modifier.verticalScroll(rememberScrollState())) {
                    Text(
                        dumpText,
                        style = MaterialTheme.typography.bodySmall,
                        fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace
                    )
                }
            },
            confirmButton = {
                TextButton(onClick = {
                    val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                    clipboard.setPrimaryClip(ClipData.newPlainText("Crash Dump", dumpText))
                    scope.launch { snackbar.showSnackbar("Reporte copiado al portapapeles.") }
                }) {
                    Icon(Icons.Filled.ContentCopy, contentDescription = null, modifier = Modifier.size(16.dp))
                    Spacer(Modifier.size(4.dp))
                    Text("Copiar")
                }
            },
            dismissButton = {
                Row {
                    TextButton(onClick = {
                        repo.clearCrashDump()
                        currentCrashDump = null
                        showCrashDumpDialog = false
                        scope.launch { snackbar.showSnackbar("Reporte eliminado.") }
                    }) {
                        Text("Eliminar")
                    }
                    TextButton(onClick = { showCrashDumpDialog = false }) {
                        Text("Cerrar")
                    }
                }
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

        BackHandler(enabled = currentSubScreen != null) {
        currentSubScreen = null
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        when (currentSubScreen) {
                            "video" -> "Vídeo y Pantalla"
                            "audio" -> "Audio y Sonido"
                            "controls" -> "Mandos y Controles"
                            "performance" -> "Rendimiento y Motor"
                            "security" -> "Seguridad y Partidas"
                            "about" -> "Acerca de y Ayuda"
                            else -> "Ajustes"
                        }
                    )
                },
                navigationIcon = {
                    if (currentSubScreen != null) {
                        IconButton(onClick = { currentSubScreen = null }) {
                            Icon(
                                Icons.AutoMirrored.Filled.ArrowBack,
                                contentDescription = "Volver",
                            )
                        }
                    }
                },
            )
        },
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

                        when (currentSubScreen) {
                null -> {
                    SettingsGroup("Categorías de Configuración") {
                        NavRow(
                            icon = Icons.Filled.Fullscreen,
                            title = "Vídeo y Pantalla",
                            subtitle = "Backend Vulkan/OpenGL ES, resolución, FSR, filtros y bezels",
                            items = 6,
                            index = 0,
                            onClick = { currentSubScreen = "video" },
                        )
                        NavRow(
                            icon = Icons.Filled.Spa,
                            title = "Audio y Sonido",
                            subtitle = "Volúmenes BGM/SFX, audio 3D binaural HRTF y WSOLA",
                            items = 6,
                            index = 1,
                            onClick = { currentSubScreen = "audio" },
                        )
                        NavRow(
                            icon = Icons.Filled.SportsEsports,
                            title = "Mandos y Controles",
                            subtitle = "Controles táctiles, D-Pad dinámico, rueda radial y gamepads",
                            items = 6,
                            index = 2,
                            onClick = { currentSubScreen = "controls" },
                        )
                        NavRow(
                            icon = Icons.Filled.Speed,
                            title = "Rendimiento y Motor",
                            subtitle = "Compilador JIT ARM64 Tier-2, afinidad Big.LITTLE y DRS",
                            items = 6,
                            index = 3,
                            onClick = { currentSubScreen = "performance" },
                        )
                        NavRow(
                            icon = Icons.Filled.Shield,
                            title = "Seguridad y Partidas",
                            subtitle = "Scoped Storage Shield, snapshots, diagnóstico y mods",
                            items = 6,
                            index = 4,
                            onClick = { currentSubScreen = "security" },
                        )
                        NavRow(
                            icon = Icons.Filled.Info,
                            title = "Acerca de y Ayuda",
                            subtitle = "Versión de la app, logs técnicos y comprobador de actualizaciones",
                            items = 6,
                            index = 5,
                            onClick = { currentSubScreen = "about" },
                        )
                    }

                    SettingsGroup("Estado del Juego y Sistema") {
                        if (installed == null) {
                            InfoRow(
                                icon = Icons.Filled.Info,
                                title = "Sin juego instalado",
                                value = "Descárgalo o impórtalo desde la pantalla principal",
                                items = 1,
                                index = 0,
                            )
                        } else {
                            InfoRow(Icons.Filled.VideogameAsset, "Versión del juego", installed.version ?: "1.9.2.2 Full", items = 3, index = 0)
                            InfoRow(Icons.Filled.Storage, "Espacio ocupado", formatBytes(installed.sizeBytes), items = 3, index = 1)
                            InfoRow(Icons.Filled.CalendarMonth, "Instalado el", formatDate(installed.installedAt), items = 3, index = 2)
                        }
                    }
                }

                "video" -> {
                    SettingsGroup("Motor Gráfico de Renderizado (Backend)") {
                                    ChoiceRow(
                                        icon = Icons.Filled.Memory,
                                        title = "OpenGL ES",
                                        subtitle = "Pipeline nativo estable para máxima compatibilidad",
                                        selected = graphicsBackend == "opengl",
                                        items = 2,
                                        index = 0,
                                        onSelect = {
                                            graphicsBackend = "opengl"
                                            prefs.graphicsBackend = "opengl"
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Speed,
                                        title = "Vulkan",
                                        subtitle = "Pipeline gráfico de bajo nivel de alto rendimiento",
                                        selected = graphicsBackend == "vulkan",
                                        items = 2,
                                        index = 1,
                                        onSelect = {
                                            graphicsBackend = "vulkan"
                                            prefs.graphicsBackend = "vulkan"
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                }

                    SettingsGroup("Ajustes de Vídeo (v0.9.0 & Gráficos)") {
                                    SwitchRow(
                                        icon = Icons.Filled.Shield,
                                        title = "Perfil Combate Extremo Mali/Infinix (1 Toque)",
                                        subtitle = "Fija 60 FPS estables en combates: escala 0.85x con nitidez CAS al 60%, FPK Early-Z activo y 40 partículas",
                                        checked = extremeCombat,
                                        items = 15,
                                        index = 0,
                                        onCheckedChange = {
                                            extremeCombat = it
                                            prefs.extremeCombat = it
                                            if (it) {
                                                prefs.applyPerformanceProfile("extreme_combat")
                                                renderScale = prefs.renderScale
                                                fsrMode = prefs.fsrMode
                                                drsEnabled = prefs.drsEnabled
                                                force60Fps = prefs.force60Fps
                                            }
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.AutoAwesome,
                                        title = "AMD FSR 1.0: Desactivado",
                                        subtitle = "Renderizado a resolución nativa estándar sin upscaling espacial",
                                        selected = fsrMode == 0,
                                        items = 15,
                                        index = 1,
                                        onSelect = {
                                            fsrMode = 0
                                            prefs.fsrMode = 0
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.AutoAwesome,
                                        title = "AMD FSR 1.0: Ultra Calidad (0.77x)",
                                        subtitle = "Edge-Adaptive Spatial Upsampling (EASU) + Robust Contrast Adaptive Sharpening (RCAS)",
                                        selected = fsrMode == 1,
                                        items = 15,
                                        index = 2,
                                        onSelect = {
                                            fsrMode = 1
                                            prefs.fsrMode = 1
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.AutoAwesome,
                                        title = "AMD FSR 1.0: Equilibrado (0.67x)",
                                        subtitle = "Balance ideal entre nitidez visual reconstruida y ganancia de fotogramas",
                                        selected = fsrMode == 2,
                                        items = 15,
                                        index = 3,
                                        onSelect = {
                                            fsrMode = 2
                                            prefs.fsrMode = 2
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.AutoAwesome,
                                        title = "AMD FSR 1.0: Máximo Rendimiento (0.50x)",
                                        subtitle = "Reconstruye la imagen desde la mitad de resolución para máxima tasa de cuadros",
                                        selected = fsrMode == 3,
                                        items = 15,
                                        index = 4,
                                        onSelect = {
                                            fsrMode = 3
                                            prefs.fsrMode = 3
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.LightMode,
                                        title = "Iluminación 2D Dinámica por Ataques",
                                        subtitle = "Ataques como Lanzallamas, Rayo o Bola Sombra emiten luces puntuales y sombras en tiempo real",
                                        checked = lighting2d,
                                        items = 15,
                                        index = 5,
                                        onCheckedChange = {
                                            lighting2d = it
                                            prefs.lighting2d = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Palette,
                                        title = "Filtro Gráfico: Original (Sharp Bilinear)",
                                        subtitle = "Aspecto fiel y nítido original de Pokémon Reloaded",
                                        selected = videoFilter == "sharp" && !xbrzEnabled,
                                        items = 15,
                                        index = 6,
                                        onSelect = {
                                            videoFilter = "sharp"
                                            prefs.videoFilter = "sharp"
                                            xbrzEnabled = false
                                            prefs.xbrzEnabled = false
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Palette,
                                        title = "Filtro Gráfico: Super-Resolución xBRZ 4x HD",
                                        subtitle = "Vectorización neuronal en tiempo real de curvas de pixel-art para pantallas 1080p",
                                        selected = xbrzEnabled && videoFilter == "xbrz_4x",
                                        items = 15,
                                        index = 7,
                                        onSelect = {
                                            videoFilter = "xbrz_4x"
                                            prefs.videoFilter = "xbrz_4x"
                                            xbrzEnabled = true
                                            prefs.xbrzEnabled = true
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Palette,
                                        title = "Filtro Gráfico: Super-Resolución xBRZ 6x HD",
                                        subtitle = "Máxima definición de contornos pixel-art para pantallas 2K/AMOLED",
                                        selected = xbrzEnabled && videoFilter == "xbrz_6x",
                                        items = 15,
                                        index = 8,
                                        onSelect = {
                                            videoFilter = "xbrz_6x"
                                            prefs.videoFilter = "xbrz_6x"
                                            xbrzEnabled = true
                                            prefs.xbrzEnabled = true
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Palette,
                                        title = "Filtro Gráfico: AMD FidelityFX FSR Shader",
                                        subtitle = "Aplica pase de nitidez RCAS adaptativo al contraste en la composición final",
                                        selected = videoFilter == "fsr",
                                        items = 15,
                                        index = 9,
                                        onSelect = {
                                            videoFilter = "fsr"
                                            prefs.videoFilter = "fsr"
                                            xbrzEnabled = false
                                            prefs.xbrzEnabled = false
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Tune,
                                        title = "Accesibilidad Cromática: Desactivado",
                                        subtitle = "Gama de colores RGB natural sin transformaciones",
                                        selected = colorblindMode == 0,
                                        items = 15,
                                        index = 10,
                                        onSelect = {
                                            colorblindMode = 0
                                            prefs.colorblindMode = 0
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Tune,
                                        title = "Accesibilidad Cromática: Protanopía",
                                        subtitle = "Matriz de corrección GPU para deficiencia en conos rojos",
                                        selected = colorblindMode == 1,
                                        items = 15,
                                        index = 11,
                                        onSelect = {
                                            colorblindMode = 1
                                            prefs.colorblindMode = 1
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Tune,
                                        title = "Accesibilidad Cromática: Deuteranopía",
                                        subtitle = "Matriz de corrección GPU para deficiencia en conos verdes",
                                        selected = colorblindMode == 2,
                                        items = 15,
                                        index = 12,
                                        onSelect = {
                                            colorblindMode = 2
                                            prefs.colorblindMode = 2
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Tune,
                                        title = "Accesibilidad Cromática: Tritanopía",
                                        subtitle = "Matriz de corrección GPU para deficiencia en conos azules",
                                        selected = colorblindMode == 3,
                                        items = 15,
                                        index = 13,
                                        onSelect = {
                                            colorblindMode = 3
                                            prefs.colorblindMode = 3
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Speed,
                                        title = "Tasa de Refresco Adaptativa LTPO / VRR",
                                        subtitle = "Modula la pantalla entre 10 Hz/30 Hz en diálogos y 60 Hz/120 Hz en movimiento con Surface.setFrameRate",
                                        checked = adaptiveLtpo,
                                        items = 15,
                                        index = 14,
                                        onCheckedChange = {
                                            adaptiveLtpo = it
                                            prefs.adaptiveLtpo = it
                                            prefs.writeVideo(context.filesDir)
                                        },
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

                    SettingsGroup("Escala de Resolución Interna (Render Scale)") {
                                    ChoiceRow(
                                        icon = Icons.Filled.AspectRatio,
                                        title = "0.75x (Modo Rendimiento / GPUs antiguas)",
                                        subtitle = "Alivia la tasa de relleno y gana entre 5 y 15 FPS en GPUs PowerVR/Mali",
                                        selected = renderScale == "0.75",
                                        items = 4,
                                        index = 0,
                                        onSelect = {
                                            renderScale = "0.75"
                                            prefs.renderScale = "0.75"
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.AspectRatio,
                                        title = "1.0x (Nativo Pixel-Perfect)",
                                        subtitle = "Resolución original del juego con nitidez estándar",
                                        selected = renderScale == "1.0",
                                        items = 4,
                                        index = 1,
                                        onSelect = {
                                            renderScale = "1.0"
                                            prefs.renderScale = "1.0"
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.AspectRatio,
                                        title = "1.25x (Super-Sampled)",
                                        subtitle = "Mayor suavidad de bordes para pantallas FHD/OLED",
                                        selected = renderScale == "1.25",
                                        items = 4,
                                        index = 2,
                                        onSelect = {
                                            renderScale = "1.25"
                                            prefs.renderScale = "1.25"
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.AspectRatio,
                                        title = "1.5x (Ultra Definición)",
                                        subtitle = "Máxima calidad visual para terminales de gama alta",
                                        selected = renderScale == "1.5",
                                        items = 4,
                                        index = 3,
                                        onSelect = {
                                            renderScale = "1.5"
                                            prefs.renderScale = "1.5"
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                }

                    SettingsGroup("Filtros y Shaders Gráficos") {
                                    ChoiceRow(
                                        icon = Icons.Filled.Palette,
                                        title = "Sharp Bilinear (Recomendado)",
                                        subtitle = "Suavizado limpio que evita deformaciones de píxeles en pantallas panorámicas",
                                        selected = videoFilter == "sharp",
                                        items = 4,
                                        index = 0,
                                        onSelect = {
                                            videoFilter = "sharp"
                                            prefs.videoFilter = "sharp"
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Palette,
                                        title = "Pixel-Perfect (Nearest Neighbor)",
                                        subtitle = "Píxeles retro 100% nítidos sin filtrado",
                                        selected = videoFilter == "nearest",
                                        items = 4,
                                        index = 1,
                                        onSelect = {
                                            videoFilter = "nearest"
                                            prefs.videoFilter = "nearest"
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Palette,
                                        title = "CRT Retro Scanlines",
                                        subtitle = "Líneas de barrido nostálgicas de monitor clásico",
                                        selected = videoFilter == "crt",
                                        items = 4,
                                        index = 2,
                                        onSelect = {
                                            videoFilter = "crt"
                                            prefs.videoFilter = "crt"
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Palette,
                                        title = "Anti-Aliasing FXAA Ligero",
                                        subtitle = "Suaviza bordes irregulares con impacto mínimo en GPU",
                                        selected = videoFilter == "fxaa",
                                        items = 4,
                                        index = 3,
                                        onSelect = {
                                            videoFilter = "fxaa"
                                            prefs.videoFilter = "fxaa"
                                            prefs.writeVideo(context.filesDir)
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

                    SettingsGroup("Optimización de Clima y Partículas") {
                                    ChoiceRow(
                                        icon = Icons.Filled.HealthAndSafety,
                                        title = "Normal (100% Efectos PC)",
                                        subtitle = "Todas las partículas de lluvia, nieve, niebla y arena activadas",
                                        selected = weatherOpt == "normal",
                                        items = 3,
                                        index = 0,
                                        onSelect = {
                                            weatherOpt = "normal"
                                            prefs.weatherOpt = "normal"
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.HealthAndSafety,
                                        title = "Optimizado (50% Partículas)",
                                        subtitle = "Mantiene la ambientación visual reduciendo a la mitad los quads de renderizado",
                                        selected = weatherOpt == "optimized",
                                        items = 3,
                                        index = 1,
                                        onSelect = {
                                            weatherOpt = "optimized"
                                            prefs.weatherOpt = "optimized"
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.HealthAndSafety,
                                        title = "Mínimo (Tinte Ambiental)",
                                        subtitle = "Sustituye las partículas por un filtro de color para máximos FPS en combates dobles",
                                        selected = weatherOpt == "minimal",
                                        items = 3,
                                        index = 2,
                                        onSelect = {
                                            weatherOpt = "minimal"
                                            prefs.weatherOpt = "minimal"
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                }

                    SettingsGroup("Tasa de Refresco y Rendimiento (60 FPS)") {
                                    SwitchRow(
                                        icon = Icons.Filled.Speed,
                                        title = "Desbloquear 60 FPS (Forzar en zonas a 30 FPS)",
                                        subtitle = "Elimina el límite de 30 FPS en menús y salas fijas para una experiencia fluida a 60 FPS",
                                        checked = force60Fps,
                                        items = 1,
                                        index = 0,
                                        onCheckedChange = {
                                            force60Fps = it
                                            prefs.force60Fps = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                }

                }

                "audio" -> {
                    SettingsGroup("Audio Independiente (BGM y SFX)") {
                                    ChoiceRow(
                                        icon = Icons.Filled.Extension,
                                        title = "Música BGM: 100% (Normal)",
                                        subtitle = "Banda sonora completa del juego",
                                        selected = bgmVolume == 100,
                                        items = 5,
                                        index = 0,
                                        onSelect = {
                                            bgmVolume = 100
                                            prefs.bgmVolume = 100
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Extension,
                                        title = "Música BGM: 50% (Volumen suave)",
                                        subtitle = "Música de fondo atenuada",
                                        selected = bgmVolume == 50,
                                        items = 5,
                                        index = 1,
                                        onSelect = {
                                            bgmVolume = 50
                                            prefs.bgmVolume = 50
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Extension,
                                        title = "Música BGM: 0% (Silenciada)",
                                        subtitle = "Silencia la música del juego para escuchar tu propia música o podcasts",
                                        selected = bgmVolume == 0,
                                        items = 5,
                                        index = 2,
                                        onSelect = {
                                            bgmVolume = 0
                                            prefs.bgmVolume = 0
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Extension,
                                        title = "Efectos SFX: 100% (Normal)",
                                        subtitle = "Sonidos de golpes, ataques, Pokéballs y menús audibles",
                                        selected = sfxVolume == 100,
                                        items = 5,
                                        index = 3,
                                        onSelect = {
                                            sfxVolume = 100
                                            prefs.sfxVolume = 100
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Extension,
                                        title = "Efectos SFX: 0% (Silenciados)",
                                        subtitle = "Silencia todos los efectos sonoros",
                                        selected = sfxVolume == 0,
                                        items = 5,
                                        index = 4,
                                        onSelect = {
                                            sfxVolume = 0
                                            prefs.sfxVolume = 0
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                }

                    SettingsGroup("Ajustes de Audio (v0.9.0 & DSP)") {
                                    SwitchRow(
                                        icon = Icons.Filled.Extension,
                                        title = "Audio Espacial 3D Binaural (HRTF)",
                                        subtitle = "Filtro Head-Related Transfer Function para ubicar la posición acústica de ataques y Pokémon en auriculares",
                                        checked = audio3d,
                                        items = 3,
                                        index = 0,
                                        onCheckedChange = {
                                            audio3d = it
                                            prefs.audio3d = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Extension,
                                        title = "Música Reactiva de Combate (Stems de Tensión)",
                                        subtitle = "Mezcla dinámica en tiempo real que intensifica percusiones y sintetizadores cuando el HP cae por debajo del 20%",
                                        checked = reactiveStems,
                                        items = 3,
                                        index = 1,
                                        onCheckedChange = {
                                            reactiveStems = it
                                            prefs.reactiveStems = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Speed,
                                        title = "Preservación de Tono en Modo Turbo (WSOLA)",
                                        subtitle = "Resampler armónico Waveform Similarity Overlap-Add: acelera el juego de 2x a 8x sin agudos chillones",
                                        checked = wsola,
                                        items = 3,
                                        index = 2,
                                        onCheckedChange = {
                                            wsola = it
                                            prefs.wsola = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                }

                }

                "controls" -> {
                    SettingsGroup("Mando y Controles") {
                                    NavRow(
                                        icon = Icons.Filled.Tune,
                                        title = "Controles táctiles",
                                        subtitle = "Estilo (Moderno/Clásico), movimiento (Joystick/Cruceta), opacidad y editor",
                                        items = 4,
                                        index = 0,
                                        onClick = onTouchSettings,
                                    )
                                    NavRow(
                                        icon = Icons.Filled.SportsEsports,
                                        title = "Configuración de Mando Físico",
                                        subtitle = "Mapeo de botones de mandos Bluetooth/USB y prueba de entrada",
                                        items = 4,
                                        index = 1,
                                        onClick = onControls,
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.SportsEsports,
                                        title = "Autofire / Disparo continuo rápido (15 pulsaciones/s)",
                                        subtitle = "Mantén pulsado Z o X para avanzar textos automáticamente o atacar en ráfagas",
                                        checked = autofireEnabled,
                                        items = 4,
                                        index = 2,
                                        onCheckedChange = {
                                            autofireEnabled = it
                                            prefs.autofireEnabled = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.SportsEsports,
                                        title = "D-Pad Flotante Dinámico",
                                        subtitle = "El joystick aparece dinámicamente donde apoyes el pulgar izquierdo en la pantalla",
                                        checked = dynamicDpad,
                                        items = 4,
                                        index = 3,
                                        onCheckedChange = {
                                            dynamicDpad = it
                                            prefs.dynamicDpad = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                }

                    SettingsGroup("Ajustes de Controles (v0.9.0 & Movilidad)") {
                                    ChoiceRow(
                                        icon = Icons.Filled.SportsEsports,
                                        title = "D-Pad Fijo Clásico",
                                        subtitle = "Posición estática fija del pad virtual en la esquina inferior izquierda",
                                        selected = !dynamicDpad,
                                        items = 7,
                                        index = 0,
                                        onSelect = {
                                            dynamicDpad = false
                                            prefs.dynamicDpad = false
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.SportsEsports,
                                        title = "D-Pad Flotante Dinámico",
                                        subtitle = "El centro del joystick se ancla automáticamente bajo el pulgar al tocar la pantalla",
                                        selected = dynamicDpad,
                                        items = 7,
                                        index = 1,
                                        onSelect = {
                                            dynamicDpad = true
                                            prefs.dynamicDpad = true
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Tune,
                                        title = "Rueda Radial Táctica de Combate (Doom/GTA)",
                                        subtitle = "Aplica dilatación temporal (0.2x) y despliega menú radial a 60 FPS para cambios y objetos rápidos",
                                        checked = radialMenu,
                                        items = 7,
                                        index = 2,
                                        onCheckedChange = {
                                            radialMenu = it
                                            prefs.radialMenu = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.TouchApp,
                                        title = "Gestos Multitáctiles Inteligentes",
                                        subtitle = "Deslizar 3 dedos para Rebobinar tiempo, pellizco con 2 dedos para Zoom y doble toque lateral para Turbo",
                                        checked = gestures,
                                        items = 7,
                                        index = 3,
                                        onCheckedChange = {
                                            gestures = it
                                            prefs.gestures = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.SportsEsports,
                                        title = "Disposición de Controles: Estándar",
                                        subtitle = "Distribución equilibrada tradicional para agarre a dos manos",
                                        selected = !oneHanded,
                                        items = 7,
                                        index = 4,
                                        onSelect = {
                                            oneHanded = false
                                            prefs.oneHanded = false
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.SportsEsports,
                                        title = "Disposición de Controles: Ergonómica Una Mano",
                                        subtitle = "Reposiciona el D-Pad y los botones en arco curvo accesible para el pulgar derecho",
                                        selected = oneHanded,
                                        items = 7,
                                        index = 5,
                                        onSelect = {
                                            oneHanded = true
                                            prefs.oneHanded = true
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Vibration,
                                        title = "Vibración Háptica HD en Mandos Físicos Pro",
                                        subtitle = "Control independiente de motores de baja y alta frecuencia para DualSense, Xbox y Razer Kishi",
                                        checked = fishingHaptic,
                                        items = 7,
                                        index = 6,
                                        onCheckedChange = {
                                            fishingHaptic = it
                                            prefs.fishingHaptic = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                }

                    SettingsGroup("Juego (Comodidad Externa y Opcional)") {
                                    SwitchRow(
                                        icon = Icons.Filled.Spa,
                                        title = "Reloj de Cosecha de Bayas (10 min)",
                                        subtitle = "Aviso HUD al acumular 10 minutos de juego activo cuando las bayas se regeneran",
                                        checked = berryTracker,
                                        items = 11,
                                        index = 0,
                                        onCheckedChange = {
                                            berryTracker = it
                                            prefs.berryTracker = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.DirectionsBike,
                                        title = "Cronómetro de Movimiento para Huevos",
                                        subtitle = "Mide los segundos acumulados pedaleando o corriendo para estimar el avance de eclosión",
                                        checked = eggHatchTimer,
                                        items = 11,
                                        index = 1,
                                        onCheckedChange = {
                                            eggHatchTimer = it
                                            prefs.eggHatchTimer = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.AutoAwesome,
                                        title = "Alerta Visual de Variocolor (Shiny Beacon)",
                                        subtitle = "Destello perimetral dorado en la pantalla al aparecer un Pokémon variocolor en combate",
                                        checked = shinyBeacon,
                                        items = 11,
                                        index = 2,
                                        onCheckedChange = {
                                            shinyBeacon = it
                                            prefs.shinyBeacon = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.TouchApp,
                                        title = "Avance de Diálogos por Toque",
                                        subtitle = "Toca la mitad inferior de la pantalla para pasar textos cómodamente como en una novela visual",
                                        checked = dialogTapAdvance,
                                        items = 11,
                                        index = 3,
                                        onCheckedChange = {
                                            dialogTapAdvance = it
                                            prefs.dialogTapAdvance = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.DirectionsRun,
                                        title = "Modo Auto-Run / Correr Permanente",
                                        subtitle = "Mantiene virtualmente presionado el botón de carrera para evitar la fatiga del pulgar",
                                        checked = autoRun,
                                        items = 11,
                                        index = 4,
                                        onCheckedChange = {
                                            autoRun = it
                                            prefs.autoRun = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Backpack,
                                        title = "Recordatorio de Repelente Agotado",
                                        subtitle = "Muestra un atajo directo de 1 toque para reaplicar repelente al disiparse su efecto",
                                        checked = repelReminder,
                                        items = 11,
                                        index = 5,
                                        onCheckedChange = {
                                            repelReminder = it
                                            prefs.repelReminder = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Star,
                                        title = "Rueda Radial para Objetos Clave",
                                        subtitle = "Atajos rápidos en la rueda radial para Bici, Cañas y Zahorí sin abrir la mochila",
                                        checked = keyItemsRadial,
                                        items = 11,
                                        index = 6,
                                        onCheckedChange = {
                                            keyItemsRadial = it
                                            prefs.keyItemsRadial = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.HourglassEmpty,
                                        title = "Pausa de Encuentros en Salas de Puzles",
                                        subtitle = "Permite silenciar temporalmente los encuentros salvajes mientras se resuelven puzles",
                                        checked = puzzlePause,
                                        items = 11,
                                        index = 7,
                                        onCheckedChange = {
                                            puzzlePause = it
                                            prefs.puzzlePause = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Vibration,
                                        title = "Asistente Háptico de Picada de Pesca",
                                        subtitle = "Pulso de vibración en el teléfono en el instante exacto en que aparece el signo de exclamación al pescar",
                                        checked = fishingHaptic,
                                        items = 11,
                                        index = 8,
                                        onCheckedChange = {
                                            fishingHaptic = it
                                            prefs.fishingHaptic = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Bookmark,
                                        title = "Guardado Rápido Externo en 1 Toque",
                                        subtitle = "Atajo flotante o en rueda radial para volcar una copia de seguridad instantánea del estado",
                                        checked = quickSave,
                                        items = 11,
                                        index = 9,
                                        onCheckedChange = {
                                            quickSave = it
                                            prefs.quickSave = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Alarm,
                                        title = "Recordatorio Preventivo de Guardado",
                                        subtitle = "Aviso periódico configurable para recordar guardar la partida antes de batallas importantes",
                                        checked = saveReminder,
                                        items = 11,
                                        index = 10,
                                        onCheckedChange = {
                                            saveReminder = it
                                            prefs.saveReminder = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                }

                }

                "performance" -> {
                    SettingsGroup("Ajustes de Rendimiento (v0.9.0 & Bajo Nivel)") {
                                    SwitchRow(
                                        icon = Icons.Filled.Memory,
                                        title = "Compilador JIT Tier-2 ARM64 (AArch64 Nativo)",
                                        subtitle = "Recompilador JIT en Rust que emite código máquina nativo en páginas seguras W^X puenteando el intérprete GML",
                                        checked = jitArm64,
                                        items = 5,
                                        index = 0,
                                        onCheckedChange = {
                                            jitArm64 = it
                                            prefs.jitArm64 = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Speed,
                                        title = "Afinidad Big.LITTLE de CPU (sched_setaffinity)",
                                        subtitle = "Ancla los hilos de Lógica y Render a núcleos Cortex-A7x/X y tareas auxiliares a núcleos Cortex-A5x",
                                        checked = bigLittleAffinity,
                                        items = 5,
                                        index = 1,
                                        onCheckedChange = {
                                            bigLittleAffinity = it
                                            prefs.bigLittleAffinity = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Speed,
                                        title = "Resolución Dinámica DRS con Controlador PID",
                                        subtitle = "Muestrea el frametime cada 20 cuadros: modula la resolución entre 0.85x y 1.0x para garantizar 60 FPS",
                                        checked = drsEnabled,
                                        items = 5,
                                        index = 2,
                                        onCheckedChange = {
                                            drsEnabled = it
                                            prefs.drsEnabled = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Shield,
                                        title = "Despertador de Frecuencia GPU Mali (Anti-Downclock)",
                                        subtitle = "Envía sugerencias de 16.66 ms a APerformanceHintManager e inyecta glFlush() para mantener 800-950 MHz",
                                        checked = gpuWakeup,
                                        items = 5,
                                        index = 3,
                                        onCheckedChange = {
                                            gpuWakeup = it
                                            prefs.gpuWakeup = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Memory,
                                        title = "Mega-Atlas Virtual Zero Draw Calls",
                                        subtitle = "Fusiona quads y texturas en un VBO mapeado persistente reduciendo las llamadas de dibujo a 1-3 por frame",
                                        checked = megaAtlas,
                                        items = 5,
                                        index = 4,
                                        onCheckedChange = {
                                            megaAtlas = it
                                            prefs.megaAtlas = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                }

                    SettingsGroup("Velocidad Turbo / Fast-Forward (Botón >>)") {
                                    ChoiceRow(
                                        icon = Icons.Filled.Speed,
                                        title = "Velocidad 1.5x (Acelerado suave)",
                                        subtitle = "Combates dinámicos y fluidos con precisión de movimiento",
                                        selected = turboMultiplier == 1.5f,
                                        items = 5,
                                        index = 0,
                                        onSelect = {
                                            turboMultiplier = 1.5f
                                            prefs.turboMultiplier = 1.5f
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Speed,
                                        title = "Velocidad 2.0x (Doble velocidad)",
                                        subtitle = "Recomendado para exploración y rutas normales",
                                        selected = turboMultiplier == 2.0f,
                                        items = 5,
                                        index = 1,
                                        onSelect = {
                                            turboMultiplier = 2.0f
                                            prefs.turboMultiplier = 2.0f
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Speed,
                                        title = "Velocidad 4.0x (Cuádruple velocidad)",
                                        subtitle = "Farmeo intensivo de niveles y eclosión rápida de huevos",
                                        selected = turboMultiplier == 4.0f,
                                        items = 5,
                                        index = 2,
                                        onSelect = {
                                            turboMultiplier = 4.0f
                                            prefs.turboMultiplier = 4.0f
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Speed,
                                        title = "Velocidad 6.0x (Sextuple velocidad)",
                                        subtitle = "Avance acelerado para jugadores veteranos",
                                        selected = turboMultiplier == 6.0f,
                                        items = 5,
                                        index = 3,
                                        onSelect = {
                                            turboMultiplier = 6.0f
                                            prefs.turboMultiplier = 6.0f
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Speed,
                                        title = "Velocidad 8.0x (Ultra Turbo)",
                                        subtitle = "Máxima velocidad para saltar diálogos y trayectos largos",
                                        selected = turboMultiplier == 8.0f,
                                        items = 5,
                                        index = 4,
                                        onSelect = {
                                            turboMultiplier = 8.0f
                                            prefs.turboMultiplier = 8.0f
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                }

                    SettingsGroup("Salto de Fotogramas (Frame Skip)") {
                                    ChoiceRow(
                                        icon = Icons.Filled.Speed,
                                        title = "Desactivado (0 - Fluidez máxima)",
                                        subtitle = "Dibuja todos los fotogramas (60 FPS puros)",
                                        selected = frameSkip == "0",
                                        items = 4,
                                        index = 0,
                                        onSelect = {
                                            frameSkip = "0"
                                            prefs.frameSkip = "0"
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Speed,
                                        title = "Automático Adaptativo",
                                        subtitle = "Salta cuadros únicamente cuando la GPU experimente caídas",
                                        selected = frameSkip == "auto",
                                        items = 4,
                                        index = 1,
                                        onSelect = {
                                            frameSkip = "auto"
                                            prefs.frameSkip = "auto"
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Speed,
                                        title = "1 Fotograma",
                                        subtitle = "Salta 1 de cada 2 cuadros en pantalla (ideal para chips modestos)",
                                        selected = frameSkip == "1",
                                        items = 4,
                                        index = 2,
                                        onSelect = {
                                            frameSkip = "1"
                                            prefs.frameSkip = "1"
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Speed,
                                        title = "2 Fotogramas",
                                        subtitle = "Máximo ahorro de GPU para dispositivos de gama de entrada",
                                        selected = frameSkip == "2",
                                        items = 4,
                                        index = 3,
                                        onSelect = {
                                            frameSkip = "2"
                                            prefs.frameSkip = "2"
                                            prefs.writeVideo(context.filesDir)
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

                }

                "security" -> {
                    SettingsGroup("Seguridad y Partidas") {
                                    NavRow(
                                        icon = Icons.Filled.Save,
                                        title = "Mis Partidas Guardadas",
                                        subtitle = if (installed == null) {
                                            "El juego no está instalado"
                                        } else {
                                            "Consulta, exporta o gestiona tus archivos de guardado"
                                        },
                                        items = 5,
                                        index = 0,
                                        enabled = installed != null,
                                        onClick = onMySaves,
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Shield,
                                        title = "Escudo Scoped Storage Shield",
                                        subtitle = "Réplica en Documents/PokemonReloadedSaves/ para no perder partidas si desinstalas el APK",
                                        checked = persistentShield,
                                        items = 5,
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
                                        items = 5,
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
                                        items = 5,
                                        index = 3,
                                        enabled = installed != null,
                                        onClick = {
                                            showHealthDialog = true
                                        },
                                    )
                                    NavRow(
                                        icon = Icons.Filled.Extension,
                                        title = "Instalar Mod o Parche (.zip)",
                                        subtitle = if (installingMod) "Instalando parche/mod..." else "Aplica packs de música, sprites .gif o parches a custom_assets/ sin reinstalar",
                                        items = 5,
                                        index = 4,
                                        enabled = installed != null && !installingMod,
                                        onClick = { modPickerLauncher.launch(arrayOf("application/zip", "application/octet-stream", "*/*")) },
                                    )
                                }

                    SettingsGroup("Ajustes de Interfaz & QoL (v0.9.0)") {
                                    ChoiceRow(
                                        icon = Icons.Filled.Palette,
                                        title = "Estilo de Interfaz: Moderno Glassmorphism",
                                        subtitle = "Paneles traslúcidos con desenfoque Gaussiano/Dual-Kawase del juego en vivo y paleta Material You",
                                        selected = uiStyle == "glassmorphic",
                                        items = 9,
                                        index = 0,
                                        onSelect = {
                                            uiStyle = "glassmorphic"
                                            prefs.uiStyle = "glassmorphic"
                                        },
                                    )
                                    ChoiceRow(
                                        icon = Icons.Filled.Palette,
                                        title = "Estilo de Interfaz: Clásico Sólido",
                                        subtitle = "Diseño opaco tradicional con bordes redondeados y estilo clásico",
                                        selected = uiStyle == "classic",
                                        items = 9,
                                        index = 1,
                                        onSelect = {
                                            uiStyle = "classic"
                                            prefs.uiStyle = "classic"
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Save,
                                        title = "Carrusel de Guardados Rápidos (WebP)",
                                        subtitle = "Muestra capturas WebP ultraligeras (160x120) con estado del equipo y mapa en ranuras de guardado",
                                        checked = quickSave,
                                        items = 9,
                                        index = 2,
                                        onCheckedChange = {
                                            quickSave = it
                                            prefs.quickSave = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Info,
                                        title = "Asistente Flotante de Debilidades de Pokédex",
                                        subtitle = "Widget compacto en combate que calcula y muestra debilidades (x2, x4), resistencias e inmunidades del rival",
                                        checked = berryTracker,
                                        items = 9,
                                        index = 3,
                                        onCheckedChange = {
                                            berryTracker = it
                                            prefs.berryTracker = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Fullscreen,
                                        title = "Modo Ventana Flotante / Picture-in-Picture (PiP)",
                                        subtitle = "Continúa farmeando o caminando con el emulador en miniatura sobre otras aplicaciones",
                                        checked = pipMode,
                                        items = 9,
                                        index = 4,
                                        onCheckedChange = {
                                            pipMode = it
                                            prefs.pipMode = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Alarm,
                                        title = "Notificación Dinámica en Segundo Plano (Live Activity)",
                                        subtitle = "Aviso en vivo de Android 14/15/16 en barra de estado con pasos restantes para huevos y repelente",
                                        checked = liveActivity,
                                        items = 9,
                                        index = 5,
                                        onCheckedChange = {
                                            liveActivity = it
                                            prefs.liveActivity = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Speed,
                                        title = "Salto de Intro de Batalla (Smart Pre-Roll Skip)",
                                        subtitle = "Omite la animación del remolino al entrar en combate en hierba alta para entrar directo a la acción",
                                        checked = smartPrerollSkip,
                                        items = 9,
                                        index = 6,
                                        onCheckedChange = {
                                            smartPrerollSkip = it
                                            prefs.smartPrerollSkip = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Article,
                                        title = "Caja Negra de Repetición Determinista (<30 KB)",
                                        subtitle = "Graba la semilla RNG y las pulsaciones por cuadro para exportar y reproducir batallas a 60/120 FPS",
                                        checked = replayBlackbox,
                                        items = 9,
                                        index = 7,
                                        onCheckedChange = {
                                            replayBlackbox = it
                                            prefs.replayBlackbox = it
                                            prefs.writeVideo(context.filesDir)
                                        },
                                    )
                                    SwitchRow(
                                        icon = Icons.Filled.Star,
                                        title = "Sistema de Logros Retro con Banners Animados",
                                        subtitle = "Notificaciones emergentes a 60 FPS al cumplir hitos de gimnasio, variocolores o retos especiales",
                                        checked = achievements,
                                        items = 9,
                                        index = 8,
                                        onCheckedChange = {
                                            achievements = it
                                            prefs.achievements = it
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

                    SettingsGroup("Actualizaciones de la Aplicación") {
                        NavRow(
                            icon = Icons.Filled.SystemUpdate,
                            title = "🔄 Buscar actualizaciones de versión (GitHub)",
                            subtitle = "Instalada: v$appVersion (build 33) · Toca para comprobar GitHub Releases",
                            items = 1,
                            index = 0,
                            enabled = !checking,
                            onClick = {
                                checking = true
                                scope.launch {
                                    val result = Update.check(appVersion)
                                    checking = false
                                    val found = result.getOrNull()
                                    if (found != null) {
                                        onUpdateFound(found)
                                    } else {
                                        snackbar.showSnackbar("¡Estás al día! Tienes la última versión instalada (v$appVersion EX).")
                                    }
                                }
                            },
                        )
                    }
                }

                "about" -> {
                    SettingsGroup("Actualizaciones y Ayuda") {
                                    NavRow(
                                        icon = Icons.Filled.SystemUpdate,
                                        title = "🔄 Buscar actualizaciones de versión (GitHub)",
                                        subtitle = "Instalada: v$appVersion (build 33) · Toca para comprobar GitHub Releases",
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
                                                    snackbar.showSnackbar("¡Estás al día! Tienes la última versión instalada (v$appVersion EX).")
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
                                        items = if (currentCrashDump != null) 4 else 3,
                                        index = 2,
                                        onClick = onAbout,
                                    )
                                    if (currentCrashDump != null) {
                                        NavRow(
                                            icon = Icons.Filled.BugReport,
                                            title = "Reporte de Fallo (Crash Dump)",
                                            subtitle = "Se detectó un informe de error del motor. Toca para ver y copiar",
                                            items = 4,
                                            index = 3,
                                            onClick = { showCrashDumpDialog = true },
                                        )
                                    }
                                }

                }
            }

            Spacer(Modifier.height(BottomNavSpace))
        }
    }
}
