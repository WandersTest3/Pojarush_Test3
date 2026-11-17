package com.wanderstest3.pojarush.horizon

import com.wanderstest3.pojarush.BuildConfig
import java.io.IOException
import java.net.URLEncoder
import java.security.MessageDigest
import java.util.concurrent.TimeUnit
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONArray
import org.json.JSONObject

object Modrinth {
    private const val API = "https://api.modrinth.com/v2"
    private const val GAME = "26.3"
    const val PAGE = 30

    private val client = OkHttpClient.Builder()
        .connectTimeout(20, TimeUnit.SECONDS)
        .readTimeout(60, TimeUnit.SECONDS)
        .followRedirects(true)
        .build()

    private fun get(url: String): String {
        val request = Request.Builder().url(url)
            .header("User-Agent", "WandersTest3/Pojarush_Test3/${BuildConfig.VERSION_NAME} (github.com/WandersTest3/Pojarush_Test3)")
            .build()
        return client.newCall(request).execute().use { r ->
            if (!r.isSuccessful) throw IOException("Modrinth HTTP ${r.code}")
            r.body.string()
        }
    }

    private fun type(kind: PackKind): String = when (kind) {
        PackKind.RESOURCE -> "resourcepack"
        PackKind.SHADER -> "shader"
        PackKind.MOD -> throw IOException("Mods come from the Horizon registry")
    }

    fun site(kind: PackKind): String = when (kind) {
        PackKind.SHADER -> "https://modrinth.com/shaders"
        else -> "https://modrinth.com/resourcepacks"
    }

    fun search(kind: PackKind, query: String, offset: Int): List<RegistryEntry> {
        val facets = JSONArray().put(JSONArray().put("project_type:${type(kind)}")).toString()
        val url = "$API/search?limit=$PAGE&offset=$offset&index=" + (if (query.isBlank()) "downloads" else "relevance") +
            "&facets=" + URLEncoder.encode(facets, "UTF-8") +
            (if (query.isBlank()) "" else "&query=" + URLEncoder.encode(query.trim(), "UTF-8"))
        val hits = JSONObject(get(url)).optJSONArray("hits") ?: return emptyList()
        return (0 until hits.length()).mapNotNull { i ->
            val h = hits.optJSONObject(i) ?: return@mapNotNull null
            val versions = h.optJSONArray("versions")
            val games = versions?.let { v -> (0 until v.length()).map { v.optString(it) } }.orEmpty()
            RegistryEntry(
                id = h.optString("project_id"),
                kind = type(kind),
                name = h.optString("title").ifEmpty { h.optString("slug") },
                summary = h.optString("description"),
                authors = h.optString("author"),
                icon = h.optString("icon_url").takeIf { it.startsWith("https://") },
                version = games.lastOrNull().orEmpty(),
                game = if (GAME in games) GAME else games.lastOrNull().orEmpty(),
                fileName = "",
                url = "",
                size = 0L,
                sha256 = "",
                downloads = h.optLong("downloads", 0L),
                source = RegistryEntry.MODRINTH,
                slug = h.optString("slug"),
            )
        }
    }

    private class File(val name: String, val url: String, val size: Long, val sha512: String)

    private fun latestFile(entry: RegistryEntry): File {
        val loaders = if (entry.kind == "shader") "[\"iris\",\"optifine\"]" else "[\"minecraft\"]"
        val versions = JSONArray(get("$API/project/${entry.id}/version?loaders=" + URLEncoder.encode(loaders, "UTF-8")))
            .let { a -> (0 until a.length()).mapNotNull { a.optJSONObject(it) } }
            .ifEmpty { JSONArray(get("$API/project/${entry.id}/version")).let { a -> (0 until a.length()).mapNotNull { a.optJSONObject(it) } } }
        val pick = versions.firstOrNull { v ->
            v.optJSONArray("game_versions")?.let { g -> (0 until g.length()).any { g.optString(it) == GAME } } == true
        } ?: versions.firstOrNull() ?: throw IOException("No file to download")
        val files = pick.optJSONArray("files") ?: throw IOException("No file to download")
        val file = (0 until files.length()).mapNotNull { files.optJSONObject(it) }
            .let { all -> all.firstOrNull { it.optBoolean("primary") } ?: all.firstOrNull() }
            ?: throw IOException("No file to download")
        val name = file.optString("filename")
        if (name.isBlank() || name.contains('/') || name.contains('\\') || name.startsWith(".")) throw IOException("Invalid file name")
        return File(name, file.optString("url"), file.optLong("size"), file.optJSONObject("hashes")?.optString("sha512").orEmpty())
    }

    fun install(entry: RegistryEntry, progress: (Float) -> Unit): PackEntry {
        val kind = entry.packKind ?: throw IOException("Not supported")
        val file = latestFile(entry)
        if (!file.url.startsWith("https://")) throw IOException("Invalid download URL")
        HorizonManager.ensureFolders()
        return GameFiles.install(kind, file.name) { out ->
            val digest = MessageDigest.getInstance("SHA-512")
            client.newCall(Request.Builder().url(file.url).build()).execute().use { r ->
                if (!r.isSuccessful) throw IOException("HTTP ${r.code}")
                val body = r.body
                val total = body.contentLength().takeIf { it > 0 } ?: file.size
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
            if (file.sha512.isNotEmpty() && !hex.equals(file.sha512, ignoreCase = true)) throw IOException("SHA-512 mismatch")
        }.also { InstalledModrinth.remember(entry.id, file.name) }
    }
}

object InstalledModrinth {
    private val names = HashMap<String, String>()
    fun remember(id: String, file: String) = synchronized(names) { names[id] = file }
    fun file(id: String): String? = synchronized(names) { names[id] }
}
