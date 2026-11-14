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

val Purple = Color(0xFF57197F)

/// The Horizon site/launcher's background: near-black with a violet cast, not the
/// flatter dark-purple the app used before.
val BackgroundTop = Color(0xFF0B0712)
val BackgroundBottom = Color(0xFF150A22)
val CardBorder = Color(0xFF2E1E42)

val PojarushColors = darkColorScheme(
    primary = Color(0xFFB98CF2),
    onPrimary = Color(0xFF1A0A2A),
    primaryContainer = Purple,
    onPrimaryContainer = Color(0xFFF3E4FF),
    secondary = Color(0xFFFFC857),
    secondaryContainer = Color(0xFF3B2050),
    onSecondaryContainer = Color(0xFFF3E4FF),
    background = BackgroundTop,
    surface = Color(0xFF160D22),
    surfaceVariant = Color(0xFF231433),
    onSurface = Color(0xFFEDE0F5),
    onSurfaceVariant = Color(0xFFA996BC),
    error = Color(0xFFFF8A80),
)

object UiPrefs {
    var menuBackground by androidx.compose.runtime.mutableFloatStateOf(0.60f)
    var minecraftFont by androidx.compose.runtime.mutableStateOf(true)
    var customFont by androidx.compose.runtime.mutableStateOf("")
    var fontMode by androidx.compose.runtime.mutableStateOf("minecraft")

    fun load(context: android.content.Context) {
        val s = com.wanderstest3.pojarush.controls.ControlSettings(context)
        menuBackground = s.menuBackground
        minecraftFont = s.minecraftFont
        customFont = s.customFont
        fontMode = s.fontMode
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
    MaterialTheme(colorScheme = PojarushColors, typography = typography) {
        androidx.compose.runtime.CompositionLocalProvider(androidx.compose.material3.LocalContentColor provides PojarushColors.onSurface) {
            androidx.compose.material3.ProvideTextStyle(androidx.compose.ui.text.TextStyle(fontFamily = family), content = content)
        }
    }
}

/// The app's full-screen backdrop: a subtle top-to-bottom gradient instead of a flat
/// fill, matching the Horizon site/launcher's depth.
fun pojarushBackgroundBrush(): androidx.compose.ui.graphics.Brush =
    androidx.compose.ui.graphics.Brush.verticalGradient(listOf(BackgroundTop, BackgroundBottom))

fun Modifier.liquidGlass(shape: androidx.compose.ui.graphics.Shape = RoundedCornerShape(22.dp), tint: Color = Color.Black, selected: Boolean = false): Modifier {
    val strength = UiPrefs.menuBackground
    val base = if (selected) Purple else tint
    val body = androidx.compose.ui.graphics.Brush.linearGradient(
        listOf(
            base.copy(alpha = (strength * 0.78f).coerceIn(0f, 1f)),
            base.copy(alpha = (strength * 0.52f).coerceIn(0f, 1f)),
        ),
        start = androidx.compose.ui.geometry.Offset(0f, 0f),
        end = androidx.compose.ui.geometry.Offset(0f, Float.POSITIVE_INFINITY),
    )
    val sheen = androidx.compose.ui.graphics.Brush.verticalGradient(
        0f to Color.White.copy(alpha = 0.16f),
        0.45f to Color.White.copy(alpha = 0.04f),
        1f to Color.Transparent,
    )
    val rim = androidx.compose.ui.graphics.Brush.linearGradient(
        0f to Color.White.copy(alpha = 0.60f),
        0.35f to Color.White.copy(alpha = 0.14f),
        0.7f to Color.White.copy(alpha = 0.08f),
        1f to Color.White.copy(alpha = 0.34f),
    )
    return this
        .glassShadow(shape)
        .clip(shape)
        .background(body)
        .background(sheen)
        .border(1.dp, rim, shape)
}

private fun Modifier.glassShadow(shape: androidx.compose.ui.graphics.Shape): Modifier =
    this.shadow(10.dp, shape, clip = false, ambientColor = Color.Black.copy(alpha = 0.5f), spotColor = Color.Black.copy(alpha = 0.5f))

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
    (((range.endInclusive - range.start) / step).toInt() - 1).coerceAtLeast(0)

@Composable
fun SettingSlider(
    label: String,
    value: Float,
    range: ClosedFloatingPointRange<Float>,
    step: Float,
    display: (Float) -> String,
    onDone: (Float) -> Unit,
) {
    var current by remember(value) { mutableFloatStateOf(value) }
    Column {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(label, modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
            Text(display(current), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.primary)
        }
        Slider(
            value = current,
            onValueChange = { current = it },
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
