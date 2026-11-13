package com.wanderstest3.pojarush.ui

import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.foundation.basicMarquee
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.wanderstest3.pojarush.R
import com.wanderstest3.pojarush.controls.ControlLayouts
import com.wanderstest3.pojarush.controls.ControlSettings
import java.io.File

enum class Change { LAYOUT, CONTROLS, MOUSE, RESOLUTION, MENU_BUTTON }

@Composable
fun LayoutSettings(settings: ControlSettings, onEdit: (() -> Unit)?, onChange: (Change) -> Unit) {
    val context = LocalContext.current
    val defaultName = stringResource(R.string.controls_default_name)
    var version by remember { mutableIntStateOf(0) }
    var selected by remember { mutableStateOf(settings.layout) }
    val entries = remember(version) { ControlLayouts.list(context, defaultName) }
    val imported = stringResource(R.string.controls_imported, "%s")
    val failed = stringResource(R.string.controls_import_failed, "%s")
    fun choose(id: String) {
        settings.layout = id
        selected = id
        onChange(Change.LAYOUT)
    }
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
        if (uris.isEmpty()) return@rememberLauncherForActivityResult
        var last: com.wanderstest3.pojarush.controls.ControlLayouts.Entry? = null
        uris.forEach { uri ->
            runCatching { last = ControlLayouts.import(context, uri) }
                .onFailure { Toast.makeText(context, failed.replace("%s", it.message.orEmpty()), Toast.LENGTH_LONG).show() }
        }
        last?.let {
            version++
            choose(it.id)
            Toast.makeText(context, imported.replace("%s", it.name), Toast.LENGTH_SHORT).show()
        }
    }
    var renaming by remember { mutableStateOf<Pair<String, String>?>(null) }
    renaming?.let { (id, name) ->
        NameDialog(stringResource(R.string.controls_rename), name, stringResource(R.string.controls_rename), { renaming = null }) { newName ->
            val newId = ControlLayouts.rename(id, newName)
            renaming = null
            version++
            if (selected == id) choose(newId)
        }
    }
    PojarushCard {
        CardTitle(stringResource(R.string.controls_layout))
        entries.forEach { e ->
            Row(
                modifier = Modifier.fillMaxWidth().clip(RoundedCornerShape(10.dp)).clickable { choose(e.id) },
                verticalAlignment = Alignment.CenterVertically
            ) {
                RadioButton(selected = selected == e.id, onClick = { choose(e.id) })
                Text(e.name, modifier = Modifier.weight(1f).basicMarquee(), style = MaterialTheme.typography.bodyMedium, maxLines = 1)
                if (e.removable) {
                    TextButton(onClick = { renaming = e.id to e.name }) { Text(stringResource(R.string.controls_rename)) }
                    TextButton(onClick = {
                        ControlLayouts.delete(e.id)
                        version++
                        if (selected == e.id) choose(entries.first().id)
                    }) { Text(stringResource(R.string.controls_delete)) }
                }
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            onEdit?.let { Button(onClick = it) { Text(stringResource(R.string.controls_edit)) } }
            OutlinedButton(onClick = { picker.launch(arrayOf("application/json", "text/plain", "application/octet-stream", "*/*")) }) {
                Text(stringResource(R.string.controls_import))
            }
        }
        Hint(stringResource(R.string.controls_folder, ControlLayouts.dir.absolutePath))
    }
}

