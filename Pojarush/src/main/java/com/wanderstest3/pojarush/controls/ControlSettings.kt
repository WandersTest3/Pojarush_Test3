package com.wanderstest3.pojarush.controls

import android.content.Context
import android.content.SharedPreferences
import android.net.Uri
import com.wanderstest3.pojarush.Paths
import java.io.File

class ControlSettings(context: Context) {
    private val prefs: SharedPreferences = context.applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    var layout: String
        get() = prefs.getString("layout", DefaultLayout.ID) ?: DefaultLayout.ID
        set(v) = prefs.edit().putString("layout", v).apply()
    var opacity: Float
        get() = prefs.getFloat("opacity", 0.80f)
        set(v) = prefs.edit().putFloat("opacity", v).apply()
    var buttonScale: Float
        get() = prefs.getFloat("button_scale", 1.00f)
        set(v) = prefs.edit().putFloat("button_scale", v).apply()
    var lookSensitivity: Float
        get() = prefs.getFloat("look_sensitivity", 1.00f).coerceIn(0.10f, 2.00f)
        set(v) = prefs.edit().putFloat("look_sensitivity", v.coerceIn(0.10f, 2.00f)).apply()
    var longPressMs: Int
        get() = prefs.getInt("long_press_ms", 350)
        set(v) = prefs.edit().putInt("long_press_ms", v).apply()
    var tapToUse: Boolean
        get() = prefs.getBoolean("tap_to_use", true)
        set(v) = prefs.edit().putBoolean("tap_to_use", v).apply()
    var vibrate: Boolean
        get() = prefs.getBoolean("vibrate", false)
        set(v) = prefs.edit().putBoolean("vibrate", v).apply()
    var hotbarTouch: Boolean
        get() = prefs.getBoolean("hotbar_touch", true)
        set(v) = prefs.edit().putBoolean("hotbar_touch", v).apply()
    var virtualMouse: Boolean
        get() = prefs.getBoolean("virtual_mouse", true)
        set(v) = prefs.edit().putBoolean("virtual_mouse", v).apply()
    var mouseSpeed: Float
        get() = prefs.getFloat("mouse_speed", 1.75f)
        set(v) = prefs.edit().putFloat("mouse_speed", v).apply()
    var cursorSize: Float
        get() = prefs.getFloat("cursor_size", 0.75f)
        set(v) = prefs.edit().putFloat("cursor_size", v).apply()
    var tapToClick: Boolean
        get() = prefs.getBoolean("tap_to_click", true)
        set(v) = prefs.edit().putBoolean("tap_to_click", v).apply()
    var renderScale: Float
        get() = prefs.getFloat("render_scale", 1.00f)
        set(v) = prefs.edit().putFloat("render_scale", v).apply()
    var fixedResolution: Boolean
        get() = prefs.getBoolean("fixed_resolution", false)
        set(v) = prefs.edit().putBoolean("fixed_resolution", v).apply()
    var fixedWidth: Int
        get() = prefs.getInt("fixed_width", 1280)
        set(v) = prefs.edit().putInt("fixed_width", v).apply()
    var fixedHeight: Int
        get() = prefs.getInt("fixed_height", 720)
        set(v) = prefs.edit().putInt("fixed_height", v).apply()
    var menuX: Float
        get() = prefs.getFloat("menu_x", 0.50f)
        set(v) = prefs.edit().putFloat("menu_x", v).apply()
    var menuY: Float
        get() = prefs.getFloat("menu_y", 0.00f)
        set(v) = prefs.edit().putFloat("menu_y", v).apply()
    var cursorImage: String
        get() = prefs.getString("cursor_image", "").orEmpty()
        set(v) = prefs.edit().putString("cursor_image", v).apply()
    var javaHeapMb: Int
        get() = prefs.getInt("java_heap_mb", 0)
        set(v) = prefs.edit().putInt("java_heap_mb", v).apply()
    var graphicsApi: String
        get() = prefs.getString("graphics_api", "auto")?.takeIf { it in GRAPHICS_APIS } ?: "auto"
        set(v) = prefs.edit().putString("graphics_api", v).apply()
    var cursorCenter: Boolean
        get() = prefs.getBoolean("cursor_center", false)
        set(v) = prefs.edit().putBoolean("cursor_center", v).apply()
    var controlsHidden: Boolean
        get() = prefs.getBoolean("controls_hidden", false)
        set(v) = prefs.edit().putBoolean("controls_hidden", v).apply()
    var screenTouch: Boolean
        get() = prefs.getBoolean("screen_touch", true)
        set(v) = prefs.edit().putBoolean("screen_touch", v).apply()
    var bedrockDrop: Boolean
        get() = prefs.getBoolean("bedrock_drop", true)
        set(v) = prefs.edit().putBoolean("bedrock_drop", v).apply()
    var bedrockDropMs: Int
        get() = prefs.getInt("bedrock_drop_ms", 200).coerceIn(50, 600)
        set(v) = prefs.edit().putInt("bedrock_drop_ms", v.coerceIn(50, 1000)).apply()
    var hotbarSwap: Boolean
        get() = prefs.getBoolean("hotbar_swap", true)
        set(v) = prefs.edit().putBoolean("hotbar_swap", v).apply()
    var menuSize: Float
        get() = prefs.getFloat("menu_size", 30f).coerceIn(20f, 80f)
        set(v) = prefs.edit().putFloat("menu_size", v.coerceIn(20f, 80f)).apply()
    var menuOpacity: Float
        get() = prefs.getFloat("menu_opacity", 0.50f).coerceIn(0.05f, 1.00f)
        set(v) = prefs.edit().putFloat("menu_opacity", v.coerceIn(0.05f, 1.00f)).apply()
    var menuRadius: Float
        get() = prefs.getFloat("menu_radius", 0.50f).coerceIn(0f, 0.50f)
        set(v) = prefs.edit().putFloat("menu_radius", v.coerceIn(0f, 0.50f)).apply()
    var menuColor: Int
        get() = prefs.getInt("menu_color", 0xFF57197F.toInt())
        set(v) = prefs.edit().putInt("menu_color", v).apply()
    var menuIcon: String
        get() = prefs.getString("menu_icon", "").orEmpty()
        set(v) = prefs.edit().putString("menu_icon", v).apply()
    var menuBackground: Float
        get() = prefs.getFloat("menu_background", 0.60f).coerceIn(0f, 1.00f)
        set(v) = prefs.edit().putFloat("menu_background", v.coerceIn(0f, 1.00f)).apply()
    var minecraftFont: Boolean
        get() = prefs.getBoolean("minecraft_font", true)
        set(v) = prefs.edit().putBoolean("minecraft_font", v).apply()
    var mirrorSize: Boolean
        get() = prefs.getBoolean("mirror_size", false)
        set(v) = prefs.edit().putBoolean("mirror_size", v).apply()
    var wallpaper: String
        get() = prefs.getString("wallpaper", "").orEmpty()
        set(v) = prefs.edit().putString("wallpaper", v).apply()
    var wallpaperDim: Float
        get() = prefs.getFloat("wallpaper_dim", 0.25f).coerceIn(0f, 0.90f)
        set(v) = prefs.edit().putFloat("wallpaper_dim", v.coerceIn(0f, 0.90f)).apply()
    var wallpaperVolume: Float
        get() = prefs.getFloat("wallpaper_volume", 0f).coerceIn(0f, 1f)
        set(v) = prefs.edit().putFloat("wallpaper_volume", v.coerceIn(0f, 1f)).apply()
    var keepAlive: Boolean
        get() = prefs.getBoolean("keep_alive", true)
        set(v) = prefs.edit().putBoolean("keep_alive", v).apply()
    var wallpaperOnboardingDone: Boolean
        get() = prefs.getBoolean("wallpaper_onboarding_done", false)
        set(v) = prefs.edit().putBoolean("wallpaper_onboarding_done", v).apply()
    var modsEnabled: Boolean
        get() = prefs.getBoolean("mods_enabled", true)
        set(v) = prefs.edit().putBoolean("mods_enabled", v).apply()
    var shadersEnabled: Boolean
        get() = prefs.getBoolean("shaders_enabled", true)
        set(v) = prefs.edit().putBoolean("shaders_enabled", v).apply()

