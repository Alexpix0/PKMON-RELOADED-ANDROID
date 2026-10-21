package com.klisee.reloaded

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File
import java.net.HttpURLConnection
import java.net.URL

/**
 * Looks for a newer build on GitHub. The repository's releases are the only place a version is
 * published, so there is no server to keep running and nothing to keep in step.
 */
object Update {
    private const val REPO = "Alexpix0/PKMON-RELOADED-ANDROID"
    private const val LATEST = "https://api.github.com/repos/$REPO/releases/latest"
    const val RELEASES_PAGE = "https://github.com/$REPO/releases/latest"

    data class Release(
        val version: String,
        val apkUrl: String?,
        val pageUrl: String,
        val name: String = "",
        val body: String = "",
        val assetId: Long = 0L,
    )

    /** The newer release, or `null` when the installed build is already the latest one. */
    suspend fun check(installed: String, token: String = ""): Result<Release?> = withContext(Dispatchers.IO) {
        runCatching {
            val json = JSONObject(get(LATEST, token))
            val tag = json.optString("tag_name").trim().trimStart('v', 'V')
            if (tag.isEmpty() || !isNewer(tag, installed)) {
                return@runCatching null
            }
            var assetId = 0L
            var apkUrl: String? = null
            json.optJSONArray("assets")?.let { assets ->
                for (i in 0 until assets.length()) {
                    val obj = assets.getJSONObject(i)
                    if (obj.optString("name").endsWith(".apk", ignoreCase = true)) {
                        apkUrl = obj.optString("browser_download_url").takeIf { it.isNotEmpty() }
                        assetId = obj.optLong("id", 0L)
                        break
                    }
                }
            }
            val name = json.optString("name")
            val body = json.optString("body")
            Release(tag, apkUrl, json.optString("html_url").ifEmpty { RELEASES_PAGE }, name, body, assetId)
        }
    }

    /** Verifies connection using the given token and returns the latest release tag. */
    suspend fun testConnection(token: String): Result<String> = withContext(Dispatchers.IO) {
        runCatching {
            val json = JSONObject(get(LATEST, token))
            val tag = json.optString("tag_name")
            if (tag.isEmpty()) error("Respuesta sin tag_name")
            tag
        }
    }

    /**
     * Downloads the release's APK so the system installer can be handed a local file.
     * Handles GitHub API private asset download with AWS S3 redirection.
     */
    suspend fun download(release: Release, target: File, token: String = "", onProgress: (Long, Long) -> Unit): Result<File> = withContext(Dispatchers.IO) {
        runCatching {
            val trimmedToken = token.trim()
            val connection: HttpURLConnection
            if (trimmedToken.isNotEmpty() && release.assetId > 0L) {
                // Private repository: request release asset through GitHub API
                val assetApiUrl = "https://api.github.com/repos/$REPO/releases/assets/${release.assetId}"
                val apiConn = (URL(assetApiUrl).openConnection() as HttpURLConnection).apply {
                    connectTimeout = 20_000
                    readTimeout = 20_000
                    instanceFollowRedirects = false
                    setRequestProperty("User-Agent", "Pokemon-Reloaded-Android")
                    setRequestProperty("Authorization", "Bearer $trimmedToken")
                    setRequestProperty("Accept", "application/octet-stream")
                }
                apiConn.connect()
                val apiCode = apiConn.responseCode
                trace("update: API asset request $apiCode")
                if (apiCode == HttpURLConnection.HTTP_MOVED_TEMP || apiCode == HttpURLConnection.HTTP_MOVED_PERM || apiCode == 307 || apiCode == 308) {
                    val redirectUrl = apiConn.getHeaderField("Location") ?: error("Redirección S3 sin Location")
                    trace("update: S3 redirect recibido, descargando asset...")
                    // Connect to Amazon S3 WITHOUT Authorization header!
                    val s3Conn = (URL(redirectUrl).openConnection() as HttpURLConnection).apply {
                        connectTimeout = 30_000
                        readTimeout = 30_000
                        setRequestProperty("User-Agent", "Pokemon-Reloaded-Android")
                    }
                    connection = s3Conn
                } else if (apiCode in 200..299) {
                    connection = apiConn
                } else {
                    error("GitHub API asset request respondió $apiCode")
                }
            } else {
                // Public fallback using browser_download_url
                val directUrl = release.apkUrl ?: error("No hay URL de APK disponible en el release.")
                connection = open(directUrl, trimmedToken)
            }

            connection.connect()
            val code = connection.responseCode
            val total = connection.contentLengthLong
            trace("update: descarga $code, $total bytes")
            if (code !in 200..299) error("La descarga respondió $code.")
            connection.inputStream.use { input ->
                target.outputStream().use { output ->
                    val buffer = ByteArray(64 * 1024)
                    var written = 0L
                    while (true) {
                        val read = input.read(buffer)
                        if (read < 0) break
                        output.write(buffer, 0, read)
                        written += read
                        onProgress(written, total)
                    }
                    trace("update: descargados $written bytes exitosamente")
                }
            }
            target
        }.onFailure { trace("update: FALLÓ la descarga: ${it::class.java.simpleName}: ${it.message}") }
    }

