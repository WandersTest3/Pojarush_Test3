package com.wanderstest3.pojarush.ui

import android.graphics.BitmapFactory
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.FilterQuality
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.wanderstest3.pojarush.R
import com.wanderstest3.pojarush.controls.HudImages
import java.io.File

object HudState {
    var version by mutableIntStateOf(0)
}

@Composable
private fun Thumb(file: File, selected: Boolean, onClick: () -> Unit) {
    val bitmap = remember(file.absolutePath, file.lastModified()) {
        runCatching { BitmapFactory.decodeFile(file.absolutePath)?.asImageBitmap() }.getOrNull()
    }
    Box(
        modifier = Modifier.size(40.dp).clip(RoundedCornerShape(6.dp))
            .background(Color(0x55000000))
            .border(if (selected) 3.dp else 1.dp, if (selected) MaterialTheme.colorScheme.primary else Color.Gray, RoundedCornerShape(6.dp))
            .clickable(onClick = onClick),
        contentAlignment = Alignment.Center
    ) {
        if (bitmap != null) {
            Image(bitmap, contentDescription = file.nameWithoutExtension, modifier = Modifier.size(34.dp), contentScale = ContentScale.Fit, filterQuality = FilterQuality.None)
        }
    }
}

@Composable
fun HudImagePicker(label: String, value: String, onChange: (String) -> Unit) {
    val context = LocalContext.current
    val version = HudState.version
    val files = remember(version) { HudImages.list() }
    val failed = stringResource(R.string.controls_import_failed, "%s")
    val gallery = rememberLauncherForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
        if (uris.isEmpty()) return@rememberLauncherForActivityResult
        val before = HudImages.list().toSet()
        var total = 0
        var lastError: String? = null
        uris.forEach { uri ->
            runCatching { total += HudImages.import(context, uri) }
                .onFailure { lastError = it.message.orEmpty() }
        }
        if (total > 0) {
            HudState.version++
            (HudImages.list() - before).firstOrNull()?.let { f -> onChange(f.nameWithoutExtension) }
        }
        lastError?.let { Toast.makeText(context, failed.replace("%s", it), Toast.LENGTH_LONG).show() }
    }
    Text(label, style = MaterialTheme.typography.bodyMedium)
    Hint(if (value.isEmpty()) stringResource(R.string.hud_image_none) else value)
    Row(modifier = Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        files.forEach { f ->
            Thumb(f, value == f.nameWithoutExtension || value == f.name) { onChange(f.nameWithoutExtension) }
        }
    }
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        OutlinedButton(onClick = { gallery.launch(arrayOf("image/*", "application/zip")) }) { Text(stringResource(R.string.hud_image_gallery)) }
        if (value.isNotEmpty()) OutlinedButton(onClick = { onChange("") }) { Text(stringResource(R.string.wallpaper_remove)) }
    }
}

@Composable
fun HudImagesSettings() {
    val context = LocalContext.current
    val version = HudState.version
    val count = remember(version) { HudImages.list().size }
    val failed = stringResource(R.string.controls_import_failed, "%s")
    val done = stringResource(R.string.hud_imported, "%s")
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
        if (uris.isEmpty()) return@rememberLauncherForActivityResult
        var total = 0
        var lastError: String? = null
        uris.forEach { uri ->
            runCatching { total += HudImages.import(context, uri) }
                .onFailure { lastError = it.message.orEmpty() }
        }
        if (total > 0) {
            HudState.version++
            Toast.makeText(context, done.replace("%s", total.toString()), Toast.LENGTH_SHORT).show()
        }
        lastError?.let { Toast.makeText(context, failed.replace("%s", it), Toast.LENGTH_LONG).show() }
    }
    PojarushCard {
        CardTitle(stringResource(R.string.hud_title))
        Hint(stringResource(R.string.hud_desc))
        Hint(stringResource(R.string.hud_count, count))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = { picker.launch(arrayOf("image/*", "application/zip")) }) { Text(stringResource(R.string.hud_import)) }
            OutlinedButton(onClick = { FolderOpener.open(context, HudImages.dir()) }) { Text(stringResource(R.string.open_folder)) }
        }
        Hint(stringResource(R.string.mods_folder, HudImages.dir().absolutePath))
    }
}
