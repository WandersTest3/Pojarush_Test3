package com.wanderstest3.pojarush.ui

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.BitmapFactory
import android.graphics.ImageDecoder
import android.graphics.Matrix
import android.graphics.SurfaceTexture
import android.graphics.drawable.AnimatedImageDrawable
import android.graphics.drawable.Drawable
import android.media.AudioAttributes
import android.media.MediaPlayer
import android.os.Build
import android.view.Surface
import android.view.TextureView
import android.widget.ImageView
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Button
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.wanderstest3.pojarush.R
import com.wanderstest3.pojarush.controls.ControlLayouts
import com.wanderstest3.pojarush.controls.ControlSettings
import java.io.File

object WallpaperState {
    var version by mutableIntStateOf(0)
}

@Composable
fun WallpaperBackground() {
    val context = LocalContext.current
    val settings = remember { ControlSettings(context) }
    val version = WallpaperState.version
    val path = remember(version) { settings.wallpaper }
    val dim = remember(version) { settings.wallpaperDim }
    val volume = remember(version) { settings.wallpaperVolume }
    val file = remember(path, version) { File(path).takeIf { path.isNotEmpty() && it.isFile } } ?: return
    val stamp = file.lastModified()
    Box(Modifier.fillMaxSize()) {
        if (ControlLayouts.isVideo(file.name)) {
            androidx.compose.runtime.key(file.absolutePath, stamp) {
                AndroidView(
                    factory = { WallpaperVideoView(it, file, volume) },
                    update = { it.setVolume(volume) },
                    modifier = Modifier.fillMaxSize(),
                    onRelease = { it.release() }
                )
            }
        } else {
            androidx.compose.runtime.key(file.absolutePath, stamp) {
                AndroidView(
                    factory = { ctx ->
                        ImageView(ctx).apply {
                            scaleType = ImageView.ScaleType.CENTER_CROP
                            setImageDrawable(loadDrawable(ctx, file))
                        }
                    },
                    modifier = Modifier.fillMaxSize(),
                    onRelease = { (it.drawable as? AnimatedImageDrawable)?.stop() }
                )
            }
        }
        Box(Modifier.fillMaxSize().background(Color.Black.copy(alpha = dim)))
    }
}

private fun loadDrawable(context: Context, file: File): Drawable? = runCatching {
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
        val source = ImageDecoder.createSource(file)
        ImageDecoder.decodeDrawable(source) { decoder, info, _ ->
            val longest = maxOf(info.size.width, info.size.height)
            val limit = 2560
            if (longest > limit) decoder.setTargetSampleSize((longest + limit - 1) / limit)
        }.also { if (it is AnimatedImageDrawable) { it.repeatCount = AnimatedImageDrawable.REPEAT_INFINITE; it.start() } }
    } else {
        val opts = BitmapFactory.Options().apply { inSampleSize = 2 }
        BitmapFactory.decodeFile(file.absolutePath, opts)?.let { android.graphics.drawable.BitmapDrawable(context.resources, it) }
    }
}.getOrNull()

@SuppressLint("ViewConstructor")
private class WallpaperVideoView(context: Context, private val file: File, private var volume: Float) : TextureView(context), TextureView.SurfaceTextureListener {
    private var player: MediaPlayer? = null
    private var surface: Surface? = null
    private var videoW = 0
    private var videoH = 0

    init {
        surfaceTextureListener = this
        isOpaque = false
    }

    fun setVolume(v: Float) {
        volume = v
        runCatching { player?.setVolume(v, v) }
    }

    override fun onSurfaceTextureAvailable(texture: SurfaceTexture, width: Int, height: Int) {
        val s = Surface(texture)
        surface = s
        player = runCatching {
            MediaPlayer().apply {
                setAudioAttributes(AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA).setContentType(AudioAttributes.CONTENT_TYPE_MOVIE).build())
                setDataSource(file.absolutePath)
                setSurface(s)
                isLooping = true
                setVolume(volume, volume)
                setOnVideoSizeChangedListener { _, w, h ->
                    videoW = w
                    videoH = h
                    crop()
                }
                setOnPreparedListener {
                    it.setVolume(volume, volume)
                    it.start()
                }
                setOnErrorListener { _, _, _ -> true }
                prepareAsync()
            }
        }.getOrNull()
    }

    private fun crop() {
        if (videoW <= 0 || videoH <= 0 || width <= 0 || height <= 0) return
        val view = width.toFloat() / height
        val video = videoW.toFloat() / videoH
        val (sx, sy) = if (video > view) video / view to 1f else 1f to view / video
        setTransform(Matrix().apply { setScale(sx, sy, width / 2f, height / 2f) })
    }

    override fun onWindowVisibilityChanged(visibility: Int) {
        super.onWindowVisibilityChanged(visibility)
        runCatching {
            val p = player ?: return
            if (visibility == VISIBLE) p.start() else if (p.isPlaying) p.pause()
        }
    }

    override fun onSurfaceTextureSizeChanged(texture: SurfaceTexture, width: Int, height: Int) = crop()

    override fun onSurfaceTextureDestroyed(texture: SurfaceTexture): Boolean {
        release()
        return true
    }

    override fun onSurfaceTextureUpdated(texture: SurfaceTexture) {}

    fun release() {
        runCatching { player?.release() }
        player = null
        runCatching { surface?.release() }
        surface = null
    }
}

