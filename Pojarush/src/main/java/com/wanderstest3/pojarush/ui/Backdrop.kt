package com.wanderstest3.pojarush.ui

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.media.MediaMetadataRetriever
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.graphics.FilterQuality
import androidx.compose.ui.graphics.asComposeRenderEffect
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.layer.drawLayer
import androidx.compose.ui.unit.dp
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import com.wanderstest3.pojarush.controls.ControlLayouts
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File
import kotlin.math.max
import kotlin.math.roundToInt

val GpuBlur: Boolean = android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.S

class Backdrop {
    var image by mutableStateOf<ImageBitmap?>(null)
    var blur by mutableStateOf(0f)
    var dim by mutableStateOf(0f)
    var origin by mutableStateOf(Offset.Zero)
    var size by mutableStateOf(IntSize.Zero)
}

val LocalBackdrop = staticCompositionLocalOf<Backdrop?> { null }

@Composable
fun WithBackdrop(content: @Composable () -> Unit) {
    val backdrop = remember { Backdrop() }
    CompositionLocalProvider(LocalBackdrop provides backdrop, content = content)
}

@Composable
fun BackdropSource(file: File?, stamp: Long, dim: Float, blur: Float) {
    val backdrop = LocalBackdrop.current ?: return
    backdrop.dim = dim
    backdrop.blur = blur
    LaunchedEffect(file?.absolutePath, stamp, blur >= 1f, if (GpuBlur) 0f else blur) {
        backdrop.image = if (file == null || blur < 1f) null else withContext(Dispatchers.IO) {
            runCatching { (if (GpuBlur) sharp(file) else blurred(file, blur))?.asImageBitmap() }.getOrNull()
        }
    }
}

fun Modifier.backdropArea(): Modifier = composed {
    val backdrop = LocalBackdrop.current
    if (backdrop == null) return@composed this
    this.onGloballyPositioned {
        backdrop.origin = it.positionInWindow()
        backdrop.size = IntSize(it.size.width, it.size.height)
    }
}

fun Modifier.frostedBackdrop(): Modifier = composed {
    val backdrop = LocalBackdrop.current
    val image = backdrop?.image
    if (backdrop == null || image == null) return@composed this
    var at by remember { mutableStateOf(Offset.Zero) }
    this
        .onGloballyPositioned { at = it.positionInWindow() }
        .drawWithCache {
            val layer = if (GpuBlur) obtainGraphicsLayer() else null
            onDrawBehind {
                val root = backdrop.size
                if (root.width <= 0 || root.height <= 0) return@onDrawBehind
                val scale = max(root.width.toFloat() / image.width, root.height.toFloat() / image.height)
                val w = (image.width * scale).roundToInt()
                val h = (image.height * scale).roundToInt()
                val left = (root.width - w) / 2f + backdrop.origin.x - at.x
                val top = (root.height - h) / 2f + backdrop.origin.y - at.y
                val paint: DrawScope.() -> Unit = {
                    drawImage(
                        image = image,
                        dstOffset = IntOffset(left.roundToInt(), top.roundToInt()),
                        dstSize = IntSize(w, h),
                        filterQuality = FilterQuality.High,
                    )
                }
                if (layer != null) {
                    layer.renderEffect = glassEffect(backdrop.blur * 4.dp.toPx())
                    layer.record { paint() }
                    drawLayer(layer)
                } else {
                    paint()
                }
                if (backdrop.dim > 0f) drawRect(Color.Black.copy(alpha = backdrop.dim))
            }
        }
}

private val glassEffects = HashMap<Int, androidx.compose.ui.graphics.RenderEffect>()

private fun glassEffect(radius: Float): androidx.compose.ui.graphics.RenderEffect? {
    if (!GpuBlur || radius < 0.5f) return null
    val key = radius.roundToInt()
    return glassEffects.getOrPut(key) {
        val r = key.toFloat().coerceAtLeast(1f)
        val blur = android.graphics.RenderEffect.createBlurEffect(r, r, android.graphics.Shader.TileMode.CLAMP)
        val vivid = android.graphics.ColorMatrix().apply { setSaturation(1.35f) }
        android.graphics.RenderEffect.createChainEffect(
            android.graphics.RenderEffect.createColorFilterEffect(android.graphics.ColorMatrixColorFilter(vivid)),
            blur,
        ).asComposeRenderEffect()
    }
}

private fun sharp(file: File): Bitmap? {
    val decoded = frame(file) ?: return null
    val source = if (decoded.config == Bitmap.Config.HARDWARE || decoded.config == null) decoded.copy(Bitmap.Config.ARGB_8888, false) ?: return null else decoded
    val ratio = 480f / max(source.width, source.height)
    if (ratio >= 1f) return source
    return Bitmap.createScaledBitmap(source, max(1, (source.width * ratio).roundToInt()), max(1, (source.height * ratio).roundToInt()), true)
        .also { if (it !== source) source.recycle() }
}

