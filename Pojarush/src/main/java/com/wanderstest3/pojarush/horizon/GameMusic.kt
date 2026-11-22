package com.wanderstest3.pojarush.horizon

import android.content.Context
import android.media.AudioAttributes
import android.media.MediaPlayer
import android.net.Uri
import com.wanderstest3.pojarush.controls.ControlSettings
import java.io.File

class GameMusic(private val context: Context) {
    private val settings = ControlSettings(context)
    private var player: MediaPlayer? = null
    private var path: String? = null
    private var userPaused = false
    private var foreground = false

    val available: Boolean
        get() = settings.menuMusic.isNotEmpty() && File(settings.menuMusic).isFile

    val playing: Boolean
        get() = runCatching { player?.isPlaying == true }.getOrDefault(false)

    fun onResume() {
        foreground = true
        refresh()
    }

    fun onPause() {
        foreground = false
        runCatching { if (player?.isPlaying == true) player?.pause() }
    }

    fun release() {
        foreground = false
        runCatching { player?.stop() }
        player?.release()
        player = null
        path = null
    }

    fun refresh() {
        if (!settings.gameMusic || !available) {
            release()
            return
        }
        val wanted = settings.menuMusic
        if (player == null || path != wanted) {
            runCatching { player?.release() }
            player = runCatching {
                MediaPlayer().apply {
                    setAudioAttributes(
                        AudioAttributes.Builder()
                            .setUsage(AudioAttributes.USAGE_GAME)
                            .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                            .build()
                    )
                    setDataSource(context, Uri.fromFile(File(wanted)))
                    isLooping = true
                    prepare()
                }
            }.getOrNull()
            path = wanted
        }
        val v = settings.gameMusicVolume
        runCatching { player?.setVolume(v, v) }
        if (foreground && !userPaused) runCatching { if (player?.isPlaying == false) player?.start() }
    }

    fun setPaused(paused: Boolean) {
        userPaused = paused
        if (paused) runCatching { if (player?.isPlaying == true) player?.pause() } else refresh()
    }

    val paused: Boolean
        get() = userPaused
}
