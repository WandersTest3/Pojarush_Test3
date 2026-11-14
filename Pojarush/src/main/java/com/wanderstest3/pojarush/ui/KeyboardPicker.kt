package com.wanderstest3.pojarush.ui

import android.content.Context
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shadow
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.wanderstest3.pojarush.R
import com.wanderstest3.pojarush.controls.Action
import com.wanderstest3.pojarush.controls.ActionCatalog

private data class Key(val label: String, val action: Action?, val weight: Float = 1f, val blocked: Boolean = false)

private val Outline = Color(0xFF000000)
private val Light = Color(0xFFB1B2B5)
private val Face = Color(0xFF6F6F6F)
private val Dark = Color(0xFF3C3C3C)
private val FaceUsed = Color(0xFF2B2B2B)
private val FacePressed = Color(0xFF5A5A5A)
private val Panel = Color(0xFF1E1E1E)

private val short = mapOf(
    "Caps Lock" to "CapsLk", "Left Shift" to "Shift", "Right Shift" to "Shift", "Left Ctrl" to "Ctrl", "Right Ctrl" to "Ctrl",
    "Left Alt" to "Alt", "Right Alt" to "Alt", "Page Up" to "PgUp", "Page Down" to "PgDn", "Insert" to "Ins", "Home" to "Hom",
    "Delete" to "Del", "Backspace" to "Bksp", "Num Lock" to "Num", "Scroll Lock" to "Scr", "Pause" to "Pau",
    "Num /" to "/", "Num *" to "*", "Num -" to "-", "Num +" to "+", "Num ." to ".", "Num Enter" to "Ent",
    "Num 0" to "0", "Num 1" to "1", "Num 2" to "2", "Num 3" to "3", "Num 4" to "4", "Num 5" to "5", "Num 6" to "6", "Num 7" to "7", "Num 8" to "8", "Num 9" to "9",
)

private class Layout(val main: List<List<Key>>, val nav: List<List<Key>>, val pad: List<List<Key>>)

private fun layout(context: Context): Layout {
    val all = ActionCatalog.all(context).associate { (name, action) -> name to action }
    fun k(name: String, weight: Float = 1f, label: String? = null) = Key(label ?: short[name] ?: name, all[name], weight)
    fun gap(weight: Float) = Key("", null, weight)
    fun off(label: String, weight: Float = 1f) = Key(label, null, weight, true)
    val main = listOf(
        listOf(k("Esc"), gap(1f), k("F1"), k("F2"), k("F3"), k("F4"), gap(0.5f), k("F5"), k("F6"), k("F7"), k("F8"), gap(0.5f), k("F9"), k("F10"), k("F11"), k("F12")),
        listOf(k("`"), k("1"), k("2"), k("3"), k("4"), k("5"), k("6"), k("7"), k("8"), k("9"), k("0"), k("-"), k("="), k("Backspace", 2f)),
        listOf(k("Tab", 1.5f), k("Q"), k("W"), k("E"), k("R"), k("T"), k("Y"), k("U"), k("I"), k("O"), k("P"), k("["), k("]"), k("\\", 1.5f)),
        listOf(k("Caps Lock", 1.75f), k("A"), k("S"), k("D"), k("F"), k("G"), k("H"), k("J"), k("K"), k("L"), k(";"), k("'"), k("Enter", 2.25f)),
        listOf(k("Left Shift", 2.25f), k("Z"), k("X"), k("C"), k("V"), k("B"), k("N"), k("M"), k(","), k("."), k("/"), k("Right Shift", 2.75f)),
        listOf(k("Left Ctrl", 1.25f), off("Win", 1.25f), k("Left Alt", 1.25f), k("Space", 6.25f), k("Right Alt", 1.25f), off("Win", 1.25f), off("Menu", 1.25f), k("Right Ctrl", 1.25f)),
    )
    val nav = listOf(
        listOf(off("Prt"), k("Scroll Lock"), k("Pause")),
        listOf(k("Insert"), k("Home"), k("Page Up")),
        listOf(k("Delete"), k("End"), k("Page Down")),
        listOf(gap(3f)),
        listOf(gap(1f), k("↑"), gap(1f)),
        listOf(k("←"), k("↓"), k("→")),
    )
    val pad = listOf(
        listOf(gap(4f)),
        listOf(k("Num Lock"), k("Num /"), k("Num *"), k("Num -")),
        listOf(k("Num 7"), k("Num 8"), k("Num 9"), k("Num +")),
        listOf(k("Num 4"), k("Num 5"), k("Num 6"), gap(1f)),
        listOf(k("Num 1"), k("Num 2"), k("Num 3"), k("Num Enter")),
        listOf(k("Num 0", 2f), k("Num ."), gap(1f)),
    )
    return Layout(main, nav, pad)
}

