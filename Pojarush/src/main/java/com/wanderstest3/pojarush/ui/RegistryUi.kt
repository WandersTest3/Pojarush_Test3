package com.wanderstest3.pojarush.ui

import android.content.Intent
import android.graphics.Bitmap
import android.net.Uri
import android.widget.Toast
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.basicMarquee
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.FilterQuality
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.wanderstest3.pojarush.R
import com.wanderstest3.pojarush.horizon.ModRegistry
import com.wanderstest3.pojarush.horizon.PackKind
import com.wanderstest3.pojarush.horizon.RegistryEntry
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

private object RegistryState {
    val progress = mutableStateMapOf<String, Float>()
}

@Composable
fun RegistryBrowser(kind: PackKind, onInstalled: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var entries by remember { mutableStateOf(ModRegistry.cachedEntries()) }
    var error by remember { mutableStateOf<String?>(null) }
    var loading by remember { mutableStateOf(false) }
    var reload by remember { mutableIntStateOf(0) }
    var query by rememberSaveable { mutableStateOf("") }
    var installedVersion by remember { mutableIntStateOf(0) }
    LaunchedEffect(reload) {
        if (entries != null && reload == 0) return@LaunchedEffect
        loading = true
        error = null
        runCatching { withContext(Dispatchers.IO) { ModRegistry.fetch(force = reload > 0) } }
            .onSuccess { entries = it }
            .onFailure { error = it.message ?: it.javaClass.simpleName }
        loading = false
    }
    val failedText = stringResource(R.string.registry_failed, "%s")
    val doneText = stringResource(R.string.registry_installed_toast, "%s")
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
        OutlinedButton(onClick = { reload++ }, enabled = !loading) { Text(stringResource(R.string.registry_refresh)) }
        OutlinedButton(onClick = {
            runCatching { context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(ModRegistry.SITE))) }
        }) { Text(stringResource(R.string.registry_open_site), maxLines = 1, modifier = Modifier.basicMarquee()) }
    }
    OutlinedTextField(
        value = query,
        onValueChange = { query = it },
        singleLine = true,
        label = { Text(stringResource(R.string.registry_search)) },
        modifier = Modifier.fillMaxWidth(),
    )
    if (loading) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp)
            Hint(stringResource(R.string.registry_loading))
        }
    }
    error?.let { Hint(stringResource(R.string.registry_failed, it)) }
    val all = entries ?: return
    val wanted = when (kind) {
        PackKind.MOD -> "mod"
        PackKind.RESOURCE -> "resourcepack"
        PackKind.SHADER -> "shader"
    }
    val q = query.trim().lowercase()
    val shown = all.filter { it.kind == wanted }.filter {
        q.isEmpty() || it.name.lowercase().contains(q) || it.summary.lowercase().contains(q) || it.authors.lowercase().contains(q)
    }
    if (kind == PackKind.MOD) {
        val native = all.count { it.kind == "native-mod" }
        if (native > 0) Hint(stringResource(R.string.registry_native_hidden, native))
    }
    if (shown.isEmpty() && !loading) Hint(stringResource(R.string.registry_empty))
    shown.forEach { entry ->
        val installed = remember(entry.fileName, installedVersion) { ModRegistry.installedFile(entry) != null }
        val progress = RegistryState.progress[entry.id]
        RegistryRow(entry, installed, progress) {
            RegistryState.progress[entry.id] = 0f
            scope.launch {
                val result = runCatching {
                    withContext(Dispatchers.IO) {
                        ModRegistry.install(entry) { p -> scope.launch { RegistryState.progress[entry.id] = p } }
                    }
                }
                RegistryState.progress.remove(entry.id)
                result.onSuccess {
                    installedVersion++
                    onInstalled()
                    Toast.makeText(context, doneText.replace("%s", entry.name), Toast.LENGTH_SHORT).show()
                }.onFailure {
                    Toast.makeText(context, failedText.replace("%s", it.message.orEmpty()), Toast.LENGTH_LONG).show()
                }
            }
        }
    }
}

@Composable
private fun RegistryRow(entry: RegistryEntry, installed: Boolean, progress: Float?, onInstall: () -> Unit) {
    val icon by produceState<Bitmap?>(null, entry.icon) {
        value = withContext(Dispatchers.IO) { ModRegistry.icon(entry) }
    }
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(10.dp))
            .background(Color.White.copy(alpha = 0.06f))
            .padding(10.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Box(
                modifier = Modifier.size(44.dp).clip(RoundedCornerShape(8.dp)).background(Color(0x3357197F)),
                contentAlignment = Alignment.Center,
            ) {
                icon?.let {
                    Image(
                        bitmap = it.asImageBitmap(),
                        contentDescription = null,
                        modifier = Modifier.size(44.dp),
                        filterQuality = FilterQuality.None,
                    )
                }
            }
            Column(modifier = Modifier.weight(1f)) {
                Text(entry.name, style = MaterialTheme.typography.titleSmall, maxLines = 1, modifier = Modifier.basicMarquee())
                Text(
                    "${entry.version} · ${entry.authors} · ${formatSize(entry.size)}",
                    style = MaterialTheme.typography.bodySmall,
                    maxLines = 1,
                    modifier = Modifier.basicMarquee(),
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            when {
                progress != null -> CircularProgressIndicator(modifier = Modifier.size(24.dp), strokeWidth = 2.dp)
                installed -> OutlinedButton(onClick = {}, enabled = false) { Text(stringResource(R.string.registry_installed)) }
                else -> Button(onClick = onInstall) { Text(stringResource(R.string.registry_install)) }
            }
        }
        if (entry.summary.isNotBlank()) {
            Text(entry.summary, style = MaterialTheme.typography.bodySmall, maxLines = 3, overflow = TextOverflow.Ellipsis)
        }
        if (progress != null) LinearProgressIndicator(progress = { progress }, modifier = Modifier.fillMaxWidth())
    }
}

private fun formatSize(bytes: Long): String = when {
    bytes >= 1024 * 1024 -> "%.2f MB".format(bytes / 1048576.0)
    bytes >= 1024 -> "%.0f KB".format(bytes / 1024.0)
    else -> "$bytes B"
}

