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
    val source: Int = HORIZON,
    val slug: String = "",
) {
    companion object {
        const val HORIZON = 0
        const val MODRINTH = 1
        const val POJARUSH = 2
    }

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
    const val POJARUSH_SITE = "https://wanderstest3.github.io/Pojarush_Test3-Mods/"
    private const val GAME = "26.3"
    private const val INDEX = "https://horizon-rust.github.io/horizon-mods/index.json"
    private const val POJARUSH_INDEX = POJARUSH_SITE + "index.json"

    fun site(source: Int): String = if (source == RegistryEntry.POJARUSH) POJARUSH_SITE else SITE

    fun page(entry: RegistryEntry): String? =
        if (entry.source == RegistryEntry.POJARUSH) POJARUSH_SITE + "mod.html?id=" + android.net.Uri.encode(entry.id) else null

    private val client = OkHttpClient.Builder()
        .connectTimeout(20, TimeUnit.SECONDS)
        .readTimeout(60, TimeUnit.SECONDS)
        .followRedirects(true)
        .build()

    private val cached = java.util.concurrent.ConcurrentHashMap<Int, List<RegistryEntry>>()
    private val icons = HashMap<String, Bitmap?>()

    fun cachedEntries(source: Int = RegistryEntry.POJARUSH): List<RegistryEntry>? = cached[source]

    fun fetch(force: Boolean = false, source: Int = RegistryEntry.POJARUSH): List<RegistryEntry> {
        if (!force) cached[source]?.let { return it }
        val url = if (source == RegistryEntry.POJARUSH) POJARUSH_INDEX else INDEX
        val request = Request.Builder().url(url).apply { if (force) header("Cache-Control", "no-cache") }.build()
        val text = client.newCall(request).execute().use { r ->
            if (!r.isSuccessful) throw IOException("HTTP ${r.code}")
            r.body?.string() ?: throw IOException("Empty index")
        }
        val root = JSONObject(text)
        val revoked = HashSet<String>()
        val revokedIds = HashSet<String>()
        root.optJSONArray("revoked")?.let { arr ->
            for (i in 0 until arr.length()) {
                val r = arr.optJSONObject(i) ?: continue
                    val sha = r.optString("sha256")
                if (sha.isNotEmpty()) revoked += sha else revokedIds += r.optString("id")
            }
        }
        val projects = root.getJSONArray("projects")
        val out = ArrayList<RegistryEntry>()
        for (i in 0 until projects.length()) {
            val p = projects.getJSONObject(i)
            if (p.optString("id") in revokedIds) continue
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
                source = source,
            )
        }
        out.sortByDescending { it.downloads }
        cached[source] = out
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
        val name = if (entry.source == RegistryEntry.MODRINTH) InstalledModrinth.file(entry.id) ?: return null else entry.fileName
        val file = File(dir, name)
        if (file.exists()) return file
        val folder = File(dir, name.substringBeforeLast('.'))
        return folder.takeIf { it.isDirectory }
    }

    fun olderVersions(entry: RegistryEntry): List<File> {
        if (entry.source == RegistryEntry.MODRINTH || entry.packKind != PackKind.MOD) return emptyList()
        val files = GameFiles.dir(PackKind.MOD).listFiles() ?: return emptyList()
        return files.filter { f ->
            f.isFile && f.name != entry.fileName && f.name.endsWith(".hmod") &&
                (f.name == "${entry.id}.hmod" || f.name.startsWith("${entry.id}-"))
        }
    }

    fun install(entry: RegistryEntry, progress: (Float) -> Unit): PackEntry {
        val kind = entry.packKind ?: throw IOException("Not supported on Android")
        if (!entry.url.startsWith("https://")) throw IOException("Invalid download URL")
        HorizonManager.ensureFolders()
        val older = olderVersions(entry)
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
        }.also { older.forEach { it.delete() } }
    }
}
