package com.wanderstest3.pojarush.controls

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.net.Uri
import com.wanderstest3.pojarush.Paths
import java.io.File
import java.util.zip.ZipInputStream

object HudImages {
    val DEFAULT_STYLE = Style(
        fill = android.graphics.Color.argb(110, 0x57, 0x19, 0x7F),
        fillPressed = android.graphics.Color.argb(235, 0x57, 0x19, 0x7F),
        text = android.graphics.Color.WHITE,
        textPressed = android.graphics.Color.WHITE,
        stroke = android.graphics.Color.argb(170, 255, 255, 255),
        strokeDp = 1f,
        radiusDp = 10f,
        textSp = 15f,
    )

    private val IMAGE = setOf("png", "jpg", "jpeg", "webp", "gif")

    fun dir(): File = File(Paths.publicRoot, "hud_image").apply { mkdirs() }

    fun list(): List<File> {
        unpackDropped()
        return dir().walkTopDown().maxDepth(4)
            .filter { it.isFile && it.extension.lowercase() in IMAGE && !it.name.startsWith(".") }
            .sortedBy { it.name.lowercase() }.toList()
    }

    private fun key(name: String): String = name.lowercase().replace(Regex("[^a-z0-9]"), "")

    private var index: Map<String, File> = emptyMap()
    private var indexedAt = 0L

    @Synchronized
    private fun indexed(): Map<String, File> {
        val now = System.currentTimeMillis()
        if (now - indexedAt > 3000) {
            index = list().groupBy { key(it.nameWithoutExtension) }.mapValues { it.value.first() }
            indexedAt = now
        }
        return index
    }

    private fun unpackDropped() {
        val dir = dir()
        dir.listFiles { f -> f.isFile && f.extension.equals("zip", true) }?.forEach { zip ->
            runCatching {
                ZipInputStream(zip.inputStream().buffered()).use { input ->
                    while (true) {
                        val entry = input.nextEntry ?: break
                        val base = entry.name.substringAfterLast('/')
                        if (entry.isDirectory || base.startsWith(".") || base.substringAfterLast('.', "").lowercase() !in IMAGE) continue
                        File(dir, base).outputStream().use { input.copyTo(it) }
                    }
                }
                zip.renameTo(File(dir, "${zip.name}.imported"))
            }
        }
    }

    fun file(ref: String): File? {
        if (ref.isEmpty()) return null
        val direct = File(ref)
        if (direct.isAbsolute) return direct.takeIf { it.isFile }
        val base = dir()
        return File(base, ref).takeIf { it.isFile }
            ?: IMAGE.map { File(base, "$ref.$it") }.firstOrNull { it.isFile }
            ?: indexed()[key(ref.substringBeforeLast('.').ifEmpty { ref })]
    }

    fun has(ref: String): Boolean = file(ref) != null

    fun decode(ref: String): Bitmap? = file(ref)?.let { BitmapFactory.decodeFile(it.absolutePath) }

    fun import(context: Context, uri: Uri): Int {
        val resolver = context.contentResolver
        val type = resolver.getType(uri).orEmpty()
        val name = uri.lastPathSegment.orEmpty().substringAfterLast('/')
        val dir = dir()
        if (type.contains("zip") || name.endsWith(".zip", true)) {
            var count = 0
            resolver.openInputStream(uri)?.use { raw ->
                ZipInputStream(raw.buffered()).use { zip ->
                    while (true) {
                        val entry = zip.nextEntry ?: break
                        val base = entry.name.substringAfterLast('/')
                        if (entry.isDirectory || base.startsWith(".") || base.substringAfterLast('.', "").lowercase() !in IMAGE) continue
                        File(dir, base).outputStream().use { zip.copyTo(it) }
                        count++
                    }
                }
            } ?: throw IllegalArgumentException("Empty file")
            if (count == 0) throw IllegalArgumentException("No images in the zip")
            return count
        }
        val ext = when {
            type.contains("png") -> "png"
            type.contains("webp") -> "webp"
            type.contains("gif") -> "gif"
            type.contains("jpeg") || type.contains("jpg") -> "jpg"
            else -> name.substringAfterLast('.', "png").lowercase().take(4)
        }
        val base = name.substringBeforeLast('.').ifBlank { "image" }.replace(Regex("[^A-Za-z0-9_.-]"), "_")
        var target = File(dir, "$base.$ext")
        var n = 2
        while (target.exists()) target = File(dir, "${base}_${n++}.$ext")
        resolver.openInputStream(uri)?.use { input -> target.outputStream().use { input.copyTo(it) } }
            ?: throw IllegalArgumentException("Empty file")
        return 1
    }
}