@Composable
private fun PixelKey(label: String, enabled: Boolean, blocked: Boolean, modifier: Modifier, onClick: () -> Unit) {
    val source = remember { MutableInteractionSource() }
    val pressed = source.collectIsPressedAsState().value
    val face = when {
        blocked || !enabled -> FaceUsed
        pressed -> FacePressed
        else -> Face
    }
    val top = if (pressed) Dark else Light
    val bottom = if (pressed) Light else Dark
    val live = enabled && !blocked
    Box(
        modifier = modifier
            .height(30.dp)
            .drawBehind {
                val px = 2.dp.toPx()
                drawRect(Outline)
                drawRect(face, Offset(px, px), Size(size.width - px * 2, size.height - px * 2))
                if (live) {
                    drawRect(top, Offset(px, px), Size(size.width - px * 2, px))
                    drawRect(top, Offset(px, px), Size(px, size.height - px * 2))
                    drawRect(bottom, Offset(px, size.height - px * 2), Size(size.width - px * 2, px))
                    drawRect(bottom, Offset(size.width - px * 2, px), Size(px, size.height - px * 2))
                }
            }
            .then(if (live) Modifier.clickable(interactionSource = source, indication = null, onClick = onClick) else Modifier),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            label,
            modifier = Modifier.padding(horizontal = 1.dp),
            style = TextStyle(
                color = if (live) Color.White else Color(0xFF777777),
                fontSize = 10.sp,
                textAlign = TextAlign.Center,
                shadow = if (live) Shadow(Color(0xFF1B1B1B), Offset(2f, 2f), 0f) else null,
            ),
            maxLines = 1,
            overflow = TextOverflow.Clip,
        )
    }
}

@Composable
private fun KeyRow(keys: List<Key>, disabled: Set<Action>, modifier: Modifier = Modifier, onPick: (Action) -> Unit) {
    Row(modifier = modifier, horizontalArrangement = Arrangement.spacedBy(2.dp)) {
        keys.forEach { key ->
            if (key.action == null && !key.blocked) {
                Box(modifier = Modifier.weight(key.weight))
            } else {
                PixelKey(
                    label = key.label,
                    enabled = key.action != null && key.action !in disabled,
                    blocked = key.blocked,
                    modifier = Modifier.weight(key.weight),
                ) { key.action?.let(onPick) }
            }
        }
    }
}

@Composable
private fun SideTitle(text: String) {
    Text(text, style = MaterialTheme.typography.labelMedium, color = Color(0xFFFFFF55), modifier = Modifier.padding(bottom = 2.dp))
}

/// The key picker: a full pixel-art keyboard (main block, navigation keys and numpad), with the
/// mouse and launcher actions in a column beside it in the same style. Used by the control
/// editor's action picker and by the floating menu's "Send key" button. `disabled` greys out the
/// actions already used on the button being edited.
@Composable
fun VisualKeyboard(
    disabled: Set<Action> = emptySet(),
    showSpecials: Boolean = true,
    onPick: (Action) -> Unit,
) {
    val context = androidx.compose.ui.platform.LocalContext.current
    val board = remember { layout(context) }
    val specials = remember { ActionCatalog.all(context).filter { (_, action) -> action !is Action.Key } }
    Row(
        modifier = Modifier.fillMaxWidth().background(Panel).padding(6.dp).verticalScroll(rememberScrollState()),
        horizontalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            for (row in board.main.indices) {
                val spacing = if (row == 1) 4.dp else 0.dp
                Row(modifier = Modifier.fillMaxWidth().padding(top = spacing), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    KeyRow(board.main[row], disabled, Modifier.weight(15f), onPick)
                    KeyRow(board.nav[row], disabled, Modifier.weight(3f), onPick)
                    KeyRow(board.pad[row], disabled, Modifier.weight(4f), onPick)
                }
            }
        }
        if (showSpecials) {
            Column(modifier = Modifier.width(92.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                SideTitle(stringResource(R.string.editor_actions_special))
                specials.forEach { (label, action) ->
                    PixelKey(label, enabled = action !in disabled, blocked = false, modifier = Modifier.fillMaxWidth()) { onPick(action) }
                }
            }
        }
    }
}
