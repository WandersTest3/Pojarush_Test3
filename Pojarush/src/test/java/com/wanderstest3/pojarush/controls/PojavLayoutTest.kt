package com.wanderstest3.pojarush.controls

import android.view.KeyEvent
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

class PojavLayoutTest {
    private val sample = """
        {
          "mControlDataList": [
            {
              "bgColor": 285212672, "cornerRadius": 30.0, "displayInGame": true, "displayInMenu": false,
              "dynamicX": "0.5 * ${'$'}{screen_width} - ${'$'}{width} / 2",
              "dynamicY": "${'$'}{screen_height} - ${'$'}{height} - ${'$'}{margin}",
              "height": 50.0, "width": 40.0, "isSwipeable": true, "isToggle": false,
              "keycodes": [87, 68, 0, 0], "name": "", "opacity": 1.0, "passThruEnabled": false,
              "strokeColor": -8487298, "strokeWidth": 0.6
            },
            {
              "dynamicX": "0.0 * ${'$'}{screen_width} + (px(20.0) / 100 * ${'$'}{preferred_scale}) + ${'$'}{margin}",
              "dynamicY": "0", "height": 30.0, "width": 30.0, "isToggle": true,
              "keycodes": [-4, 0, 0, 0], "name": "Use", "displayInGame": true, "displayInMenu": true
            },
            { "dynamicX": "10", "dynamicY": "10", "height": 30.0, "width": 30.0, "keycodes": [-9, 0, 0, 0], "name": "" }
          ],
          "mControlInfoDataList": { "name": "null" },
          "mDrawerDataList": [],
          "mJoystickDataList": [],
          "scaledAt": 100.0,
          "version": 8
        }
    """.trimIndent()

    @Test
    fun expressionsFollowPojavVariables() {
        val vars = mapOf("screen_width" to 2000f, "width" to 100f, "margin" to 4f)
        assertEquals(896f, Expression("0.5 * ${'$'}{screen_width} - ${'$'}{width} - ${'$'}{margin}", vars, 2f).value(), 0.001f)
        assertEquals(-60f, Expression("-(px(10) + 10) * 2", emptyMap(), 2f).value(), 0.001f)
        assertEquals(5f, Expression("dp(10)", emptyMap(), 2f).value(), 0.001f)
    }

    @Test
    fun pojavLayoutImports() {
        val l = parsePojav(sample, "Fallback", 2000f, 1000f, 2f)
        assertEquals("Fallback", l.name)
        assertEquals(3, l.buttons.size)
        val walk = l.buttons[0]
        assertEquals(listOf(Action.Key(KeyEvent.KEYCODE_W), Action.Key(KeyEvent.KEYCODE_D)), walk.press)
        assertTrue(walk.text.contains("+"))
        assertEquals(Visibility.IN_GAME, walk.visibility)
        assertTrue(walk.swipe)
        assertEquals(0.5f, walk.x, 0.001f)
        assertEquals(896f / 900f, walk.y, 0.001f)
        val use = l.buttons[1]
        assertEquals(listOf(Action.Mouse(1)), use.toggle)
        assertEquals("Use", use.text)
        assertEquals(44f / 1940f, use.x, 0.001f)
        assertEquals(listOf(Action.Menu), l.buttons[2].tap)
    }

    @Test
    fun realLayoutIfPresent() {
        val file = System.getenv("POJAV_LAYOUT")?.let(::File) ?: return
        val l = parsePojav(file.readText(), "real", 2400f, 1080f, 2.75f)
        assertTrue(l.buttons.isNotEmpty())
        l.buttons.forEach {
            assertTrue("${it.id} x=${it.x}", it.x in 0f..1f)
            assertTrue("${it.id} y=${it.y}", it.y in 0f..1f)
            println("${it.id} ${it.text} x=${it.x} y=${it.y} press=${it.press} tap=${it.tap} toggle=${it.toggle}")
        }
    }
}