@Composable
fun ControlsSettings(settings: ControlSettings, onChange: (Change) -> Unit) {
    PojarushCard {
        CardTitle(stringResource(R.string.nav_controls))
        SettingSlider(stringResource(R.string.controls_opacity), settings.opacity, 0.10f..1.00f, 0.05f, { "${Math.round(it * 100)}%" }) { settings.opacity = it; onChange(Change.CONTROLS) }
        SettingSlider(stringResource(R.string.controls_size), settings.buttonScale, 0.50f..2.00f, 0.05f, { "${fmt(it)}x" }) { settings.buttonScale = it; onChange(Change.CONTROLS) }
        SettingSlider(stringResource(R.string.controls_sensitivity), settings.lookSensitivity, 0.10f..2.00f, 0.05f, { "${fmt(it)}x" }) { settings.lookSensitivity = it; onChange(Change.CONTROLS) }
        SettingSlider(stringResource(R.string.controls_long_press), settings.longPressMs.toFloat(), 150f..800f, 50f, { "${it.toInt()} ms" }) { settings.longPressMs = it.toInt(); onChange(Change.CONTROLS) }
        SettingSwitch(stringResource(R.string.controls_tap_use), settings.tapToUse) { settings.tapToUse = it; onChange(Change.CONTROLS) }
        SettingSwitch(stringResource(R.string.controls_hotbar), settings.hotbarTouch) { settings.hotbarTouch = it; onChange(Change.CONTROLS) }
        SettingSwitch(stringResource(R.string.controls_vibrate), settings.vibrate) { settings.vibrate = it; onChange(Change.CONTROLS) }
        SettingSwitch(stringResource(R.string.controls_hidden), settings.controlsHidden, stringResource(R.string.controls_hidden_desc)) { settings.controlsHidden = it; onChange(Change.CONTROLS) }
        SettingSwitch(stringResource(R.string.controls_hotbar_swap), settings.hotbarSwap, stringResource(R.string.controls_hotbar_swap_desc)) { settings.hotbarSwap = it; onChange(Change.CONTROLS) }
        var drop by remember { mutableStateOf(settings.bedrockDrop) }
        SettingSwitch(stringResource(R.string.controls_bedrock_drop), drop, stringResource(R.string.controls_bedrock_drop_desc)) {
            settings.bedrockDrop = it
            drop = it
            onChange(Change.CONTROLS)
        }
        if (drop) {
            SettingSlider(stringResource(R.string.controls_bedrock_drop_interval), settings.bedrockDropMs.toFloat(), 50f..600f, 50f, { "${it.toInt()} ms" }) {
                settings.bedrockDropMs = it.toInt()
                onChange(Change.CONTROLS)
            }
        }
        Hint(stringResource(R.string.controls_help))
    }
}

@Composable
fun MenuButtonSettings(settings: ControlSettings, onChange: (Change) -> Unit) {
    val context = LocalContext.current
    var icon by remember { mutableStateOf(settings.menuIcon) }
    var iconVersion by remember { mutableIntStateOf(0) }
    val failed = stringResource(R.string.controls_import_failed, "%s")
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
        if (uris.isEmpty()) return@rememberLauncherForActivityResult
        var last: java.io.File? = null
        uris.forEach { uri ->
            runCatching { last = ControlLayouts.importMenuIcon(context, uri) }
                .onFailure { Toast.makeText(context, failed.replace("%s", it.message.orEmpty()), Toast.LENGTH_LONG).show() }
        }
        last?.let {
            settings.menuIcon = it.absolutePath
            icon = it.absolutePath
            onChange(Change.MENU_BUTTON)
            iconVersion++
        }
    }
    PojarushCard {
        CardTitle(stringResource(R.string.menu_button_title))
        SettingSlider(stringResource(R.string.menu_button_size), settings.menuSize, 20f..80f, 2f, { "${it.toInt()} dp" }) { settings.menuSize = it; onChange(Change.MENU_BUTTON) }
        SettingSlider(stringResource(R.string.menu_button_opacity), settings.menuOpacity, 0.05f..1.00f, 0.05f, { "${Math.round(it * 100)}%" }) { settings.menuOpacity = it; onChange(Change.MENU_BUTTON) }
        SettingSlider(stringResource(R.string.menu_button_radius), settings.menuRadius, 0f..0.50f, 0.05f, { "${Math.round(it * 200)}%" }) { settings.menuRadius = it; onChange(Change.MENU_BUTTON) }
        Hint(if (icon.isEmpty()) stringResource(R.string.menu_button_icon_none) else stringResource(R.string.mouse_cursor_current, File(icon).name))
        val iconFiles = remember(iconVersion) { ControlLayouts.history(ControlLayouts.menuIconDir()) }
        if (iconFiles.isNotEmpty()) {
            HistoryGallery(
                files = iconFiles,
                current = icon,
                onSelect = { f ->
                    settings.menuIcon = f.absolutePath
                    icon = f.absolutePath
                    onChange(Change.MENU_BUTTON)
                },
                onDelete = { f ->
                    ControlLayouts.deleteFromHistory(f, settings.menuIcon) {
                        settings.menuIcon = ""
                        icon = ""
                        onChange(Change.MENU_BUTTON)
                    }
                    iconVersion++
                },
            )
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = { picker.launch(arrayOf("image/*")) }) { Text(stringResource(R.string.menu_button_icon_choose)) }
            if (icon.isNotEmpty()) {
                OutlinedButton(onClick = {
                    settings.menuIcon = ""
                    icon = ""
                    onChange(Change.MENU_BUTTON)
                }) { Text(stringResource(R.string.mouse_cursor_default)) }
            }
        }
        Hint(stringResource(R.string.menu_button_hint))
    }
}