    fun renderSize(viewWidth: Int, viewHeight: Int): Pair<Int, Int> =
        if (fixedResolution) fixedWidth.coerceIn(64, 7680) to fixedHeight.coerceIn(64, 4320)
        else {
            val s = renderScale.coerceIn(0.10f, 1.00f)
            (viewWidth * s).toInt().coerceAtLeast(1) to (viewHeight * s).toInt().coerceAtLeast(1)
        }

    companion object {
        private const val PREFS = "pojarush_controls"
        val GRAPHICS_APIS = listOf("auto", "vulkan", "opengl")
    }
}

object ControlLayouts {
    data class Entry(val id: String, val name: String, val removable: Boolean)

    val dir: File get() = File(Paths.publicRoot, "controls")

    fun list(context: Context, defaultName: String): List<Entry> {
        val builtIn = listOf(
            Entry(DefaultLayout.ID, bundled(context, "Pojarush_Default_V1.0.json")?.name ?: defaultName, false),
            Entry(BedrockLayout.ID, bundled(context, "Bedrock_Default_V1.0.json")?.name ?: BedrockLayout.NAME, false),
        )
        val files = dir.listFiles { f -> f.isFile && f.name.endsWith(".json", true) }.orEmpty()
            .sortedBy { it.name.lowercase() }
            .map { Entry("file:${it.name}", nameOf(it), true) }
        return builtIn + files
    }

