package com.wanderstest3.pojarush.horizon

import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.Gravity
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.TextView
import com.wanderstest3.pojarush.EditorBackgroundView
import com.wanderstest3.pojarush.R

class LoadingCover(context: Context) : FrameLayout(context) {
    private val handler = Handler(Looper.getMainLooper())
    private val started = SystemClock.uptimeMillis()
    private val status: TextView
    private var done = false

    init {
        isClickable = true
        isFocusable = true
        setBackgroundColor(Color.BLACK)
        addView(EditorBackgroundView(context), LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT))
        val dp = resources.displayMetrics.density
        val font = runCatching { com.wanderstest3.pojarush.ui.launcherTypeface(context) }.getOrNull()
        val column = LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_HORIZONTAL
            setPadding((24 * dp).toInt(), (20 * dp).toInt(), (24 * dp).toInt(), (20 * dp).toInt())
            background = GradientDrawable().apply {
                cornerRadius = 16 * dp
                setColor(Color.argb(150, 0, 0, 0))
                setStroke((1 * dp).toInt().coerceAtLeast(1), Color.argb(200, 0x57, 0x19, 0x7f))
            }
        }
        column.addView(ImageView(context).apply {
            setImageResource(R.mipmap.ic_launcher)
        }, LinearLayout.LayoutParams((72 * dp).toInt(), (72 * dp).toInt()))
        column.addView(TextView(context).apply {
            text = context.getString(R.string.app_name)
            setTextColor(Color.WHITE)
            textSize = 20f
            typeface = font ?: Typeface.DEFAULT_BOLD
            gravity = Gravity.CENTER
            setPadding(0, (10 * dp).toInt(), 0, (4 * dp).toInt())
        })
        status = TextView(context).apply {
            text = context.getString(R.string.loading_game)
            setTextColor(Color.argb(220, 255, 255, 255))
            textSize = 14f
            typeface = font ?: Typeface.DEFAULT
            gravity = Gravity.CENTER
        }
        column.addView(status)
        column.addView(ProgressBar(context, null, android.R.attr.progressBarStyleHorizontal).apply {
            isIndeterminate = true
            indeterminateTintList = android.content.res.ColorStateList.valueOf(Color.rgb(0x9b, 0x4d, 0xd0))
        }, LinearLayout.LayoutParams((220 * dp).toInt(), ViewGroup.LayoutParams.WRAP_CONTENT).apply {
            topMargin = (12 * dp).toInt()
        })
        addView(column, LayoutParams(LayoutParams.WRAP_CONTENT, LayoutParams.WRAP_CONTENT, Gravity.CENTER))
    }

    fun start(onReady: () -> Unit) {
        val tick = object : Runnable {
            override fun run() {
                if (done) return
                val elapsed = SystemClock.uptimeMillis() - started
                if (HorizonBridge.isReady() || elapsed > MAX_WAIT_MS) {
                    done = true
                    (parent as? ViewGroup)?.removeView(this@LoadingCover)
                    onReady()
                    return
                }
                if (elapsed > SLOW_MS) status.text = context.getString(R.string.loading_game_slow)
                handler.postDelayed(this, POLL_MS)
            }
        }
        handler.post(tick)
    }

    override fun onDetachedFromWindow() {
        done = true
        handler.removeCallbacksAndMessages(null)
        super.onDetachedFromWindow()
    }

    private companion object {
        const val POLL_MS = 50L
        const val SLOW_MS = 25_000L
        const val MAX_WAIT_MS = 180_000L
    }
}
