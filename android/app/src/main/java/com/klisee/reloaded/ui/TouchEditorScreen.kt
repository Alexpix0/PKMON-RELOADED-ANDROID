package com.klisee.reloaded.ui

import android.graphics.BitmapFactory
import android.graphics.Paint
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Keyboard
import androidx.compose.material.icons.filled.RestartAlt
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.SportsEsports
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.Saver
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.*
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import android.view.HapticFeedbackConstants
import com.klisee.reloaded.*
import java.io.File
import kotlin.math.hypot
import kotlin.math.roundToInt
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

private val LayoutSaver = Saver<Map<String, TouchPlacement>, String>(
    save = { TouchLayouts.encode(it) }, restore = { TouchLayouts.parse(it) },
)

@Composable
fun TouchEditorScreen(prefs: Preferences, filesDir: File, onExit: () -> Unit) {
    var userProfile by rememberSaveable { mutableStateOf(prefs.activeUserProfile) }
    var userProfileMenu by remember { mutableStateOf(false) }
    var values by rememberSaveable(stateSaver = LayoutSaver) { mutableStateOf(prefs.touchLayouts(userProfile)) }
    var savedText by rememberSaveable { mutableStateOf(TouchLayouts.encode(prefs.touchLayouts(userProfile))) }
    var profileId by rememberSaveable { mutableStateOf(TouchProfile.WORLD.id) }
    val profile = TouchProfile.entries.first { it.id == profileId }
    val controls = remember(profile, values) { TouchLayouts.controlsFor(profile, values) }
    var selectedId by rememberSaveable { mutableStateOf("joystick") }
    val selected = controls.firstOrNull { it.id == selectedId } ?: controls.firstOrNull()
    var opacityPercent by rememberSaveable { mutableStateOf(prefs.touchOpacityPercent.coerceIn(15, 100)) }

    var profileMenu by remember { mutableStateOf(false) }
    var controlMenu by remember { mutableStateOf(false) }
    var keyPickerMode by remember { mutableStateOf<String?>(null) }
    var confirmReset by remember { mutableStateOf(false) }
    var confirmResetTLB by remember { mutableStateOf(false) }
    var confirmExit by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var saving by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    val dirty = TouchLayouts.encode(values) != savedText || opacityPercent != prefs.touchOpacityPercent

    fun update(spec: TouchControlSpec, placement: TouchPlacement) {
        val key = TouchLayouts.key(profile, spec)
        values = if (placement == TouchLayouts.original(spec, profile) && !TouchLayouts.added(profile, spec)) values - key else values + (key to placement)
    }
    fun switchUserProfile(newIndex: Int) {
        if (newIndex == userProfile) return
        val currentSnapshot = values
        prefs.saveTouchLayouts(currentSnapshot, filesDir, userProfile)
        userProfile = newIndex
        prefs.activeUserProfile = newIndex
        values = prefs.touchLayouts(newIndex)
        savedText = TouchLayouts.encode(values)
        selectedId = TouchLayouts.controlsFor(profile, values).firstOrNull()?.id.orEmpty()
    }
    fun copyFromP1() {
        values = prefs.touchLayouts(0)
    }
    fun resetToReloadedDefault() {
        values = emptyMap()
    }
    fun exit() { if (dirty) confirmExit = true else onExit() }
    fun save() {
        if (saving) return
        saving = true
        val snapshot = values
        scope.launch {
            try {
                withContext(Dispatchers.IO) {
                    prefs.touchOpacityPercent = opacityPercent
                    prefs.activeUserProfile = userProfile
                    prefs.saveTouchLayouts(snapshot, filesDir, userProfile)
                    GameActivity.reloadLayout(userProfile)
                }
                savedText = TouchLayouts.encode(snapshot)
                onExit()
            } catch (failure: Exception) {
                error = failure.message ?: "No se pudo guardar. Vuelve a intentarlo."
            } finally { saving = false }
        }
    }
    BackHandler { if (!saving) exit() }

    Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
    BoxWithConstraints(Modifier.fillMaxSize()) {
        val gameAspect = (maxWidth.value / maxHeight.value).coerceAtLeast(1f)
        Row(Modifier.fillMaxSize().padding(12.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Column(Modifier.weight(1f).fillMaxHeight(), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    TextButton(onClick = { exit() }, enabled = !saving) { Text("Volver") }

                    // Selector de Perfil de Usuario (Perfil 1, 2, 3, 4)
                    Box {
                        OutlinedButton(
                            onClick = { userProfileMenu = true },
                            enabled = !saving,
                            modifier = Modifier.testTag("user-profile-picker")
                        ) {
                            Text("Perfil ${userProfile + 1}")
                        }
                        DropdownMenu(expanded = userProfileMenu, onDismissRequest = { userProfileMenu = false }) {
                            for (p in 0..3) {
                                DropdownMenuItem(
                                    text = { Text("Perfil ${p + 1}") },
                                    onClick = {
                                        switchUserProfile(p)
                                        userProfileMenu = false
                                    }
                                )
                            }
                        }
                    }

                    // Selector de Contexto de juego
                    Box {
                        OutlinedButton(onClick = { profileMenu = true }, enabled = !saving,
                            modifier = Modifier.testTag("profile-picker")) { Text("Modo: ${profile.title}") }
                        DropdownMenu(expanded = profileMenu, onDismissRequest = { profileMenu = false }) {
                            for (item in TouchProfile.entries) {
                                DropdownMenuItem(text = { Text(item.title) }, onClick = {
                                    profileId = item.id
                                    selectedId = TouchLayouts.controlsFor(item, values).firstOrNull()?.id.orEmpty()
                                    profileMenu = false
                                })
                            }
                        }
                    }
                }
                Text(profile.description, style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant)
                BoxWithConstraints(Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                    val previewWidth = minOf(maxWidth, maxHeight * gameAspect)
                    ControlPreview(
                        profile = profile, values = values, selectedId = selected?.id, enabled = !saving,
                        keepAspect = prefs.keepAspect, opacity = opacityPercent / 100f, appearance = prefs.touchAppearance,
                        onSelect = { selectedId = it }, onChange = { spec, value -> update(spec, value) },
                        modifier = Modifier.width(previewWidth).aspectRatio(gameAspect)
                            .clip(RoundedCornerShape(16.dp)).border(1.dp, MaterialTheme.colorScheme.outlineVariant, RoundedCornerShape(16.dp)),
                    )
                }
                Text(if (controls.isEmpty()) "Este modo conserva el toque directo."
                    else "Arrastrá un control para moverlo. También podés elegirlo en la lista.",
                    style = MaterialTheme.typography.bodySmall)
                Text("El juego cambia de perfil automáticamente.", style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.primary)
            }

            Surface(Modifier.width(260.dp).fillMaxHeight(), shape = RoundedCornerShape(20.dp),
                color = MaterialTheme.colorScheme.surfaceContainer) {
                Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Column(Modifier.weight(1f).verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text("Personalizar controles", style = MaterialTheme.typography.titleMedium)

                        // Botón Agregar nuevo botón (habilitado en todos los perfiles, incluido Menús y caja)
                        Button(
                            onClick = { keyPickerMode = "add" },
                            enabled = !saving,
                            modifier = Modifier.fillMaxWidth().testTag("add-control"),
                            contentPadding = PaddingValues(horizontal = 12.dp, vertical = 8.dp)
                        ) {
                            Icon(Icons.Filled.Add, contentDescription = null, modifier = Modifier.size(18.dp))
                            Spacer(Modifier.width(6.dp))
                            Text("Agregar nuevo botón")
                        }

                        if (selected == null) {
                            Text("Sin controles para editar en este perfil.", style = MaterialTheme.typography.bodyMedium)
                        } else {
                            val placement = TouchLayouts.placement(values, profile, selected)

                            // Selector de botón actual con tecla asignada
                            Text("Botón seleccionado:", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Box {
                                val assignedLabel = placement.customKey ?: selected.label
                                OutlinedButton(
                                    onClick = { controlMenu = true },
                                    enabled = !saving,
                                    modifier = Modifier.fillMaxWidth().testTag("control-picker"),
                                    contentPadding = PaddingValues(horizontal = 10.dp, vertical = 6.dp)
                                ) {
                                    Text(
                                        text = "${selected.title} [${assignedLabel}]" + if (placement.visible) "" else " (oculto)",
                                        maxLines = 1,
                                        style = MaterialTheme.typography.bodyMedium
                                    )
                                }
                                DropdownMenu(
                                    expanded = controlMenu,
                                    onDismissRequest = { controlMenu = false },
                                    modifier = Modifier.heightIn(max = 280.dp)
                                ) {
                                    for (spec in controls) {
                                        val p = TouchLayouts.placement(values, profile, spec)
                                        val keyLabel = p.customKey ?: spec.label
                                        DropdownMenuItem(
                                            text = { Text("${spec.title} [$keyLabel]" + if (p.visible) "" else " · oculto") },
                                            onClick = {
                                                selectedId = spec.id
                                                controlMenu = false
                                            }
                                        )
                                    }
                                }
                            }

                            // Botón Cambiar Tecla Asignada
                            if (!selected.joystick) {
                                OutlinedButton(
                                    onClick = { keyPickerMode = "remap" },
                                    enabled = !saving,
                                    modifier = Modifier.fillMaxWidth().testTag("remap-key"),
                                    contentPadding = PaddingValues(horizontal = 8.dp, vertical = 6.dp)
                                ) {
                                    Icon(Icons.Filled.Keyboard, contentDescription = null, modifier = Modifier.size(16.dp))
                                    Spacer(Modifier.width(6.dp))
                                    Text(
                                        text = "Tecla: ${placement.customKey ?: selected.label} (Cambiar)",
                                        style = MaterialTheme.typography.bodyMedium,
                                        maxLines = 1
                                    )
                                }
                            }

                            // Switch Visible
                            Row(
                                Modifier.fillMaxWidth(),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.SpaceBetween
                            ) {
                                Text("Visible", style = MaterialTheme.typography.bodyMedium)
                                Switch(
                                    checked = placement.visible,
                                    onCheckedChange = { update(selected, placement.copy(visible = it)) },
                                    enabled = !saving,
                                    modifier = Modifier.testTag("control-visible").semantics {
                                        contentDescription = "Mostrar ${selected.title} en ${profile.title}"
                                    }
                                )
                            }

                            // Slider Tamaño (50% a 200%)
                            Text("Tamaño: ${(placement.scale * 100).roundToInt()} %", style = MaterialTheme.typography.bodyMedium)
                            Slider(
                                value = placement.scale,
                                onValueChange = { update(selected, placement.copy(scale = it)) },
                                valueRange = .5f..2f,
                                steps = 29,
                                enabled = !saving,
                                modifier = Modifier.testTag("control-size").semantics { contentDescription = "Tamaño de ${selected.title}" }
                            )

                            // Slider Opacidad (15% a 100%)
                            Text("Opacidad: $opacityPercent %", style = MaterialTheme.typography.bodyMedium)
                            Slider(
                                value = opacityPercent.toFloat(),
                                onValueChange = { opacityPercent = it.roundToInt() },
                                valueRange = 15f..100f,
                                steps = 17,
                                enabled = !saving,
                                modifier = Modifier.testTag("control-opacity")
                            )

                            // Sliders Horizontal y Vertical
                            Text("Horizontal: ${(placement.x * 100).roundToInt()} %", style = MaterialTheme.typography.labelSmall)
                            Slider(
                                value = placement.x,
                                onValueChange = { update(selected, placement.copy(x = it)) },
                                valueRange = 0f..1f,
                                enabled = !saving,
                                modifier = Modifier.testTag("control-x").semantics { contentDescription = "Posición horizontal de ${selected.title}" }
                            )
                            Text("Vertical: ${(placement.y * 100).roundToInt()} %", style = MaterialTheme.typography.labelSmall)
                            Slider(
                                value = placement.y,
                                onValueChange = { update(selected, placement.copy(y = it)) },
                                valueRange = 0f..1f,
                                enabled = !saving,
                                modifier = Modifier.testTag("control-y").semantics { contentDescription = "Posición vertical de ${selected.title}" }
                            )

                            // Botón Eliminar o Ocultar botón
                            val isAdded = TouchLayouts.added(profile, selected)
                            TextButton(
                                onClick = {
                                    if (isAdded) {
                                        values = values - TouchLayouts.key(profile, selected)
                                    } else {
                                        update(selected, placement.copy(visible = false))
                                    }
                                    selectedId = TouchLayouts.controlsFor(profile, values).firstOrNull()?.id.orEmpty()
                                },
                                enabled = !saving,
                                colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
                                modifier = Modifier.fillMaxWidth().testTag("remove-control")
                            ) {
                                Icon(Icons.Filled.Delete, contentDescription = null, modifier = Modifier.size(16.dp))
                                Spacer(Modifier.width(4.dp))
                                Text(if (isAdded) "Eliminar botón" else "Ocultar botón")
                            }

                            // Botón Restablecer este botón
                            TextButton(
                                onClick = { update(selected, TouchLayouts.original(selected, profile)) },
                                enabled = !saving,
                                modifier = Modifier.fillMaxWidth()
                            ) {
                                Text("Restablecer este control")
                            }

                            // Botón Restablecer este perfil
                            OutlinedButton(
                                onClick = { confirmReset = true },
                                enabled = !saving,
                                modifier = Modifier.fillMaxWidth()
                            ) {
                                Icon(Icons.Filled.RestartAlt, contentDescription = null, modifier = Modifier.size(16.dp))
                                Spacer(Modifier.width(4.dp))
                                Text("Restablecer modo actual")
                            }

                            // Botón Copiar del Perfil 1 (si estamos en Perfil 2, 3 o 4)
                            if (userProfile != 0) {
                                OutlinedButton(
                                    onClick = { copyFromP1() },
                                    enabled = !saving,
                                    modifier = Modifier.fillMaxWidth()
                                ) {
                                    Text("Copiar diseño del Perfil 1")
                                }
                            }

                            // Botón Restablecer al predeterminado de Reloaded TLB
                            OutlinedButton(
                                onClick = { confirmResetTLB = true },
                                enabled = !saving,
                                modifier = Modifier.fillMaxWidth()
                            ) {
                                Icon(Icons.Filled.RestartAlt, contentDescription = null, modifier = Modifier.size(16.dp))
                                Spacer(Modifier.width(4.dp))
                                Text("Predeterminado TLB")
                            }
                        }
                    }
                    Button(onClick = { save() }, enabled = !saving, modifier = Modifier.fillMaxWidth().testTag("save-exit")) {
                        Text(if (saving) "Guardando…" else "Guardar y salir")
                    }
                }
            }
        }
    }
    }

    if (keyPickerMode != null) {
        val currentKeyTag = if (keyPickerMode == "remap" && selected != null) {
            TouchLayouts.placement(values, profile, selected).customKey ?: selected.label
        } else null
        KeySelectionDialog(
            title = if (keyPickerMode == "add") "Agregar nuevo botón a ${profile.title}" else "Asignar tecla a ${selected?.title}",
            currentKeyTag = currentKeyTag,
            onKeySelected = { choice ->
                if (keyPickerMode == "remap" && selected != null) {
                    val p = TouchLayouts.placement(values, profile, selected)
                    update(selected, p.copy(customKey = choice.keyTag))
                } else if (keyPickerMode == "add") {
                    val candidates = TouchLayouts.availableFor(profile).filter { !it.joystick }
                    val targetSpec = candidates.firstOrNull { it.id.equals(choice.keyTag, ignoreCase = true) && it !in controls }
                        ?: candidates.firstOrNull { it !in controls }
                        ?: candidates.firstOrNull { !TouchLayouts.placement(values, profile, it).visible }
                    if (targetSpec != null) {
                        val p = TouchPlacement(0.5f, 0.5f, 1f, true, choice.keyTag)
                        values = values + (TouchLayouts.key(profile, targetSpec) to p)
                        selectedId = targetSpec.id
                    }
                }
                keyPickerMode = null
            },
            onResetToDefault = if (keyPickerMode == "remap" && selected != null) {
                {
                    val p = TouchLayouts.placement(values, profile, selected)
                    update(selected, p.copy(customKey = null))
                    keyPickerMode = null
                }
            } else null,
            onDismiss = { keyPickerMode = null }
        )
    }

    if (confirmReset) AlertDialog(onDismissRequest = { confirmReset = false },
        title = { Text("Restablecer ${profile.title}") },
        text = { Text("Se recuperarán las posiciones, tamaños y visibilidad originales de este modo. Pulsa Guardar y salir para aplicar los cambios.") },
        confirmButton = { TextButton(onClick = { values = TouchLayouts.resetProfile(values, profile); confirmReset = false }) { Text("Restablecer") } },
        dismissButton = { TextButton(onClick = { confirmReset = false }) { Text("Cancelar") } })
    if (confirmResetTLB) AlertDialog(onDismissRequest = { confirmResetTLB = false },
        title = { Text("Restablecer al predeterminado de Reloaded TLB") },
        text = { Text("Se restablecerán todos los controles de este perfil a los valores originales predeterminados de Pokémon Reloaded TLB.") },
        confirmButton = { TextButton(onClick = { resetToReloadedDefault(); confirmResetTLB = false }) { Text("Restablecer") } },
        dismissButton = { TextButton(onClick = { confirmResetTLB = false }) { Text("Cancelar") } })
    if (confirmExit) AlertDialog(onDismissRequest = { confirmExit = false },
        title = { Text("¿Salir sin guardar?") }, text = { Text("Hay cambios en los controles que todavía no guardaste.") },
        confirmButton = { TextButton(onClick = { confirmExit = false; onExit() }) { Text("Descartar y salir") } },
        dismissButton = { TextButton(onClick = { confirmExit = false }) { Text("Seguir editando") } })
    error?.let { message -> AlertDialog(onDismissRequest = { error = null }, title = { Text("No se pudo guardar") },
        text = { Text(message) }, confirmButton = { TextButton(onClick = { error = null }) { Text("Aceptar") } }) }
}

