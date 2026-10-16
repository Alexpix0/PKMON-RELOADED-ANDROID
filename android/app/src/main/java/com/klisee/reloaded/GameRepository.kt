package com.klisee.reloaded

import android.content.Context
import android.net.Uri
import android.os.Environment
import android.provider.OpenableColumns
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File
import java.io.RandomAccessFile
import java.net.HttpURLConnection
import java.net.URL
import java.util.zip.ZipEntry
import java.util.zip.ZipFile
import java.util.zip.ZipInputStream
import java.util.zip.ZipOutputStream

/**
 * Owns the game in the app's private storage.
 *
 * The launcher inflates the game itself (so the emulator no longer depends on any archive name)
 * into `files/game/`, and records where it came from in `files/game.json`.
 */
class GameRepository(private val context: Context) {

    sealed interface Status {
        data object Missing : Status
        data class Ready(
            val exe: String,
            val version: String?,
            val source: String?,
            val installedAt: Long,
            val sizeBytes: Long,
        ) : Status
    }

    private val files: File get() = context.filesDir
    private val gameDir: File get() = File(files, "game")
    private val metaFile: File get() = File(files, "game.json")
    val logFile: File get() = File(files, "emulator.log")

    /** The emulator's executable name is not pinned, so it is discovered by extension. */
    fun status(): Status {
        autoHealIfNeeded()
        val exe = gameDir.listFiles()?.firstOrNull { it.isFile && it.extension.equals("exe", ignoreCase = true) }
            ?: return Status.Missing
        // A gutted `data` folder is not an install either: the game would start and fail.
        val data = File(gameDir, "data")
        if (!data.isDirectory || (data.listFiles()?.size ?: 0) < 5) return Status.Missing
        val meta = readMeta()
        return Status.Ready(
            exe = exe.name,
            // Nothing ever recorded a version on install, so it is read off the executable's own
            // name, which is where the author puts it: "... The Last Beta 1.9.2.2 Full.exe".
            version = meta?.optString("version")?.ifEmpty { null } ?: versionFromName(exe.name),
            source = meta?.optString("source")?.ifEmpty { null },
            installedAt = meta?.optLong("installedAt", 0L) ?: 0L,
            sizeBytes = meta?.optLong("sizeBytes", 0L) ?: 0L,
        )
    }

    /** The first version-looking number in a name, or null if it has none. */
    private fun versionFromName(name: String): String? =
        Regex("""\d+\.\d+(?:\.\d+){1,2}""").find(name)?.value

    /** Frees the extracted game, leaving the launcher ready for another import or download. */
    suspend fun deleteGame() = withContext(Dispatchers.IO) {
        gameDir.deleteRecursively()
        File(files, "game.staging").deleteRecursively()
        File(files, "import.zip").delete()
        metaFile.delete()
        Unit
    }

    /**
     * What a "save" actually is: the saved games plus the teams and custom characters the game's own
     * migration instructions tell you to carry over. Keeping the same folder names means a zip made
     * here also works by hand on the PC copy, and the other way round.
     */
    private val saveFolders = listOf("partidas", "equipos", "char_custom")

    /** One of the game's save slots, described the way its own "Cargar" screen describes it. */
    data class SaveSlot(
        val index: Int,
        val name: String?,
        val seen: String?,
        val caught: String?,
        val place: String?,
    )

    object Slots {
        /** How many slots the game's load screen offers. */
        const val COUNT = 3
    }

    /**
     * The slots the game has, read from the little summary file it writes beside each save.
     *
     * `reloaded<N>.nfo` is plain text with one field per line, in the order the game writes it: the
     * trainer's name, how many Pokemon have been seen and caught, where the save was made, the
     * character's sprite and the Pokemon beside them. Reading that file is what lets the app show
     * exactly what the game's own load screen shows, without guessing at anything.
     *
     * A slot counts as used when any `reloaded<N>.*` file exists for it: the save itself is the
     * `.savc`, while the `.data` and `.nfo` only describe it.
     */
    fun saveSlots(): List<SaveSlot> {
        val folder = File(gameDir, "data/partidas")
        val pattern = Regex("^reloaded(\\d+)\\.[a-z]+$", RegexOption.IGNORE_CASE)
        val used = folder.listFiles().orEmpty()
            .mapNotNull { pattern.find(it.name)?.groupValues?.get(1)?.toIntOrNull() }
            .toSet()

        return (1..maxOf(Slots.COUNT, used.maxOrNull() ?: 0)).map { index ->
            if (index !in used) {
                return@map SaveSlot(index, null, null, null, null)
            }
            val lines = runCatching { File(folder, "reloaded$index.nfo").readLines() }.getOrDefault(emptyList())
                // The game leaves a blank line after the trainer's name, and counting it as a field
                // shifted every value by one: the seen count came out empty and the place came out
                // as the number of captured.
                .filter { it.isNotBlank() }
            fun field(at: Int) = lines.getOrNull(at)?.trim()?.takeIf { it.isNotEmpty() }
            SaveSlot(index, field(0), field(1), field(2), field(3))
        }
    }

    /**
     * Restores one slot from a zip, into the slot that was asked for.
     *
     * The files inside an exported slot carry their own number - `reloaded1.savc` and the ones that
     * describe it - so importing them as they are would always put them back where they came from.
     * They are renamed on the way in instead, which is what turns "export from slot 1, import into
     * slot 3" into moving a save. The suffix is kept, so a backup stays a backup.
     */
    suspend fun importSlot(uri: Uri, index: Int): Result<Int> = withContext(Dispatchers.IO) {
        runCatching {
            val folder = File(gameDir, "data/partidas")
            if (!folder.isDirectory) error("Primero instala el juego.")
            val numbered = Regex("^reloaded\\d+(\\.|_)(.*)$", RegexOption.IGNORE_CASE)

            // Limpiar la partida anterior y sus copias de seguridad (_bk) antes de importar
            // para evitar que el juego restaure el estado viejo por fecha o inconsistencia.
            folder.listFiles().orEmpty()
                .filter { it.name.startsWith("reloaded$index.", ignoreCase = true) || it.name.startsWith("reloaded${index}_", ignoreCase = true) }
                .forEach { it.delete() }

            var written = 0
            context.contentResolver.openInputStream(uri)?.use { input ->
                ZipInputStream(input.buffered()).use { zip ->
                    var entry = zip.nextEntry
                    while (entry != null) {
                        val name = entry.name.replace('\\', '/')
                        val file = name.substringAfterLast('/')
                        val match = numbered.find(file)
                        // Aceptar si está en partidas/, data/partidas/ o en la raíz del zip
                        val inPartidas = name.startsWith("partidas/", ignoreCase = true) ||
                            name.startsWith("data/partidas/", ignoreCase = true) ||
                            !name.contains('/')
                        if (!entry.isDirectory && inPartidas && match != null) {
                            val separator = match.groupValues[1]
                            val rest = match.groupValues[2]
                            val target = File(folder, "reloaded$index$separator$rest")
                            target.outputStream().use { zip.copyTo(it) }
                            written++
                        }
                        zip.closeEntry()
                        entry = zip.nextEntry
                    }
                }
            }
            if (written == 0) error("Ese zip no trae ninguna partida.")
            written
        }
    }