@Composable
fun MouseSettings(settings: ControlSettings, onChange: (Change) -> Unit) {
    val context = LocalContext.current
    var image by remember { mutableStateOf(settings.cursorImage) }
    var cursorVersion by remember { mutableIntStateOf(0) }
    val failed = stringResource(R.string.controls_import_failed, "%s")
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
        if (uris.isEmpty()) return@rememberLauncherForActivityResult
        var last: java.io.File? = null
        uris.forEach { uri ->
            runCatching { last = ControlLayouts.importCursor(context, uri) }
                .onFailure { Toast.makeText(context, failed.replace("%s", it.message.orEmpty()), Toast.LENGTH_LONG).show() }
        }
        last?.let {
            settings.cursorImage = it.absolutePath
            image = it.absolutePath
            onChange(Change.MOUSE)
            cursorVersion++
        }
    }
    PojarushCard {
        CardTitle(stringResource(R.string.nav_mouse))
        SettingSwitch(stringResource(R.string.mouse_virtual), settings.virtualMouse, stringResource(R.string.mouse_virtual_desc)) { settings.virtualMouse = it; onChange(Change.MOUSE) }
        SettingSlider(stringResource(R.string.mouse_speed), settings.mouseSpeed, 0.25f..3.00f, 0.05f, { "${fmt(it)}x" }) { settings.mouseSpeed = it; onChange(Change.MOUSE) }
        SettingSlider(stringResource(R.string.mouse_size), settings.cursorSize, 0.25f..2.00f, 0.05f, { "${fmt(it)}x" }) { settings.cursorSize = it; onChange(Change.MOUSE) }
        SettingSwitch(stringResource(R.string.mouse_screen_touch), settings.screenTouch, stringResource(R.string.mouse_screen_touch_desc)) { settings.screenTouch = it; onChange(Change.MOUSE) }
    }
    PojarushCard {
        CardTitle(stringResource(R.string.mouse_cursor_image))
        val cursorFiles = remember(cursorVersion) { ControlLayouts.history(ControlLayouts.cursorDir()) }
        if (cursorFiles.isEmpty()) {
            Hint(stringResource(R.string.mouse_cursor_none))
        } else {
            HistoryGallery(
                files = cursorFiles,
                current = image,
                onSelect = { f ->
                    settings.cursorImage = f.absolutePath
                    image = f.absolutePath
                    onChange(Change.MOUSE)
                },
                onDelete = { f ->
                    ControlLayouts.deleteFromHistory(f, settings.cursorImage) {
                        settings.cursorImage = ""
                        image = ""
                        onChange(Change.MOUSE)
                    }
                    cursorVersion++
                },
            )
            if (image.isNotEmpty()) {
                TextButton(onClick = {
                    settings.cursorImage = ""
                    image = ""
                    onChange(Change.MOUSE)
                }) { Text(stringResource(R.string.mouse_cursor_default)) }
            }
        }
        OutlinedButton(onClick = { picker.launch(arrayOf("image/*")) }) { Text(stringResource(R.string.mouse_cursor_choose)) }
        SettingSwitch(stringResource(R.string.mouse_cursor_center), settings.cursorCenter) { settings.cursorCenter = it; onChange(Change.MOUSE) }
    }
}

