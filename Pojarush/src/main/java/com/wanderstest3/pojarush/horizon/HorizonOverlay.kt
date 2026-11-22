package com.wanderstest3.pojarush.horizon

import android.annotation.SuppressLint
import android.content.Context
import android.os.Handler
import android.os.Looper
import android.text.Editable
import android.text.InputType
import android.text.TextWatcher
import android.view.KeyCharacterMap
import android.view.KeyEvent
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import android.widget.EditText
import android.widget.FrameLayout
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.wanderstest3.pojarush.R
import com.wanderstest3.pojarush.controls.ControlLayouts
import com.wanderstest3.pojarush.controls.ControlSettings
import com.wanderstest3.pojarush.ui.Change
import com.wanderstest3.pojarush.ui.ControlEditor
import com.wanderstest3.pojarush.ui.ControlsSettings
import com.wanderstest3.pojarush.ui.LayoutSettings
import com.wanderstest3.pojarush.ui.MouseSettings
import com.wanderstest3.pojarush.ui.PojarushTheme
import com.wanderstest3.pojarush.ui.ResolutionSettings

@SuppressLint("ViewConstructor")
class HorizonOverlay(private val activity: HorizonActivity) : FrameLayout(activity), ControlsHost {
    private enum class Panel { NONE, SETTINGS, EDITOR, SEND_KEY }

    private val uiHandler = Handler(Looper.getMainLooper())
    private val settings = ControlSettings(activity)
    private val defaultName = activity.getString(R.string.controls_default_name)
    private val controls = ControlsView(activity, this, settings, ControlLayouts.load(activity, settings.layout, defaultName))
    private val keyMap: KeyCharacterMap = KeyCharacterMap.load(KeyCharacterMap.VIRTUAL_KEYBOARD)
    private var panel by mutableStateOf(Panel.NONE)
    private var confirmClose by mutableStateOf(false)
    private var mouseOn by mutableStateOf(controls.virtualMouse)
    private var hiddenOn by mutableStateOf(settings.controlsHidden)

    private val textInput = GameTextInput(
        context,
        onText = { typeText(it) },
        onDelete = { activity.tapKey(KeyEvent.KEYCODE_DEL) },
        onKey = { event -> activity.dispatchKeyToGame(event) },
        onEnter = {
            activity.tapKey(KeyEvent.KEYCODE_ENTER)
            hideKeyboard()
        }
    )

    private val poll = object : Runnable {
        override fun run() {
            if (panel != Panel.EDITOR) controls.setCaptured(HorizonBridge.isCaptured())
            controls.syncPointerCapture()
            controls.menuHold = panel != Panel.NONE
            controls.typing = HorizonBridge.isTyping()
            activity.fieldBottom = if (controls.typing) HorizonBridge.fieldBottom() else 1f
            controls.resumeIfPossible()
            uiHandler.postDelayed(this, 100)
        }
    }

