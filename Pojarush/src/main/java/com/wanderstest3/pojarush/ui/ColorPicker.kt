package com.wanderstest3.pojarush.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.wanderstest3.pojarush.R
import com.wanderstest3.pojarush.controls.ControlSettings

private fun hsv(color: Color): FloatArray = FloatArray(3).also { android.graphics.Color.colorToHSV(color.toArgb(), it) }

private fun fromHsv(h: Float, s: Float, v: Float) = Color(android.graphics.Color.HSVToColor(floatArrayOf(h, s, v)))

private fun hex(color: Color) = String.format("%06X", color.toArgb() and 0xFFFFFF)

@Composable
fun ColorPanel(initial: Color, onChange: (Color) -> Unit) {
    val start = remember { hsv(initial) }
    var hue by remember { mutableFloatStateOf(start[0]) }
    var sat by remember { mutableFloatStateOf(start[1]) }
    var value by remember { mutableFloatStateOf(start[2]) }
    var text by remember { mutableStateOf(hex(initial)) }
    fun emit(updateText: Boolean = true) {
        val c = fromHsv(hue, sat, value)
        if (updateText) text = hex(c)
        onChange(c)
    }
    fun set(c: Color, updateText: Boolean = true) {
        val v = hsv(c)
        hue = v[0]; sat = v[1]; value = v[2]
        emit(updateText)
    }
    val square = Modifier.fillMaxWidth().height(170.dp).clip(RoundedCornerShape(12.dp))
    Canvas(
        modifier = square
            .pointerInput(Unit) {
                fun pick(o: Offset) {
                    sat = (o.x / size.width).coerceIn(0f, 1f)
                    value = 1f - (o.y / size.height).coerceIn(0f, 1f)
                    emit()
                }
                detectTapGestures { pick(it) }
            }
            .pointerInput(Unit) {
                detectDragGestures { change, _ ->
                    sat = (change.position.x / size.width).coerceIn(0f, 1f)
                    value = 1f - (change.position.y / size.height).coerceIn(0f, 1f)
                    emit()
                }
            }
    ) {
        drawRect(Brush.horizontalGradient(listOf(Color.White, fromHsv(hue, 1f, 1f))))
        drawRect(Brush.verticalGradient(listOf(Color.Transparent, Color.Black)))
        val p = Offset(sat * size.width, (1f - value) * size.height)
        drawCircle(Color.Black, 11.dp.toPx(), p, style = Stroke(3.dp.toPx()))
        drawCircle(Color.White, 9.dp.toPx(), p, style = Stroke(2.dp.toPx()))
    }
    val hues = remember { (0..6).map { fromHsv(it * 60f % 360f, 1f, 1f) }.let { it.dropLast(1) + Color.Red } }
    Canvas(
        modifier = Modifier.fillMaxWidth().height(28.dp).clip(RoundedCornerShape(14.dp))
            .pointerInput(Unit) {
                detectTapGestures { hue = (it.x / size.width).coerceIn(0f, 1f) * 359.9f; emit() }
            }
            .pointerInput(Unit) {
                detectDragGestures { change, _ -> hue = (change.position.x / size.width).coerceIn(0f, 1f) * 359.9f; emit() }
            }
    ) {
        drawRect(Brush.horizontalGradient(hues))
        val x = hue / 359.9f * size.width
        drawCircle(Color.White, size.height / 2.4f, Offset(x, size.height / 2), style = Stroke(3.dp.toPx()))
    }
    val current = fromHsv(hue, sat, value)
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Box(Modifier.size(44.dp).clip(RoundedCornerShape(10.dp)).background(current).border(1.dp, Color.White.copy(alpha = 0.4f), RoundedCornerShape(10.dp)))
        OutlinedTextField(
            value = text,
            onValueChange = { t ->
                text = t.uppercase().filter { it in "0123456789ABCDEF" }.take(6)
                if (text.length == 6) set(Color(0xFF000000 or text.toLong(16)), updateText = false)
            },
            singleLine = true,
            label = { Text("HEX") },
            prefix = { Text("#") },
            modifier = Modifier.width(150.dp),
        )
        Text("R ${(current.red * 255).toInt()}  G ${(current.green * 255).toInt()}  B ${(current.blue * 255).toInt()}")
    }
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        listOf(0xFF57197F, 0xFF1565C0, 0xFF00897B, 0xFF2E7D32, 0xFFF9A825, 0xFFE65100, 0xFFC62828, 0xFFAD1457, 0xFF424242).forEach { c ->
            Box(Modifier.size(30.dp).clip(CircleShape).background(Color(c)).border(1.dp, Color.White.copy(alpha = 0.35f), CircleShape).clickable { set(Color(c)) })
        }
    }
}

@Composable
fun AccentSettings(settings: ControlSettings) {
    var version by remember { mutableFloatStateOf(0f) }
    PojarushCard {
        CardTitle(stringResource(R.string.accent_title))
        Hint(stringResource(R.string.accent_desc))
        androidx.compose.runtime.key(version) {
            ColorPanel(UiPrefs.accent) { c ->
                UiPrefs.accent = c
                settings.accentColor = c.toArgb()
            }
        }
        OutlinedButton(onClick = {
            UiPrefs.accent = DefaultAccent
            settings.accentColor = DefaultAccent.toArgb()
            version++
        }) { Text(stringResource(R.string.accent_reset)) }
    }
}
