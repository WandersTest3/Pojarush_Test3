package com.wanderstest3.pojarush.ui

import android.graphics.BitmapFactory
import android.media.MediaMetadataRetriever
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
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.graphics.FilterQuality
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.unit.dp
import com.wanderstest3.pojarush.controls.ControlLayouts
import java.io.File

/// Decodes a thumbnail bitmap for either an image or a video file (first
/// frame). Cached by [androidx.compose.runtime.remember] keyed on the file's
/// path and mtime, so it only re-decodes when the underlying file changes.
@Composable
private fun rememberThumb(file: File) = remember(file.absolutePath, file.lastModified()) {
    runCatching {
        if (ControlLayouts.isVideo(file.name)) {
            val retriever = MediaMetadataRetriever()
            try {
                retriever.setDataSource(file.absolutePath)
                retriever.frameAtTime
            } finally {
                retriever.release()
            }
        } else {
            BitmapFactory.decodeFile(file.absolutePath)
        }?.asImageBitmap()
    }.getOrNull()
}

/// A row of past imports for a media slot (mouse cursor, wallpaper, floating
/// menu icon) that keeps every file the user has chosen before, so switching
/// back to an earlier one is one tap instead of re-importing it. [current]
/// is the active file's absolute path; tapping a thumbnail selects it,
/// tapping its × removes it from history (and clears the slot if it was the
/// active one).
@Composable
fun HistoryGallery(
    files: List<File>,
    current: String,
    onSelect: (File) -> Unit,
    onDelete: (File) -> Unit,
) {
    if (files.isEmpty()) return
    Row(modifier = Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        files.forEach { f ->
            HistoryThumb(f, current == f.absolutePath, onClick = { onSelect(f) }, onDelete = { onDelete(f) })
        }
    }
}

@Composable
private fun HistoryThumb(file: File, selected: Boolean, onClick: () -> Unit, onDelete: () -> Unit) {
    val bitmap = rememberThumb(file)
    Box(modifier = Modifier.size(44.dp)) {
        Box(
            modifier = Modifier.size(44.dp).clip(RoundedCornerShape(6.dp))
                .background(Color(0x55000000))
                .border(if (selected) 3.dp else 1.dp, if (selected) MaterialTheme.colorScheme.primary else Color.Gray, RoundedCornerShape(6.dp))
                .clickable(onClick = onClick),
            contentAlignment = Alignment.Center
        ) {
            if (bitmap != null) {
                Image(bitmap, contentDescription = file.name, modifier = Modifier.size(38.dp), contentScale = ContentScale.Crop, filterQuality = FilterQuality.None)
            }
        }
        Box(
            modifier = Modifier.size(16.dp).align(Alignment.TopEnd)
                .clip(CircleShape)
                .background(Color(0xCC000000))
                .clickable(onClick = onDelete),
            contentAlignment = Alignment.Center
        ) {
            Text("\u00D7", color = Color.White, fontSize = androidx.compose.ui.unit.TextUnit(10f, androidx.compose.ui.unit.TextUnitType.Sp))
        }
    }
}
