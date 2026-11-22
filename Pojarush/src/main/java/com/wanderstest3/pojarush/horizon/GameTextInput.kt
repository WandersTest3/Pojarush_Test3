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
        outAttrs.inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_AUTO_CORRECT
        outAttrs.initialSelStart = 0
        outAttrs.initialSelEnd = 0
        outAttrs.imeOptions = EditorInfo.IME_FLAG_NO_EXTRACT_UI or EditorInfo.IME_FLAG_NO_FULLSCREEN or EditorInfo.IME_ACTION_SEND
        return Connection(this)
    }

    private inner class Connection(view: View) : BaseInputConnection(view, false) {
        private var composing = ""
        private val before = StringBuilder()

        private fun typed(text: String) {
            before.append(text)
            if (before.length > 256) before.delete(0, before.length - 256)
        }

        private fun erased(count: Int) {
            before.setLength((before.length - count).coerceAtLeast(0))
        }

        private fun sync() {
            val at = before.length
            (this@GameTextInput.context.getSystemService(Context.INPUT_METHOD_SERVICE) as? android.view.inputmethod.InputMethodManager)
                ?.updateSelection(this@GameTextInput, at, at, if (composing.isEmpty()) -1 else at - composing.length, if (composing.isEmpty()) -1 else at)
        }

        private fun replaceComposing(text: String) {
            var common = 0
            while (common < composing.length && common < text.length && composing[common] == text[common]) common++
            repeat(composing.length - common) { onDelete() }
            erased(composing.length - common)
            if (text.length > common) {
                onText(text.substring(common))
                typed(text.substring(common))
            }
            composing = text
        }

        override fun setComposingText(text: CharSequence?, newCursorPosition: Int): Boolean {
            replaceComposing(text?.toString().orEmpty())
            sync()
            return true
        }

        override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
            replaceComposing(text?.toString().orEmpty())
            composing = ""
            sync()
            return true
        }

        override fun finishComposingText(): Boolean {
            composing = ""
            sync()
            return true
        }

        override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
            if (composing.isNotEmpty()) {
                repeat(composing.length) { onDelete() }
                erased(composing.length)
                composing = ""
                sync()
                return true
            }
            val count = beforeLength.coerceAtLeast(1)
            repeat(count) { onDelete() }
            erased(count)
            sync()
            return true
        }

        override fun sendKeyEvent(event: KeyEvent): Boolean {
            when (event.keyCode) {
                KeyEvent.KEYCODE_ENTER -> if (event.action == KeyEvent.ACTION_UP) {
                    before.setLength(0)
                    onEnter()
                }
                else -> {
                    if (event.keyCode == KeyEvent.KEYCODE_DEL && event.action == KeyEvent.ACTION_DOWN) erased(1)
                    onKey(event)
                }
            }
            return true
        }

        override fun performEditorAction(editorAction: Int): Boolean {
            composing = ""
            before.setLength(0)
            onEnter()
            return true
        }

        override fun getTextBeforeCursor(n: Int, flags: Int): CharSequence =
            before.substring((before.length - n.coerceAtLeast(0)).coerceAtLeast(0))

        override fun getTextAfterCursor(n: Int, flags: Int): CharSequence = ""
        override fun getSelectedText(flags: Int): CharSequence? = null
    }
}
