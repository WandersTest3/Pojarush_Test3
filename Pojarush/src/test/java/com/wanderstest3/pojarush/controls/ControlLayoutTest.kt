package com.wanderstest3.pojarush.controls

import android.view.KeyEvent
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

class ControlLayoutTest {
    private fun asset(name: String) = File("src/main/assets/controls/$name").readText()

    @Test
    fun zalithDefaultLayout() {
        val l = ControlLayout.parse(asset("zalith-default.json"), "z")
        assertEquals(5, l.layers.size)
        assertEquals(14, l.buttons.size)
        assertEquals(1, l.sticks.size)
        val esc = l.buttons.first { it.text == "Esc" }
        assertEquals(listOf(Action.Key(KeyEvent.KEYCODE_ESCAPE)), esc.press)
        val pri = l.buttons.first { it.text == "PRI" }
        assertEquals(listOf(Action.Mouse(0)), pri.press)
        val ime = l.buttons.first { it.text == "IME" }
        assertEquals(listOf(Action.Keyboard), ime.press)
        val gui = l.buttons.first { it.text == "GUI" }
        assertEquals(4, gui.press.count { it is Action.Layer })
        val toggle = l.buttons.first { it.text == "□" }
        assertTrue(toggle.toggle.isNotEmpty())
        val stick = l.sticks.single()
        assertEquals(listOf(Action.Key(KeyEvent.KEYCODE_W)), stick.directions[ControlLayout.NORTH])
        assertEquals(listOf(Action.Key(KeyEvent.KEYCODE_CTRL_LEFT)), stick.lock)
        assertEquals(0.1121f, stick.x, 0.0001f)
    }

    @Test
    fun foldCraftDefaultLayout() {
        val l = ControlLayout.parse(asset("fcl-default.json"), "f")
        assertEquals("Default", l.name)
        assertEquals(12, l.buttons.size)
        assertEquals(1, l.sticks.size)
        val attack = l.buttons.first { it.text == "×" }
        assertEquals(listOf(Action.Mouse(0)), attack.press)
        assertTrue(attack.penetrable)
        val sneak = l.buttons.first { it.text == "▼" }
        assertEquals(listOf(Action.Key(KeyEvent.KEYCODE_SHIFT_LEFT)), sneak.toggle)
        val q = l.buttons.first { it.text == "Q" }
        assertEquals(listOf(Action.Key(KeyEvent.KEYCODE_Q)), q.tap)
        val input = l.buttons.first { it.text == "Input" }
        assertEquals(listOf(Action.Keyboard), input.tap)
        val stick = l.sticks.single()
        assertEquals(listOf(Action.Key(KeyEvent.KEYCODE_D)), stick.directions[ControlLayout.EAST])
        assertEquals(listOf(Action.Key(KeyEvent.KEYCODE_SHIFT_LEFT)), stick.doubleTap)
        assertEquals(0.45f, stick.size.fraction, 0.0001f)
    }

    @Test
    fun keycodeTables() {
        assertEquals(KeyEvent.KEYCODE_SPACE, Keycodes.fromGlfwName("GLFW_KEY_SPACE"))
        assertEquals(KeyEvent.KEYCODE_F5, Keycodes.fromGlfwName("GLFW_KEY_F5"))
        assertEquals(KeyEvent.KEYCODE_W, Keycodes.fromEvdev(17))
        assertEquals(KeyEvent.KEYCODE_SHIFT_LEFT, Keycodes.fromEvdev(42))
        assertEquals(KeyEvent.KEYCODE_ESCAPE, Keycodes.fromEvdev(1))
    }

    @Test
    fun pojarushFormatRoundTrip() {
        for (name in listOf("zalith-default.json", "fcl-default.json")) {
            val original = ControlLayout.parse(asset(name), "x")
            val again = ControlLayout.parse(ControlLayout.write(original), "y")
            assertEquals(original.buttons.map { it.copy(style = null) }, again.buttons.map { it.copy(style = null) })
            assertEquals(original.sticks.map { it.copy(style = null, directions = it.directions.filterValues { v -> v.isNotEmpty() }) },
                again.sticks.map { it.copy(style = null, directions = it.directions.filterValues { v -> v.isNotEmpty() }) })
            assertEquals(original.layers, again.layers)
        }
    }
}
