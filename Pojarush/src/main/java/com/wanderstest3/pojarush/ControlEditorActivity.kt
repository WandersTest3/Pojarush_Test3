package com.wanderstest3.pojarush

import android.animation.ArgbEvaluator
import android.animation.ValueAnimator
import android.graphics.drawable.GradientDrawable
import android.view.animation.LinearInterpolator
import android.os.Build
import android.os.Bundle
import android.view.ViewGroup
import android.view.WindowManager
import android.widget.FrameLayout
import androidx.activity.ComponentActivity
import androidx.compose.ui.platform.ComposeView
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import com.wanderstest3.pojarush.controls.ControlLayouts
import com.wanderstest3.pojarush.controls.ControlSettings
import com.wanderstest3.pojarush.horizon.ControlsHost
import com.wanderstest3.pojarush.horizon.ControlsView
import com.wanderstest3.pojarush.ui.ControlEditor
import com.wanderstest3.pojarush.ui.PojarushTheme

class ControlEditorActivity : ComponentActivity(), ControlsHost {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        Fullscreen.apply(this)
        val settings = ControlSettings(this)
        val controls = ControlsView(
            this,
            this,
            settings,
            ControlLayouts.load(this, settings.layout, getString(R.string.controls_default_name)),
            showHandle = false,
        )
        controls.setEditMode(true)
        val root = FrameLayout(this)
        root.addView(EditorBackgroundView(this), FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT))
        root.addView(controls, FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT))
        root.addView(ComposeView(this).apply {
            setContent { PojarushTheme { ControlEditor(controls, settings) { finish() } } }
        }, FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT))
        setContentView(root)
    }

    override fun sendKey(code: Int, down: Boolean) {}
    override fun toggleKeyboard() {}
    override fun toggleMenu() {}
    override fun sendText(text: String) {}
}

object Fullscreen {
    fun apply(activity: ComponentActivity) {
        val window = activity.window
        WindowCompat.setDecorFitsSystemWindows(window, false)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            window.attributes = window.attributes.apply {
                layoutInDisplayCutoutMode = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                    WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS
                } else {
                    WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
                }
            }
        }
        WindowInsetsControllerCompat(window, window.decorView).apply {
            hide(WindowInsetsCompat.Type.systemBars())
            systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        }
    }
}
