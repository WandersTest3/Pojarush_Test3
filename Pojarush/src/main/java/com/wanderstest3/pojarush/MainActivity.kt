package com.wanderstest3.pojarush

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Slider
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.wanderstest3.pojarush.controls.ControlSettings
import com.wanderstest3.pojarush.horizon.HorizonManager
import com.wanderstest3.pojarush.ui.CardTitle
import com.wanderstest3.pojarush.ui.ControlsSettings
import com.wanderstest3.pojarush.ui.Hint
import com.wanderstest3.pojarush.ui.LayoutSettings
import com.wanderstest3.pojarush.ui.MemorySettings
import com.wanderstest3.pojarush.ui.MouseSettings
import com.wanderstest3.pojarush.ui.PojarushCard
import com.wanderstest3.pojarush.ui.PojarushTheme
import com.wanderstest3.pojarush.ui.Purple
import com.wanderstest3.pojarush.ui.ResolutionSettings
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.util.Locale

private enum class Section(val label: Int, val glyph: String) {
    Home(R.string.nav_home, "▶"),
    Controls(R.string.nav_controls, "✥"),
    Mouse(R.string.nav_mouse, "➚"),
    Graphics(R.string.nav_graphics, "▣"),
    Logs(R.string.nav_logs, "☰"),
}

class MainActivity : ComponentActivity() {
    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) Fullscreen.apply(this)
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        Fullscreen.apply(this)
        setContent {
            PojarushTheme {
                Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
                    PojarushScreen()
                }
            }
        }
    }
}

@Composable
private fun PojarushScreen() {
    var section by rememberSaveable { mutableStateOf(Section.Home) }
    var storageVersion by remember { mutableIntStateOf(0) }
    val context = LocalContext.current
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) {
        if (Paths.refresh(context)) storageVersion++
    }
    Box(modifier = Modifier.fillMaxSize().padding(12.dp)) {
        Row(modifier = Modifier.fillMaxSize(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Column(modifier = Modifier.width(176.dp).fillMaxHeight(), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Header()
                Spacer(Modifier.height(6.dp))
                Section.entries.forEach { s ->
                    NavItem(s, s == section) { section = s }
                }
            }
            Box(modifier = Modifier.weight(1f).fillMaxHeight()) {
                when (section) {
                    Section.Logs -> LogCard(modifier = Modifier.fillMaxSize().padding(bottom = 76.dp))
                    else -> Column(
                        modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(bottom = 84.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp)
                    ) {
                        when (section) {
                            Section.Home -> {
                                val access = remember(storageVersion) { Paths.hasStorageAccess(context) }
                                if (!access) StorageCard()
                                AccountCard()
                                GameCard(storageVersion)
                            }
                            Section.Controls -> {
                                val settings = remember { ControlSettings(context) }
                                LayoutSettings(settings, onEdit = { context.startActivity(Intent(context, ControlEditorActivity::class.java)) }) {}
                                ControlsSettings(settings) {}
                            }
                            Section.Mouse -> MouseSettings(remember { ControlSettings(context) }) {}
                            Section.Graphics -> {
                                val settings = remember { ControlSettings(context) }
                                ResolutionSettings(settings) {}
                                MemorySettings(settings)
                            }
                            Section.Logs -> {}
                        }
                    }
                }
            }
        }
        PlayButton(modifier = Modifier.align(Alignment.BottomEnd))
    }
}

@Composable
private fun Header() {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Box(modifier = Modifier.size(44.dp).clip(RoundedCornerShape(12.dp)).background(Purple)) {
            Image(painterResource(R.drawable.ic_launcher_foreground), contentDescription = null, modifier = Modifier.fillMaxSize())
        }
        Spacer(Modifier.width(10.dp))
        Text(
            stringResource(R.string.app_name),
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.Bold,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis
        )
    }
}

