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

    fun dir(): File = File(Paths.publicRoot, "hud").apply { mkdirs() }

    fun list(): List<File> = dir().listFiles { f -> f.isFile && f.extension.lowercase() in IMAGE }
        ?.sortedBy { it.name.lowercase() }.orEmpty()

    fun file(ref: String): File? {
        if (ref.isEmpty()) return null
        val direct = File(ref)
        if (direct.isAbsolute) return direct.takeIf { it.isFile }
        val base = dir()
        return File(base, ref).takeIf { it.isFile }
            ?: IMAGE.map { File(base, "$ref.$it") }.firstOrNull { it.isFile }
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
