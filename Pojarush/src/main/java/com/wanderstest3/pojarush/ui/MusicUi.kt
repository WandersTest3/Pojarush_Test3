package com.wanderstest3.pojarush.ui

import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.wanderstest3.pojarush.MusicLibrary
import com.wanderstest3.pojarush.R
import com.wanderstest3.pojarush.controls.ControlSettings
import java.io.File

@Composable
fun MusicPage() {
    val context = LocalContext.current
    val settings = remember { ControlSettings(context) }
    var tracks by remember { mutableStateOf(MusicLibrary.tracks(context)) }
    var inMenu by remember { mutableStateOf(settings.musicInMenu) }
    var inGame by remember { mutableStateOf(settings.gameMusic) }
    var shuffle by remember { mutableStateOf(settings.musicShuffle) }
    val failed = stringResource(R.string.menu_music_failed)
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
        uris.forEach { uri ->
            MusicLibrary.add(context, uri).onFailure {
                Toast.makeText(context, failed.replace("%s", it.message.orEmpty()), Toast.LENGTH_LONG).show()
            }
        }
        tracks = MusicLibrary.tracks(context)
        WallpaperState.version++
    }
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(stringResource(R.string.music_title), style = MaterialTheme.typography.titleMedium)
        Text(stringResource(R.string.music_hint), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Button(onClick = { picker.launch(arrayOf("audio/*")) }) { Text(stringResource(R.string.music_add)) }
        if (tracks.isEmpty()) {
            Text(stringResource(R.string.music_empty), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        tracks.forEach { file: File ->
            Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(MusicLibrary.title(file), modifier = Modifier.weight(1f).padding(end = 4.dp), maxLines = 1, style = MaterialTheme.typography.bodyMedium)
                OutlinedButton(onClick = {
                    file.delete()
                    tracks = MusicLibrary.tracks(context)
                    WallpaperState.version++
                }) { Text(stringResource(R.string.wallpaper_remove)) }
            }
        }
        MusicSwitch(stringResource(R.string.music_in_menu), inMenu) {
            inMenu = it
            settings.musicInMenu = it
            WallpaperState.version++
        }
        MusicSwitch(stringResource(R.string.game_music_enable), inGame) {
            inGame = it
            settings.gameMusic = it
        }
        MusicSwitch(stringResource(R.string.music_shuffle), shuffle) {
            shuffle = it
            settings.musicShuffle = it
        }
        SettingSlider(stringResource(R.string.menu_music_volume), settings.menuMusicVolume, 0f..1f, 0.05f, { "${Math.round(it * 100)}%" }, default = 0.6f) {
            settings.menuMusicVolume = it
            WallpaperState.version++
        }
        SettingSlider(stringResource(R.string.game_music_volume), settings.gameMusicVolume, 0f..1f, 0.05f, { "${Math.round(it * 100)}%" }, default = 0.4f) {
            settings.gameMusicVolume = it
        }
    }
}

@Composable
private fun MusicSwitch(label: String, checked: Boolean, onChange: (Boolean) -> Unit) {
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(label, modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
        Switch(checked = checked, onCheckedChange = onChange)
    }
}