@Composable
private fun NavItem(section: Section, selected: Boolean, onClick: () -> Unit) {
    val bg = if (selected) MaterialTheme.colorScheme.primaryContainer else Color.Transparent
    Row(
        modifier = Modifier.fillMaxWidth().clip(RoundedCornerShape(14.dp)).background(bg).clickable(onClick = onClick).padding(horizontal = 14.dp, vertical = 11.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(section.glyph, modifier = Modifier.width(26.dp), color = if (selected) Color.White else MaterialTheme.colorScheme.primary)
        Text(
            stringResource(section.label),
            style = MaterialTheme.typography.bodyLarge,
            fontWeight = if (selected) FontWeight.Bold else FontWeight.Normal,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis
        )
    }
}

@Composable
private fun PlayButton(modifier: Modifier) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val state by HorizonManager.state.collectAsStateWithLifecycle()
    val profile by Accounts.profile.collectAsStateWithLifecycle()
    val busy = state is HorizonManager.State.Running
    val ready = remember(state) { HorizonManager.isSupported() && HorizonManager.isPrepared() }
    val enabled = ready && !busy && profile != null
    ExtendedFloatingActionButton(
        modifier = modifier.height(64.dp),
        containerColor = if (enabled) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surfaceVariant,
        contentColor = if (enabled) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSurfaceVariant,
        onClick = {
            if (!enabled) return@ExtendedFloatingActionButton
            scope.launch {
                val current = profile?.let { p ->
                    runCatching { withContext(Dispatchers.IO) { MicrosoftAuth.refresh(p) } }
                        .onSuccess { if (it !== p) Accounts.save(it) }
                        .onFailure { e -> Toast.makeText(context, e.message, Toast.LENGTH_LONG).show() }
                        .getOrDefault(p)
                }
                context.startActivity(HorizonManager.launchIntent(context, current))
            }
        }
    ) {
        Text("▶  " + stringResource(R.string.play), style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold)
    }
}

@Composable
private fun StorageCard() {
    val context = LocalContext.current
    val permission = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) {}
    PojarushCard {
        CardTitle(stringResource(R.string.storage_title))
        Hint(stringResource(R.string.storage_needed))
        Button(onClick = {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                val app = Intent(Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION, Uri.parse("package:${context.packageName}"))
                runCatching { context.startActivity(app) }.onFailure {
                    context.startActivity(Intent(Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION))
                }
            } else {
                permission.launch(Manifest.permission.WRITE_EXTERNAL_STORAGE)
            }
        }) { Text(stringResource(R.string.storage_grant)) }
    }
}

@Composable
private fun AccountCard() {
    val scope = rememberCoroutineScope()
    val profile by Accounts.profile.collectAsStateWithLifecycle()
    var name by rememberSaveable { mutableStateOf(profile?.takeIf { !it.microsoft }?.name.orEmpty()) }
    var error by remember { mutableStateOf<String?>(null) }
    var signing by remember { mutableStateOf(false) }
    val login = rememberLauncherForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
        val code = result.data?.getStringExtra(MicrosoftLoginActivity.EXTRA_CODE)
        if (result.resultCode != Activity.RESULT_OK || code == null) {
            error = result.data?.getStringExtra(MicrosoftLoginActivity.EXTRA_ERROR)
            return@rememberLauncherForActivityResult
        }
        signing = true
        scope.launch {
            runCatching { withContext(Dispatchers.IO) { MicrosoftAuth.loginWithCode(code) } }
                .onSuccess { Accounts.save(it) }
                .onFailure { e -> error = e.message }
            signing = false
        }
    }
    val context = LocalContext.current

    PojarushCard {
        CardTitle(stringResource(R.string.account_title))
        Text(
            text = when {
                profile == null -> stringResource(R.string.account_none)
                profile!!.microsoft -> stringResource(R.string.account_signed_in, profile!!.name)
                else -> stringResource(R.string.account_offline, profile!!.name)
            },
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.primary
        )
        if (profile?.microsoft != true) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it.take(16) },
                    modifier = Modifier.weight(1f),
                    singleLine = true,
                    label = { Text(stringResource(R.string.account_name_hint)) },
                    isError = name.isNotEmpty() && !Accounts.isValidOfflineName(name),
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done)
                )
                Button(enabled = Accounts.isValidOfflineName(name), onClick = { Accounts.setOfflineName(name) }) {
                    Text(stringResource(R.string.account_save))
                }
            }
            if (name.isNotEmpty() && !Accounts.isValidOfflineName(name)) {
                Text(stringResource(R.string.account_name_invalid), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            if (profile?.microsoft == true) {
                OutlinedButton(onClick = { Accounts.signOut(); name = "" }) { Text(stringResource(R.string.account_sign_out)) }
            } else {
                OutlinedButton(enabled = !signing, onClick = {
                    error = null
                    login.launch(Intent(context, MicrosoftLoginActivity::class.java))
                }) { Text(stringResource(if (signing) R.string.account_signing_in else R.string.account_microsoft)) }
            }
        }
        error?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
    }
}