private fun frame(file: File): Bitmap? {
    if (ControlLayouts.isVideo(file.name)) {
        val r = MediaMetadataRetriever()
        return try {
            r.setDataSource(file.absolutePath)
            val us = (r.extractMetadata(MediaMetadataRetriever.METADATA_KEY_DURATION)?.toLongOrNull() ?: 0L) * 1000L
            var fallback: Bitmap? = null
            for (f in listOf(0.5, 0.25, 0.75, 0.1)) {
                val b = r.getFrameAtTime((us * f).toLong(), MediaMetadataRetriever.OPTION_CLOSEST_SYNC) ?: continue
                if (brightness(b) > 0.06f) return b
                if (fallback == null) fallback = b
            }
            fallback
        } finally {
            r.release()
        }
    }
    val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
    BitmapFactory.decodeFile(file.absolutePath, bounds)
    var sample = 1
    while (max(bounds.outWidth, bounds.outHeight) / (sample * 2) >= 480) sample *= 2
    return BitmapFactory.decodeFile(file.absolutePath, BitmapFactory.Options().apply { inSampleSize = sample })
}

private fun brightness(b: Bitmap): Float {
    val s = Bitmap.createScaledBitmap(b, 16, 16, true)
    var sum = 0L
    for (y in 0 until 16) for (x in 0 until 16) {
        val c = s.getPixel(x, y)
        sum += ((c shr 16) and 0xFF) + ((c shr 8) and 0xFF) + (c and 0xFF)
    }
    return sum / (256f * 3f * 255f)
}

private fun blurred(file: File, strength: Float): Bitmap? {
    val decoded = frame(file) ?: return null
    val source = if (decoded.config == Bitmap.Config.HARDWARE || decoded.config == null) decoded.copy(Bitmap.Config.ARGB_8888, false) ?: return null else decoded
    val target = 160
    val ratio = target.toFloat() / max(source.width, source.height)
    val w = max(1, (source.width * ratio).roundToInt())
    val h = max(1, (source.height * ratio).roundToInt())
    val small = Bitmap.createScaledBitmap(source, w, h, true)
    if (small !== source) source.recycle()
    val radius = max(1, (strength * 0.6f).roundToInt())
    val px = IntArray(w * h)
    small.getPixels(px, 0, w, 0, 0, w, h)
    repeat(3) {
        boxBlur(px, w, h, radius, true)
        boxBlur(px, w, h, radius, false)
    }
    val out = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
    out.setPixels(px, 0, w, 0, 0, w, h)
    small.recycle()
    return out
}

class LiveBlur(private val strength: Float) {
    private val outs = arrayOfNulls<Bitmap>(2)
    private var flip = 0
    private var px = IntArray(0)

    fun blur(src: Bitmap): Bitmap {
        val w = src.width
        val h = src.height
        if (px.size != w * h) px = IntArray(w * h)
        src.getPixels(px, 0, w, 0, 0, w, h)
        val radius = max(1, (strength * 0.6f).roundToInt())
        repeat(2) {
            boxBlur(px, w, h, radius, true)
            boxBlur(px, w, h, radius, false)
        }
        flip = 1 - flip
        val out = outs[flip]?.takeIf { it.width == w && it.height == h } ?: Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888).also { outs[flip] = it }
        out.setPixels(px, 0, w, 0, 0, w, h)
        return out
    }
}

private fun boxBlur(px: IntArray, w: Int, h: Int, r: Int, horizontal: Boolean) {
    val lines = if (horizontal) h else w
    val len = if (horizontal) w else h
    val buf = IntArray(len)
    for (line in 0 until lines) {
        for (i in 0 until len) buf[i] = px[if (horizontal) line * w + i else i * w + line]
        var rs = 0
        var gs = 0
        var bs = 0
        val span = r * 2 + 1
        for (k in -r..r) {
            val c = buf[k.coerceIn(0, len - 1)]
            rs += (c shr 16) and 0xFF
            gs += (c shr 8) and 0xFF
            bs += c and 0xFF
        }
        for (i in 0 until len) {
            val idx = if (horizontal) line * w + i else i * w + line
            px[idx] = (0xFF shl 24) or ((rs / span) shl 16) or ((gs / span) shl 8) or (bs / span)
            val add = buf[(i + r + 1).coerceIn(0, len - 1)]
            val sub = buf[(i - r).coerceIn(0, len - 1)]
            rs += ((add shr 16) and 0xFF) - ((sub shr 16) and 0xFF)
            gs += ((add shr 8) and 0xFF) - ((sub shr 8) and 0xFF)
            bs += (add and 0xFF) - (sub and 0xFF)
        }
    }
}
