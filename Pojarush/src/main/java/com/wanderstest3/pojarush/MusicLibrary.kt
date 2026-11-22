package com.wanderstest3.pojarush

import android.content.Context
import android.media.AudioAttributes
import android.media.MediaPlayer
import android.net.Uri
import android.provider.OpenableColumns
import java.io.File

object MusicLibrary {
    private val extensions = setOf("mp3", "ogg", "oga", "wav", "m4a", "aac", "flac", "opus")

    fun dir(context: Context): File = File(context.filesDir, "menu_music").apply { mkdirs() }

    fun tracks(context: Context): List<File> =
        dir(context).listFiles()
            ?.filter { it.isFile && it.extension.lowercase() in extensions }
            ?.sortedBy { it.name.lowercase() }
            .orEmpty()

    fun add(context: Context, uri: Uri): Result<File> = runCatching {
        val raw = context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)
            ?.use { c -> if (c.moveToFirst()) c.getString(0) else null }
            ?.replace(Regex("[^A-Za-z0-9._ -]"), "_")
            ?.takeIf { it.isNotBlank() } ?: "music.mp3"
        val name = if (raw.substringAfterLast('.', "").lowercase() in extensions) raw else "$raw.mp3"
        var out = File(dir(context), name)
        var n = 1
        while (out.exists()) {
            out = File(dir(context), "${name.substringBeforeLast('.')} ($n).${name.substringAfterLast('.')}")
            n++
        }
        context.contentResolver.openInputStream(uri)?.use { input -> out.outputStream().use { input.copyTo(it) } }
            ?: error("No access to the file")
        out
    }

    fun title(file: File): String = file.nameWithoutExtension
}

class PlaylistPlayer(
    private val context: Context,
    private val volume: () -> Float,
    private val shuffle: () -> Boolean,
) {
    private var player: MediaPlayer? = null
    private var current: File? = null
    private var wanted = false
    var paused = false
        private set

    val playing: Boolean
        get() = runCatching { player?.isPlaying == true }.getOrDefault(false)

    val nowPlaying: String?
        get() = current?.let(MusicLibrary::title)

    fun start() {
        wanted = true
        if (paused) return
        if (player == null) next() else {
            applyVolume()
            runCatching { if (player?.isPlaying == false) player?.start() }
        }
    }

    fun stop() {
        wanted = false
        runCatching { if (player?.isPlaying == true) player?.pause() }
    }

    fun setPaused(value: Boolean) {
        paused = value
        if (value) runCatching { if (player?.isPlaying == true) player?.pause() } else if (wanted) start()
    }

    fun applyVolume() {
        val v = volume().coerceIn(0f, 1f)
        runCatching { player?.setVolume(v, v) }
    }

    fun reload() {
        val tracks = MusicLibrary.tracks(context)
        if (tracks.isEmpty()) {
            release()
            return
        }
        if (current != null && current !in tracks) next()
        else if (player != null) {
            player?.isLooping = tracks.size == 1
            applyVolume()
        }
    }

    fun next() {
        val tracks = MusicLibrary.tracks(context)
        if (tracks.isEmpty()) {
            release()
            return
        }
        val last = current
        val pick = when {
            tracks.size == 1 -> tracks[0]
            shuffle() -> tracks.filter { it != last }.random()
            else -> tracks[((tracks.indexOf(last) + 1) % tracks.size)]
        }
        play(pick, tracks.size == 1)
    }

    private fun play(file: File, loop: Boolean) {
        runCatching { player?.release() }
        player = null
        current = file
        player = runCatching {
            MediaPlayer().apply {
                setAudioAttributes(
                    AudioAttributes.Builder()
                        .setUsage(AudioAttributes.USAGE_GAME)
                        .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                        .build()
                )
                setDataSource(context, Uri.fromFile(file))
                isLooping = loop
                setOnCompletionListener { if (!loop) next() }
                setOnErrorListener { _, _, _ ->
                    current = null
                    release()
                    true
                }
                prepare()
            }
        }.getOrNull()
        applyVolume()
        if (wanted && !paused) runCatching { player?.start() }
    }

    fun release() {
        runCatching { player?.stop() }
        runCatching { player?.release() }
        player = null
        current = null
    }
}
