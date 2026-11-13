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
import com.wanderstest3.pojarush.ui.GraphicsApiSettings
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
    private val sentinel = " "
    private var panel by mutableStateOf(Panel.NONE)
    private var mouseOn by mutableStateOf(controls.virtualMouse)
    private var hiddenOn by mutableStateOf(settings.controlsHidden)

    private val textInput = EditText(context).apply {
        alpha = 0f
        isFocusableInTouchMode = true
        inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_AUTO_CORRECT
        imeOptions = EditorInfo.IME_FLAG_NO_EXTRACT_UI or EditorInfo.IME_ACTION_SEND
        setText(sentinel)
    }

    private val poll = object : Runnable {
        override fun run() {
            if (panel != Panel.EDITOR) controls.setCaptured(HorizonBridge.isCaptured())
            controls.menuHold = panel != Panel.NONE
            controls.typing = HorizonBridge.isTyping()
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
        setupTextInput()
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
                    listOf(R.string.nav_controls, R.string.nav_mouse, R.string.nav_graphics).forEachIndexed { i, label ->
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
                    QuickChip(stringResource(R.string.overlay_close_game)) {
                        activity.finishAndRemoveTask()
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
                        else -> {
                            ResolutionSettings(settings, ::apply)
                            GraphicsApiSettings(settings)
                        }
                    }
                }
            }
        }
    }

    @Composable
    private fun SendKeyPanel() {
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
                    Text(stringResource(R.string.overlay_send_key), style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
                    TextButton(onClick = { panel = Panel.SETTINGS }) { Text(stringResource(R.string.overlay_close)) }
                }
                com.wanderstest3.pojarush.ui.VisualKeyboard(showSpecials = false) { action ->
                    if (action is com.wanderstest3.pojarush.controls.Action.Key) {
                        activity.tapKey(action.code)
                    }
                }
            }
        }
    }

    override fun toggleKeyboard() {
        val imm = context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
        if (textInput.hasFocus()) {
            imm.hideSoftInputFromWindow(textInput.windowToken, 0)
            textInput.clearFocus()
        } else {
            textInput.requestFocus()
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

    private fun setupTextInput() {
        textInput.setOnEditorActionListener { _, _, _ ->
            activity.tapKey(KeyEvent.KEYCODE_ENTER)
            hideKeyboard()
            true
        }
        textInput.setOnKeyListener { _, keyCode, event ->
            if (keyCode == KeyEvent.KEYCODE_DEL || keyCode == KeyEvent.KEYCODE_ENTER) {
                activity.sendKey(keyCode, event.action == KeyEvent.ACTION_DOWN)
                if (keyCode == KeyEvent.KEYCODE_ENTER && event.action == KeyEvent.ACTION_UP) hideKeyboard()
                true
            } else false
        }
        textInput.addTextChangedListener(object : TextWatcher {
            private var busy = false
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) {}
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {}
            override fun afterTextChanged(s: Editable?) {
                if (busy || s == null) return
                val text = s.toString()
                busy = true
                when {
                    text.length < sentinel.length -> activity.tapKey(KeyEvent.KEYCODE_DEL)
                    text.length > sentinel.length -> typeText(text.substring(sentinel.length))
                }
                s.replace(0, s.length, sentinel)
                textInput.setSelection(sentinel.length)
                busy = false
            }
        })
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
