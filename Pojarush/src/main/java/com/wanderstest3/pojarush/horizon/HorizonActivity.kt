package com.wanderstest3.pojarush.horizon

import android.os.Build
import android.os.Bundle
import android.os.ParcelFileDescriptor
import android.os.Process
import android.os.SystemClock
import android.system.Os
import android.view.KeyEvent
import android.view.ViewGroup
import android.view.WindowManager
import androidx.activity.OnBackPressedCallback
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import com.google.androidgamesdk.GameActivity
import com.wanderstest3.pojarush.controls.ControlSettings
import java.io.File

class HorizonActivity : GameActivity() {
    companion object {
        const val EXTRA_ARGS = "horizon_args"
        const val EXTRA_GAME_DIR = "horizon_game_dir"
        const val EXTRA_HOME = "horizon_home"
    }

    private var stdio: ParcelFileDescriptor? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        val args = intent.getStringArrayListExtra(EXTRA_ARGS).orEmpty()
        val gameDir = intent.getStringExtra(EXTRA_GAME_DIR).orEmpty()
        val home = intent.getStringExtra(EXTRA_HOME).orEmpty()
        File(home, "logs").mkdirs()
        Os.setenv("HORIZON_ANDROID_ARGS", args.joinToString("\u001f"), true)
        Os.setenv("POJARUSH_GAME_DIR", gameDir, true)
        Os.setenv("POJARUSH_EXIT_FILE", File(home, "exit-code").absolutePath, true)
        Os.setenv("HORIZON_HOME", home, true)
        Os.setenv("HOME", home, true)
        Os.setenv("TMPDIR", cacheDir.absolutePath, true)
        File(home, "exit-code").delete()
        runCatching {
            stdio = ParcelFileDescriptor.open(
                File(home, "logs/stdio.log"),
                ParcelFileDescriptor.MODE_CREATE or ParcelFileDescriptor.MODE_TRUNCATE or ParcelFileDescriptor.MODE_WRITE_ONLY
            ).also { pfd ->
                Os.dup2(pfd.fileDescriptor, 1)
                Os.dup2(pfd.fileDescriptor, 2)
            }
        }
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            window.attributes = window.attributes.apply {
                layoutInDisplayCutoutMode = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                    WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS
                } else {
                    WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
                }
            }
        }
        WindowCompat.setDecorFitsSystemWindows(window, false)
        super.onCreate(savedInstanceState)
        WindowInsetsControllerCompat(window, window.decorView).apply {
            hide(WindowInsetsCompat.Type.systemBars())
            systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        }
        mSurfaceView?.addOnLayoutChangeListener { _, l, t, r, b, ol, ot, or, ob ->
            if (r - l != or - ol || b - t != ob - ot) applyResolution()
        }
        val view = HorizonOverlay(this)
        overlay = view
        addContentView(
            view,
            ViewGroup.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT)
        )
        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                if (overlay?.back() != true) tapKey(KeyEvent.KEYCODE_ESCAPE)
            }
        })
    }

    private var overlay: HorizonOverlay? = null

    fun applyResolution() {
        val surface = mSurfaceView ?: return
        val vw = surface.width
        val vh = surface.height
        if (vw <= 0 || vh <= 0) return
        val settings = ControlSettings(this)
        val full = !settings.fixedResolution && settings.renderScale >= 0.995f
        if (full) {
            surface.holder.setSizeFromLayout()
            HorizonBridge.setScale(1f, 1f)
        } else {
            val (w, h) = settings.renderSize(vw, vh)
            surface.holder.setFixedSize(w, h)
            HorizonBridge.setScale(w.toFloat() / vw, h.toFloat() / vh)
        }
    }

    override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean {
        if (keyCode == KeyEvent.KEYCODE_BACK) {
            if (event.repeatCount == 0 && overlay?.back() != true) sendKey(KeyEvent.KEYCODE_ESCAPE, true)
            return true
        }
        return super.onKeyDown(keyCode, event)
    }

    override fun onKeyUp(keyCode: Int, event: KeyEvent): Boolean {
        if (keyCode == KeyEvent.KEYCODE_BACK) {
            sendKey(KeyEvent.KEYCODE_ESCAPE, false)
            return true
        }
        return super.onKeyUp(keyCode, event)
    }

    fun sendKey(keyCode: Int, down: Boolean, meta: Int = 0) {
        val now = SystemClock.uptimeMillis()
        val event = KeyEvent(now, now, if (down) KeyEvent.ACTION_DOWN else KeyEvent.ACTION_UP, keyCode, 0, meta)
        dispatchKeyToGame(event)
    }

    fun tapKey(keyCode: Int) {
        sendKey(keyCode, true)
        sendKey(keyCode, false)
    }

    fun dispatchKeyToGame(event: KeyEvent) {
        if (event.action == KeyEvent.ACTION_DOWN) super.onKeyDown(event.keyCode, event)
        else super.onKeyUp(event.keyCode, event)
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) {
            WindowInsetsControllerCompat(window, window.decorView).hide(WindowInsetsCompat.Type.systemBars())
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        runCatching { stdio?.close() }
        Process.killProcess(Process.myPid())
    }
}