@Composable
private fun GameCard(storageVersion: Int) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val state by HorizonManager.state.collectAsStateWithLifecycle()
    var withSounds by rememberSaveable { mutableStateOf(true) }
    var refresh by remember { mutableIntStateOf(0) }
    val supported = remember { HorizonManager.isSupported() }
    val prepared = remember(state, refresh) { HorizonManager.isPrepared() }
    val sounds = remember(state, refresh) { HorizonManager.hasSounds() }
    val busy = state is HorizonManager.State.Running

    PojarushCard {
        CardTitle(stringResource(R.string.game_title))
        StatusLine(stringResource(R.string.status_device), supported)
        StatusLine(stringResource(R.string.status_prepared), prepared)
        StatusLine(stringResource(R.string.status_sounds), sounds)
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(stringResource(R.string.with_sounds), modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
            Switch(checked = withSounds, onCheckedChange = { withSounds = it }, enabled = !busy)
        }
        when (val s = state) {
            is HorizonManager.State.Running -> {
                Text(s.step, style = MaterialTheme.typography.bodySmall)
                if (s.progress != null) {
                    LinearProgressIndicator(progress = { s.progress }, modifier = Modifier.fillMaxWidth())
                } else {
                    LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
                }
            }
            is HorizonManager.State.Failed -> Text(s.message, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
            else -> {}
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            if (busy) {
                OutlinedButton(onClick = { HorizonManager.cancel() }) { Text(stringResource(R.string.cancel)) }
            } else {
                OutlinedButton(
                    enabled = supported,
                    onClick = { HorizonManager.prepare(context.applicationContext, withSounds) }
                ) { Text(stringResource(if (prepared) R.string.prepare_again else R.string.prepare)) }
                val done = stringResource(R.string.free_space_done)
                TextButton(onClick = {
                    scope.launch {
                        withContext(Dispatchers.IO) { HorizonManager.freeSpace() }
                        refresh++
                        Toast.makeText(context, done, Toast.LENGTH_SHORT).show()
                    }
                }) { Text(stringResource(R.string.free_space)) }
            }
        }
        HorizonManager.lastExitCode()?.takeIf { it != "0" }?.let {
            Text(stringResource(R.string.last_exit, it), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
        }
        val folder = remember(storageVersion) { HorizonManager.gameDir.absolutePath }
        Hint(stringResource(R.string.folder_info, folder))
        Hint(stringResource(R.string.ownership))
    }
}

@Composable
private fun StatusLine(label: String, ok: Boolean) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Text(
            text = if (ok) "✔" else "✖",
            color = if (ok) MaterialTheme.colorScheme.secondary else MaterialTheme.colorScheme.error,
            modifier = Modifier.padding(end = 8.dp)
        )
        Text(label, style = MaterialTheme.typography.bodyMedium)
    }
}

@Composable
private fun LogCard(modifier: Modifier) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val prepareLog by HorizonManager.log.collectAsStateWithLifecycle()
    var tab by rememberSaveable { mutableIntStateOf(0) }
    var refresh by remember { mutableIntStateOf(0) }
    var fileLines by remember { mutableStateOf<List<String>>(emptyList()) }
    LaunchedEffect(tab, refresh) {
        fileLines = when (tab) {
            1 -> withContext(Dispatchers.IO) { HorizonManager.readTail(HorizonManager.gameLog).lines() }
            2 -> withContext(Dispatchers.IO) { HorizonManager.readTail(HorizonManager.stdioLog).lines() }
            else -> withContext(Dispatchers.IO) { HorizonManager.readTail(HorizonManager.prepareLog).lines() }
        }.filter { it.isNotEmpty() }
    }
    val lines = if (tab == 0 && prepareLog.isNotEmpty()) prepareLog else fileLines
    Card(modifier = modifier, colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface)) {
        Column(modifier = Modifier.fillMaxSize().padding(12.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                listOf(R.string.tab_prepare, R.string.tab_game, R.string.tab_stdio).forEachIndexed { i, label ->
                    FilterChip(selected = tab == i, onClick = { tab = i }, label = { Text(stringResource(label), maxLines = 1) })
                }
                Spacer(Modifier.weight(1f))
                TextButton(onClick = { refresh++ }) { Text(stringResource(R.string.refresh), maxLines = 1) }
                TextButton(onClick = {
                    scope.launch(Dispatchers.IO) { runCatching { HorizonManager.shareLogs(context) } }
                }) { Text(stringResource(R.string.share_logs), maxLines = 1) }
            }
            val listState = rememberLazyListState()
            LaunchedEffect(lines.size, tab) {
                if (lines.isNotEmpty()) listState.scrollToItem(lines.size - 1)
            }
            if (lines.isEmpty()) {
                Text(stringResource(R.string.log_empty), modifier = Modifier.padding(top = 12.dp), style = MaterialTheme.typography.bodySmall)
            } else {
                LazyColumn(state = listState, modifier = Modifier.fillMaxSize().padding(top = 8.dp), contentPadding = PaddingValues(bottom = 8.dp)) {
                    items(lines) { line ->
                        Text(line, style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace)
                    }
                }
            }
        }
    }
}
