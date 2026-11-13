package com.wanderstest3.pojarush.ui

import android.view.KeyEvent
import android.widget.Toast
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.offset
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.wanderstest3.pojarush.R
import com.wanderstest3.pojarush.controls.Action
import com.wanderstest3.pojarush.controls.ActionCatalog
import com.wanderstest3.pojarush.controls.ControlButton
import com.wanderstest3.pojarush.controls.ControlLayer
import com.wanderstest3.pojarush.controls.ControlLayout
import com.wanderstest3.pojarush.controls.ControlLayouts
import com.wanderstest3.pojarush.controls.ControlSettings
import com.wanderstest3.pojarush.controls.ControlStick
import com.wanderstest3.pojarush.controls.Size
import com.wanderstest3.pojarush.controls.Visibility
import com.wanderstest3.pojarush.horizon.ControlsView
import com.wanderstest3.pojarush.horizon.Selection
import java.util.UUID

private enum class Mode { HOLD, TAP, TOGGLE }

private fun isHoldable(a: Action) = a is Action.Key || a is Action.Mouse

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun ControlEditor(controls: ControlsView, settings: ControlSettings, onExit: () -> Unit) {
    val context = LocalContext.current
    var selection by remember { mutableStateOf(controls.selection) }
    var version by remember { mutableIntStateOf(0) }
    var dirty by remember { mutableStateOf(false) }
    var onLeft by remember { mutableStateOf(false) }
    var confirmExit by remember { mutableStateOf(false) }
    val savedText = stringResource(R.string.editor_saved, "%s")
    val suffix = stringResource(R.string.editor_suffix)
    val defaultName = stringResource(R.string.controls_default_name)
    var exportButton by remember { mutableStateOf<ControlButton?>(null) }
    var pickingImport by remember { mutableStateOf(false) }
    var importing by remember { mutableStateOf<ControlLayout?>(null) }
    var savedLayout by remember { mutableStateOf<ControlLayout?>(null) }
    var pendingImport by remember { mutableStateOf<ControlButton?>(null) }
    val exportedText = stringResource(R.string.editor_exported)

    DisposableEffect(controls) {
        controls.onSelectionChanged = { s ->
            val imp = importing
            if (imp != null) {
                if (s is Selection.Button) pendingImport = imp.buttons.getOrNull(s.index)
            } else {
                selection = s
                version++
                if (s != null) {
                    val w = controls.width.toFloat().coerceAtLeast(1f)
                    val cx = when (s) {
                        is Selection.Button -> controls.layout.buttons.getOrNull(s.index)?.x ?: 0.5f
                        is Selection.Stick -> controls.layout.sticks.getOrNull(s.index)?.x ?: 0.5f
                    }
                    onLeft = cx > 0.5f || w <= 0f
                }
            }
        }
        controls.onLayoutEdited = {
            if (importing == null) {
                dirty = true
                version++
            }
        }
        onDispose {
            controls.onSelectionChanged = null
            controls.onLayoutEdited = null
        }
    }

    fun layerId(layout: ControlLayout): Pair<ControlLayout, String> {
        val existing = layout.layers.firstOrNull { it.visible }?.id ?: layout.layers.firstOrNull()?.id
        if (existing != null) return layout to existing
        val id = "pojarush"
        return layout.copy(layers = layout.layers + ControlLayer(id, true, Visibility.ALWAYS)) to id
    }

    fun density() = context.resources.displayMetrics.density

    fun dpOf(size: Size): Float =
        size.dp ?: (size.px(controls.width, controls.height, density()) / density())

    fun updateButton(index: Int, change: (ControlButton) -> ControlButton) {
        val l = controls.layout
        val b = l.buttons.getOrNull(index) ?: return
        controls.editLayout(l.copy(buttons = l.buttons.toMutableList().also { it[index] = change(b) }), Selection.Button(index))
    }

    fun updateStick(index: Int, change: (ControlStick) -> ControlStick) {
        val l = controls.layout
        val s = l.sticks.getOrNull(index) ?: return
        controls.editLayout(l.copy(sticks = l.sticks.toMutableList().also { it[index] = change(s) }), Selection.Stick(index))
    }

    var naming by remember { mutableStateOf(false) }
    var exitAfterSave by remember { mutableStateOf(false) }
    var panelX by remember { mutableFloatStateOf(0f) }
    var panelY by remember { mutableFloatStateOf(0f) }

    fun save(name: String) {
        val id = ControlLayouts.saveNamed(controls.layout, settings.layout, name)
        settings.layout = id
        dirty = false
        Toast.makeText(context, savedText.replace("%s", name), Toast.LENGTH_SHORT).show()
    }

    fun suggestedName(): String {
        val current = settings.layout
        return if (current.startsWith("file:")) ControlLayouts.nameOf(current, controls.layout.name)
        else "${controls.layout.name.ifBlank { defaultName }} $suffix"
    }

    if (naming) {
        NameDialog(stringResource(R.string.editor_save), suggestedName(), stringResource(R.string.editor_save), {
            naming = false
            exitAfterSave = false
        }) { name ->
            naming = false
            save(name)
            if (exitAfterSave) onExit()
        }
    }

    fun finishImport(add: ControlButton?) {
        val base = savedLayout
        importing = null
        pendingImport = null
        savedLayout = null
        if (base == null) return
        controls.setLayout(base)
        controls.select(null)
        if (add != null) {
            val (l, layer) = layerId(base)
            controls.editLayout(l.copy(buttons = l.buttons + add.copy(id = UUID.randomUUID().toString(), layer = layer)), Selection.Button(l.buttons.size))
        } else {
            controls.invalidate()
        }
    }

    exportButton?.let { btn ->
        LayoutPickerDialog(
            stringResource(R.string.editor_export),
            ControlLayouts.list(context, defaultName).filter { it.removable },
            { exportButton = null },
        ) { entry ->
            val target = ControlLayouts.load(context, entry.id, defaultName)
            val (tl, layer) = layerId(target)
            ControlLayouts.saveNamed(tl.copy(buttons = tl.buttons + btn.copy(id = UUID.randomUUID().toString(), layer = layer)), entry.id, target.name)
            exportButton = null
            Toast.makeText(context, exportedText, Toast.LENGTH_SHORT).show()
        }
    }

    if (pickingImport) {
        LayoutPickerDialog(
            stringResource(R.string.editor_import),
            ControlLayouts.list(context, defaultName),
            { pickingImport = false },
        ) { entry ->
            pickingImport = false
            savedLayout = controls.layout
            val other = ControlLayouts.load(context, entry.id, defaultName)
            importing = other
            controls.select(null)
            controls.setLayout(other)
            controls.invalidate()
        }
    }

    pendingImport?.let { b ->
        AlertDialog(
            onDismissRequest = { pendingImport = null; controls.select(null) },
            title = { Text(stringResource(R.string.editor_import_confirm)) },
            text = { ImportPreview(b) },
            confirmButton = { TextButton(onClick = { finishImport(b) }) { Text(stringResource(R.string.ok)) } },
            dismissButton = { TextButton(onClick = { pendingImport = null; controls.select(null) }) { Text(stringResource(R.string.cancel)) } },
        )
    }

    Box(modifier = Modifier.fillMaxSize()) {
        if (importing != null) {
            TextButton(
                onClick = { finishImport(null) },
                modifier = Modifier
                    .align(Alignment.BottomCenter)
                    .padding(12.dp)
                    .clip(RoundedCornerShape(14.dp))
                    .background(Color(0xDD1E1129)),
            ) { Text(stringResource(R.string.cancel)) }
        }
        Row(
            modifier = Modifier
                .align(Alignment.TopCenter)
                .padding(top = 6.dp)
                .offset { androidx.compose.ui.unit.IntOffset(panelX.toInt(), panelY.toInt()) }
                .pointerInput(Unit) {
                    detectDragGestures { change, drag ->
                        change.consume()
                        panelX += drag.x
                        panelY += drag.y
                    }
                }
                .clip(RoundedCornerShape(14.dp))
                .background(Color(0xDD1E1129))
                .padding(horizontal = 6.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(2.dp)
        ) {
            Text("⠿", color = Color.White.copy(alpha = 0.6f), modifier = Modifier.padding(horizontal = 4.dp))
            TextButton(onClick = {
                val (l, layer) = layerId(controls.layout)
                val b = ControlButton(
                    id = UUID.randomUUID().toString(), text = "A", layer = layer, x = 0.5f, y = 0.5f,
                    width = Size(56f), height = Size(56f), visibility = Visibility.IN_GAME,
                    press = listOf(Action.Key(KeyEvent.KEYCODE_A)),
                )
                controls.editLayout(l.copy(buttons = l.buttons + b), Selection.Button(l.buttons.size))
            }) { Text(stringResource(R.string.editor_add_button)) }
            TextButton(onClick = {
                val (l, layer) = layerId(controls.layout)
                val s = ControlStick(
                    id = UUID.randomUUID().toString(), layer = layer, x = 0.1f, y = 0.85f, size = Size(140f),
                    visibility = Visibility.IN_GAME,
                    directions = ControlLayout.wasd(KeyEvent.KEYCODE_W, KeyEvent.KEYCODE_S, KeyEvent.KEYCODE_A, KeyEvent.KEYCODE_D),
                    lock = listOf(Action.Key(KeyEvent.KEYCODE_CTRL_LEFT)), canLock = true, lockThreshold = 0.25f, deadZone = 0.15f,
                )
                controls.editLayout(l.copy(sticks = l.sticks + s), Selection.Stick(l.sticks.size))
            }) { Text(stringResource(R.string.editor_add_stick)) }
            TextButton(onClick = { pickingImport = true }) { Text(stringResource(R.string.editor_import)) }
            if (dirty) {
                Text(stringResource(R.string.editor_modified), color = MaterialTheme.colorScheme.secondary, style = MaterialTheme.typography.bodySmall, fontWeight = FontWeight.Bold)
            }
            TextButton(onClick = { naming = true }) {
                Text(stringResource(R.string.editor_save), fontWeight = FontWeight.Bold, color = if (dirty) MaterialTheme.colorScheme.secondary else MaterialTheme.colorScheme.primary)
            }
            TextButton(onClick = { if (dirty) confirmExit = true else onExit() }) { Text(stringResource(R.string.editor_exit)) }
        }
        if (confirmExit) {
            AlertDialog(
                onDismissRequest = { confirmExit = false },
                title = { Text(stringResource(R.string.editor_unsaved_title)) },
                text = { Text(stringResource(R.string.editor_unsaved_text)) },
                confirmButton = {
                    TextButton(onClick = {
                        confirmExit = false
                        exitAfterSave = true
                        naming = true
                    }) { Text(stringResource(R.string.editor_save_exit)) }
                },
                dismissButton = {
                    Row {
                        TextButton(onClick = { confirmExit = false }) { Text(stringResource(R.string.cancel)) }
                        TextButton(onClick = {
                            confirmExit = false
                            onExit()
                        }) { Text(stringResource(R.string.editor_discard), color = MaterialTheme.colorScheme.error) }
                    }
                }
            )
        }
        val current = selection.takeIf { importing == null }
        if (current != null) {
            Column(
                modifier = Modifier
                    .align(if (onLeft) Alignment.CenterStart else Alignment.CenterEnd)
                    .padding(8.dp)
                    .width(300.dp)
                    .fillMaxHeight()
                    .clip(RoundedCornerShape(16.dp))
                    .background(Color(0xEE1E1129))
                    .padding(10.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    TextButton(onClick = { onLeft = !onLeft }) { Text("⇆") }
                    Box(modifier = Modifier.weight(1f))
                    if (current is Selection.Button) {
                        TextButton(onClick = { exportButton = controls.layout.buttons.getOrNull(current.index) }) { Text(stringResource(R.string.editor_export)) }
                    }
                    Box(
                        modifier = Modifier
                            .border(1.dp, Color.White.copy(alpha = 0.5f), RoundedCornerShape(6.dp))
                            .clip(RoundedCornerShape(6.dp))
                    ) {
                        TextButton(onClick = { controls.select(null) }) { Text("✕") }
                    }
                }
                Column(
                    modifier = Modifier.fillMaxWidth().weight(1f).verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                when (current) {
                    is Selection.Button -> controls.layout.buttons.getOrNull(current.index)?.let { b ->
                        ButtonEditor(b, version, ::dpOf, onChange = { updateButton(current.index, it) }, onDelete = {
                            val l = controls.layout
                            controls.editLayout(l.copy(buttons = l.buttons.filterIndexed { i, _ -> i != current.index }), null)
                        }, onDuplicate = {
                            val l = controls.layout
                            val copy = b.copy(id = UUID.randomUUID().toString(), x = (b.x + 0.05f).coerceAtMost(1f), y = (b.y + 0.05f).coerceAtMost(1f))
                            controls.editLayout(l.copy(buttons = l.buttons + copy), Selection.Button(l.buttons.size))
                        })
                    }
                    is Selection.Stick -> controls.layout.sticks.getOrNull(current.index)?.let { st ->
                        StickEditor(st, version, ::dpOf, onChange = { updateStick(current.index, it) }, onDelete = {
                            val l = controls.layout
                            controls.editLayout(l.copy(sticks = l.sticks.filterIndexed { i, _ -> i != current.index }), null)
                        })
                    }
                }
                }
            }
        }
    }
}

@Composable
private fun VisibilityChips(value: Visibility, onChange: (Visibility) -> Unit) {
    Text(stringResource(R.string.editor_visibility), style = MaterialTheme.typography.bodyMedium)
    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        listOf(Visibility.ALWAYS to R.string.vis_always, Visibility.IN_GAME to R.string.vis_game, Visibility.IN_MENU to R.string.vis_menu).forEach { (v, label) ->
            FilterChip(selected = value == v, onClick = { onChange(v) }, label = { Text(stringResource(label)) })
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun ButtonEditor(
    b: ControlButton,
    version: Int,
    dpOf: (Size) -> Float,
    onChange: ((ControlButton) -> ControlButton) -> Unit,
    onDelete: () -> Unit,
    onDuplicate: () -> Unit,
) {
    val context = LocalContext.current
    var text by remember(b.id) { mutableStateOf(b.text) }
    val all = b.press + b.tap + b.toggle
    val mode = when {
        b.toggle.isNotEmpty() -> Mode.TOGGLE
        b.tap.isNotEmpty() && b.press.isEmpty() -> Mode.TAP
        else -> Mode.HOLD
    }
    fun withActions(actions: List<Action>, m: Mode): (ControlButton) -> ControlButton = { cb ->
        when (m) {
            Mode.HOLD -> cb.copy(press = actions, tap = emptyList(), toggle = emptyList())
            Mode.TAP -> cb.copy(press = emptyList(), tap = actions, toggle = emptyList())
            Mode.TOGGLE -> cb.copy(press = actions.filterNot(::isHoldable), tap = emptyList(), toggle = actions.filter(::isHoldable))
        }
    }
    var kbOpen by remember { mutableStateOf(false) }

    OutlinedTextField(
        value = text,
        onValueChange = { v -> text = v.take(60); onChange { it.copy(text = text) } },
        modifier = Modifier.fillMaxWidth(),
        singleLine = false,
        maxLines = 3,
        label = { Text(stringResource(R.string.editor_text)) }
    )
    val prefs = remember { com.wanderstest3.pojarush.controls.ControlSettings(context) }
    var mirror by remember { mutableStateOf(prefs.mirrorSize) }
    val w = dpOf(b.width)
    val h = dpOf(b.height)
    val imageRes = remember(b.style?.image) {
        b.style?.image?.takeIf { it.isNotEmpty() }?.let { ref ->
            com.wanderstest3.pojarush.controls.HudImages.decode(ref)?.let { it.width to it.height }
        }
    }
    SettingNumber(stringResource(R.string.editor_width), w, 8f..400f, 1f, "dp") { v ->
        onChange { if (mirror) it.copy(width = Size(v), height = Size(v)) else it.copy(width = Size(v)) }
    }
    imageRes?.let { (iw, ih) ->
        val ideal = if (ih > 0) w / iw * ih else w
        if (kotlin.math.abs(ideal - h) > 0.5f) {
            Text(
                stringResource(R.string.editor_distortion_height, ideal.toInt()),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.secondary,
                modifier = Modifier.padding(start = 4.dp)
            )
        }
    }
    SettingNumber(stringResource(R.string.editor_height), h, 8f..400f, 1f, "dp") { v ->
        onChange { if (mirror) it.copy(width = Size(v), height = Size(v)) else it.copy(height = Size(v)) }
    }
    imageRes?.let { (iw, ih) ->
        val ideal = if (iw > 0) h / ih * iw else h
        if (kotlin.math.abs(ideal - w) > 0.5f) {
            Text(
                stringResource(R.string.editor_distortion_width, ideal.toInt()),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.secondary,
                modifier = Modifier.padding(start = 4.dp)
            )
        }
    }
    SettingSwitch(stringResource(R.string.editor_mirror_size), mirror, stringResource(R.string.editor_mirror_size_desc)) {
        mirror = it
        prefs.mirrorSize = it
    }
    val screenW = remember { context.resources.displayMetrics.widthPixels }
    val screenH = remember { context.resources.displayMetrics.heightPixels }
    val density = remember { context.resources.displayMetrics.density }
    var xText by remember(b.id) { mutableStateOf(kotlin.math.round(b.x * screenW / density).toString()) }
    var yText by remember(b.id) { mutableStateOf(kotlin.math.round(b.y * screenH / density).toString()) }
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        OutlinedTextField(
            value = xText,
            onValueChange = { t ->
                xText = t.filter { c -> c.isDigit() }.take(5)
                xText.toFloatOrNull()?.let { px ->
                    val fx = (px * density / screenW).coerceIn(0f, 1f)
                    onChange { it.copy(x = fx) }
                }
            },
            modifier = Modifier.weight(1f),
            singleLine = true,
            label = { Text("X (dp)") },
            keyboardOptions = androidx.compose.foundation.text.KeyboardOptions(keyboardType = androidx.compose.ui.text.input.KeyboardType.Number, imeAction = androidx.compose.ui.text.input.ImeAction.Next)
        )
        OutlinedTextField(
            value = yText,
            onValueChange = { t ->
                yText = t.filter { c -> c.isDigit() }.take(5)
                yText.toFloatOrNull()?.let { py ->
                    val fy = (py * density / screenH).coerceIn(0f, 1f)
                    onChange { it.copy(y = fy) }
                }
            },
            modifier = Modifier.weight(1f),
            singleLine = true,
            label = { Text("Y (dp)") },
            keyboardOptions = androidx.compose.foundation.text.KeyboardOptions(keyboardType = androidx.compose.ui.text.input.KeyboardType.Number, imeAction = androidx.compose.ui.text.input.ImeAction.Done)
        )
    }
    Text(stringResource(R.string.editor_mode), style = MaterialTheme.typography.bodyMedium)
    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        listOf(Mode.HOLD to R.string.editor_mode_hold, Mode.TOGGLE to R.string.editor_mode_toggle).forEach { (m, label) ->
            FilterChip(selected = mode == m, onClick = { onChange(withActions(all, m)) }, label = { Text(stringResource(label)) })
        }
    }
    Text(stringResource(R.string.editor_actions), style = MaterialTheme.typography.bodyMedium)
    FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        all.forEachIndexed { i, a ->
            AssistChip(onClick = { onChange(withActions(all.filterIndexed { j, _ -> j != i }, mode)) }, label = { Text("${ActionCatalog.label(context, a)}  ×") })
        }
        AssistChip(onClick = { kbOpen = true }, label = { Text(stringResource(R.string.editor_add_action)) })
        if (kbOpen) {
            ActionPickerDialog(context, all) { action ->
                kbOpen = false
                if (action != null && action !in all) onChange(withActions(all + action, mode))
            }
        }
    }
    VisibilityChips(b.visibility) { v -> onChange { it.copy(visibility = v) } }
    HudImagePicker(stringResource(R.string.hud_image), b.style?.image.orEmpty()) { ref ->
        onChange { it.copy(style = (it.style ?: com.wanderstest3.pojarush.controls.HudImages.DEFAULT_STYLE).copy(image = ref)) }
    }
    HudImagePicker(stringResource(R.string.hud_image_pressed), b.style?.imagePressed.orEmpty()) { ref ->
        onChange { it.copy(style = (it.style ?: com.wanderstest3.pojarush.controls.HudImages.DEFAULT_STYLE).copy(imagePressed = ref)) }
    }
    BackgroundColorPicker(b.style ?: com.wanderstest3.pojarush.controls.HudImages.DEFAULT_STYLE) { st ->
        onChange { it.copy(style = st) }
    }
    SettingSlider(
        stringResource(R.string.editor_opacity),
        (b.style ?: com.wanderstest3.pojarush.controls.HudImages.DEFAULT_STYLE).opacity,
        0.05f..1f, 0.05f, { "${Math.round(it * 100)}%" },
    ) { v ->
        onChange { it.copy(style = (it.style ?: com.wanderstest3.pojarush.controls.HudImages.DEFAULT_STYLE).copy(opacity = v)) }
    }
    SettingSwitch(stringResource(R.string.editor_swipe), b.swipe) { v -> onChange { it.copy(swipe = v) } }
    SettingSwitch(stringResource(R.string.editor_penetrable), b.penetrable) { v -> onChange { it.copy(penetrable = v) } }
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        OutlinedButton(onClick = onDuplicate) { Text(stringResource(R.string.editor_duplicate)) }
        OutlinedButton(onClick = onDelete) { Text(stringResource(R.string.editor_delete), color = MaterialTheme.colorScheme.error) }
    }
}

