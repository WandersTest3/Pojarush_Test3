package com.wanderstest3.pojarush.horizon

import android.content.Context
import com.wanderstest3.pojarush.MusicLibrary
import com.wanderstest3.pojarush.PlaylistPlayer
import com.wanderstest3.pojarush.controls.ControlSettings

class GameMusic(private val context: Context) {
    private val settings = ControlSettings(context)
    private val player = PlaylistPlayer(context, { settings.gameMusicVolume }, { settings.musicShuffle })
    private var foreground = false

    val available: Boolean
        get() = MusicLibrary.tracks(context).isNotEmpty()

    val paused: Boolean
        get() = player.paused

    val nowPlaying: String?
        get() = player.nowPlaying

    fun onResume() {
        foreground = true
        refresh()
    }

    fun onPause() {
        foreground = false
        player.stop()
    }

    fun release() {
        foreground = false
        player.stop()
        player.release()
    }

    fun refresh() {
        if (!settings.gameMusic || !available) {
            player.stop()
            player.release()
            return
        }
        player.applyVolume()
        if (foreground) player.start()
    }

    fun setPaused(value: Boolean) = player.setPaused(value)

    fun next() = player.next()
}
