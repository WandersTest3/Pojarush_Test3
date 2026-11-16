package com.wanderstest3.pojarush.ui

import android.widget.Toast
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.basicMarquee
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Button
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.wanderstest3.pojarush.Nbt
import com.wanderstest3.pojarush.R
import com.wanderstest3.pojarush.horizon.HorizonManager
import java.io.File

private fun nbtFiles(): List<File> {
    val game = HorizonManager.gameDir
    val out = ArrayList<File>()
    game.listFiles { f -> f.isFile && f.extension == "dat" }?.sortedBy { it.name }?.let(out::addAll)
    File(game, "saves").listFiles { f -> f.isDirectory }?.sortedBy { it.name.lowercase() }?.forEach { world ->
        File(world, "level.dat").takeIf { it.isFile }?.let(out::add)
        for (sub in listOf("playerdata", "data")) {
            File(world, sub).listFiles { f -> f.isFile && f.extension == "dat" }?.sortedBy { it.name }?.let(out::addAll)
        }
    }
    return out
}

private class Step(val label: String, val node: Nbt)

@Composable
fun NbtPage() {
    val context = LocalContext.current
    var doc by remember { mutableStateOf<Nbt.Document?>(null) }
    val path = remember { mutableStateListOf<Step>() }
    var refresh by remember { mutableIntStateOf(0) }
    var editing by remember { mutableStateOf<Pair<String, Nbt>?>(null) }
    var dirty by remember { mutableStateOf(false) }
    val failed = stringResource(R.string.controls_import_failed, "%s")
    val saved = stringResource(R.string.nbt_saved)
    val current = doc
    PojarushCard {
        CardTitle(stringResource(R.string.nbt_title))
        if (current == null) {
            Hint(stringResource(R.string.nbt_desc))
            val files = remember(refresh) { nbtFiles() }
            if (files.isEmpty()) Hint(stringResource(R.string.nbt_empty))
            val game = HorizonManager.gameDir
            files.forEach { f ->
                Text(
                    f.relativeTo(game).path,
                    modifier = Modifier.fillMaxWidth().clickable {
                        runCatching { Nbt.Document.open(f) }
                            .onSuccess { doc = it; path.clear(); path.add(Step(f.name, it.root)); dirty = false }
                            .onFailure { Toast.makeText(context, failed.replace("%s", it.message.orEmpty()), Toast.LENGTH_LONG).show() }
                    }.padding(vertical = 8.dp),
                    style = MaterialTheme.typography.bodyMedium,
                )
            }
            return@PojarushCard
        }
        Text(path.joinToString(" / ") { it.label }, style = MaterialTheme.typography.labelLarge, modifier = Modifier.basicMarquee(), maxLines = 1)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedButton(onClick = {
                if (path.size > 1) path.removeAt(path.lastIndex) else { doc = null; refresh++ }
            }) { Text(stringResource(R.string.nbt_back)) }
            Button(enabled = dirty, onClick = {
                runCatching { current.save() }
                    .onSuccess { dirty = false; Toast.makeText(context, saved, Toast.LENGTH_SHORT).show() }
                    .onFailure { Toast.makeText(context, failed.replace("%s", it.message.orEmpty()), Toast.LENGTH_LONG).show() }
            }) { Text(stringResource(R.string.nbt_save)) }
        }
        val node = path.last().node
        val entries: List<Pair<String, Nbt>> = when (node) {
            is Nbt.Cmp -> node.v.entries.map { it.key to it.value }
            is Nbt.Lst -> node.v.mapIndexed { i, n -> "[$i]" to n }
            else -> emptyList()
        }
        androidx.compose.runtime.key(dirty, path.size) {
            Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                entries.take(400).forEach { (name, value) ->
                    Row(
                        modifier = Modifier.fillMaxWidth().clickable {
                            if (value is Nbt.Cmp || value is Nbt.Lst) path.add(Step(name, value)) else editing = name to value
                        }.padding(vertical = 6.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(Nbt.NAMES[value.type], modifier = Modifier.width(72.dp), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.primary)
                        Text(name, modifier = Modifier.weight(1f), fontWeight = FontWeight.Bold, maxLines = 1, style = MaterialTheme.typography.bodyMedium)
                        Text(
                            if (value is Nbt.Cmp || value is Nbt.Lst) "${value.display()} ›" else value.display().take(40),
                            modifier = Modifier.weight(1f),
                            maxLines = 1,
                            style = MaterialTheme.typography.bodySmall,
                        )
                    }
                }
                if (entries.size > 400) Hint("… ${entries.size - 400}")
            }
        }
    }
    editing?.let { (name, value) ->
        var text by remember(value) { mutableStateOf(value.display()) }
        var bad by remember(value) { mutableStateOf(false) }
        AlertDialog(
            onDismissRequest = { editing = null },
            title = { Text("$name (${Nbt.NAMES[value.type]})") },
            text = {
                OutlinedTextField(value = text, onValueChange = { text = it; bad = false }, isError = bad, modifier = Modifier.fillMaxWidth())
            },
            confirmButton = {
                TextButton(onClick = {
                    if (value.parse(text)) { dirty = true; editing = null } else bad = true
                }) { Text("OK") }
            },
            dismissButton = { TextButton(onClick = { editing = null }) { Text(stringResource(R.string.cancel)) } },
        )
    }
}
