package com.klisee.reloaded.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.SystemUpdate
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearWavyProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.klisee.reloaded.Update

/**
 * Offers the newer build, and leaves a way out for anyone who would rather not trust an in-app
 * updater: that last button opens the release page, so the APK can be fetched by hand.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun UpdateDialog(
    release: Update.Release,
    busy: Boolean,
    written: Long,
    total: Long,
    problem: String?,
    onDownload: () -> Unit,
    onDismiss: () -> Unit,
    onOpenReleases: () -> Unit,
) {
    val percent = if (total > 0) (written * 100 / total).toInt() else 0
    AlertDialog(
        onDismissRequest = onDismiss,
        icon = { Icon(Icons.Filled.SystemUpdate, contentDescription = null) },
        title = { Text(if (busy) "Descargando actualización..." else "¡Nueva versión disponible: v${release.version}!") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                if (release.name.isNotEmpty()) {
                    Text(
                        text = release.name,
                        style = MaterialTheme.typography.titleMedium,
                    )
                }
                if (release.body.isNotEmpty() && !busy) {
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .heightIn(max = 180.dp)
                            .verticalScroll(rememberScrollState())
                            .background(
                                MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                                RoundedCornerShape(8.dp)
                            )
                            .padding(8.dp)
                    ) {
                        Text(
                            text = release.body,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
                if (busy) {
                    LinearWavyProgressIndicator(
                        progress = { if (total > 0) written.toFloat() / total.toFloat() else 0f },
                        modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
                    )
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(top = 4.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text(
                            text = "${inMegabytes(written)} de ${if (total > 0) inMegabytes(total) else "?"} MB",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Text(
                            text = "$percent%",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
                if (problem == null && !busy) {
                    Text(
                        text = "Se descarga directamente desde GitHub Releases y se inicia el instalador nativo del sistema.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                if (problem != null) {
                    Text(
                        text = problem,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.padding(top = 6.dp),
                    )
                }
                TextButton(onClick = onOpenReleases, modifier = Modifier.fillMaxWidth()) {
                    Text(
                        text = "Ver release en GitHub",
                        style = MaterialTheme.typography.labelLarge,
                    )
                }
            }
        },
        confirmButton = {
            TextButton(onClick = onDownload, enabled = !busy) {
                Text(if (busy) "Descargando..." else "Descargar e Instalar APK ahora")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss, enabled = !busy) { Text(if (busy) "Ocultar" else "Cancelar") }
        },
    )
}

private fun inMegabytes(bytes: Long) = "%.1f".format(bytes / 1024.0 / 1024.0)
