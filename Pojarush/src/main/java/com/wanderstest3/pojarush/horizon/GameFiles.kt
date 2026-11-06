package com.wanderstest3.pojarush.horizon

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import java.io.File
import java.io.IOException
import java.util.zip.ZipFile
import org.json.JSONArray

enum class PackKind { MOD, RESOURCE, SHADER }

data class PackEntry(
    val file: File,
    val name: String,
    val id: String?,
    val enabled: Boolean?,
)

object GameFiles {
    private val idPattern = Regex("""(?m)^\s*id\s*=\s*"([a-z0-9_]{1,64})"""")
    private val namePattern = Regex("""(?m)^\s*name\s*=\s*"([^"]*)"""")

    fun dir(kind: PackKind): File = when (kind) {
        PackKind.MOD -> HorizonManager.modsDir
        PackKind.RESOURCE -> HorizonManager.resourcePacksDir
        PackKind.SHADER -> HorizonManager.shaderPacksDir
    }

    fun extensions(kind: PackKind): List<String> = when (kind) {
        PackKind.MOD -> listOf("hmod")
        PackKind.RESOURCE -> listOf("zip")
        PackKind.SHADER -> listOf("hshader", "zip")
    }

    fun list(kind: PackKind): List<PackEntry> {
        val files = dir(kind).listFiles().orEmpty()
            .filter { !it.name.startsWith(".") && (it.isDirectory || extensions(kind).contains(it.extension.lowercase())) }
            .sortedBy { it.name.lowercase() }
        val enabled = if (kind == PackKind.MOD) readEnabled() else emptyMap()
        val packs = if (kind == PackKind.RESOURCE) readPackSelection() else emptyList()
        return files.map { f ->
            when (kind) {
                PackKind.MOD -> {
                    val manifest = readManifest(f)
                    val id = manifest?.first
                    PackEntry(f, manifest?.second ?: f.nameWithoutExtension, id, id?.let { enabled[it] ?: false })
                }
                PackKind.RESOURCE -> PackEntry(f, f.nameWithoutExtension.ifEmpty { f.name }, packId(f), packId(f) in packs)
                PackKind.SHADER -> PackEntry(f, f.nameWithoutExtension.ifEmpty { f.name }, null, null)
            }
        }
    }

    private fun packId(file: File) = "file/${file.name}"

    private val optionsFile: File get() = File(HorizonManager.gameDir, "options.txt")

    private fun readList(key: String): List<String> {
        val line = optionsFile.takeIf { it.isFile }?.readLines()?.firstOrNull { it.startsWith("$key:") } ?: return emptyList()
        return runCatching {
            val a = JSONArray(line.substringAfter(':'))
            (0 until a.length()).map { a.getString(it) }
        }.getOrDefault(emptyList())
    }

    private fun writeLists(values: Map<String, List<String>>) {
        val lines = if (optionsFile.isFile) optionsFile.readLines().toMutableList() else mutableListOf()
        for ((key, list) in values) {
            val line = "$key:" + list.joinToString(",", "[", "]") { "\"" + it.replace("\\", "\\\\").replace("\"", "\\\"") + "\"" }
            val index = lines.indexOfFirst { it.startsWith("$key:") }
            if (index >= 0) lines[index] = line else lines += line
        }
        optionsFile.parentFile?.mkdirs()
        optionsFile.writeText(lines.joinToString("\n") + "\n")
    }

    private fun readPackSelection(): List<String> = readList("resourcePacks")

    fun setPackEnabled(file: File, on: Boolean) {
        val id = packId(file)
        val selected = readPackSelection().filter { it != id }.toMutableList()
        if ("vanilla" !in selected) selected.add(0, "vanilla")
        if (on) selected += id
        val incompatible = readList("incompatibleResourcePacks").filter { it != id }.toMutableList()
        if (on) incompatible += id
        writeLists(mapOf("resourcePacks" to selected, "incompatibleResourcePacks" to incompatible))
    }

    fun setEnabled(kind: PackKind, entry: PackEntry, on: Boolean) {
        when (kind) {
            PackKind.MOD -> entry.id?.let { setModEnabled(it, on) }
            PackKind.RESOURCE -> setPackEnabled(entry.file, on)
            PackKind.SHADER -> {}
        }
    }

    private fun manifestText(file: File): String? =
        if (file.isDirectory) {
            File(file, "horizon-mod.toml").takeIf { it.isFile }?.readText()
        } else {
            ZipFile(file).use { zip ->
                val entry = zip.getEntry("horizon-mod.toml")
                if (entry == null) null else zip.getInputStream(entry).bufferedReader().use { it.readText() }
            }
        }

    private fun readManifest(file: File): Pair<String, String?>? {
        val text = runCatching { manifestText(file) }.getOrNull() ?: return null
        val id = idPattern.find(text)?.groupValues?.get(1) ?: return null
        return id to namePattern.find(text)?.groupValues?.get(1)
    }

    private fun readEnabled(): Map<String, Boolean> {
        val file = HorizonManager.modsFile
        if (!file.isFile) return emptyMap()
        return runCatching {
            file.readLines().mapNotNull { line ->
                val trimmed = line.trim()
                if (!trimmed.startsWith("mod.")) return@mapNotNull null
                val body = trimmed.removePrefix("mod.")
                val colon = body.lastIndexOf(':')
                if (colon <= 0) return@mapNotNull null
                body.substring(0, colon) to (body.substring(colon + 1).trim() == "true")
            }.toMap()
        }.getOrDefault(emptyMap())
    }

    fun setModEnabled(id: String, on: Boolean) {
        val file = HorizonManager.modsFile
        file.parentFile?.mkdirs()
        val prefix = "mod.$id:"
        val lines = if (file.isFile) file.readLines().toMutableList() else mutableListOf()
        val index = lines.indexOfFirst { it.trim().startsWith(prefix) }
        val line = "$prefix$on"
        if (index >= 0) lines[index] = line else lines += line
        file.writeText(lines.joinToString("\n") + "\n")
    }

    private fun displayName(context: Context, uri: Uri): String {
        runCatching {
            context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { c ->
                if (c.moveToFirst()) c.getString(0)?.takeIf { it.isNotBlank() }?.let { return it }
            }
        }
        return uri.lastPathSegment?.substringAfterLast('/') ?: "file"
    }

    fun import(context: Context, kind: PackKind, uri: Uri): PackEntry {
        val raw = displayName(context, uri)
        val ext = raw.substringAfterLast('.', "").lowercase()
        if (kind == PackKind.MOD && ext == "jar") {
            throw IOException("Fabric and NeoForge .jar mods do not run in Horizon; use a Horizon .hmod file")
        }
        if (kind == PackKind.MOD && ext == "hnmod") {
            throw IOException("Native mods (.hnmod) must be built into the game; this launcher cannot load them")
        }
        if (ext !in extensions(kind)) throw IOException("Expected: " + extensions(kind).joinToString(", ") { ".$it" })
        val safe = raw.replace(Regex("[^A-Za-z0-9._ ()+-]"), "_").take(96)
        val target = dir(kind).apply { mkdirs() }.let { d ->
            var f = File(d, safe)
            var n = 2
            while (f.exists()) f = File(d, "${safe.substringBeforeLast('.')} ($n).${ext}").also { n++ }
            f
        }
        val temp = File(target.parentFile, target.name + ".part")
        context.contentResolver.openInputStream(uri)?.use { input -> temp.outputStream().use { input.copyTo(it) } }
            ?: throw IOException("Empty file")
        val nested = try {
            validate(kind, temp)
        } catch (e: Exception) {
            temp.delete()
            throw e
        }
        if (nested != null) {
            val folder = File(target.parentFile, target.name.substringBeforeLast('.'))
            try {
                extractFolder(temp, nested, folder)
            } finally {
                temp.delete()
            }
            setPackEnabled(folder, true)
            return PackEntry(folder, folder.name, packId(folder), true)
        }
        if (!temp.renameTo(target)) {
            temp.copyTo(target, overwrite = true)
            temp.delete()
        }
        if (kind == PackKind.RESOURCE) {
            setPackEnabled(target, true)
            return PackEntry(target, target.nameWithoutExtension, packId(target), true)
        }
        val manifest = if (kind == PackKind.MOD) readManifest(target) else null
        if (manifest != null) setModEnabled(manifest.first, true)
        return PackEntry(target, manifest?.second ?: target.nameWithoutExtension, manifest?.first, manifest?.let { true })
    }

    private fun validate(kind: PackKind, file: File): String? {
        val zip = runCatching { ZipFile(file) }.getOrElse { throw IOException("Not a valid zip file") }
        zip.use {
            when (kind) {
                PackKind.MOD -> if (it.getEntry("horizon-mod.toml") == null) throw IOException("horizon-mod.toml is missing")
                PackKind.RESOURCE -> if (it.getEntry("pack.mcmeta") == null) {
                    val inner = it.entries().asSequence().map { e -> e.name }
                        .filter { n -> n.endsWith("/pack.mcmeta") && n.count { c -> c == '/' } == 1 }
                        .toList()
                    if (inner.size != 1) throw IOException("pack.mcmeta is missing at the root of the zip")
                    return inner[0].substringBefore('/') + "/"
                }
                PackKind.SHADER -> {}
            }
        }
        return null
    }

    private fun extractFolder(zipFile: File, prefix: String, out: File) {
        if (out.exists()) throw IOException("${out.name} already exists")
        val root = out.canonicalFile
        ZipFile(zipFile).use { zip ->
            for (e in zip.entries()) {
                if (!e.name.startsWith(prefix) || e.name == prefix) continue
                val dest = File(root, e.name.removePrefix(prefix)).canonicalFile
                if (!dest.path.startsWith(root.path + File.separator)) throw IOException("Bad path in zip: ${e.name}")
                if (e.isDirectory) dest.mkdirs() else {
                    dest.parentFile?.mkdirs()
                    zip.getInputStream(e).use { input -> dest.outputStream().use { input.copyTo(it) } }
                }
            }
        }
    }

    fun delete(kind: PackKind, entry: PackEntry) {
        if (kind == PackKind.MOD && entry.id != null) removeModLine(entry.id)
        if (kind == PackKind.RESOURCE) setPackEnabled(entry.file, false)
        if (entry.file.isDirectory) entry.file.deleteRecursively() else entry.file.delete()
    }

    private fun removeModLine(id: String) {
        val file = HorizonManager.modsFile
        if (!file.isFile) return
        val prefix = "mod.$id:"
        runCatching { file.writeText(file.readLines().filterNot { it.trim().startsWith(prefix) }.joinToString("\n") + "\n") }
    }
}