@Composable
fun InterfaceSettings(settings: ControlSettings) {
    var font by remember { mutableStateOf(settings.minecraftFont) }
    PojarushCard {
        CardTitle(stringResource(R.string.interface_title))
        SettingSlider(stringResource(R.string.menu_background), settings.menuBackground, 0f..1.00f, 0.05f, { "${Math.round(it * 100)}%" }) {
            settings.menuBackground = it
            UiPrefs.menuBackground = it
        }
        SettingSwitch(stringResource(R.string.minecraft_font), font, stringResource(R.string.minecraft_font_desc)) {
            settings.minecraftFont = it
            font = it
            UiPrefs.minecraftFont = it
        }
    }
}

@Composable
fun LodSettings() {
    val steps = com.wanderstest3.pojarush.horizon.HorizonManager.LOD_STEPS
    var lod by remember { mutableStateOf(com.wanderstest3.pojarush.horizon.HorizonManager.lodDistance()) }
    val index = steps.indexOfFirst { it >= lod }.let { if (it < 0) steps.lastIndex else it }
    val off = stringResource(R.string.lod_off)
    PojarushCard {
        CardTitle(stringResource(R.string.lod_title))
        SettingSlider(stringResource(R.string.lod_distance), index.toFloat(), 0f..steps.lastIndex.toFloat(), 1f, { v ->
            val chunks = steps[v.toInt().coerceIn(0, steps.lastIndex)]
            if (chunks == 0) off else "$chunks"
        }) { v ->
            lod = steps[Math.round(v).coerceIn(0, steps.lastIndex)]
            com.wanderstest3.pojarush.horizon.HorizonManager.setLodDistance(lod)
        }
        Hint(stringResource(R.string.lod_desc))
    }
}

@Composable
fun ExtrasSettings(settings: ControlSettings) {
    var keep by remember { mutableStateOf(settings.keepAlive) }
    PojarushCard {
        CardTitle(stringResource(R.string.extras_title))
        SettingSwitch(stringResource(R.string.keep_alive), keep, stringResource(R.string.keep_alive_desc)) {
            settings.keepAlive = it
            keep = it
        }
    }
}

@Composable
fun WallpaperSettings(settings: ControlSettings) {
    val context = LocalContext.current
    var path by remember { mutableStateOf(settings.wallpaper) }
    val failed = stringResource(R.string.controls_import_failed, "%s")
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        uri ?: return@rememberLauncherForActivityResult
        runCatching { ControlLayouts.importWallpaper(context, uri) }
            .onSuccess {
                settings.wallpaper = it.absolutePath
                path = it.absolutePath
                WallpaperState.version++
            }
            .onFailure { Toast.makeText(context, failed.replace("%s", it.message.orEmpty()), Toast.LENGTH_LONG).show() }
    }
    PojarushCard {
        CardTitle(stringResource(R.string.wallpaper_title))
        Hint(if (path.isEmpty()) stringResource(R.string.wallpaper_none) else stringResource(R.string.mouse_cursor_current, File(path).name))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = { picker.launch(arrayOf("image/*", "video/*")) }) { Text(stringResource(R.string.wallpaper_choose)) }
            if (path.isNotEmpty()) {
                OutlinedButton(onClick = {
                    File(path).delete()
                    settings.wallpaper = ""
                    path = ""
                    WallpaperState.version++
                }) { Text(stringResource(R.string.wallpaper_remove)) }
            }
        }
        if (path.isNotEmpty()) {
            SettingSlider(stringResource(R.string.wallpaper_dim), settings.wallpaperDim, 0f..0.90f, 0.05f, { "${Math.round(it * 100)}%" }) {
                settings.wallpaperDim = it
                WallpaperState.version++
            }
        }
        if (path.isNotEmpty() && ControlLayouts.isVideo(File(path).name)) {
            SettingSlider(stringResource(R.string.wallpaper_volume), settings.wallpaperVolume, 0f..1f, 0.05f, { "${Math.round(it * 100)}%" }) {
                settings.wallpaperVolume = it
                WallpaperState.version++
            }
        }
        Hint(stringResource(R.string.wallpaper_hint))
    }
}