    private fun nameOf(file: File): String =
        runCatching { ControlLayout.parse(file.readText(), file.nameWithoutExtension).name }.getOrDefault(file.nameWithoutExtension)

    fun load(context: Context, id: String, defaultName: String): ControlLayout = runCatching {
        when {
            id.startsWith("asset:") -> context.assets.open("controls/${id.removePrefix("asset:")}").bufferedReader().use {
                ControlLayout.parse(it.readText(), id.removePrefix("asset:"))
            }
            id == BedrockLayout.ID -> bundled(context, "Bedrock_Default_V1.0.json") ?: BedrockLayout.create()
            id == DefaultLayout.ID -> bundled(context, "Pojarush_Default_V1.0.json") ?: DefaultLayout.create(defaultName)
            id.startsWith("file:") -> File(dir, id.removePrefix("file:")).let { ControlLayout.parse(it.readText(), it.nameWithoutExtension) }
            else -> null
        }
    }.getOrNull() ?: DefaultLayout.create(defaultName)

    private fun bundled(context: Context, name: String): ControlLayout? = runCatching {
        context.assets.open("controls/$name").bufferedReader().use { ControlLayout.parse(it.readText(), name) }
    }.getOrNull()

    fun import(context: Context, uri: Uri): Entry {
        val text = context.contentResolver.openInputStream(uri)?.bufferedReader()?.use { it.readText() }
            ?: throw IllegalArgumentException("Empty file")
        val base = uri.lastPathSegment?.substringAfterLast('/')?.substringBeforeLast('.')?.ifBlank { null } ?: "layout"
        val layout = ControlLayout.parse(text, base)
        dir.mkdirs()
        val safe = base.replace(Regex("[^\\p{L}0-9._ -]"), "_").take(48)
        var file = File(dir, "$safe.json")
        var n = 2
        while (file.exists()) file = File(dir, "$safe ($n).json").also { n++ }
        file.writeText(text)
        return Entry("file:${file.name}", layout.name, true)
    }

