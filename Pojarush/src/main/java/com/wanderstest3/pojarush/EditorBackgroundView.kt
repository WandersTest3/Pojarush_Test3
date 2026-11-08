package com.wanderstest3.pojarush

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.LinearGradient
import android.graphics.Paint
import android.graphics.RadialGradient
import android.graphics.Shader
import android.os.SystemClock
import android.view.View
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin
import kotlin.random.Random

class EditorBackgroundView(context: Context) : View(context) {
    private enum class Shape { BLOCK, PIXEL, ORB, RING, SPARK, PLUS }

    private class Particle(
        var x: Float, var y: Float, var vx: Float, var vy: Float, var size: Float,
        var angle: Float, var spin: Float, var color: Int, var alpha: Float, var phase: Float, var shape: Int,
    )

    private val rng = Random(SystemClock.uptimeMillis())
    private val palette = intArrayOf(
        0xFF57197F.toInt(), 0xFF8E3CC8.toInt(), 0xFFB56BFF.toInt(), 0xFF2A0B40.toInt(),
        0xFFFFFFFF.toInt(), 0xFF8E8E8E.toInt(), 0xFF3A3A3A.toInt(), 0xFF6FE3FF.toInt(), 0xFFFFB86B.toInt(),
    )
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val bg = Paint()
    private val particles = ArrayList<Particle>()
    private var last = 0L
    private var hueShift = rng.nextFloat() * 360f
    private val blobs = Array(4) { floatArrayOf(rng.nextFloat(), rng.nextFloat(), rng.nextFloat() * 2f * PI.toFloat(), 0.1f + rng.nextFloat() * 0.2f) }

    private fun spawn(w: Float, h: Float, anywhere: Boolean): Particle {
        val shape = rng.nextInt(Shape.entries.size)
        val size = when (Shape.entries[shape]) {
            Shape.BLOCK -> 14f + rng.nextFloat() * 42f
            Shape.PIXEL -> 4f + rng.nextFloat() * 8f
            Shape.ORB -> 30f + rng.nextFloat() * 120f
            Shape.RING -> 18f + rng.nextFloat() * 60f
            Shape.SPARK -> 3f + rng.nextFloat() * 5f
            Shape.PLUS -> 8f + rng.nextFloat() * 16f
        } * resources.displayMetrics.density / 2f
        val dir = rng.nextFloat() * 2f * PI.toFloat()
        val speed = 8f + rng.nextFloat() * 40f
        val x = if (anywhere) rng.nextFloat() * w else if (rng.nextBoolean()) -size else w + size
        val y = if (anywhere) rng.nextFloat() * h else rng.nextFloat() * h
        return Particle(
            x, y, cos(dir) * speed, sin(dir) * speed - 6f, size,
            rng.nextFloat() * 360f, (rng.nextFloat() - 0.5f) * 90f, palette[rng.nextInt(palette.size)],
            0.15f + rng.nextFloat() * 0.55f, rng.nextFloat() * 6.28f, shape,
        )
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        particles.clear()
        val count = (w * h / 14000).coerceIn(40, 140)
        repeat(count) { particles += spawn(w.toFloat(), h.toFloat(), true) }
    }

    override fun onDraw(canvas: Canvas) {
        val now = SystemClock.uptimeMillis()
        val dt = if (last == 0L) 0f else ((now - last) / 1000f).coerceAtMost(0.05f)
        last = now
        val w = width.toFloat()
        val h = height.toFloat()
        val t = now / 1000f
        hueShift = (hueShift + dt * 6f) % 360f
        val top = Color.HSVToColor(floatArrayOf((275f + 25f * sin(t * 0.13f)) % 360f, 0.75f, 0.16f))
        val bottom = Color.HSVToColor(floatArrayOf((hueShift + 200f) % 360f, 0.55f, 0.10f))
        bg.shader = LinearGradient(0f, 0f, w * (0.5f + 0.5f * sin(t * 0.07f)), h, top, bottom, Shader.TileMode.CLAMP)
        canvas.drawRect(0f, 0f, w, h, bg)
        for (b in blobs) {
            b[2] += dt * b[3]
            val bx = w * (0.5f + 0.45f * sin(b[2] * 1.3f + b[0] * 6f))
            val by = h * (0.5f + 0.45f * cos(b[2] * 0.9f + b[1] * 6f))
            val r = minOf(w, h) * (0.35f + 0.15f * sin(b[2] * 2f))
            val c = Color.HSVToColor(110, floatArrayOf((hueShift + b[0] * 360f) % 360f, 0.8f, 0.6f))
            paint.shader = RadialGradient(bx, by, r, c, Color.TRANSPARENT, Shader.TileMode.CLAMP)
            canvas.drawCircle(bx, by, r, paint)
        }
        paint.shader = null
        for (i in particles.indices) {
            val p = particles[i]
            p.x += p.vx * dt
            p.y += p.vy * dt + sin(t + p.phase) * 6f * dt
            p.angle += p.spin * dt
            if (p.x < -p.size * 3 || p.x > w + p.size * 3 || p.y < -p.size * 3 || p.y > h + p.size * 3) {
                particles[i] = spawn(w, h, false)
                continue
            }
            val pulse = 0.65f + 0.35f * sin(t * 1.7f + p.phase)
            val a = (p.alpha * pulse * 255).toInt().coerceIn(0, 255)
            paint.color = Color.argb(a, Color.red(p.color), Color.green(p.color), Color.blue(p.color))
            canvas.save()
            canvas.translate(p.x, p.y)
            canvas.rotate(p.angle)
            val s = p.size
            when (Shape.entries[p.shape]) {
                Shape.BLOCK -> {
                    paint.style = Paint.Style.FILL
                    canvas.drawRect(-s / 2, -s / 2, s / 2, s / 2, paint)
                    paint.color = Color.argb(a / 2, 0, 0, 0)
                    canvas.drawRect(-s / 2, s / 6, s / 2, s / 2, paint)
                    paint.color = Color.argb(a / 3, 255, 255, 255)
                    canvas.drawRect(-s / 2, -s / 2, s / 2, -s / 3, paint)
                }
                Shape.PIXEL -> {
                    paint.style = Paint.Style.FILL
                    canvas.drawRect(-s / 2, -s / 2, s / 2, s / 2, paint)
                }
                Shape.ORB -> {
                    paint.style = Paint.Style.FILL
                    paint.color = Color.argb(a / 4, Color.red(p.color), Color.green(p.color), Color.blue(p.color))
                    canvas.drawCircle(0f, 0f, s / 2, paint)
                }
                Shape.RING -> {
                    paint.style = Paint.Style.STROKE
                    paint.strokeWidth = s / 10f
                    canvas.drawCircle(0f, 0f, s / 2, paint)
                }
                Shape.SPARK -> {
                    paint.style = Paint.Style.FILL
                    canvas.drawCircle(0f, 0f, s * pulse, paint)
                }
                Shape.PLUS -> {
                    paint.style = Paint.Style.FILL
                    canvas.drawRect(-s / 2, -s / 6, s / 2, s / 6, paint)
                    canvas.drawRect(-s / 6, -s / 2, s / 6, s / 2, paint)
                }
            }
            canvas.restore()
        }
        paint.style = Paint.Style.FILL
        postInvalidateOnAnimation()
    }
}
