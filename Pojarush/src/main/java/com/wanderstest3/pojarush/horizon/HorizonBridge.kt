package com.wanderstest3.pojarush.horizon

object HorizonBridge {
    @JvmStatic external fun nativeCommand(line: String)
    @JvmStatic external fun nativeLook(dx: Float, dy: Float)
    @JvmStatic external fun nativeWheel(lines: Float)
    @JvmStatic external fun nativeCursor(x: Float, y: Float)
    @JvmStatic external fun nativeButton(button: Int, down: Boolean)
    @JvmStatic external fun nativeSetScale(x: Float, y: Float)
    @JvmStatic external fun nativeHotbarSlot(x: Float, y: Float): Int
    @JvmStatic external fun nativeIsCaptured(): Boolean
    @JvmStatic external fun nativeIsReady(): Boolean
    @JvmStatic external fun nativeSetMuted(muted: Boolean)
    @JvmStatic external fun nativeIsTyping(): Boolean

    fun command(line: String) = runCatching { nativeCommand(line) }
    fun look(dx: Float, dy: Float) = runCatching { nativeLook(dx, dy) }
    fun wheel(lines: Float) = runCatching { nativeWheel(lines) }
    fun cursor(x: Float, y: Float) = runCatching { nativeCursor(x, y) }
    fun button(button: Int, down: Boolean) = runCatching { nativeButton(button, down) }
    fun setScale(x: Float, y: Float) = runCatching { nativeSetScale(x, y) }
    fun hotbarSlot(x: Float, y: Float): Int = runCatching { nativeHotbarSlot(x, y) }.getOrDefault(-1)
    fun isCaptured(): Boolean = runCatching { nativeIsCaptured() }.getOrDefault(false)
    fun setMuted(muted: Boolean) { runCatching { nativeSetMuted(muted) } }
    fun isTyping(): Boolean = runCatching { nativeIsTyping() }.getOrDefault(false)
    fun isReady(): Boolean = runCatching { nativeIsReady() }.getOrDefault(false)
}