@Composable
private fun StickEditor(
    s: ControlStick,
    version: Int,
    dpOf: (Size) -> Float,
    onChange: ((ControlStick) -> ControlStick) -> Unit,
    onDelete: () -> Unit,
) {
    SettingNumber(stringResource(R.string.editor_size), dpOf(s.size), 30f..600f, 1f, "dp") { v -> onChange { it.copy(size = Size(v)) } }
    VisibilityChips(s.visibility) { v -> onChange { it.copy(visibility = v) } }
    HudImagePicker(stringResource(R.string.hud_stick_frame), s.style?.image.orEmpty()) { ref ->
        onChange { it.copy(style = (it.style ?: com.wanderstest3.pojarush.controls.HudImages.DEFAULT_STYLE).copy(image = ref)) }
    }
    HudImagePicker(stringResource(R.string.hud_stick_knob), s.style?.imagePressed.orEmpty()) { ref ->
        onChange { it.copy(style = (it.style ?: com.wanderstest3.pojarush.controls.HudImages.DEFAULT_STYLE).copy(imagePressed = ref)) }
    }
    SettingSlider(
        stringResource(R.string.editor_opacity),
        (s.style ?: com.wanderstest3.pojarush.controls.HudImages.DEFAULT_STYLE).opacity,
        0.05f..1f, 0.05f, { "${Math.round(it * 100)}%" },
    ) { v ->
        onChange { it.copy(style = (it.style ?: com.wanderstest3.pojarush.controls.HudImages.DEFAULT_STYLE).copy(opacity = v)) }
    }
    SettingSwitch(stringResource(R.string.editor_sprint_lock), s.canLock) { v -> onChange { it.copy(canLock = v, lock = if (v && it.lock.isEmpty()) listOf(Action.Key(KeyEvent.KEYCODE_CTRL_LEFT)) else it.lock) } }
    SettingSwitch(stringResource(R.string.editor_auto_run), s.autoRun, stringResource(R.string.editor_auto_run_desc)) { v -> onChange { it.copy(autoRun = v, lock = if (v && it.lock.isEmpty()) listOf(Action.Key(KeyEvent.KEYCODE_CTRL_LEFT)) else it.lock) } }
    OutlinedButton(onClick = onDelete) { Text(stringResource(R.string.editor_delete), color = MaterialTheme.colorScheme.error) }
}

