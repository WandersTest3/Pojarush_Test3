package com.wanderstest3.pojarush.horizon

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import java.io.File
import java.io.IOException
import java.security.MessageDigest
import java.util.concurrent.TimeUnit
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject

data class RegistryEntry(
    val id: String,
    val kind: String,
    val name: String,
    val summary: String,
    val authors: String,
    val icon: String?,
    val version: String,
    val game: String,
    val fileName: String,
    val url: String,
    val size: Long,
    val sha256: String,
    val downloads: Long,
) {
    val packKind: PackKind?
        get() = when (kind) {
            "mod" -> PackKind.MOD
            "resourcepack" -> PackKind.RESOURCE
            "shader" -> PackKind.SHADER
            else -> null
        }
}

object ModRegistry {
    const val SITE = "https://playhorizon.dev/mods"
    private const val GAME = "26.3"
    private const val INDEX = "https://horizon-rust.github.io/horizon-mods/index.json"

    private val client = OkHttpClient.Builder()
        .connectTimeout(20, TimeUnit.SECONDS)
        .readTimeout(60, TimeUnit.SECONDS)
        .followRedirects(true)
        .build()

    @Volatile private var cached: List<RegistryEntry>? = null
    private val icons = HashMap<String, Bitmap?>()

    fun cachedEntries(): List<RegistryEntry>? = cached

    fun fetch(force: Boolean = false): List<RegistryEntry> {
        if (!force) cached?.let { return it }
        val text = client.newCall(Request.Builder().url(INDEX).build()).execute().use { r ->
            if (!r.isSuccessful) throw IOException("HTTP ${r.code}")
            r.body?.string() ?: throw IOException("Empty index")
        }
        val root = JSONObject(text)
        val revoked = HashSet<String>()
        root.optJSONArray("revoked")?.let { arr ->
            for (i in 0 until arr.length()) {
                val r = arr.optJSONObject(i) ?: continue
                revoked += r.optString("sha256")
            }
        }
        val projects = root.getJSONArray("projects")
        val out = ArrayList<RegistryEntry>()
        for (i in 0 until projects.length()) {
            val p = projects.getJSONObject(i)
            val versions = p.optJSONArray("versions") ?: continue
            if (versions.length() == 0) continue
            val v = (0 until versions.length()).map { versions.getJSONObject(it) }
                .firstOrNull { it.optString("game") == GAME } ?: versions.getJSONObject(0)
            val file = v.optJSONObject("file") ?: continue
            val sha = file.optString("sha256")
            if (sha in revoked) continue
            val authors = p.optJSONArray("authors")?.let { a -> (0 until a.length()).joinToString(", ") { a.optString(it) } }.orEmpty()
            out += RegistryEntry(
                id = p.optString("id"),
                kind = p.optString("kind"),
                name = p.optString("name").ifEmpty { p.optString("id") },
                summary = p.optString("summary"),
                authors = authors,
                icon = p.optString("icon").takeIf { it.startsWith("https://") },
                version = v.optString("version"),
                game = v.optString("game"),
                fileName = file.optString("name"),
                url = file.optString("url"),
                size = file.optLong("size"),
                sha256 = sha,
                downloads = p.optLong("downloads", 0L),
            )
        }
        out.sortByDescending { it.downloads }
        cached = out
        return out
    }

    fun icon(entry: RegistryEntry): Bitmap? {
        val url = entry.icon ?: return null
        synchronized(icons) { if (icons.containsKey(url)) return icons[url] }
        val bitmap = runCatching {
            client.newCall(Request.Builder().url(url).build()).execute().use { r ->
                if (!r.isSuccessful) null else r.body?.bytes()?.let { BitmapFactory.decodeByteArray(it, 0, it.size) }
            }
        }.getOrNull()
        synchronized(icons) { icons[url] = bitmap }
        return bitmap
    }

    fun installedFile(entry: RegistryEntry): File? {
        val kind = entry.packKind ?: return null
        val dir = GameFiles.dir(kind)
        val file = File(dir, entry.fileName)
        if (file.exists()) return file
        val folder = File(dir, entry.fileName.substringBeforeLast('.'))
        return folder.takeIf { it.isDirectory }
    }

    fun install(entry: RegistryEntry, progress: (Float) -> Unit): PackEntry {
        val kind = entry.packKind ?: throw IOException("Not supported on Android")
        if (!entry.url.startsWith("https://")) throw IOException("Invalid download URL")
        HorizonManager.ensureFolders()
        return GameFiles.install(kind, entry.fileName) { out ->
            val digest = MessageDigest.getInstance("SHA-256")
            client.newCall(Request.Builder().url(entry.url).build()).execute().use { r ->
                if (!r.isSuccessful) throw IOException("HTTP ${r.code}")
                val body = r.body ?: throw IOException("Empty download")
                val total = body.contentLength().takeIf { it > 0 } ?: entry.size
                body.byteStream().use { input ->
                    val buf = ByteArray(64 * 1024)
                    var done = 0L
                    while (true) {
                        val n = input.read(buf)
                        if (n < 0) break
                        out.write(buf, 0, n)
                        digest.update(buf, 0, n)
                        done += n
                        if (total > 0) progress((done.toFloat() / total).coerceIn(0f, 1f))
                    }
                }
            }
            val hex = digest.digest().joinToString("") { "%02x".format(it) }
            if (entry.sha256.isNotEmpty() && !hex.equals(entry.sha256, ignoreCase = true)) {
                throw IOException("SHA-256 mismatch")
            }
        }
    }
}