    /** Compatibility overload taking direct url */
    suspend fun download(url: String, target: File, onProgress: (Long, Long) -> Unit): Result<File> =
        download(Release("", url, ""), target, "", onProgress)

    private fun trace(message: String) {
        android.util.Log.i("Reloaded", message)
    }

    fun isNewer(candidate: String, installed: String): Boolean {
        val a = numbers(candidate)
        val b = numbers(installed)
        for (index in 0 until maxOf(a.size, b.size)) {
            val left = a.getOrElse(index) { 0 }
            val right = b.getOrElse(index) { 0 }
            if (left != right) return left > right
        }
        return false
    }

    private fun numbers(value: String) = value.trim().split('.', '-', '+').mapNotNull { it.toIntOrNull() }

    private fun open(url: String, token: String = "") = (URL(url).openConnection() as HttpURLConnection).apply {
        connectTimeout = 20_000
        readTimeout = 20_000
        setRequestProperty("User-Agent", "Pokemon-Reloaded-Android")
        setRequestProperty("Accept", "application/vnd.github+json")
        if (token.isNotBlank()) {
            setRequestProperty("Authorization", "Bearer ${token.trim()}")
        }
    }

    private fun get(url: String, token: String = ""): String {
        val connection = open(url, token)
        connection.connect()
        if (connection.responseCode !in 200..299) error("GitHub respondió ${connection.responseCode}.")
        return connection.inputStream.bufferedReader().use { it.readText() }
    }
}

/**
 * Hands a downloaded APK to the system installer, which is the only thing allowed to replace the
 * app. Android asks the user to allow installs from this app the first time, and if that has not
 * been granted there is nothing to open - so that case is reported instead of failing quietly.
 */
fun installUpdate(context: android.content.Context, apk: File): String? {
    // Without this Android will not open an installer for a file from this app, and it fails with
    // nothing to see. Sending the user straight to the switch is the only way to get anywhere.
    if (!context.packageManager.canRequestPackageInstalls()) {
        return runCatching {
            context.startActivity(
                android.content.Intent(android.provider.Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES)
                    .setData(android.net.Uri.parse("package:${context.packageName}"))
                    .addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK),
            )
            "Activa \"Permitir desde esta fuente\" y vuelve a darle a Descargar."
        }.getOrElse { "Android no deja instalar desde esta app: ${it.message}" }
    }
    return runCatching {
        val uri = androidx.core.content.FileProvider.getUriForFile(context, "${context.packageName}.updates", apk)
        val intent = android.content.Intent(android.content.Intent.ACTION_VIEW)
            .setDataAndType(uri, "application/vnd.android.package-archive")
            .addFlags(android.content.Intent.FLAG_GRANT_READ_URI_PERMISSION or android.content.Intent.FLAG_ACTIVITY_NEW_TASK)
        context.startActivity(intent)
        null
    }.getOrElse { "No pude abrir el instalador: ${it::class.java.simpleName}: ${it.message}" }
}
