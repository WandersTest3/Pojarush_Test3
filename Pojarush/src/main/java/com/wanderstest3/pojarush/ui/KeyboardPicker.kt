package com.wanderstest3.pojarush.ui

import android.content.Context
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.wanderstest3.pojarush.R
import com.wanderstest3.pojarush.controls.Action
import com.wanderstest3.pojarush.controls.ActionCatalog

/// One key or special button on the visual keyboard, with its relative
/// width (1f = a normal key). `null` leaves an empty gap.
private data class Key(val label: String, val action: Action?, val weight: Float = 1f)

private fun row(vararg keys: Key) = keys.toList()

private fun keyboardRows(context: Context): List<List<Key>> {
    val k = ActionCatalog.all(context).associate { (name, action) -> name to action }
    fun k(name: String, weight: Float = 1f) = Key(name, k[name], weight)
    return listOf(
        row(
            k("Esc"), Key("", null, 0.5f),
            k("F1"), k("F2"), k("F3"), k("F4"), Key("", null, 0.4f),
            k("F5"), k("F6"), k("F7"), k("F8"), Key("", null, 0.4f),
            k("F9"), k("F10"), k("F11"), k("F12"),
        ),
        row(
            k("`"), k("1"), k("2"), k("3"), k("4"), k("5"), k("6"), k("7"), k("8"), k("9"), k("0"), k("-"), k("="), k("Backspace", 2f),
        ),
        row(
            k("Tab", 1.5f), k("Q"), k("W"), k("E"), k("R"), k("T"), k("Y"), k("U"), k("I"), k("O"), k("P"), k("["), k("]"),
        ),
        row(
            k("Caps Lock", 1.8f), k("A"), k("S"), k("D"), k("F"), k("G"), k("H"), k("J"), k("K"), k("L"), k(";"), k("'"), k("Enter", 1.8f),
        ),
        row(
            k("Left Shift", 2.2f), k("Z"), k("X"), k("C"), k("V"), k("B"), k("N"), k("M"), k(","), k("."), k("/"), k("Right Shift", 2.2f),
        ),
        row(
            k("Left Ctrl", 1.3f), k("Left Alt", 1.3f), k("Space", 5f), k("Right Alt", 1.3f), k("Right Ctrl", 1.3f),
        ),
        row(
            k("Insert"), k("Home"), k("Page Up"), k("Delete"), k("End"), k("Page Down"),
        ),
        row(
            Key("", null, 3f), k("↑"), Key("", null, 3f),
        ),
        row(
            k("←"), k("↓"), k("→"),
        ),
    )
}

private fun specialRow(context: Context): List<Pair<String, Action>> =
    ActionCatalog.all(context).filter { (_, action) -> action !is Action.Key }

/// A visual keyboard, laid out like a real one (row widths match a physical
/// board), used both by the control editor's action picker and by the
/// floating menu's "Send key" button. `disabled` greys out and disables keys
/// already used on the button being edited (empty in the floating menu).
@OptIn(androidx.compose.foundation.layout.ExperimentalLayoutApi::class)
@Composable
fun VisualKeyboard(
    disabled: Set<Action> = emptySet(),
    showSpecials: Boolean = true,
    onPick: (Action) -> Unit,
) {
    val context = androidx.compose.ui.platform.LocalContext.current
    Column(
        modifier = Modifier.verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        if (showSpecials) {
            Text(stringResource(R.string.editor_actions_special), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.primary)
            androidx.compose.foundation.layout.FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                specialRow(context).forEach { (label, action) ->
                    val used = action in disabled
                    androidx.compose.material3.AssistChip(
                        onClick = { if (!used) onPick(action) },
                        label = { Text(label) },
                        enabled = !used,
                    )
                }
            }
            Text(stringResource(R.string.editor_actions_keys), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.primary)
        }
        keyboardRows(context).forEach { keys ->
            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(3.dp)) {
                keys.forEach { key ->
                    if (key.action == null) {
                        Row(modifier = Modifier.weight(key.weight)) {}
                    } else {
                        val used = key.action in disabled
                        Row(
                            modifier = Modifier
                                .weight(key.weight)
                                .height(38.dp)
                                .clip(RoundedCornerShape(6.dp))
                                .background(if (used) MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.4f) else MaterialTheme.colorScheme.surfaceVariant)
                                .clickable(enabled = !used) { onPick(key.action) },
                            horizontalArrangement = Arrangement.Center,
                        ) {
                            Text(
                                key.label,
                                modifier = Modifier.padding(vertical = 10.dp),
                                style = MaterialTheme.typography.labelSmall,
                                textAlign = TextAlign.Center,
                                maxLines = 1,
                            )
                        }
                    }
                }
            }
        }
    }
}