    init {
        controls.onHideControls = {
            settings.controlsHidden = !settings.controlsHidden
            hiddenOn = settings.controlsHidden
            controls.invalidate()
        }
        addView(textInput, LayoutParams(1, 1))
        addView(controls, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT))
        addView(ComposeView(context).apply { setContent { PojarushTheme { OverlayUi() } } }, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT))
    }

    override fun onAttachedToWindow() {
        super.onAttachedToWindow()
        uiHandler.post(poll)
    }

    override fun onDetachedFromWindow() {
        uiHandler.removeCallbacksAndMessages(null)
        super.onDetachedFromWindow()
    }

    override fun sendKey(code: Int, down: Boolean) = activity.sendKey(code, down)

    fun mouseButton(button: Int, down: Boolean) = controls.mouseButton(button, down)

    fun back(): Boolean = when (panel) {
        Panel.NONE -> false
        Panel.EDITOR -> {
            closeEditor()
            true
        }
        else -> {
            panel = Panel.NONE
            true
        }
    }

    override fun toggleMenu() {
        if (panel == Panel.EDITOR) return
        controls.pause()
        mouseOn = controls.virtualMouse
        hiddenOn = settings.controlsHidden
        panel = if (panel == Panel.NONE) Panel.SETTINGS else Panel.NONE
    }

    private fun apply(change: Change) {
        when (change) {
            Change.LAYOUT -> controls.setLayout(ControlLayouts.load(activity, settings.layout, defaultName))
            Change.CONTROLS, Change.MOUSE -> controls.reloadSettings()
            Change.MENU_BUTTON -> {
                controls.reloadSettings()
                controls.relayout()
            }
            Change.RESOLUTION -> activity.applyResolution()
        }
        mouseOn = controls.virtualMouse
        hiddenOn = settings.controlsHidden
    }

    private fun openEditor() {
        panel = Panel.EDITOR
        controls.setEditMode(true)
    }

    private fun closeEditor() {
        controls.setEditMode(false)
        controls.setLayout(ControlLayouts.load(activity, settings.layout, defaultName))
        panel = Panel.NONE
    }

    @Composable
    private fun OverlayUi() {
        when (panel) {
            Panel.NONE -> {}
            Panel.SETTINGS -> SettingsPanel()
            Panel.SEND_KEY -> SendKeyPanel()
            Panel.EDITOR -> ControlEditor(controls, settings) { closeEditor() }
        }
        if (confirmClose) {
            androidx.compose.material3.AlertDialog(
                onDismissRequest = { confirmClose = false },
                title = { androidx.compose.material3.Text(stringResource(R.string.overlay_close_game)) },
                text = { androidx.compose.material3.Text(stringResource(R.string.overlay_close_game_confirm)) },
                confirmButton = {
                    androidx.compose.material3.TextButton(onClick = {
                        confirmClose = false
                        activity.finishAndRemoveTask()
                    }) { androidx.compose.material3.Text(stringResource(R.string.overlay_close_game)) }
                },
                dismissButton = {
                    androidx.compose.material3.TextButton(onClick = { confirmClose = false }) {
                        androidx.compose.material3.Text(stringResource(android.R.string.cancel))
                    }
                },
            )
        }
    }

    @Composable
    private fun QuickChip(label: String, selected: Boolean = false, onClick: () -> Unit) {
        FilterChip(selected = selected, onClick = onClick, label = { Text(label, maxLines = 1, modifier = Modifier.basicMarquee()) })
    }

    @Composable
    private fun SettingsPanel() {
        var tab by remember { mutableIntStateOf(0) }
        Box(
            modifier = Modifier.fillMaxSize().background(Color(0x99000000)).clickable(
                interactionSource = remember { MutableInteractionSource() },
                indication = null
            ) {},
            contentAlignment = Alignment.Center
        ) {
            Column(
                modifier = Modifier.fillMaxWidth(0.86f).fillMaxHeight(0.92f).clip(RoundedCornerShape(18.dp))
                    .background(MaterialTheme.colorScheme.background).padding(12.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    listOf(R.string.nav_controls, R.string.nav_mouse, R.string.nav_graphics, R.string.nav_music).forEachIndexed { i, label ->
                        FilterChip(selected = tab == i, onClick = { tab = i }, label = { Text(stringResource(label)) })
                    }
                    Box(modifier = Modifier.weight(1f))
                    TextButton(onClick = { panel = Panel.NONE }) { Text(stringResource(R.string.overlay_close)) }
                }
                Row(
                    modifier = Modifier.horizontalScroll(rememberScrollState()),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    QuickChip(stringResource(if (mouseOn) R.string.overlay_mouse_on else R.string.overlay_mouse_off), mouseOn) {
                        controls.setVirtualMouse(!controls.virtualMouse)
                        mouseOn = controls.virtualMouse
                    }
                    QuickChip(stringResource(R.string.overlay_keyboard)) {
                        panel = Panel.NONE
                        toggleKeyboard()
                    }
                    QuickChip(stringResource(R.string.overlay_send_key)) {
                        panel = Panel.SEND_KEY
                    }
                }
                Row(
                    modifier = Modifier.horizontalScroll(rememberScrollState()),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    QuickChip(stringResource(R.string.overlay_close_game)) {
                        confirmClose = true
                    }
                    QuickChip(stringResource(R.string.overlay_hide_controls), hiddenOn) {
                        settings.controlsHidden = !settings.controlsHidden
                        hiddenOn = settings.controlsHidden
                        controls.invalidate()
                    }
                }
                Column(
                    modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    when (tab) {
                        0 -> {
                            LayoutSettings(settings, onEdit = { openEditor() }, onChange = ::apply)
                            ControlsSettings(settings, ::apply)
                        }
                        1 -> MouseSettings(settings, ::apply)
                        3 -> MusicSettings()
                        else -> {
                            ResolutionSettings(settings, ::apply)
                        }
                    }
                }
            }
        }
    }

    @Composable
    private fun MusicSettings() {
        val music = activity.music
        var enabled by remember { mutableStateOf(settings.gameMusic) }
        var paused by remember { mutableStateOf(music.paused) }
        if (!music.available) {
            Text(stringResource(R.string.game_music_none), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(stringResource(R.string.game_music_enable), modifier = Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
            androidx.compose.material3.Switch(checked = enabled, onCheckedChange = {
                enabled = it
                settings.gameMusic = it
                music.refresh()
            })
        }
        if (enabled && music.available) {
            com.wanderstest3.pojarush.ui.SettingSlider(
                stringResource(R.string.game_music_volume), settings.gameMusicVolume, 0f..1f, 0.05f, { "${Math.round(it * 100)}%" }, default = 0.4f
            ) {
                settings.gameMusicVolume = it
                music.refresh()
            }
            QuickChip(stringResource(if (paused) R.string.game_music_play else R.string.game_music_pause), paused) {
                paused = !paused
                music.setPaused(paused)
            }
        }
    }

    @Composable
    private fun SendKeyPanel() {
        val names = remember {
            com.wanderstest3.pojarush.controls.ActionCatalog.all(context).associate { (name, action) -> action to name }
        }
        val chosen = remember { androidx.compose.runtime.mutableStateListOf<com.wanderstest3.pojarush.controls.Action.Key>() }
        Box(
            modifier = Modifier.fillMaxSize().background(Color(0x99000000)).clickable(
                interactionSource = remember { MutableInteractionSource() },
                indication = null
            ) { panel = Panel.SETTINGS },
            contentAlignment = Alignment.Center
        ) {
            Column(
                modifier = Modifier.fillMaxWidth(0.9f).fillMaxHeight(0.85f).clip(RoundedCornerShape(18.dp))
                    .background(MaterialTheme.colorScheme.background).padding(12.dp)
                    .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) {},
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text(stringResource(R.string.overlay_send_key), style = MaterialTheme.typography.titleMedium)
                        Text(
                            if (chosen.isEmpty()) stringResource(R.string.overlay_send_keys_hint)
                            else chosen.joinToString(" + ") { names[it] ?: "?" },
                            style = MaterialTheme.typography.bodySmall,
                            color = if (chosen.isEmpty()) MaterialTheme.colorScheme.onSurfaceVariant else Color(0xFFB98BDB)
                        )
                    }
                    TextButton(onClick = { chosen.clear() }, enabled = chosen.isNotEmpty()) {
                        Text(stringResource(R.string.overlay_send_keys_clear))
                    }
                    TextButton(onClick = {
                        val keys = chosen.map { it.code }
                        panel = Panel.NONE
                        sendCombo(keys)
                    }, enabled = chosen.isNotEmpty()) { Text(stringResource(R.string.overlay_send_keys_send)) }
                    TextButton(onClick = { panel = Panel.SETTINGS }) { Text(stringResource(R.string.overlay_close)) }
                }
                com.wanderstest3.pojarush.ui.VisualKeyboard(showSpecials = false, selected = chosen) { action ->
                    if (action is com.wanderstest3.pojarush.controls.Action.Key) {
                        if (action in chosen) chosen.remove(action)
                        else if (chosen.size < 4) chosen.add(action)
                    }
                }
            }
        }
    }

    private fun sendCombo(keys: List<Int>) {
        if (keys.isEmpty()) return
        val step = 40L
        var at = 120L
        keys.forEach { code ->
            uiHandler.postDelayed({ activity.sendKey(code, true) }, at)
            at += step
        }
        at += 60L
        keys.asReversed().forEach { code ->
            uiHandler.postDelayed({ activity.sendKey(code, false) }, at)
            at += step
        }
    }

    override fun toggleKeyboard() {
        val imm = context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
        if (textInput.hasFocus()) {
            imm.hideSoftInputFromWindow(textInput.windowToken, 0)
            textInput.clearFocus()
        } else {
            textInput.requestFocus()
            imm.restartInput(textInput)
            imm.showSoftInput(textInput, 0)
        }
    }

    private fun hideKeyboard() {
        val imm = context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
        imm.hideSoftInputFromWindow(textInput.windowToken, 0)
        textInput.clearFocus()
    }

    override fun sendText(text: String) {
        activity.tapKey(KeyEvent.KEYCODE_T)
        uiHandler.postDelayed({ HorizonBridge.command("type $text") }, 200)
        uiHandler.postDelayed({ activity.tapKey(KeyEvent.KEYCODE_ENTER) }, 400)
    }

    private fun typeText(text: String) {
        for (ch in text) {
            if (ch == '\n') {
                activity.tapKey(KeyEvent.KEYCODE_ENTER)
                continue
            }
            val events = keyMap.getEvents(charArrayOf(ch))
            if (events != null) {
                events.forEach { activity.dispatchKeyToGame(it) }
            } else {
                HorizonBridge.command("type $ch")
            }
        }
    }
}
