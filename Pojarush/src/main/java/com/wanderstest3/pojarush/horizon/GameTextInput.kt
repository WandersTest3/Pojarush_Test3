package com.wanderstest3.pojarush.horizon

import android.content.Context
import android.text.InputType
import android.view.KeyEvent
import android.view.View
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection

class GameTextInput(
    context: Context,
    private val onText: (String) -> Unit,
    private val onDelete: () -> Unit,
    private val onKey: (KeyEvent) -> Unit,
    private val onEnter: () -> Unit,
) : View(context) {
    init {
        alpha = 0f
        isFocusable = true
        isFocusableInTouchMode = true
    }

    override fun onCheckIsTextEditor(): Boolean = true

    override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection {
        outAttrs.inputType = InputType.TYPE_CLASS_TEXT or
            InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS or
            InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD
        outAttrs.imeOptions = EditorInfo.IME_FLAG_NO_EXTRACT_UI or EditorInfo.IME_FLAG_NO_FULLSCREEN or EditorInfo.IME_ACTION_SEND
        return Connection(this)
    }

    private inner class Connection(view: View) : BaseInputConnection(view, false) {
        private var composing = ""

        private fun replaceComposing(text: String) {
            var common = 0
            while (common < composing.length && common < text.length && composing[common] == text[common]) common++
            repeat(composing.length - common) { onDelete() }
            if (text.length > common) onText(text.substring(common))
            composing = text
        }

        override fun setComposingText(text: CharSequence?, newCursorPosition: Int): Boolean {
            replaceComposing(text?.toString().orEmpty())
            return true
        }

        override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
            replaceComposing(text?.toString().orEmpty())
            composing = ""
            return true
        }

        override fun finishComposingText(): Boolean {
            composing = ""
            return true
        }

        override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
            if (composing.isNotEmpty()) {
                repeat(composing.length) { onDelete() }
                composing = ""
                return true
            }
            repeat(beforeLength.coerceAtLeast(1)) { onDelete() }
            return true
        }

        override fun sendKeyEvent(event: KeyEvent): Boolean {
            when (event.keyCode) {
                KeyEvent.KEYCODE_ENTER -> if (event.action == KeyEvent.ACTION_UP) onEnter()
                else -> onKey(event)
            }
            return true
        }

        override fun performEditorAction(editorAction: Int): Boolean {
            composing = ""
            onEnter()
            return true
        }

        override fun getTextBeforeCursor(n: Int, flags: Int): CharSequence = ""
        override fun getTextAfterCursor(n: Int, flags: Int): CharSequence = ""
        override fun getSelectedText(flags: Int): CharSequence? = null
    }
}