    fun save(layout: ControlLayout, currentId: String, suffix: String): String {
        dir.mkdirs()
        val existing = currentId.takeIf { it.startsWith("file:") }?.let { File(dir, it.removePrefix("file:")) }
        val target = if (existing != null && existing.isFile) {
            existing
        } else {
            val base = "${layout.name} $suffix".replace(Regex("[^\\p{L}0-9._ ()-]"), "_").trim().take(48)
            var f = File(dir, "$base.json")
            var n = 2
            while (f.exists()) f = File(dir, "$base $n.json").also { n++ }
            f
        }
        val named = if (target == existing) layout else layout.copy(name = target.nameWithoutExtension)
        target.writeText(ControlLayout.write(named))
        return "file:${target.name}"
    }

    fun cursorDir(): File = File(Paths.publicRoot, "cursor_image")

    fun importCursor(context: Context, uri: Uri): File = importMedia(context, uri, cursorDir(), "cursor")

    fun importMenuIcon(context: Context, uri: Uri): File = importMedia(context, uri, File(Paths.publicRoot, "menu-button"), "icon")

    fun importWallpaper(context: Context, uri: Uri): File = importMedia(context, uri, File(Paths.publicRoot, "wallpaper"), "wallpaper")

    fun isVideo(path: String): Boolean = path.substringAfterLast('.', "").lowercase() in VIDEO

    private val VIDEO = setOf("mp4", "webm", "mkv", "3gp", "mov", "m4v")

    private fun importMedia(context: Context, uri: Uri, dir: File, base: String): File {
        val type = context.contentResolver.getType(uri).orEmpty()
        val ext = when {
            type.contains("gif") -> "gif"
            type.contains("png") -> "png"
            type.contains("webp") -> "webp"
            type.contains("jpeg") || type.contains("jpg") -> "jpg"
            type.contains("mp4") -> "mp4"
            type.contains("webm") -> "webm"
            type.contains("matroska") -> "mkv"
            type.contains("3gpp") -> "3gp"
            type.contains("quicktime") -> "mov"
            type.startsWith("video/") -> "mp4"
            else -> uri.lastPathSegment?.substringAfterLast('.', "png")?.lowercase()?.take(4) ?: "png"
        }
        dir.mkdirs()
        val temp = File(dir, ".$base.part")
        context.contentResolver.openInputStream(uri)?.use { input -> temp.outputStream().use { input.copyTo(it) } }
            ?: throw IllegalArgumentException("Empty file")
        dir.listFiles()?.forEach { if (it != temp) it.delete() }
        val file = File(dir, "$base.$ext")
        if (!temp.renameTo(file)) {
            temp.copyTo(file, overwrite = true)
            temp.delete()
        }
        return file
    }

    private fun safeName(name: String) = name.replace(Regex("[^\\p{L}0-9._ ()-]"), "_").trim().take(48).ifEmpty { "Layout" }

    private fun freeFile(base: String, keep: File? = null): File {
        var f = File(dir, "$base.json")
        var n = 2
        while (f.exists() && f != keep) f = File(dir, "$base $n.json").also { n++ }
        return f
    }

    fun saveNamed(layout: ControlLayout, currentId: String, name: String): String {
        dir.mkdirs()
        val existing = currentId.takeIf { it.startsWith("file:") }?.let { File(dir, it.removePrefix("file:")) }?.takeIf { it.isFile }
        val target = freeFile(safeName(name), existing)
        target.writeText(ControlLayout.write(layout.copy(name = name.trim().ifEmpty { target.nameWithoutExtension })))
        if (existing != null && existing != target) existing.delete()
        return "file:${target.name}"
    }

    fun rename(id: String, name: String): String {
        if (!id.startsWith("file:")) return id
        val file = File(dir, id.removePrefix("file:"))
        if (!file.isFile) return id
        val layout = ControlLayout.parse(file.readText(), file.nameWithoutExtension)
        return saveNamed(layout, id, name)
    }

    fun nameOf(id: String, fallback: String): String = runCatching {
        if (!id.startsWith("file:")) return fallback
        val file = File(dir, id.removePrefix("file:"))
        ControlLayout.parse(file.readText(), file.nameWithoutExtension).name
    }.getOrDefault(fallback)

    fun delete(id: String) {
        if (id.startsWith("file:")) File(dir, id.removePrefix("file:")).delete()
    }
}