@Composable
fun MemorySettings(settings: ControlSettings) {
    val context = LocalContext.current
    val totalMb = remember {
        val info = android.app.ActivityManager.MemoryInfo()
        (context.getSystemService(android.content.Context.ACTIVITY_SERVICE) as android.app.ActivityManager).getMemoryInfo(info)
        (info.totalMem / (1024L * 1024L)).toInt()
    }
    val maxMb = (totalMb - 1024).coerceAtLeast(1024)
    var auto by remember { mutableStateOf(settings.javaHeapMb <= 0) }
    var heapText by remember { mutableStateOf(settings.javaHeapMb.coerceIn(512, maxMb).toString()) }
    fun applyHeap(mb: Int) {
        val v = mb.coerceIn(512, maxMb)
        settings.javaHeapMb = v
        heapText = v.toString()
    }
    PojarushCard {
        CardTitle(stringResource(R.string.memory_title))
        Hint(stringResource(R.string.memory_desc))
        Hint(stringResource(R.string.memory_total, totalMb))
        SettingSwitch(stringResource(R.string.memory_auto), auto, stringResource(R.string.memory_auto_desc)) {
            auto = it
            settings.javaHeapMb = if (it) 0 else 2048.coerceAtMost(maxMb)
            heapText = settings.javaHeapMb.coerceIn(512, maxMb).toString()
        }
        if (!auto) {
            SettingSlider(
                stringResource(R.string.memory_heap),
                settings.javaHeapMb.coerceIn(512, maxMb).toFloat(),
                512f..maxMb.toFloat(),
                256f,
                { context.getString(R.string.memory_value, it.toInt()) }
            ) { applyHeap(Math.round(it)) }
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = heapText,
                    onValueChange = { t ->
                        heapText = t.filter(Char::isDigit).take(5)
                        heapText.toIntOrNull()?.takeIf { it in 512..maxMb }?.let { settings.javaHeapMb = it }
                    },
                    modifier = Modifier.width(150.dp),
                    singleLine = true,
                    label = { Text(stringResource(R.string.memory_exact)) },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number, imeAction = ImeAction.Done),
                    keyboardActions = KeyboardActions(onDone = { heapText.toIntOrNull()?.let(::applyHeap) })
                )
                Text("MB")
            }
            Hint(stringResource(R.string.memory_range, 512, maxMb))
        }
    }
}

