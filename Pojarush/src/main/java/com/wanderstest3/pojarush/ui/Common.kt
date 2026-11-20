package com.wanderstest3.pojarush.ui

import android.content.Context
import android.os.Build
import android.view.WindowManager
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.clickable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import java.util.Locale

val DefaultAccent = Color(0xFF57197F)

val Purple: Color get() = UiPrefs.accent

private fun mix(a: Color, b: Color, t: Float) = Color(
    a.red + (b.red - a.red) * t,
    a.green + (b.green - a.green) * t,
    a.blue + (b.blue - a.blue) * t,
    1f,
)

val BackgroundTop: Color get() = mix(Color(0xFF07070A), UiPrefs.accent, 0.06f)
val BackgroundBottom: Color get() = mix(Color(0xFF0B0B10), UiPrefs.accent, 0.14f)
val CardBorder: Color get() = mix(Color(0xFF1C1C22), UiPrefs.accent, 0.35f)

fun pojarushColors(accent: Color) = darkColorScheme(
    primary = mix(accent, Color.White, 0.45f),
    onPrimary = mix(accent, Color.Black, 0.80f),
    primaryContainer = accent,
    onPrimaryContainer = mix(accent, Color.White, 0.90f),
    secondary = Color(0xFFFFC857),
    secondaryContainer = mix(accent, Color(0xFF101014), 0.65f),
    onSecondaryContainer = mix(accent, Color.White, 0.90f),
    background = mix(Color(0xFF07070A), accent, 0.06f),
    surface = mix(Color(0xFF0E0E12), accent, 0.12f),
    surfaceVariant = mix(Color(0xFF16161B), accent, 0.18f),
    onSurface = mix(Color.White, accent, 0.06f),
    onSurfaceVariant = mix(Color(0xFFB0B0B8), accent, 0.15f),
    error = Color(0xFFFF8A80),
)

object UiPrefs {
    var accent by androidx.compose.runtime.mutableStateOf(DefaultAccent)
    var menuBackground by androidx.compose.runtime.mutableFloatStateOf(0.50f)
    var menuBlur by androidx.compose.runtime.mutableFloatStateOf(12f)
    var minecraftFont by androidx.compose.runtime.mutableStateOf(true)
    var customFont by androidx.compose.runtime.mutableStateOf("")
    var fontMode by androidx.compose.runtime.mutableStateOf("minecraft")
    var fontScale by androidx.compose.runtime.mutableStateOf(1.00f)

    fun load(context: android.content.Context) {
        val s = com.wanderstest3.pojarush.controls.ControlSettings(context)
        menuBackground = s.menuBackground
        menuBlur = s.menuBlur
        accent = Color(s.accentColor)
        minecraftFont = s.minecraftFont
        customFont = s.customFont
        fontMode = s.fontMode
        fontScale = s.fontScale
    }
}

val MinecraftFont = androidx.compose.ui.text.font.FontFamily(androidx.compose.ui.text.font.Font(com.wanderstest3.pojarush.R.font.minecraft))

private fun androidx.compose.material3.Typography.withFont(f: androidx.compose.ui.text.font.FontFamily) = androidx.compose.material3.Typography(
    displayLarge = displayLarge.copy(fontFamily = f), displayMedium = displayMedium.copy(fontFamily = f), displaySmall = displaySmall.copy(fontFamily = f),
    headlineLarge = headlineLarge.copy(fontFamily = f), headlineMedium = headlineMedium.copy(fontFamily = f), headlineSmall = headlineSmall.copy(fontFamily = f),
    titleLarge = titleLarge.copy(fontFamily = f), titleMedium = titleMedium.copy(fontFamily = f), titleSmall = titleSmall.copy(fontFamily = f),
    bodyLarge = bodyLarge.copy(fontFamily = f), bodyMedium = bodyMedium.copy(fontFamily = f), bodySmall = bodySmall.copy(fontFamily = f),
    labelLarge = labelLarge.copy(fontFamily = f), labelMedium = labelMedium.copy(fontFamily = f), labelSmall = labelSmall.copy(fontFamily = f),
)

fun launcherTypeface(context: Context): android.graphics.Typeface? {
    val settings = com.wanderstest3.pojarush.controls.ControlSettings(context)
    return when (settings.fontMode) {
        "custom" -> java.io.File(settings.customFont).takeIf { settings.customFont.isNotEmpty() && it.isFile }
            ?.let { runCatching { android.graphics.Typeface.Builder(it).build() }.getOrNull() }
        "minecraft" -> runCatching { androidx.core.content.res.ResourcesCompat.getFont(context, com.wanderstest3.pojarush.R.font.minecraft) }.getOrNull()
        else -> null
    }
}

