package com.wanderstest3.pojarush.ui

import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.foundation.basicMarquee
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.wanderstest3.pojarush.R
import com.wanderstest3.pojarush.horizon.GameFiles
import com.wanderstest3.pojarush.horizon.HorizonManager
import com.wanderstest3.pojarush.horizon.PackKind

@Composable
fun ModsPage() {
    val context = LocalContext.current
    var tab by rememberSaveable { mutableIntStateOf(0) }
    var version by remember { mutableIntStateOf(0) }
    val kind = PackKind.entries[tab]
    val entries = remember(tab, version) {
        HorizonManager.ensureFolders()
        GameFiles.list(kind)
    }
    val failed = stringResource(R.string.controls_import_failed, "%s")
    val imported = stringResource(R.string.controls_imported, "%s")
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        uri ?: return@rememberLauncherForActivityResult
        runCatching { GameFiles.import(context, kind, uri) }
            .onSuccess {
                version++
                Toast.makeText(context, imported.replace("%s", it.name), Toast.LENGTH_SHORT).show()
            }
            .onFailure { Toast.makeText(context, failed.replace("%s", it.message.orEmpty()), Toast.LENGTH_LONG).show() }
    }
    PojarushCard {
        CardTitle(stringResource(R.string.nav_mods))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            listOf(R.string.mods_tab_mods, R.string.mods_tab_resource, R.string.mods_tab_shaders).forEachIndexed { i, label ->
                FilterChip(selected = tab == i, onClick = { tab = i }, label = { Text(stringResource(label), maxLines = 1, modifier = Modifier.basicMarquee()) })
            }
        }
        Hint(
            stringResource(
                when (kind) {
                    PackKind.MOD -> R.string.mods_desc_mods
                    PackKind.RESOURCE -> R.string.mods_desc_resource
                    PackKind.SHADER -> R.string.mods_desc_shaders
                }
            )
        )
        if (entries.isEmpty()) Hint(stringResource(R.string.mods_empty))
        entries.forEach { e ->
            Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Text(e.name, modifier = Modifier.weight(1f).basicMarquee(), maxLines = 1, style = MaterialTheme.typography.bodyMedium)
                if (e.enabled != null) {
                    SettingSwitchInline(e.enabled) {
                        GameFiles.setEnabled(kind, e, it)
                        version++
                    }
                }
                TextButton(onClick = {
                    GameFiles.delete(kind, e)
                    version++
                }) { Text(stringResource(R.string.controls_delete)) }
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = { picker.launch(arrayOf("*/*")) }) { Text(stringResource(R.string.mods_import)) }
            androidx.compose.material3.OutlinedButton(onClick = { FolderOpener.open(context, GameFiles.dir(kind)) }) { Text(stringResource(R.string.open_folder)) }
        }
        Hint(stringResource(R.string.mods_folder, GameFiles.dir(kind).absolutePath))
        when (kind) {
            PackKind.MOD -> Hint(stringResource(R.string.mods_restart))
            PackKind.RESOURCE -> Hint(stringResource(R.string.mods_resource_switch))
            PackKind.SHADER -> Hint(stringResource(R.string.mods_pick_in_game))
        }
    }
}

@Composable
private fun SettingSwitchInline(value: Boolean, onChange: (Boolean) -> Unit) {
    var current by remember(value) { androidx.compose.runtime.mutableStateOf(value) }
    Switch(checked = current, onCheckedChange = { current = it; onChange(it) })
}