@Composable
fun ResolutionSettings(settings: ControlSettings, onChange: (Change) -> Unit) {
    val context = LocalContext.current
    val (screenW, screenH) = remember { screenSize(context) }
    var fixed by remember { mutableStateOf(settings.fixedResolution) }
    var percent by remember { mutableIntStateOf(Math.round(settings.renderScale * 100)) }
    var percentText by remember { mutableStateOf(percent.toString()) }
    var widthText by remember { mutableStateOf(settings.fixedWidth.toString()) }
    var heightText by remember { mutableStateOf(settings.fixedHeight.toString()) }
    var version by remember { mutableIntStateOf(0) }
    fun applyPercent(p: Int) {
        val v = p.coerceIn(10, 100)
        percent = v
        percentText = v.toString()
        settings.renderScale = v / 100f
        version++
        onChange(Change.RESOLUTION)
    }
    fun applyFixed() {
        val w = widthText.toIntOrNull()?.coerceIn(64, 7680) ?: settings.fixedWidth
        val h = heightText.toIntOrNull()?.coerceIn(64, 4320) ?: settings.fixedHeight
        settings.fixedWidth = w
        settings.fixedHeight = h
        widthText = w.toString()
        heightText = h.toString()
        version++
        onChange(Change.RESOLUTION)
    }
    PojarushCard {
        CardTitle(stringResource(R.string.graphics_resolution))
        Hint(stringResource(R.string.graphics_screen, screenW, screenH))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            FilterChip(selected = !fixed, onClick = {
                fixed = false
                settings.fixedResolution = false
                version++
                onChange(Change.RESOLUTION)
            }, label = { Text(stringResource(R.string.graphics_mode_percent)) })
            FilterChip(selected = fixed, onClick = {
                fixed = true
                settings.fixedResolution = true
                version++
                onChange(Change.RESOLUTION)
            }, label = { Text(stringResource(R.string.graphics_mode_fixed)) })
        }
        if (!fixed) {
            SettingSlider(stringResource(R.string.graphics_mode_percent), percent.toFloat(), 10f..100f, 1f, { "${it.toInt()}%" }) { applyPercent(it.toInt()) }
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = percentText,
                    onValueChange = { t ->
                        percentText = t.filter(Char::isDigit).take(3)
                        percentText.toIntOrNull()?.takeIf { it in 10..100 }?.let {
                            percent = it
                            settings.renderScale = it / 100f
                            version++
                            onChange(Change.RESOLUTION)
                        }
                    },
                    modifier = Modifier.width(150.dp),
                    singleLine = true,
                    label = { Text("%") },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number, imeAction = ImeAction.Done),
                    keyboardActions = KeyboardActions(onDone = { percentText.toIntOrNull()?.let(::applyPercent) })
                )
            }
        } else {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = widthText,
                    onValueChange = { t ->
                        widthText = t.filter(Char::isDigit).take(4)
                        widthText.toIntOrNull()?.takeIf { it in 64..7680 }?.let {
                            settings.fixedWidth = it
                            version++
                            onChange(Change.RESOLUTION)
                        }
                    },
                    modifier = Modifier.width(120.dp),
                    singleLine = true,
                    label = { Text(stringResource(R.string.graphics_width)) },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number, imeAction = ImeAction.Next)
                )
                Text("×")
                OutlinedTextField(
                    value = heightText,
                    onValueChange = { t ->
                        heightText = t.filter(Char::isDigit).take(4)
                        heightText.toIntOrNull()?.takeIf { it in 64..4320 }?.let {
                            settings.fixedHeight = it
                            version++
                            onChange(Change.RESOLUTION)
                        }
                    },
                    modifier = Modifier.width(120.dp),
                    singleLine = true,
                    label = { Text(stringResource(R.string.graphics_height)) },
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number, imeAction = ImeAction.Done),
                    keyboardActions = KeyboardActions(onDone = { applyFixed() })
                )
            }
            Hint(stringResource(R.string.graphics_stretch))
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            listOf(1100 to 720, 1600 to 1080).forEach { (w, h) ->
                FilterChip(
                    selected = fixed && settings.fixedWidth == w && settings.fixedHeight == h,
                    onClick = {
                        fixed = true
                        settings.fixedResolution = true
                        widthText = w.toString()
                        heightText = h.toString()
                        applyFixed()
                    },
                    label = { Text(stringResource(R.string.graphics_rec_stretch, w, h)) }
                )
            }
        }
        val (rw, rh) = remember(version) { settings.renderSize(screenW, screenH) }
        Text(stringResource(R.string.graphics_render, rw, rh), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.primary)
        Hint(stringResource(R.string.graphics_more))
    }
}

@Composable
fun GraphicsApiSettings(settings: ControlSettings) {
    var api by remember { mutableStateOf(settings.graphicsApi) }
    PojarushCard {
        CardTitle(stringResource(R.string.graphics_api_title))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            listOf(
                "auto" to R.string.graphics_api_auto,
                "vulkan" to R.string.graphics_api_vulkan,
                "opengl" to R.string.graphics_api_opengl,
            ).forEach { (id, label) ->
                FilterChip(selected = api == id, onClick = {
                    api = id
                    settings.graphicsApi = id
                }, label = { Text(stringResource(label)) })
            }
        }
        Hint(stringResource(R.string.graphics_api_desc))
    }
}