@Composable
fun PojarushTheme(content: @Composable () -> Unit) {
    val custom = UiPrefs.customFont
    val mode = UiPrefs.fontMode
    val family = remember(custom, mode) {
        when (mode) {
            "custom" -> java.io.File(custom).takeIf { custom.isNotEmpty() && it.isFile && runCatching { android.graphics.Typeface.Builder(it).build() }.getOrNull() != null }
                ?.let { androidx.compose.ui.text.font.FontFamily(androidx.compose.ui.text.font.Font(it)) }
            "minecraft" -> MinecraftFont
            else -> null
        }
    }
    val typography = if (family != null) androidx.compose.material3.Typography().withFont(family) else androidx.compose.material3.Typography()
    val base = androidx.compose.ui.platform.LocalDensity.current
    val density = remember(base, UiPrefs.fontScale) {
        androidx.compose.ui.unit.Density(base.density, base.fontScale * UiPrefs.fontScale)
    }
    val colors = remember(UiPrefs.accent) { pojarushColors(UiPrefs.accent) }
    MaterialTheme(colorScheme = colors, typography = typography) {
        androidx.compose.runtime.CompositionLocalProvider(
            androidx.compose.material3.LocalContentColor provides colors.onSurface,
            androidx.compose.ui.platform.LocalDensity provides density,
        ) {
            androidx.compose.material3.ProvideTextStyle(androidx.compose.ui.text.TextStyle(fontFamily = family), content = content)
        }
    }
}

/// The app's full-screen backdrop: a subtle top-to-bottom gradient instead of a flat
/// fill, matching the Horizon site/launcher's depth.
fun pojarushBackgroundBrush(): androidx.compose.ui.graphics.Brush =
    androidx.compose.ui.graphics.Brush.verticalGradient(listOf(BackgroundTop, BackgroundBottom))

fun Modifier.liquidGlass(shape: androidx.compose.ui.graphics.Shape = RoundedCornerShape(22.dp), tint: Color = Color.Black, selected: Boolean = false): Modifier {
    val strength = UiPrefs.menuBackground.coerceIn(0f, 1f)
    val base = if (selected) Purple else tint
    val body = androidx.compose.ui.graphics.Brush.verticalGradient(
        listOf(base.copy(alpha = strength), base.copy(alpha = strength * 0.88f)),
    )
    val sheen = androidx.compose.ui.graphics.Brush.verticalGradient(
        0f to Color.White.copy(alpha = 0.16f * strength),
        0.45f to Color.White.copy(alpha = 0.04f * strength),
        1f to Color.Transparent,
    )
    val rim = androidx.compose.ui.graphics.Brush.linearGradient(
        0f to Color.White.copy(alpha = 0.60f * strength),
        0.35f to Color.White.copy(alpha = 0.14f * strength),
        0.7f to Color.White.copy(alpha = 0.08f * strength),
        1f to Color.White.copy(alpha = 0.34f * strength),
    )
    return this
        .glassShadow(shape, strength)
        .clip(shape)
        .background(body)
        .background(sheen)
        .border(1.dp, rim, shape)
}

private fun Modifier.glassShadow(shape: androidx.compose.ui.graphics.Shape, strength: Float): Modifier =
    if (strength <= 0.02f) this
    else this.shadow(
        (10 * strength).dp,
        shape,
        clip = false,
        ambientColor = Color.Black.copy(alpha = 0.5f),
        spotColor = Color.Black.copy(alpha = 0.5f),
    )

@Composable
fun PojarushCard(content: @Composable () -> Unit) {
    Column(
        modifier = Modifier.fillMaxWidth().liquidGlass(),
    ) {
        Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) { content() }
    }
}