@Composable
private fun KeySelectionDialog(
    title: String,
    currentKeyTag: String?,
    onKeySelected: (PcKeyChoice) -> Unit,
    onResetToDefault: (() -> Unit)? = null,
    onDismiss: () -> Unit,
) {
    val cyanAccent = Color(0xFF00E5FF)
    var searchQuery by remember { mutableStateOf("") }
    var selectedCategory by remember { mutableStateOf("Todas") }
    var detectedKey by remember { mutableStateOf<PcKeyChoice?>(null) }
    val view = LocalView.current

    val categories = listOf("Todas", "Letras", "Números", "Funciones", "Especiales", "Flechas", "Mando")

    val filteredKeys = remember(searchQuery, selectedCategory) {
        val q = searchQuery.trim().lowercase()
        if (q.isNotEmpty()) {
            AVAILABLE_KEY_CHOICES.filter {
                it.name.lowercase().contains(q) ||
                it.keyTag.lowercase().contains(q) ||
                it.category.lowercase().contains(q)
            }
        } else if (selectedCategory == "Todas") {
            AVAILABLE_KEY_CHOICES
        } else {
            AVAILABLE_KEY_CHOICES.filter { it.category.startsWith(selectedCategory) }
        }
    }

    Dialog(
        onDismissRequest = onDismiss,
        properties = DialogProperties(usePlatformDefaultWidth = false)
    ) {
        Surface(
            modifier = Modifier
                .fillMaxWidth(0.94f)
                .fillMaxHeight(0.88f),
            shape = RoundedCornerShape(16.dp),
            color = MaterialTheme.colorScheme.surface,
            tonalElevation = 6.dp,
            shadowElevation = 12.dp
        ) {
            Row(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(16.dp),
                horizontalArrangement = Arrangement.spacedBy(16.dp)
            ) {
                // PANEL IZQUIERDO (40% de ancho)
                Column(
                    modifier = Modifier
                        .weight(0.40f)
                        .fillMaxHeight(),
                    verticalArrangement = Arrangement.spacedBy(10.dp)
                ) {
                    // Cabecera con icono, título y badge de tecla actual
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Icon(
                            Icons.Filled.Keyboard,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.size(24.dp)
                        )
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = title,
                                style = MaterialTheme.typography.titleMedium,
                                maxLines = 1
                            )
                            if (currentKeyTag != null) {
                                Text(
                                    text = "Actual: $currentKeyTag",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.primary
                                )
                            }
                        }
                    }

                    // Buscador OutlinedTextField
                    OutlinedTextField(
                        value = searchQuery,
                        onValueChange = { searchQuery = it },
                        modifier = Modifier.fillMaxWidth(),
                        placeholder = { Text("Buscar tecla o botón (Z, Espacio...)", style = MaterialTheme.typography.bodySmall) },
                        leadingIcon = { Icon(Icons.Filled.Search, contentDescription = "Buscar", modifier = Modifier.size(18.dp)) },
                        trailingIcon = {
                            if (searchQuery.isNotEmpty()) {
                                IconButton(onClick = { searchQuery = "" }) {
                                    Icon(Icons.Filled.Close, contentDescription = "Limpiar", modifier = Modifier.size(18.dp))
                                }
                            }
                        },
                        singleLine = true,
                        textStyle = MaterialTheme.typography.bodyMedium
                    )

                    // Tarjeta reactiva PadProbe (resalta en cian al detectar tecla física o mando)
                    Surface(
                        color = if (detectedKey != null) cyanAccent.copy(alpha = 0.25f) else MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.6f),
                        border = if (detectedKey != null) BorderStroke(1.5.dp, cyanAccent) else null,
                        shape = RoundedCornerShape(10.dp),
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Row(
                            modifier = Modifier.padding(horizontal = 10.dp, vertical = 8.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Icon(
                                Icons.Filled.SportsEsports,
                                contentDescription = null,
                                modifier = Modifier.size(20.dp),
                                tint = if (detectedKey != null) cyanAccent else MaterialTheme.colorScheme.onSurfaceVariant
                            )
                            Text(
                                text = if (detectedKey != null) "¡Detectado: ${detectedKey!!.name} (${detectedKey!!.keyTag})!"
                                else "Pulsa un botón físico de mando o teclado para asignar",
                                style = MaterialTheme.typography.labelSmall,
                                color = if (detectedKey != null) cyanAccent else MaterialTheme.colorScheme.onSurfaceVariant
                            )
                        }
                    }

                    AndroidView(
                        factory = { ctx -> PadProbe(ctx).apply { requestFocus() } },
                        modifier = Modifier.fillMaxWidth().height(1.dp),
                        update = { probe ->
                            probe.onKey = { code ->
                                val choice = keyChoiceForCode(code)
                                detectedKey = choice
                                onKeySelected(choice)
                            }
                            probe.requestFocus()
                        }
                    )

                    // Selector de categorías en scroll horizontal
                    ScrollableTabRow(
                        selectedTabIndex = categories.indexOf(selectedCategory).coerceAtLeast(0),
                        edgePadding = 0.dp,
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        categories.forEach { cat ->
                            Tab(
                                selected = selectedCategory == cat,
                                onClick = { selectedCategory = cat },
                                text = { Text(cat, style = MaterialTheme.typography.labelSmall) }
                            )
                        }
                    }

                    Spacer(modifier = Modifier.weight(1f))

                    // Botones inferiores de acción
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        OutlinedButton(
                            onClick = onDismiss,
                            modifier = Modifier.weight(1f)
                        ) {
                            Text("Cancelar", style = MaterialTheme.typography.labelMedium)
                        }
                        if (onResetToDefault != null) {
                            FilledTonalButton(
                                onClick = onResetToDefault,
                                modifier = Modifier.weight(1f)
                            ) {
                                Text("Restablecer a TLB Original", style = MaterialTheme.typography.labelMedium)
                            }
                        }
                    }
                }

                // Divisor vertical
                Box(
                    modifier = Modifier
                        .fillMaxHeight()
                        .width(1.dp)
                        .background(MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.5f))
                )

                // PANEL DERECHO (60% de ancho)
                Column(
                    modifier = Modifier
                        .weight(0.60f)
                        .fillMaxHeight(),
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = "${filteredKeys.size} opciones disponibles",
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                        Text(
                            text = "Toca para asignar o agregar al instante",
                            style = MaterialTheme.typography.labelSmall,
                            color = cyanAccent
                        )
                    }

                    LazyVerticalGrid(
                        columns = GridCells.Adaptive(minSize = 72.dp),
                        modifier = Modifier
                            .fillMaxWidth()
                            .weight(1f),
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                        verticalArrangement = Arrangement.spacedBy(6.dp),
                        contentPadding = PaddingValues(vertical = 4.dp)
                    ) {
                        items(filteredKeys) { choice ->
                            val isSelected = currentKeyTag?.equals(choice.keyTag, ignoreCase = true) == true
                            Surface(
                                shape = RoundedCornerShape(8.dp),
                                color = if (isSelected) cyanAccent.copy(alpha = 0.22f) else MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                                border = BorderStroke(
                                    if (isSelected) 1.5.dp else 1.dp,
                                    if (isSelected) cyanAccent else MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.3f)
                                ),
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .clickable {
                                        view.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
                                        onKeySelected(choice)
                                    }
                            ) {
                                Column(
                                    modifier = Modifier.padding(horizontal = 6.dp, vertical = 8.dp),
                                    horizontalAlignment = Alignment.CenterHorizontally,
                                    verticalArrangement = Arrangement.Center
                                ) {
                                    Text(
                                        text = choice.name,
                                        style = MaterialTheme.typography.titleSmall,
                                        color = if (isSelected) cyanAccent else MaterialTheme.colorScheme.onSurface,
                                        maxLines = 1
                                    )
                                    Text(
                                        text = choice.keyTag,
                                        style = MaterialTheme.typography.labelSmall,
                                        color = if (isSelected) cyanAccent.copy(alpha = 0.85f) else MaterialTheme.colorScheme.onSurfaceVariant,
                                        maxLines = 1
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun ControlPreview(
    profile: TouchProfile, values: Map<String, TouchPlacement>, selectedId: String?, enabled: Boolean,
    keepAspect: Boolean, opacity: Float, appearance: TouchAppearance, onSelect: (String) -> Unit,
    onChange: (TouchControlSpec, TouchPlacement) -> Unit, modifier: Modifier = Modifier,
) {
    val images = controlImages(appearance, profile = profile)
    val specs = remember(profile, values) { TouchLayouts.controlsFor(profile, values) }
    val currentSpecs by rememberUpdatedState(specs)
    val currentValues by rememberUpdatedState(values)
    val currentSelection by rememberUpdatedState(selectedId)
    val select by rememberUpdatedState(onSelect)
    val update by rememberUpdatedState(onChange)
    val touchRadius = with(LocalDensity.current) { 20.dp.toPx() }
    val paint = remember { Paint(Paint.ANTI_ALIAS_FLAG).apply { textAlign = Paint.Align.CENTER; typeface = android.graphics.Typeface.create("sans-serif-black", android.graphics.Typeface.BOLD) } }

    fun pick(point: Offset, width: Float, height: Float): TouchControlSpec? = currentSpecs
        .filter { TouchLayouts.placement(currentValues, profile, it).visible || it.id == currentSelection }
        .map { spec ->
            val g = TouchLayouts.geometry(spec, TouchLayouts.placement(currentValues, profile, spec), width, height, keepAspect, TouchLayouts.added(profile, spec))
            val dx = kotlin.math.abs(point.x - g.x)
            val dy = kotlin.math.abs(point.y - g.y)
            val inside = if (spec.asset != null && !spec.joystick)
                dx <= maxOf(touchRadius, g.halfWidth) && dy <= maxOf(touchRadius, g.radius)
            else hypot(dx, dy) <= maxOf(touchRadius, g.radius)
            Triple(spec, hypot(dx, dy), inside)
        }.filter { it.third }.minByOrNull { it.second }?.first

    Canvas(modifier.testTag("control-preview").semantics { contentDescription = "Vista previa de ${profile.title}; arrastra para mover los controles" }
        .pointerInput(profile, keepAspect, enabled) {
            if (!enabled) return@pointerInput
            detectTapGestures { point -> pick(point, size.width.toFloat(), size.height.toFloat())?.let { select(it.id) } }
        }.pointerInput(profile, keepAspect, enabled) {
            if (!enabled) return@pointerInput
            var dragging: TouchControlSpec? = null
            var dragPlacement: TouchPlacement? = null
            detectDragGestures(onDragStart = { point ->
                dragging = pick(point, size.width.toFloat(), size.height.toFloat())
                dragging?.let {
                    dragPlacement = TouchLayouts.placement(currentValues, profile, it)
                    select(it.id)
                }
            }, onDragEnd = { dragging = null; dragPlacement = null },
                onDragCancel = { dragging = null; dragPlacement = null }) { change, amount ->
                dragging?.let { spec ->
                    change.consume()
                    val p = dragPlacement ?: TouchLayouts.placement(currentValues, profile, spec)
                    val next = TouchLayouts.moved(spec, p, amount.x, amount.y, size.width.toFloat(), size.height.toFloat(), keepAspect, TouchLayouts.added(profile, spec))
                    dragPlacement = next
                    update(spec, next)
                }
            }
        }) {
        drawRect(Color(0xFF131B25))
        val viewport = TouchLayouts.viewport(size.width, size.height, keepAspect)
        if (keepAspect) {
            drawRect(Color(0xFF090C12))
            drawRect(Color(0xFF131B25), Offset(viewport[0] * size.width, viewport[1] * size.height),
                Size(viewport[2] * size.width, viewport[3] * size.height))
        }
        for (i in 1..9) {
            drawLine(Color.White.copy(alpha = .035f), Offset(size.width * i / 10f, 0f), Offset(size.width * i / 10f, size.height))
        }
        for (i in 1..5) drawLine(Color.White.copy(alpha = .035f), Offset(0f, size.height * i / 6f), Offset(size.width, size.height * i / 6f))
        for (spec in specs) {
            val p = TouchLayouts.placement(values, profile, spec)
            val selected = spec.id == selectedId
            if (!p.visible && !selected) continue
            val g = TouchLayouts.geometry(spec, p, size.width, size.height, keepAspect, TouchLayouts.added(profile, spec))
            val center = Offset(g.x, g.y)
            val alpha = if (!p.visible) .18f else if (spec.joystick || spec.asset == null && images["custom/" + spec.id] == null) 1f else opacity
            fun sprite(name: String, half: Float) {
                images[name]?.let { bitmap ->
                    drawImage(bitmap, dstOffset = IntOffset((g.x - half).roundToInt(), (g.y - half).roundToInt()),
                        dstSize = IntSize((half * 2).roundToInt().coerceAtLeast(1), (half * 2).roundToInt().coerceAtLeast(1)),
                        alpha = alpha, filterQuality = FilterQuality.None)
                }
            }
            if (spec.joystick) {
                if (appearance.style == TouchStyle.MODERN && images["joystick-base.png"] != null) {
                    sprite("joystick-base.png", g.radius * 1.05f)
                    sprite("joystick-knob.png", g.radius * .88f)
                } else drawModernMovement(g, alpha, paint, tint = appearance.tint)
            } else {
                val effectiveSpec = if (p.customKey != null) spec.copy(label = p.customKey) else spec
                drawControl(effectiveSpec, g, appearance, images, alpha, paint)
            }
            if (selected) drawCircle(Color(0xFF63E0CA), g.radius + 3.dp.toPx(), center, style = Stroke(2.dp.toPx()))
            if (!p.visible) drawLine(Color(0xFF63E0CA), center - Offset(g.radius, g.radius), center + Offset(g.radius, g.radius), 2.dp.toPx())
        }
    }
}