    /**
     * Deletes a slot: the save itself, the files that describe it, and the backup the game keeps
     * beside it. The backup goes too, because leaving it behind would let the game offer to restore
     * the very save that was meant to be gone.
     */
    suspend fun deleteSlot(index: Int): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val folder = File(gameDir, "data/partidas")
            val files = folder.listFiles().orEmpty()
                .filter { it.name.startsWith("reloaded$index.") || it.name.startsWith("reloaded${index}_") }
            if (files.isEmpty()) error("Esa partida ya no esta.")
            files.forEach { it.delete() }
        }
    }

    /** Zips a single slot, so one save can be carried on its own. */
    suspend fun exportSlot(index: Int): Result<File> = withContext(Dispatchers.IO) {
        runCatching {
            val folder = File(gameDir, "data/partidas")
            val files = folder.listFiles().orEmpty().filter { it.name.startsWith("reloaded$index.") }
            if (files.isEmpty()) error("Esa partida esta vacia.")
            val out = File(context.cacheDir, "partida$index.zip")
            ZipOutputStream(out.outputStream().buffered()).use { zip ->
                files.forEach { file ->
                    zip.putNextEntry(ZipEntry("partidas/" + file.name))
                    file.inputStream().use { it.copyTo(zip) }
                    zip.closeEntry()
                }
            }
            out
        }
    }

    /**
     * Automatic backup of saves into `files/backups_auto/` before starting the game.
     * Keeps up to 5 historical rollbacks so saves can never be lost if the device powers off.
     */
    fun autoBackupSaves(): Boolean {
        return runCatching {
            val data = File(gameDir, "data")
            if (!data.isDirectory) return false
            val backupRoot = File(files, "backups_auto").apply { mkdirs() }
            val time = java.text.SimpleDateFormat("yyyyMMdd_HHmmss", java.util.Locale.US).format(java.util.Date())
            val targetZip = File(backupRoot, "reloaded_backup_$time.zip")
            val existing = backupRoot.listFiles()?.filter { it.extension == "zip" }?.sortedBy { it.lastModified() }
            if (existing != null && existing.size >= 5) {
                existing.take(existing.size - 4).forEach { it.delete() }
            }
            var written = 0
            ZipOutputStream(targetZip.outputStream().buffered()).use { zip ->
                saveFolders.forEach { folder ->
                    val root = File(data, folder)
                    if (!root.isDirectory) return@forEach
                    root.walkTopDown().filter { it.isFile }.forEach { file ->
                        zip.putNextEntry(ZipEntry("$folder/" + file.relativeTo(root).invariantSeparatorsPath))
                        file.inputStream().use { it.copyTo(zip) }
                        zip.closeEntry()
                        written++
                    }
                }
            }
            if (written == 0) {
                targetZip.delete()
                false
            } else {
                // Scoped Storage Shield: Replicate backup to public Documents/PokemonReloadedSaves/
                runCatching {
                    val shieldDir = File(Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOCUMENTS), "PokemonReloadedSaves").apply { mkdirs() }
                    val shieldZip = File(shieldDir, "reloaded_backup_$time.zip")
                    targetZip.copyTo(shieldZip, overwrite = true)
                    val existingShield = shieldDir.listFiles()?.filter { it.extension.equals("zip", ignoreCase = true) }?.sortedBy { it.lastModified() }
                    if (existingShield != null && existingShield.size > 5) {
                        existingShield.take(existingShield.size - 5).forEach { it.delete() }
                    }
                }
                true
            }
        }.getOrDefault(false)
    }

    /** Returns list of auto-backup zip files, ordered newest to oldest, merging internal and Scoped Storage Shield backups. */
    fun autoBackupsList(): List<File> {
        val backupRoot = File(files, "backups_auto")
        val internalList = if (backupRoot.isDirectory) {
            backupRoot.listFiles()?.filter { it.extension.equals("zip", ignoreCase = true) }?.toList().orEmpty()
        } else emptyList()

        val shieldDir = File(Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOCUMENTS), "PokemonReloadedSaves")
        val shieldList = if (shieldDir.isDirectory) {
            shieldDir.listFiles()?.filter { it.extension.equals("zip", ignoreCase = true) }?.toList().orEmpty()
        } else emptyList()

        return (internalList + shieldList)
            .distinctBy { it.name }
            .sortedByDescending { it.lastModified() }
    }

    /** Restores an automatic backup zip file directly. */
    suspend fun restoreAutoBackup(zipFile: File): Result<Int> = withContext(Dispatchers.IO) {
        runCatching {
            val data = File(gameDir, "data")
            if (!data.isDirectory) error("Primero instala el juego.")
            if (!zipFile.isFile) error("No se encontró el archivo de copia.")
            var filesCount = 0
            var saves = 0
            val partidasFolder = File(data, "partidas").apply { mkdirs() }
            val cleanedSlots = mutableSetOf<String>()

            ZipInputStream(zipFile.inputStream().buffered()).use { zip ->
                var entry = zip.nextEntry
                while (entry != null) {
                    var name = entry.name.replace('\\', '/')
                    if (name.startsWith("data/", ignoreCase = true)) {
                        name = name.substring(5)
                    }
                    val isDirectSave = !name.contains('/') && (
                        name.startsWith("reloaded", ignoreCase = true) ||
                        name.endsWith(".savc", ignoreCase = true) ||
                        name.endsWith(".data", ignoreCase = true) ||
                        name.endsWith(".nfo", ignoreCase = true)
                    )
                    if (isDirectSave) {
                        name = "partidas/$name"
                    }
                    val slotMatch = Regex("^partidas/reloaded(\\d+)[._]", RegexOption.IGNORE_CASE).find(name)
                    if (slotMatch != null) {
                        val slotNum = slotMatch.groupValues[1]
                        if (cleanedSlots.add(slotNum)) {
                            partidasFolder.listFiles().orEmpty()
                                .filter { it.name.startsWith("reloaded$slotNum.", ignoreCase = true) || it.name.startsWith("reloaded${slotNum}_", ignoreCase = true) }
                                .forEach { it.delete() }
                        }
                    }
                    val target = File(data, name)
                    val wanted = saveFolders.any { name.startsWith("$it/", ignoreCase = true) }
                    val safe = target.canonicalPath.startsWith(data.canonicalPath + File.separator)
                    if (!entry.isDirectory && wanted && safe) {
                        target.parentFile?.mkdirs()
                        target.outputStream().use { zip.copyTo(it) }
                        filesCount++
                        val isSlot = name.startsWith("partidas/", ignoreCase = true) &&
                            name.endsWith(".savc", ignoreCase = true) &&
                            !name.contains("_bk", ignoreCase = true)
                        if (isSlot) saves++
                    }
                    zip.closeEntry()
                    entry = zip.nextEntry
                }
            }
            if (filesCount == 0) error("La copia de seguridad no contenía partidas válidas.")
            saves
        }
    }

    /**
     * Clones an existing save slot into a target slot (e.g. slot 1 to slot 2).
     */
    suspend fun cloneSlot(fromIndex: Int, toIndex: Int): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            if (fromIndex == toIndex) return@runCatching
            val folder = File(gameDir, "data/partidas")
            if (!folder.isDirectory) error("Primero instala el juego.")
            val prefix = "reloaded$fromIndex"
            val sourceFiles = folder.listFiles().orEmpty()
                .filter { it.name.startsWith("$prefix.", ignoreCase = true) || it.name.startsWith("${prefix}_", ignoreCase = true) }
            if (sourceFiles.isEmpty()) error("La ranura $fromIndex está vacía.")

            // Delete target slot files first
            val targetPrefix = "reloaded$toIndex"
            folder.listFiles().orEmpty()
                .filter { it.name.startsWith("$targetPrefix.", ignoreCase = true) || it.name.startsWith("${targetPrefix}_", ignoreCase = true) }
                .forEach { it.delete() }

            // Copy each file with new slot name
            sourceFiles.forEach { src ->
                val rest = src.name.substring(prefix.length)
                val targetFile = File(folder, "$targetPrefix$rest")
                src.copyTo(targetFile, overwrite = true)
            }
        }
    }

    data class SnapshotItem(
        val file: File,
        val slot: Int,
        val label: String,
        val timestamp: Long,
        val sizeBytes: Long,
        val dateFormatted: String,
    )

    enum class HealthLevel { EXCELLENT, GOOD, WARNING, CRITICAL }

    data class HealthReport(
        val level: HealthLevel,
        val totalFiles: Int,
        val totalSizeBytes: Long,
        val zeroByteFiles: Int,
        val battleSpritesCount: Int,
        val midisCount: Int,
        val missingCoreFiles: List<String>,
        val summary: String,
    )

    /**
     * Creates an instant safety snapshot ("Punto de Retorno") of a save slot before a gym/boss battle.
     * Persists in internal storage and replicates to public Documents/PokemonReloadedSnapshots.
     */
    suspend fun createSnapshot(slot: Int, label: String = "Punto de Retorno"): Result<File> = withContext(Dispatchers.IO) {
        runCatching {
            val folder = File(gameDir, "data/partidas")
            if (!folder.isDirectory) error("Primero instala el juego.")
            val prefix = "reloaded$slot"
            val targetFiles = folder.listFiles().orEmpty()
                .filter { it.name.startsWith("$prefix.", ignoreCase = true) || it.name.startsWith("${prefix}_", ignoreCase = true) }
            if (targetFiles.isEmpty()) error("La ranura $slot no tiene ninguna partida guardada.")

            val snapshotsDir = File(files, "snapshots").apply { mkdirs() }
            val time = java.text.SimpleDateFormat("yyyyMMdd_HHmmss", java.util.Locale.US).format(java.util.Date())
            val timestamp = System.currentTimeMillis()
            val snapFile = File(snapshotsDir, "snapshot_slot${slot}_$time.zip")

            ZipOutputStream(snapFile.outputStream().buffered()).use { zip ->
                targetFiles.forEach { file ->
                    zip.putNextEntry(ZipEntry("partidas/" + file.name))
                    file.inputStream().use { it.copyTo(zip) }
                    zip.closeEntry()
                }
                // Internal metadata file
                zip.putNextEntry(ZipEntry("snapshot_meta.json"))
                val metaJson = JSONObject()
                    .put("slot", slot)
                    .put("label", label)
                    .put("timestamp", timestamp)
                    .put("version", "${com.klisee.reloaded.BuildConfig.VERSION_NAME}-EX")
                    .toString()
                zip.write(metaJson.toByteArray())
                zip.closeEntry()
            }

            // Scoped Storage Shield: Replicate snapshot into Documents/PokemonReloadedSnapshots
            runCatching {
                val shieldDir = File(Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOCUMENTS), "PokemonReloadedSnapshots").apply { mkdirs() }
                snapFile.copyTo(File(shieldDir, snapFile.name), overwrite = true)
            }

            snapFile
        }
    }

    /** Lists all saved snapshots, newest first. */
    fun listSnapshots(): List<SnapshotItem> {
        val internalDir = File(files, "snapshots")
        val internalFiles = if (internalDir.isDirectory) internalDir.listFiles().orEmpty().filter { it.extension.equals("zip", ignoreCase = true) } else emptyList()
        val shieldDir = File(Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOCUMENTS), "PokemonReloadedSnapshots")
        val shieldFiles = if (shieldDir.isDirectory) shieldDir.listFiles().orEmpty().filter { it.extension.equals("zip", ignoreCase = true) } else emptyList()

        val all = (internalFiles + shieldFiles).distinctBy { it.name }
        val formatter = java.text.SimpleDateFormat("dd/MM/yyyy HH:mm:ss", java.util.Locale.getDefault())

        return all.map { file ->
            var slot = 1
            var label = "Punto de Retorno"
            var timestamp = file.lastModified()

            // Try reading embedded metadata
            runCatching {
                ZipFile(file).use { zip ->
                    val entry = zip.getEntry("snapshot_meta.json")
                    if (entry != null) {
                        zip.getInputStream(entry).reader().use {
                            val json = JSONObject(it.readText())
                            slot = json.optInt("slot", 1)
                            label = json.optString("label", label)
                            timestamp = json.optLong("timestamp", timestamp)
                        }
                    } else {
                        val match = Regex("slot(\\d+)", RegexOption.IGNORE_CASE).find(file.name)
                        if (match != null) {
                            slot = match.groupValues[1].toIntOrNull() ?: 1
                        }
                    }
                }
            }

            SnapshotItem(
                file = file,
                slot = slot,
                label = label,
                timestamp = timestamp,
                sizeBytes = file.length(),
                dateFormatted = formatter.format(java.util.Date(timestamp)),
            )
        }.sortedByDescending { it.timestamp }
    }

    /** Restores a snapshot file back into its original save slot. */
    suspend fun restoreSnapshot(snapshotFile: File): Result<Int> = withContext(Dispatchers.IO) {
        runCatching {
            val folder = File(gameDir, "data/partidas")
            if (!folder.isDirectory) error("Primero instala el juego.")
            var slot = 1
            // Determine slot
            runCatching {
                ZipFile(snapshotFile).use { zip ->
                    val entry = zip.getEntry("snapshot_meta.json")
                    if (entry != null) {
                        zip.getInputStream(entry).reader().use {
                            val json = JSONObject(it.readText())
                            slot = json.optInt("slot", 1)
                        }
                    } else {
                        val match = Regex("slot(\\d+)", RegexOption.IGNORE_CASE).find(snapshotFile.name)
                        if (match != null) slot = match.groupValues[1].toIntOrNull() ?: 1
                    }
                }
            }

            // Clean previous save in that slot
            folder.listFiles().orEmpty()
                .filter { it.name.startsWith("reloaded$slot.", ignoreCase = true) || it.name.startsWith("reloaded${slot}_", ignoreCase = true) }
                .forEach { it.delete() }

            var written = 0
            ZipInputStream(snapshotFile.inputStream().buffered()).use { zip ->
                var entry = zip.nextEntry
                while (entry != null) {
                    val name = entry.name.replace('\\', '/')
                    if (!entry.isDirectory && name.startsWith("partidas/", ignoreCase = true)) {
                        val fname = name.substringAfterLast('/')
                        val target = File(folder, fname)
                        target.outputStream().use { zip.copyTo(it) }
                        written++
                    }
                    zip.closeEntry()
                    entry = zip.nextEntry
                }
            }
            if (written == 0) error("El snapshot no contenía archivos de partida válidos.")
            slot
        }
    }

    /** Deletes a snapshot file and its mirrored copy in Documents. */
    fun deleteSnapshot(snapshotFile: File): Boolean {
        var ok = snapshotFile.delete()
        val shieldDir = File(Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOCUMENTS), "PokemonReloadedSnapshots")
        val mirrored = File(shieldDir, snapshotFile.name)
        if (mirrored.exists()) {
            ok = mirrored.delete() || ok
        }
        return ok
    }

    /**
     * Deep Health Checker of game installation integrity across all 34,388 assets.
     */
    suspend fun runHealthCheck(): HealthReport = withContext(Dispatchers.IO) {
        var totalFiles = 0
        var totalSize = 0L
        var zeroByteCount = 0
        val missing = mutableListOf<String>()

        if (!gameDir.isDirectory) {
            return@withContext HealthReport(
                level = HealthLevel.CRITICAL,
                totalFiles = 0,
                totalSizeBytes = 0L,
                zeroByteFiles = 0,
                battleSpritesCount = 0,
                midisCount = 0,
                missingCoreFiles = listOf("Directorio principal game/ no existe"),
                summary = "El juego no está instalado. Instálalo desde el archivo ZIP para comenzar.",
            )
        }

        val exe = gameDir.listFiles().orEmpty().firstOrNull { it.isFile && it.extension.equals("exe", ignoreCase = true) }
        if (exe == null) {
            missing.add("Ejecutable principal (.exe)")
        }

        val dataDir = File(gameDir, "data")
        if (!dataDir.isDirectory) {
            missing.add("Carpeta de recursos (data/)")
        }

        val pkmnbattle = File(dataDir, "pkmnbattle")
        val battleCount = if (pkmnbattle.isDirectory) {
            pkmnbattle.listFiles().orEmpty().count { it.isFile && it.extension.equals("gmspr", ignoreCase = true) }
        } else {
            missing.add("Sprites de combate (data/pkmnbattle/)")
            0
        }

        val midisDir = File(dataDir, "midis")
        val midisCount = if (midisDir.isDirectory) {
            midisDir.listFiles().orEmpty().count { it.isFile }
        } else {
            missing.add("Música y efectos (data/midis/)")
            0
        }

        val efectividades = File(dataDir, "image/Efectividades.png")
        if (!efectividades.isFile) {
            missing.add("Tabla de tipos (data/image/Efectividades.png)")
        }

        gameDir.walkTopDown().filter { it.isFile }.forEach { f ->
            totalFiles++
            val len = f.length()
            totalSize += len
            if (len == 0L) {
                zeroByteCount++
            }
        }

        val level = when {
            missing.isNotEmpty() || battleCount < 1000 -> HealthLevel.CRITICAL
            zeroByteCount > 10 || midisCount < 100 -> HealthLevel.WARNING
            zeroByteCount > 0 -> HealthLevel.GOOD
            else -> HealthLevel.EXCELLENT
        }

        val summary = when (level) {
            HealthLevel.EXCELLENT -> "¡Instalación perfecta! Los $totalFiles archivos están íntegros, incluidos los $battleCount sprites de combate y $midisCount temas musicales."
            HealthLevel.GOOD -> "Instalación en buen estado ($totalFiles archivos). Se detectaron $zeroByteCount archivos residuales de 0 bytes no esenciales."
            HealthLevel.WARNING -> "Instalación funcional con advertencias: se encontraron $zeroByteCount archivos vacíos (0 bytes) que podrían provocar fallos en audio o texturas."
            HealthLevel.CRITICAL -> "Instalación crítica: Faltan componentes esenciales: ${missing.joinToString(", ")}. Se recomienda reinstalar el juego."
        }

        HealthReport(
            level = level,
            totalFiles = totalFiles,
            totalSizeBytes = totalSize,
            zeroByteFiles = zeroByteCount,
            battleSpritesCount = battleCount,
            midisCount = midisCount,
            missingCoreFiles = missing,
            summary = summary,
        )
    }

    /**
     * Applies a lightweight patch or mod (.zip) directly over the game or custom_assets folder
     * without re-downloading or reinstalling the full 700MB base package.
     */
    suspend fun applyPatchOrMod(uri: Uri): Result<Int> = withContext(Dispatchers.IO) {
        runCatching {
            if (!gameDir.isDirectory) error("Primero instala el juego base antes de aplicar mods o parches.")
            val tempZip = File(context.cacheDir, "incoming_patch.zip")
            context.contentResolver.openInputStream(uri)?.use { input ->
                tempZip.outputStream().use { input.copyTo(it) }
            } ?: error("No se pudo leer el archivo seleccionado.")

            val customAssetsDir = File(gameDir, "custom_assets").apply { mkdirs() }
            var updatedCount = 0

            ZipFile(tempZip).use { zip ->
                val entries = zip.entries()
                while (entries.hasMoreElements()) {
                    val entry = entries.nextElement()
                    val name = entry.name
                    if (entry.isDirectory || name.startsWith("__MACOSX") || name.startsWith(".")) continue

                    // Determine destination:
                    // If the zip explicitly contains "custom_assets/", extract relative to gameDir.
                    // If it contains "data/", extract directly into gameDir.
                    // If it's loose mod assets (e.g. "pkmnbattle/", "músic/", "image/"), extract into custom_assets/
                    val destFile = if (name.startsWith("custom_assets/", ignoreCase = true) || name.startsWith("data/", ignoreCase = true)) {
                        File(gameDir, name)
                    } else if (name.endsWith(".exe", ignoreCase = true) || name.endsWith(".dll", ignoreCase = true)) {
                        File(gameDir, name)
                    } else {
                        File(customAssetsDir, name)
                    }

                    destFile.parentFile?.mkdirs()
                    zip.getInputStream(entry).use { inStream ->
                        destFile.outputStream().use { inStream.copyTo(it) }
                    }
                    updatedCount++
                }
            }
            tempZip.delete()
            trace("patch: aplicado con éxito ($updatedCount archivos actualizados)")
            updatedCount
        }
    }

    /**
     * Reads the crash dump if the game engine panicked during the last session.
     */
    fun getCrashDump(): String? {
        val dumpFile = File(files, "crash_dump.txt")
        if (dumpFile.isFile && dumpFile.length() > 0) {
            return runCatching { dumpFile.readText() }.getOrNull()
        }
        val altDump = File(gameDir, "crash_dump.txt")
        if (altDump.isFile && altDump.length() > 0) {
            return runCatching { altDump.readText() }.getOrNull()
        }
        return null
    }

    /** Clears the crash dump file. */
    fun clearCrashDump(): Boolean {
        var ok = File(files, "crash_dump.txt").delete()
        ok = File(gameDir, "crash_dump.txt").delete() || ok
        return ok
    }

    /** Zips the saves into the cache, ready to be handed to a file picker. */
    suspend fun exportSaves(): Result<File> = withContext(Dispatchers.IO) {
        runCatching {
            val data = File(gameDir, "data")
            val out = File(context.cacheDir, "partidas.zip")
            var written = 0
            ZipOutputStream(out.outputStream().buffered()).use { zip ->
                saveFolders.forEach { folder ->
                    val root = File(data, folder)
                    if (!root.isDirectory) return@forEach
                    root.walkTopDown().filter { it.isFile }.forEach { file ->
                        zip.putNextEntry(ZipEntry("$folder/" + file.relativeTo(root).invariantSeparatorsPath))
                        file.inputStream().use { it.copyTo(zip) }
                        zip.closeEntry()
                        written++
                    }
                }
            }
            if (written == 0) {
                out.delete()
                error("No hay partidas que exportar todavia.")
            }
            out
        }
    }

    /** Restores an exported zip into the game's data folder, replacing what is there. */
    suspend fun importSaves(uri: Uri): Result<Int> = withContext(Dispatchers.IO) {
        runCatching {
            val data = File(gameDir, "data")
            if (!data.isDirectory) error("Primero instala el juego.")
            var files = 0
            var saves = 0
            val partidasFolder = File(data, "partidas").apply { mkdirs() }
            val cleanedSlots = mutableSetOf<String>()

            context.contentResolver.openInputStream(uri)?.use { input ->
                ZipInputStream(input.buffered()).use { zip ->
                    var entry = zip.nextEntry
                    while (entry != null) {
                        var name = entry.name.replace('\\', '/')
                        if (name.startsWith("data/", ignoreCase = true)) {
                            name = name.substring(5)
                        }

                        // Si los archivos están en la raíz del zip (ej: el usuario comprimió seleccionando
                        // los archivos de la carpeta directamente: reloaded1.savc, reloaded1.nfo...)
                        val isDirectSave = !name.contains('/') && (
                            name.startsWith("reloaded", ignoreCase = true) ||
                            name.endsWith(".savc", ignoreCase = true) ||
                            name.endsWith(".data", ignoreCase = true) ||
                            name.endsWith(".nfo", ignoreCase = true)
                        )
                        if (isDirectSave) {
                            name = "partidas/$name"
                        }

                        // Limpiar copias de seguridad viejas (_bk) del slot antes de escribir
                        val slotMatch = Regex("^partidas/reloaded(\\d+)[._]", RegexOption.IGNORE_CASE).find(name)
                        if (slotMatch != null) {
                            val slotNum = slotMatch.groupValues[1]
                            if (cleanedSlots.add(slotNum)) {
                                partidasFolder.listFiles().orEmpty()
                                    .filter { it.name.startsWith("reloaded$slotNum.", ignoreCase = true) || it.name.startsWith("reloaded${slotNum}_", ignoreCase = true) }
                                    .forEach { it.delete() }
                            }
                        }

                        val target = File(data, name)
                        // Only the save folders, and never a path that escapes them.
                        val wanted = saveFolders.any { name.startsWith("$it/", ignoreCase = true) }
                        val safe = target.canonicalPath.startsWith(data.canonicalPath + File.separator)
                        if (!entry.isDirectory && wanted && safe) {
                            target.parentFile?.mkdirs()
                            target.outputStream().use { zip.copyTo(it) }
                            files++
                            // Backups (`reloaded1_bk.savc`) sit beside the real slot, so they are not
                            // counted as separate saves.
                            val isSlot = name.startsWith("partidas/", ignoreCase = true) &&
                                name.endsWith(".savc", ignoreCase = true) &&
                                !name.contains("_bk", ignoreCase = true)
                            if (isSlot) saves++
                        }
                        zip.closeEntry()
                        entry = zip.nextEntry
                    }
                }
            } ?: error("No pude abrir el archivo.")
            if (files == 0) error("Ese zip no trae partidas del juego.")
            saves
        }
    }

    private fun readMeta(): JSONObject? = runCatching { JSONObject(metaFile.readText()) }.getOrNull()

    /**
     * Copies the bundled MIDI soundfont next to the game, which is where the emulator looks for it.
     * It ships inside the APK so music works on any device without depending on external paths.
     */
    suspend fun prepare(): Unit = withContext(Dispatchers.IO) {
        val exe = gameDir.listFiles()?.firstOrNull { it.isFile && it.extension.equals("exe", ignoreCase = true) }
        if (exe != null && File(gameDir, "data").isDirectory) {
            // Installs made before the launcher recorded these still need their size and date.
            val meta = readMeta() ?: JSONObject()
            if (meta.optLong("sizeBytes", 0L) == 0L || meta.optLong("installedAt", 0L) == 0L) {
                meta.put("sizeBytes", gameDir.walkTopDown().filter { it.isFile }.sumOf { it.length() })
                if (meta.optLong("installedAt", 0L) == 0L) meta.put("installedAt", System.currentTimeMillis())
                metaFile.writeText(meta.toString())
            }
        }
        // The touch controls' pixel-art artwork ships inside the APK, but the engine can only read
        // files from its own folder, so every sprite is copied there (a few hundred bytes each,
        // refreshed on every start so an app update takes effect). Any `<control>.png` dropped in
        // the assets is picked up.
        val sprites = runCatching { context.assets.list("")?.filter { it.endsWith(".png", ignoreCase = true) } }
            .getOrNull()
            .orEmpty()
        for (name in sprites) {
            runCatching {
                context.assets.open(name).use { input ->
                    File(files, name).outputStream().use { output -> input.copyTo(output) }
                }
            }
        }
        if (!gameDir.isDirectory) return@withContext
        val target = File(gameDir, "soundfont.sf2")
        if (target.isFile && target.length() > 0) return@withContext
        runCatching {
            context.assets.open("soundfont.sf2").use { input ->
                target.outputStream().use { output -> input.copyTo(output) }
            }
        }
        Unit
    }

    /** Imports a zip picked with the system file chooser using resilient dual-engine processing. */
    suspend fun import(uri: Uri, onProgress: (Int) -> Unit): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            trace("import: inicio de importacion desde uri=$uri")
            val temp = File(files, "import.zip")
            var imported = false

            // Metodo 1 (Principal): Copia segura a almacenamiento local + extraccion nativa ultrarrapida con ZipFile
            try {
                trace("import: copiando zip a almacenamiento seguro temporal...")
                copyFromUri(uri, temp, onProgress = { onProgress((it / 2).coerceIn(0, 49)) })
                trace("import: extrayendo archivo zip con motor ZipFile...")
                installFromZipFile(temp, version = null, source = "manual", onProgress = { onProgress((50 + it / 2).coerceIn(50, 99)) }).getOrThrow()
                prepare()
                imported = true
            } catch (e: Throwable) {
                trace("import: copia segura o extraccion local fallo (${e.message}); probando modo streaming directo...")
            } finally {
                if (temp.exists()) temp.delete()
            }

            // Metodo 2 (Fallback): Streaming directo por si el dispositivo tiene poco espacio temporal libre
            if (!imported) {
                val total = contentResolverSize(uri).coerceAtLeast(1L)
                val stream = context.contentResolver.openInputStream(uri)
                    ?: error("No pude abrir el archivo seleccionado.")
                try {
                    installFromStream(stream, total, version = null, source = "manual", onProgress = onProgress).getOrThrow()
                    prepare()
                    imported = true
                } finally {
                    runCatching { stream.close() }
                }
            }

            onProgress(100)
            trace("import: importacion completada exitosamente")
        }
    }

    /**
     * Copies the chosen zip into `dest`, tolerating flaky streams and metadata mismatches.
     */
    private fun copyFromUri(uri: Uri, dest: File, onProgress: (Int) -> Unit) {
        val expected = contentResolverSize(uri)
        var last: Throwable? = null
        for (attempt in 1..3) {
            try {
                if (dest.exists()) dest.delete()
                context.contentResolver.openInputStream(uri)?.use { input ->
                    dest.outputStream().buffered(512 * 1024).use { output ->
                        val buffer = ByteArray(512 * 1024)
                        var copied = 0L
                        while (true) {
                            val n = input.read(buffer)
                            if (n < 0) break
                            output.write(buffer, 0, n)
                            copied += n
                            if (expected > 0) {
                                onProgress(((copied * 100) / expected).toInt().coerceIn(0, 99))
                            }
                        }
                    }
                } ?: error("No pude abrir el archivo seleccionado.")

                val size = dest.length()
                if (size < 1024 * 1024) {
                    error("El archivo copiado es demasiado pequeño ($size bytes).")
                }

                // Verify ZIP header magic 'PK' (0x50, 0x4B)
                RandomAccessFile(dest, "r").use { raf ->
                    if (raf.length() < 4) error("Archivo vacio o incompleto.")
                    val b0 = raf.read()
                    val b1 = raf.read()
                    if (b0 != 0x50 || b1 != 0x4B) {
                        error("El archivo no es un archivo ZIP valido.")
                    }
                }
                return
            } catch (e: Throwable) {
                last = e
                trace("import: intento $attempt de copiar fallo (${e.message}); reintentando...")
            }
        }
        throw last ?: error("No pude copiar el archivo zip.")
    }

    /** The size the content provider advertises for a document, or -1 if it does not say. */
    private fun contentResolverSize(uri: Uri): Long = runCatching {
        context.contentResolver.query(uri, arrayOf(OpenableColumns.SIZE), null, null, null)?.use { cursor ->
            if (cursor.moveToFirst() && !cursor.isNull(0)) cursor.getLong(0) else -1L
        } ?: -1L
    }.getOrDefault(-1L)

    /** Downloads a zip from a direct URL and unpacks into gameDir. */
    suspend fun download(url: String, cookies: String?, onProgress: (Int) -> Unit): Result<Unit> = withContext(Dispatchers.IO) {
        runCatching {
            val connection = (URL(url).openConnection() as HttpURLConnection).apply {
                instanceFollowRedirects = true
                connectTimeout = 30_000
                readTimeout = 30_000
                setRequestProperty("User-Agent", "Mozilla/5.0 (Android) ReloadedLauncher/1.0")
                if (!cookies.isNullOrEmpty()) setRequestProperty("Cookie", cookies)
            }
            connection.connect()
            if (connection.responseCode !in 200..299) error("El servidor respondio ${connection.responseCode}.")
            val total = connection.contentLengthLong.takeIf { it > 0 } ?: -1L
            val temp = File(files, "import.zip")
            try {
                if (temp.exists()) temp.delete()
                connection.inputStream.use { input ->
                    temp.outputStream().buffered(512 * 1024).use { output ->
                        val buffer = ByteArray(512 * 1024)
                        var downloaded = 0L
                        while (true) {
                            val n = input.read(buffer)
                            if (n < 0) break
                            output.write(buffer, 0, n)
                            downloaded += n
                            if (total > 0) onProgress(((downloaded * 100) / total).toInt().coerceIn(0, 49))
                        }
                    }
                }
                installFromZipFile(temp, version = null, source = "descarga", onProgress = { onProgress((50 + it / 2).coerceIn(50, 99)) }).getOrThrow()
                prepare()
                onProgress(100)
            } finally {
                if (temp.exists()) temp.delete()
            }
        }
    }

    /** Extracts and installs a game directly from a local zip file using ZipFile. */
    private fun installFromZipFile(
        zipFile: File,
        version: String?,
        source: String,
        onProgress: (Int) -> Unit
    ): Result<Unit> = runCatching {
        val staging = File(files, "game.staging")
        staging.deleteRecursively()
        staging.mkdirs()

        try {
            trace("import: extrayendo desde ${zipFile.name} (${zipFile.length() / (1024 * 1024)} MB)...")
            ZipFile(zipFile, java.nio.charset.StandardCharsets.ISO_8859_1).use { zip ->
                val totalEntries = zip.size()
                val buffer = ByteArray(256 * 1024)
                var count = 0
                val createdDirs = HashSet<File>()
                createdDirs.add(staging)
                fun ensureDir(dir: File) {
                    if (createdDirs.add(dir)) dir.mkdirs()
                }

                val entries = zip.entries()
                while (entries.hasMoreElements()) {
                    val entry = entries.nextElement()
                    val rawName = entry.name.replace('\\', '/')
                    val cleanName = rawName.replace('\u00A1', 'i').replace('\u00A2', 'o').replace('í', 'i').replace('ó', 'o')
                    val normalized = cleanName.trimStart('/', '\\')
                    if (normalized.isEmpty() || normalized == "." || normalized == "./") continue

                    val target = File(staging, normalized)
                    val stagingCanon = staging.canonicalPath
                    val targetCanon = target.canonicalPath
                    if (targetCanon != stagingCanon && !targetCanon.startsWith(stagingCanon + File.separator)) {
                        trace("import: omitiendo entrada fuera de staging: $rawName")
                        continue
                    }

                    if (entry.isDirectory || normalized.endsWith('/')) {
                        ensureDir(target)
                    } else {
                        target.parentFile?.let { ensureDir(it) }
                        zip.getInputStream(entry).use { input ->
                            target.outputStream().buffered(256 * 1024).use { output ->
                                var n = input.read(buffer)
                                while (n > 0) {
                                    output.write(buffer, 0, n)
                                    n = input.read(buffer)
                                }
                            }
                        }
                    }

                    count++
                    if (count % 2000 == 0 && totalEntries > 0) {
                        onProgress(((count * 100) / totalEntries).coerceIn(0, 99))
                    }
                }
            }

            if (!normalizeGameStructure(staging)) {
                error("El archivo zip no contiene los archivos de Pokemon Reloaded (.exe y carpeta data/).")
            }

            moveDirectoryContents(staging, gameDir)
            staging.deleteRecursively()

            val exe = gameDir.listFiles()?.firstOrNull { it.isFile && it.extension.equals("exe", ignoreCase = true) }
            metaFile.writeText(
                JSONObject()
                    .put("version", version ?: (exe?.let { versionFromName(it.name) } ?: ""))
                    .put("source", source)
                    .put("installedAt", System.currentTimeMillis())
                    .put("sizeBytes", gameDir.walkTopDown().filter { it.isFile }.sumOf { it.length() })
                    .toString(),
            )
            onProgress(100)
        } catch (e: Throwable) {
            staging.deleteRecursively()
            throw e
        }
    }

    /** Validates, inflates and swaps in a game zip from an input stream with high-throughput buffering. */
    private fun installFromStream(
        stream: java.io.InputStream,
        totalBytes: Long,
        version: String?,
        source: String,
        onProgress: (Int) -> Unit
    ): Result<Unit> = runCatching {
        val staging = File(files, "game.staging")
        staging.deleteRecursively()
        staging.mkdirs()

        val createdDirs = HashSet<File>()
        createdDirs.add(staging)
        fun ensureDir(dir: File) {
            if (createdDirs.add(dir)) {
                dir.mkdirs()
            }
        }

        try {
            trace("import: extrayendo en streaming directo...")
            ZipInputStream(java.io.BufferedInputStream(stream, 512 * 1024), java.nio.charset.StandardCharsets.ISO_8859_1).use { input ->
                val buffer = ByteArray(512 * 1024)
                var written = 0L
                var entries = 0
                var entry = input.nextEntry
                while (entry != null) {
                    val rawName = entry.name.replace('\\', '/')
                    val cleanName = rawName.replace('\u00A1', 'i').replace('\u00A2', 'o').replace('í', 'i').replace('ó', 'o')
                    val normalized = cleanName.trimStart('/', '\\')
                    if (normalized.isEmpty() || normalized == "." || normalized == "./") {
                        input.closeEntry()
                        entry = input.nextEntry
                        continue
                    }

                    val target = File(staging, normalized)
                    val stagingCanon = staging.canonicalPath
                    val targetCanon = target.canonicalPath
                    if (targetCanon != stagingCanon && !targetCanon.startsWith(stagingCanon + File.separator)) {
                        trace("import: omitiendo ruta fuera de staging: $rawName")
                        input.closeEntry()
                        entry = input.nextEntry
                        continue
                    }

                    if (entry.isDirectory || normalized.endsWith('/')) {
                        ensureDir(target)
                    } else {
                        target.parentFile?.let { ensureDir(it) }
                        target.outputStream().buffered(256 * 1024).use { output ->
                            var n = input.read(buffer)
                            while (n > 0) {
                                output.write(buffer, 0, n)
                                written += n
                                if (totalBytes > 0) {
                                    onProgress(((written * 100) / totalBytes).toInt().coerceIn(0, 99))
                                }
                                n = input.read(buffer)
                            }
                        }
                    }
                    input.closeEntry()
                    entries++
                    if (entries % 5000 == 0) {
                        trace("import: extraidas $entries entradas (${written / (1024 * 1024)} MB)")
                    }
                    entry = input.nextEntry
                }
                trace("import: extraccion en streaming terminada ($entries entradas, ${written / (1024 * 1024)} MB)")
            }

            if (!normalizeGameStructure(staging)) {
                error("El zip no trae el .exe y la carpeta data/.")
            }

            moveDirectoryContents(staging, gameDir)
            staging.deleteRecursively()

            val exe = gameDir.listFiles()?.firstOrNull { it.isFile && it.extension.equals("exe", ignoreCase = true) }
            metaFile.writeText(
                JSONObject()
                    .put("version", version ?: (exe?.let { versionFromName(it.name) } ?: ""))
                    .put("source", source)
                    .put("installedAt", System.currentTimeMillis())
                    .put("sizeBytes", gameDir.walkTopDown().filter { it.isFile }.sumOf { it.length() })
                    .toString(),
            )
            onProgress(100)
        } catch (e: Throwable) {
            staging.deleteRecursively()
            throw e
        }
    }

    /** Cheap structural check before doing the expensive extraction. */
    private fun requireGameZip(file: File) {
        ZipFile(file).use { zip ->
            var exe = false
            var data = false
            var seen = 0
            val first = StringBuilder()
            val entries = zip.entries()
            while (entries.hasMoreElements()) {
                val name = entries.nextElement().name
                if (seen < 12) {
                    first.append(name).append(" | ")
                    seen++
                }
                if (name.endsWith(".exe", ignoreCase = true)) exe = true
                if (name.startsWith("data/") || name.contains("/data/")) data = true
            }
            trace("import: exe=$exe data=$data; empieza por: $first")
            // Only a sanity check. The layout is settled once the zip is unpacked, because a zip
            // made by right-clicking the game folder keeps everything one level down.
            if (!exe || !data) error("Ese zip no parece Pokemon Reloaded: no encuentro el .exe ni la carpeta data/.")
        }
    }

    /**
     * Writes one line to logcat, which is what the log screen reads, and also to a small file.
     *
     * logcat only holds what the running process printed, so anything logged before the phone was
     * restarted is gone - which is exactly when somebody sits down to report a bug. The file keeps
     * the last stretch, and is rolled over so it cannot grow without end.
     */
    private fun trace(message: String) {
        android.util.Log.i("Reloaded", message)
        runCatching {
            val file = File(files, "app.log")
            if (file.length() > 256 * 1024) {
                File(files, "app.log.1").delete()
                file.renameTo(File(files, "app.log.1"))
            }
            file.appendText("$message\n")
        }
    }

    /**
     * Folds the entry names that are not valid UTF-8 down to plain ASCII, in place and without
     * changing a single length.
     *
     * The official game zip was made with a DOS code page, so two of its names carry an accent as a
     * single byte - `0xA1` for the "i" of "Especificas", `0xA2` for the "o" of "Informacion". The
     * zip does not set the UTF-8 flag, and Android's zip reader refuses the *whole* archive when it
     * meets a name that is not valid UTF-8, which is why importing the game failed for everybody.
     * Rewriting one byte for one byte leaves every offset and length in the file untouched, so the
     * archive stays perfectly valid.
     */
    private fun foldZipNamesToAscii(file: File) {
        runCatching {
            RandomAccessFile(file, "rw").use { raf ->
                val eocd = findEndOfCentralDirectory(raf) ?: return
                raf.seek(eocd + 10)
                val count = readLe16(raf)
                val cdSize = readLe32(raf)
                val cdOffset = readLe32(raf)
                if (cdSize <= 0L || cdSize > 64L * 1024 * 1024) return

                val cd = ByteArray(cdSize.toInt())
                raf.seek(cdOffset)
                raf.readFully(cd)

                var p = 0
                repeat(count) {
                    if (p + 46 > cd.size) return
                    if (cd[p] != 'P'.code.toByte() || cd[p + 1] != 'K'.code.toByte() ||
                        cd[p + 2] != 1.toByte() || cd[p + 3] != 2.toByte()
                    ) {
                        return
                    }
                    val nameLen = le16(cd, p + 28)
                    val extraLen = le16(cd, p + 30)
                    val commentLen = le16(cd, p + 32)
                    val localOffset = le32(cd, p + 42)

                    var changed = false
                    for (i in 0 until nameLen) {
                        val at = p + 46 + i
                        val byte = cd[at].toInt() and 0xff
                        if (byte > 0x7f) {
                            cd[at] = foldToAscii(byte)
                            changed = true
                        }
                    }

                    if (changed) {
                        trace("import: nombre arreglado: ${asciiName(cd, p + 46, nameLen)}")
                        raf.seek(cdOffset + p + 46)
                        raf.write(cd, p + 46, nameLen)

                        // The local header repeats the name, and readers may use either copy.
                        raf.seek(localOffset + 26)
                        val localNameLen = readLe16(raf)
                        raf.seek(localOffset + 30)
                        raf.write(cd, p + 46, minOf(nameLen, localNameLen))
                    }
                    p += 46 + nameLen + extraLen + commentLen
                }
            }
        }
    }

    private fun asciiName(bytes: ByteArray, at: Int, length: Int) =
        String(bytes, at, length, Charsets.ISO_8859_1)

    private fun findEndOfCentralDirectory(raf: RandomAccessFile): Long? {
        val window = minOf(raf.length(), 64L * 1024).toInt()
        if (window < 4) return null
        val tail = ByteArray(window)
        raf.seek(raf.length() - window)
        raf.readFully(tail)
        for (i in window - 4 downTo 0) {
            if (tail[i] == 'P'.code.toByte() && tail[i + 1] == 'K'.code.toByte() &&
                tail[i + 2] == 5.toByte() && tail[i + 3] == 6.toByte()
            ) {
                return raf.length() - window + i
            }
        }
        return null
    }

    private fun le16(bytes: ByteArray, at: Int) =
        (bytes[at].toInt() and 0xff) or ((bytes[at + 1].toInt() and 0xff) shl 8)

    private fun le32(bytes: ByteArray, at: Int) = le16(bytes, at).toLong() or ((le16(bytes, at + 2)).toLong() shl 16)

    private fun readLe16(raf: RandomAccessFile) = le16(ByteArray(2).also { raf.readFully(it) }, 0)

    private fun readLe32(raf: RandomAccessFile) = le32(ByteArray(4).also { raf.readFully(it) }, 0)

    /** The DOS code page byte for a Spanish accent, folded to the plain letter it stands for. */
    private fun foldToAscii(byte: Int): Byte = when (byte) {
        0x82, 0x90 -> 'e'.code.toByte()
        0xA0, 0xB5 -> 'a'.code.toByte()
        0xA1, 0xD6 -> 'i'.code.toByte()
        0xA2, 0xE0 -> 'o'.code.toByte()
        0xA3, 0xE9 -> 'u'.code.toByte()
        0xA4, 0xA5 -> 'n'.code.toByte()
        0x81, 0x9A -> 'u'.code.toByte()
        else -> '_'.code.toByte()
    }

    /**
     * Automatically attempts to recover existing installs where files may be trapped in
     * game.staging or nested inside subdirectories of gameDir.
     */
    private fun autoHealIfNeeded() {
        runCatching {
            val staging = File(files, "game.staging")
            val tempZip = File(files, "import.zip")

            // Case 1: Check if gameDir has nested folders or uppercase Data
            if (gameDir.isDirectory && (gameDir.listFiles()?.isNotEmpty() == true)) {
                normalizeGameStructure(gameDir)
            }

            val currentExe = gameDir.listFiles()?.firstOrNull { it.isFile && it.extension.equals("exe", ignoreCase = true) }
            val currentData = File(gameDir, "data")
            var isValid = currentExe != null && currentData.isDirectory && (currentData.listFiles()?.size ?: 0) >= 5

            // Case 2: gameDir is empty or incomplete, but game.staging has files
            if (!isValid && staging.isDirectory && (staging.listFiles()?.isNotEmpty() == true)) {
                if (normalizeGameStructure(staging)) {
                    trace("status: autoHeal rescatando juego valido desde game.staging a gameDir")
                    moveDirectoryContents(staging, gameDir)
                    staging.deleteRecursively()
                    isValid = true
                }
            }

            // Case 3: Once valid, clean up lingering temp files and ensure metadata exists
            if (isValid) {
                if (staging.exists()) staging.deleteRecursively()
                if (tempZip.exists()) tempZip.delete()

                val exe = gameDir.listFiles()?.firstOrNull { it.isFile && it.extension.equals("exe", ignoreCase = true) }
                if (!metaFile.exists() && exe != null) {
                    metaFile.writeText(
                        JSONObject()
                            .put("version", versionFromName(exe.name) ?: "")
                            .put("source", "recuperado")
                            .put("installedAt", System.currentTimeMillis())
                            .put("sizeBytes", gameDir.walkTopDown().filter { it.isFile }.sumOf { it.length() })
                            .toString()
                    )
                }
            }
        }
    }

    /**
     * Moves all files and directories from [src] to [dst], handling overwrites safely.
     */
    private fun moveDirectoryContents(src: File, dst: File) {
        if (!dst.exists()) dst.mkdirs()
        src.listFiles()?.forEach { child ->
            val target = File(dst, child.name)
            if (target.exists()) target.deleteRecursively()
            if (!child.renameTo(target)) {
                if (child.isDirectory) {
                    child.copyRecursively(target, overwrite = true)
                    child.deleteRecursively()
                } else {
                    child.copyTo(target, overwrite = true)
                    child.delete()
                }
            }
        }
    }

    /**
     * Normalizes the game folder structure so that the .exe and data/ folder
     * are located directly at the root of [root], handling nested folders,
     * accompanying files (Leeme.txt, etc.), and casing differences (Data -> data).
     */
    private fun normalizeGameStructure(root: File): Boolean {
        if (!root.isDirectory) return false

        // 1. Check if root already has .exe and data/
        var exeFile = root.listFiles()?.firstOrNull { it.isFile && it.extension.equals("exe", ignoreCase = true) }
        var dataFolder = root.listFiles()?.firstOrNull { it.isDirectory && it.name.equals("data", ignoreCase = true) }

        // 2. If not found at root, search subdirectories (up to depth 2)
        if (exeFile == null || dataFolder == null) {
            val candidateDirs = mutableListOf<File>()
            root.listFiles()?.filter { it.isDirectory && it.name != "__MACOSX" && !it.name.startsWith(".") }?.forEach { sub1 ->
                candidateDirs.add(sub1)
                sub1.listFiles()?.filter { it.isDirectory && it.name != "__MACOSX" && !it.name.startsWith(".") }?.forEach { sub2 ->
                    candidateDirs.add(sub2)
                }
            }

            for (cand in candidateDirs) {
                val candExe = cand.listFiles()?.firstOrNull { it.isFile && it.extension.equals("exe", ignoreCase = true) }
                val candData = cand.listFiles()?.firstOrNull { it.isDirectory && it.name.equals("data", ignoreCase = true) }
                if (candExe != null && candData != null) {
                    trace("import: encontrado juego anidado en ${cand.relativeTo(root)}; desenvolviendo a la raiz")
                    val children = cand.listFiles().orEmpty()
                    for (child in children) {
                        val dest = File(root, child.name)
                        if (dest.exists()) dest.deleteRecursively()
                        if (!child.renameTo(dest)) {
                            if (child.isDirectory) {
                                child.copyRecursively(dest, overwrite = true)
                                child.deleteRecursively()
                            } else {
                                child.copyTo(dest, overwrite = true)
                                child.delete()
                            }
                        }
                    }
                    cand.deleteRecursively()
                    var parent = cand.parentFile
                    while (parent != null && parent != root) {
                        if (parent.listFiles()?.isEmpty() == true) {
                            parent.delete()
                        }
                        parent = parent.parentFile
                    }
                    break
                }
            }
        }

        // 3. Normalize data folder name to lowercase "data"
        val foundData = root.listFiles()?.firstOrNull { it.isDirectory && it.name.equals("data", ignoreCase = true) }
        if (foundData != null && foundData.name != "data") {
            val targetData = File(root, "data")
            if (!targetData.exists()) {
                val tempRename = File(root, "data_norm_temp_${System.currentTimeMillis()}")
                if (foundData.renameTo(tempRename)) {
                    tempRename.renameTo(targetData)
                }
            }
        }

        // 4. Remove system metadata/junk files
        File(root, "__MACOSX").takeIf { it.exists() }?.deleteRecursively()
        File(root, ".DS_Store").takeIf { it.exists() }?.delete()
        File(root, "desktop.ini").takeIf { it.exists() }?.delete()
        File(root, "Thumbs.db").takeIf { it.exists() }?.delete()

        // 5. Final validation
        val finalExe = root.listFiles()?.firstOrNull { it.isFile && it.extension.equals("exe", ignoreCase = true) }
        val finalData = File(root, "data")
        return finalExe != null && finalData.isDirectory && (finalData.listFiles()?.size ?: 0) >= 5
    }
}