@Composable
fun CardTitle(text: String) {
    Text(text, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
}

@Composable
fun Hint(text: String) {
    Text(text, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
}

fun fmt(v: Float): String = String.format(Locale.ROOT, "%.2f", v)

fun stepsFor(range: ClosedFloatingPointRange<Float>, step: Float): Int =
    (Math.round((range.endInclusive - range.start) / step) - 1).coerceAtLeast(0)

@Composable
fun ResetButton(enabled: Boolean, onClick: () -> Unit) {
    val color = MaterialTheme.colorScheme.primary.copy(alpha = if (enabled) 1f else 0.35f)
    androidx.compose.foundation.layout.Box(
        modifier = Modifier
            .padding(start = 6.dp)
            .size(26.dp)
            .clip(androidx.compose.foundation.shape.CircleShape)
            .clickable(enabled = enabled, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        androidx.compose.foundation.Canvas(modifier = Modifier.size(16.dp)) {
            val stroke = size.minDimension * 0.14f
            val r = size.minDimension / 2f - stroke
            val c = center
            drawArc(
                color = color,
                startAngle = -60f,
                sweepAngle = 300f,
                useCenter = false,
                topLeft = androidx.compose.ui.geometry.Offset(c.x - r, c.y - r),
                size = androidx.compose.ui.geometry.Size(r * 2f, r * 2f),
                style = androidx.compose.ui.graphics.drawscope.Stroke(width = stroke, cap = androidx.compose.ui.graphics.StrokeCap.Round),
            )
            val a = Math.toRadians(-60.0)
            val tip = androidx.compose.ui.geometry.Offset(c.x + r * Math.cos(a).toFloat(), c.y + r * Math.sin(a).toFloat())
            val head = androidx.compose.ui.graphics.Path().apply {
                moveTo(tip.x + stroke * 2.2f, tip.y - stroke * 0.4f)
                lineTo(tip.x - stroke * 0.6f, tip.y - stroke * 2.4f)
                lineTo(tip.x - stroke * 0.2f, tip.y + stroke * 1.6f)
                close()
            }
            drawPath(head, color)
        }
    }
}

@Composable
fun SettingSlider(
    label: String,
    value: Float,
    range: ClosedFloatingPointRange<Float>,
    step: Float,
    display: (Float) -> String,
    default: Float? = null,
    onDone: (Float) -> Unit,
) {
    var current by remember(value) { mutableFloatStateOf(value) }
    Column {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(label, modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
            Text(display(current), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.primary)
            if (default != null) {
                ResetButton(enabled = kotlin.math.abs(current - default) > step / 2f) {
                    current = default
                    onDone(default)
                }
            }
        }
        Slider(
            value = current,
            onValueChange = { v ->
                val snapped = range.start + Math.round((v - range.start) / step) * step
                current = Math.round(snapped * 10000f) / 10000f
            },
            onValueChangeFinished = { onDone(current) },
            valueRange = range,
            steps = stepsFor(range, step)
        )
    }
}

@Composable
fun NameDialog(title: String, initial: String, confirm: String, onDismiss: () -> Unit, onConfirm: (String) -> Unit) {
    var name by remember { mutableStateOf(initial) }
    androidx.compose.material3.AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            androidx.compose.material3.OutlinedTextField(
                value = name,
                onValueChange = { name = it.take(48) },
                singleLine = true,
                label = { Text(androidx.compose.ui.res.stringResource(com.wanderstest3.pojarush.R.string.layout_name)) },
            )
        },
        confirmButton = { androidx.compose.material3.TextButton(onClick = { onConfirm(name.trim()) }, enabled = name.isNotBlank()) { Text(confirm) } },
        dismissButton = { androidx.compose.material3.TextButton(onClick = onDismiss) { Text(androidx.compose.ui.res.stringResource(com.wanderstest3.pojarush.R.string.cancel)) } },
    )
}

@Composable
fun SettingNumber(
    label: String,
    value: Float,
    range: ClosedFloatingPointRange<Float>,
    step: Float,
    unit: String,
    onDone: (Float) -> Unit,
) {
    var current by remember(value) { mutableFloatStateOf(value) }
    var text by remember(value) { mutableStateOf(value.toInt().toString()) }
    Column {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(label, modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
            androidx.compose.material3.OutlinedTextField(
                value = text,
                onValueChange = { t ->
                    text = t.filter(Char::isDigit).take(4)
                    text.toIntOrNull()?.toFloat()?.takeIf { it in range }?.let {
                        current = it
                        onDone(it)
                    }
                },
                modifier = Modifier.width(96.dp),
                singleLine = true,
                suffix = { Text(unit) },
                keyboardOptions = androidx.compose.foundation.text.KeyboardOptions(keyboardType = androidx.compose.ui.text.input.KeyboardType.Number),
            )
        }
        Slider(
            value = current.coerceIn(range),
            onValueChange = {
                current = it
                text = it.toInt().toString()
            },
            onValueChangeFinished = { onDone(current) },
            valueRange = range,
            steps = stepsFor(range, step)
        )
    }
}

@Composable
fun SettingSwitch(label: String, value: Boolean, description: String? = null, onChange: (Boolean) -> Unit) {
    var current by remember(value) { mutableStateOf(value) }
    Row(verticalAlignment = Alignment.CenterVertically) {
        Column(modifier = Modifier.weight(1f)) {
            Text(label, style = MaterialTheme.typography.bodyMedium)
            description?.let { Hint(it) }
        }
        Switch(checked = current, onCheckedChange = { current = it; onChange(it) })
    }
}

fun screenSize(context: Context): Pair<Int, Int> {
    val wm = context.getSystemService(Context.WINDOW_SERVICE) as WindowManager
    val (w, h) = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
        wm.maximumWindowMetrics.bounds.let { it.width() to it.height() }
    } else {
        val m = android.util.DisplayMetrics()
        @Suppress("DEPRECATION")
        wm.defaultDisplay.getRealMetrics(m)
        m.widthPixels to m.heightPixels
    }
    return maxOf(w, h) to minOf(w, h)
}
