package com.wanderstest3.pojarush.ui

import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.asImageBitmap
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
import androidx.compose.runtime.mutableStateOf
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
import com.wanderstest3.pojarush.controls.ControlSettings
import com.wanderstest3.pojarush.horizon.PackKind

@Composable
fun ModsPage() {
    val context = LocalContext.current
    var tab by rememberSaveable { mutableIntStateOf(0) }
    var browse by rememberSaveable { mutableStateOf(false) }
    var creating by rememberSaveable { mutableStateOf(false) }
    var version by remember { mutableIntStateOf(0) }
    val settings = remember { ControlSettings(context) }
    remember { settings.modsEnabled = true; settings.shadersEnabled = true }
    val kind = PackKind.entries[tab]
    val entries = remember(tab, version) {
        HorizonManager.ensureFolders()
        HorizonManager.removeLegacyBundledMods(context)
        GameFiles.list(kind)
    }
    val restart = { HorizonManager.restartPrewarm(context, com.wanderstest3.pojarush.Accounts.profile.value) }
    val failed = stringResource(R.string.controls_import_failed, "%s")
    val imported = stringResource(R.string.controls_imported, "%s")
    val inboxText = stringResource(R.string.mods_inbox, "%s")
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        uri ?: return@rememberLauncherForActivityResult
        if (com.wanderstest3.pojarush.BuildConfig.STARLING && kind == PackKind.MOD) {
            runCatching { com.wanderstest3.pojarush.horizon.NativeModProjects.toInbox(context, uri) }
                .onSuccess { Toast.makeText(context, inboxText.replace("%s", it), Toast.LENGTH_LONG).show() }
                .onFailure { Toast.makeText(context, failed.replace("%s", it.message.orEmpty()), Toast.LENGTH_LONG).show() }
            return@rememberLauncherForActivityResult
        }
        runCatching { GameFiles.import(context, kind, uri) }
            .onSuccess {
                version++
                restart()
                Toast.makeText(context, imported.replace("%s", it.name), Toast.LENGTH_SHORT).show()
            }
            .onFailure { Toast.makeText(context, failed.replace("%s", it.message.orEmpty()), Toast.LENGTH_LONG).show() }
    }
    PojarushCard {
        CardTitle(stringResource(R.string.nav_mods))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            listOf(R.string.mods_tab_mods, R.string.mods_tab_resource, R.string.mods_tab_shaders).forEachIndexed { i, label ->
                FilterChip(selected = tab == i && !creating, onClick = { tab = i; creating = false }, label = { Text(stringResource(label), maxLines = 1, modifier = Modifier.basicMarquee()) })
            }
            if (com.wanderstest3.pojarush.BuildConfig.STARLING) {
                FilterChip(selected = creating, onClick = { creating = true }, label = { Text(stringResource(R.string.mods_tab_create), maxLines = 1, modifier = Modifier.basicMarquee()) })
            }
        }
        if (creating) {
            NativeModCreator()
            return@PojarushCard
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            FilterChip(selected = !browse, onClick = { browse = false }, label = { Text(stringResource(R.string.registry_tab_installed), maxLines = 1) })
            FilterChip(selected = browse, onClick = { browse = true }, label = { Text(stringResource(R.string.registry_tab_download), maxLines = 1) })
        }
        if (browse) {
            Hint(stringResource(R.string.registry_desc))
            RegistryBrowser(kind) {
                version++
                restart()
            }
            return@PojarushCard
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
        if (kind == PackKind.MOD) {
            entries.forEach { e ->
                ModMenuCard(e, onToggle = {
                    GameFiles.setEnabled(kind, e, it)
                    version++
                    restart()
                }, onDelete = {
                    GameFiles.delete(kind, e)
                    version++
                    restart()
                })
            }
        } else entries.forEach { e ->
            Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Text(e.name, modifier = Modifier.weight(1f).basicMarquee(), maxLines = 1, style = MaterialTheme.typography.bodyMedium)
                if (e.enabled != null) {
                    SettingSwitchInline(e.enabled) {
                        GameFiles.setEnabled(kind, e, it)
                        version++
                        restart()
                    }
                }
                TextButton(onClick = {
                    GameFiles.delete(kind, e)
                    version++
                    restart()
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
private fun ModMenuCard(entry: com.wanderstest3.pojarush.horizon.PackEntry, onToggle: (Boolean) -> Unit, onDelete: () -> Unit) {
    val context = LocalContext.current
    val info = remember(entry.file.path, entry.file.lastModified()) { GameFiles.modInfo(entry.file) }
    val icon = remember(info) {
        info?.icon?.let { runCatching { android.graphics.BitmapFactory.decodeByteArray(it, 0, it.size)?.asImageBitmap() }.getOrNull() }
    }
    var open by rememberSaveable(entry.file.path) { mutableStateOf(false) }
    val purple = androidx.compose.ui.graphics.Color(0xFF57197F)
    androidx.compose.material3.Surface(
        modifier = Modifier.fillMaxWidth().clickable { open = !open },
        shape = androidx.compose.foundation.shape.RoundedCornerShape(12.dp),
        color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.55f),
    ) {
        Column(modifier = Modifier.padding(10.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                if (icon != null) {
                    androidx.compose.foundation.Image(
                        bitmap = icon,
                        contentDescription = null,
                        filterQuality = androidx.compose.ui.graphics.FilterQuality.None,
                        modifier = Modifier.size(48.dp).clip(androidx.compose.foundation.shape.RoundedCornerShape(8.dp)),
                    )
                } else {
                    Box(
                        modifier = Modifier.size(48.dp).clip(androidx.compose.foundation.shape.RoundedCornerShape(8.dp)).background(purple),
                        contentAlignment = Alignment.Center,
                    ) {
                        Text(entry.name.take(1).uppercase(), color = androidx.compose.ui.graphics.Color.White, style = MaterialTheme.typography.titleLarge)
                    }
                }
                Column(modifier = Modifier.weight(1f)) {
                    Row(verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        Text(entry.name, maxLines = 1, overflow = TextOverflow.Ellipsis, style = MaterialTheme.typography.titleSmall, modifier = Modifier.weight(1f, fill = false))
                        if (!info?.version.isNullOrEmpty()) Text(info!!.version, maxLines = 1, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    if (!info?.authors.isNullOrEmpty()) {
                        Text(stringResource(R.string.mod_menu_by, info!!.authors.joinToString(", ")), maxLines = 1, overflow = TextOverflow.Ellipsis, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    if (!info?.description.isNullOrEmpty()) {
                        Text(info!!.description, maxLines = if (open) Int.MAX_VALUE else 2, overflow = TextOverflow.Ellipsis, style = MaterialTheme.typography.bodySmall)
                    }
                }
                if (entry.enabled != null) SettingSwitchInline(entry.enabled, onToggle)
            }
            if (open) {
                if (info != null) {
                    Text(
                        listOfNotNull(info.id, info.api.takeIf { it.isNotEmpty() }?.let { "API $it" }, info.side.takeIf { it.isNotEmpty() }).joinToString(" · "),
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    if (info.capabilities.isNotEmpty()) {
                        Text(stringResource(R.string.mod_menu_capabilities, info.capabilities.joinToString(", ")), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
                Text(entry.file.name, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
                Row(horizontalArrangement = Arrangement.spacedBy(4.dp), verticalAlignment = Alignment.CenterVertically) {
                    info?.links.orEmpty().forEach { (key, url) ->
                        val label = when (key) {
                            "homepage" -> R.string.mod_menu_homepage
                            "sources" -> R.string.mod_menu_sources
                            "issues" -> R.string.mod_menu_issues
                            else -> R.string.mod_menu_license
                        }
                        TextButton(onClick = {
                            runCatching { context.startActivity(android.content.Intent(android.content.Intent.ACTION_VIEW, android.net.Uri.parse(url))) }
                        }) { Text(stringResource(label), maxLines = 1) }
                    }
                    androidx.compose.foundation.layout.Spacer(Modifier.weight(1f))
                    TextButton(onClick = onDelete) { Text(stringResource(R.string.controls_delete)) }
                }
            }
        }
    }
}

@Composable
private fun SettingSwitchInline(value: Boolean, onChange: (Boolean) -> Unit) {
    var current by remember(value) { androidx.compose.runtime.mutableStateOf(value) }
    Switch(checked = current, onCheckedChange = { current = it; onChange(it) })
}

@Composable
private fun NativeModCreator() {
    val context = LocalContext.current
    var id by rememberSaveable { mutableStateOf("") }
    var name by rememberSaveable { mutableStateOf("") }
    var description by rememberSaveable { mutableStateOf("") }
    var version by remember { mutableIntStateOf(0) }
    val projects = remember(version) { com.wanderstest3.pojarush.horizon.NativeModProjects.list() }
    val invalid = stringResource(R.string.modcreator_invalid)
    val exists = stringResource(R.string.modcreator_exists)
    val installed = stringResource(R.string.modcreator_installed, "%s")
    val failed = stringResource(R.string.controls_import_failed, "%s")
    Hint(stringResource(R.string.modcreator_desc))
    androidx.compose.material3.OutlinedTextField(
        value = id,
        onValueChange = { id = it.lowercase().filter { c -> c in 'a'..'z' || c in '0'..'9' || c == '_' }.take(32) },
        label = { Text(stringResource(R.string.modcreator_id)) },
        singleLine = true,
        modifier = Modifier.fillMaxWidth(),
    )
    androidx.compose.material3.OutlinedTextField(
        value = name,
        onValueChange = { name = it.take(48) },
        label = { Text(stringResource(R.string.modcreator_name)) },
        singleLine = true,
        modifier = Modifier.fillMaxWidth(),
    )
    androidx.compose.material3.OutlinedTextField(
        value = description,
        onValueChange = { description = it.take(200) },
        label = { Text(stringResource(R.string.modcreator_description)) },
        modifier = Modifier.fillMaxWidth(),
    )
    Button(onClick = {
        val p = com.wanderstest3.pojarush.horizon.NativeModProjects
        when {
            !p.validId(id) -> Toast.makeText(context, invalid, Toast.LENGTH_SHORT).show()
            java.io.File(p.root, id).exists() -> Toast.makeText(context, exists, Toast.LENGTH_SHORT).show()
            else -> runCatching { p.create(id, name.ifBlank { id }, description) }
                .onSuccess { version++; id = ""; name = ""; description = "" }
                .onFailure { Toast.makeText(context, failed.replace("%s", it.message.orEmpty()), Toast.LENGTH_LONG).show() }
        }
    }) { Text(stringResource(R.string.modcreator_create)) }
    if (projects.isEmpty()) Hint(stringResource(R.string.modcreator_empty))
    projects.forEach { dir ->
        Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(dir.name, modifier = Modifier.weight(1f).basicMarquee(), maxLines = 1, style = MaterialTheme.typography.bodyMedium)
            TextButton(onClick = { FolderOpener.open(context, dir) }) { Text(stringResource(R.string.open_folder)) }
            TextButton(onClick = {
                runCatching { com.wanderstest3.pojarush.horizon.NativeModProjects.pack(context, dir) }
                    .onSuccess { Toast.makeText(context, installed.replace("%s", it), Toast.LENGTH_LONG).show() }
                    .onFailure { Toast.makeText(context, failed.replace("%s", it.message.orEmpty()), Toast.LENGTH_LONG).show() }
            }) { Text(stringResource(R.string.modcreator_pack)) }
        }
    }
    Hint(stringResource(R.string.modcreator_folder, com.wanderstest3.pojarush.horizon.NativeModProjects.root.absolutePath))
}