@Composable
private fun ActionPickerDialog(context: android.content.Context, existing: List<Action>, onPick: (Action?) -> Unit) {
    AlertDialog(
        onDismissRequest = { onPick(null) },
        title = { Text(stringResource(R.string.editor_add_action)) },
        text = { VisualKeyboard(disabled = existing.toSet()) { onPick(it) } },
        confirmButton = {},
        dismissButton = { TextButton(onClick = { onPick(null) }) { Text(stringResource(R.string.cancel)) } }
    )
}

private val BACKGROUND_SWATCHES = listOf(
    0xFF57197F.toInt(), 0xFF000000.toInt(), 0xFF404040.toInt(), 0xFFFFFFFF.toInt(), 0xFFE53935.toInt(), 0xFFFB8C00.toInt(),
    0xFFFDD835.toInt(), 0xFF43A047.toInt(), 0xFF00ACC1.toInt(), 0xFF1E88E5.toInt(), 0xFF8E24AA.toInt(), 0xFFEC407A.toInt(),
)

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun BackgroundColorPicker(style: com.wanderstest3.pojarush.controls.Style, onChange: (com.wanderstest3.pojarush.controls.Style) -> Unit) {
    val alpha = android.graphics.Color.alpha(style.fill)
    val transparent = alpha == 0
    fun apply(rgb: Int, a: Int): com.wanderstest3.pojarush.controls.Style {
        val fill = android.graphics.Color.argb(a, android.graphics.Color.red(rgb), android.graphics.Color.green(rgb), android.graphics.Color.blue(rgb))
        val pressed = if (a == 0) android.graphics.Color.argb(80, 255, 255, 255)
        else android.graphics.Color.argb((a + 90).coerceAtMost(255), android.graphics.Color.red(rgb), android.graphics.Color.green(rgb), android.graphics.Color.blue(rgb))
        return style.copy(fill = fill, fillPressed = pressed)
    }
    val rgb = if (transparent) BACKGROUND_SWATCHES[0] else style.fill
    Text(stringResource(R.string.editor_background), style = MaterialTheme.typography.bodyMedium)
    androidx.compose.foundation.layout.FlowRow(
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        androidx.compose.material3.FilterChip(
            selected = transparent,
            onClick = { onChange(apply(rgb, 0)) },
            label = { Text(stringResource(R.string.editor_transparent), maxLines = 1) },
        )
        BACKGROUND_SWATCHES.forEach { c ->
            val selected = !transparent && (style.fill and 0xFFFFFF) == (c and 0xFFFFFF)
            androidx.compose.foundation.layout.Box(
                modifier = Modifier
                    .size(32.dp)
                    .clip(androidx.compose.foundation.shape.CircleShape)
                    .background(androidx.compose.ui.graphics.Color(c))
                    .border(if (selected) 3.dp else 1.dp, if (selected) androidx.compose.ui.graphics.Color.Yellow else androidx.compose.ui.graphics.Color.White.copy(alpha = 0.5f), androidx.compose.foundation.shape.CircleShape)
                    .clickable { onChange(apply(c, if (transparent) 150 else alpha)) },
            )
        }
    }
    if (!transparent) {
        SettingSlider(stringResource(R.string.editor_background_alpha), alpha / 255f, 0.05f..1f, 0.05f, { "${(it * 100).toInt()}%" }) { v ->
            onChange(apply(style.fill, (v * 255).toInt().coerceIn(1, 255)))
        }
    }
}

@Composable
private fun LayoutPickerDialog(title: String, entries: List<ControlLayouts.Entry>, onDismiss: () -> Unit, onPick: (ControlLayouts.Entry) -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            Column(modifier = Modifier.heightIn(max = 280.dp).verticalScroll(rememberScrollState())) {
                entries.forEach { e ->
                    TextButton(onClick = { onPick(e) }, modifier = Modifier.fillMaxWidth()) { Text(e.name) }
                }
            }
        },
        confirmButton = {},
        dismissButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) } },
    )
}

@Composable
private fun ImportPreview(b: ControlButton) {
    val st = b.style
    Box(
        modifier = Modifier
            .size(80.dp)
            .clip(RoundedCornerShape(minOf(st?.radiusDp ?: 8f, 40f).dp))
            .background(Color(st?.fill ?: 0x66FFFFFF)),
        contentAlignment = Alignment.Center,
    ) {
        Text(b.text, color = Color(st?.text ?: 0xFFFFFFFF.toInt()))
    }
}
