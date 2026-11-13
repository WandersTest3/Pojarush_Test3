package com.wanderstest3.pojarush

import android.content.Context
import android.system.Os
import okhttp3.Request
import org.apache.commons.compress.archivers.tar.TarArchiveInputStream
import org.apache.commons.compress.compressors.xz.XZCompressorInputStream
import java.io.File
import java.io.IOException
import java.io.InputStream
import java.security.MessageDigest

object JavaRuntime {
    const val MAJOR = 25
    private const val VERSION = "7a0266e745d9b4acf400afa189b58e672900f710"
    private const val SOURCE = "https://raw.githubusercontent.com/WandersTest3/Pojarush_Test3/6c3bd125f658d1fb8a887f9f20b46c0a95a48e33/runtimes/jre-25"

    private class Part(val name: String, val sha256: String, val size: Long)

    private val UNIVERSAL = Part("universal.tar.xz", "23141bf4e0ed46ff0b751c6cc32b3863d39e6a37b720f3d86a4fe9318aeb8e9f", 32_814_644)
    private val ARM64 = Part("bin-arm64.tar.xz", "16f09d1d163c008ae556be68c624057c4791c21ae6d31302334e61d75439567b", 5_453_168)
    private val ARM = Part("bin-arm.tar.xz", "b7ee2a249bc81b6c2422f3763687e642df02b5abac6cb6f2f58888ef1c7f8e0f", 4_216_116)

    val home: File get() = File(Paths.runtimes, "jre-$MAJOR")
    private val versionFile: File get() = File(home, ".pojarush-version")
    private val downloads: File get() = File(Paths.runtimes, "download")

    fun ensure(context: Context, progress: (String) -> Unit): File {
        val arm64 = android.os.Process.is64Bit()
        val wanted = VERSION + "-" + if (arm64) "arm64" else "arm"
        if (versionFile.isFile && versionFile.readText() == wanted && File(home, "lib/libjli.so").isFile) return home
        val parts = listOf(UNIVERSAL, if (arm64) ARM64 else ARM)
        val total = parts.sumOf { it.size }
        var done = 0L
        val files = parts.map { part ->
            val before = done
            fetch(part) { read -> progress("Downloading Java $MAJOR (${(before + read) / 1_048_576} / ${total / 1_048_576} MB)") }
                .also { done += part.size }
        }
        progress("Unpacking Java $MAJOR")
        home.deleteRecursively()
        home.mkdirs()
        try {
            files.forEach { file -> file.inputStream().use { untarXz(it, home) } }
            if (!File(home, "lib/libjli.so").isFile) throw IOException("Java $MAJOR is incomplete after unpacking")
            versionFile.writeText(wanted)
        } catch (e: Exception) {
            home.deleteRecursively()
            throw e
        }
        downloads.deleteRecursively()
        return home
    }

    private fun fetch(part: Part, progress: (Long) -> Unit): File {
        downloads.mkdirs()
        val target = File(downloads, part.name)
        if (target.isFile && target.length() == part.size && sha256(target) == part.sha256) return target
        val temp = File(downloads, part.name + ".part")
        var last: Exception? = null
        repeat(3) {
            try {
                Http.client.newCall(Request.Builder().url("$SOURCE/${part.name}").build()).execute().use { response ->
                    if (!response.isSuccessful) throw IOException("HTTP ${response.code} for Java $MAJOR (${part.name})")
                    response.body.byteStream().use { input ->
                        temp.outputStream().use { out ->
                            val buffer = ByteArray(1 shl 16)
                            var read = 0L
                            var shown = 0L
                            while (true) {
                                val n = input.read(buffer)
                                if (n < 0) break
                                out.write(buffer, 0, n)
                                read += n
                                if (read - shown >= 1_048_576) {
                                    shown = read
                                    progress(read)
                                }
                            }
                        }
                    }
                }
                val got = sha256(temp)
                if (got != part.sha256) throw IOException("Checksum mismatch for Java $MAJOR (${part.name})")
                if (!temp.renameTo(target)) {
                    temp.copyTo(target, overwrite = true)
                    temp.delete()
                }
                return target
            } catch (e: Exception) {
                last = e
                temp.delete()
            }
        }
        throw IOException("Java $MAJOR could not be downloaded: ${last?.message}", last)
    }

    private fun sha256(file: File): String {
        val digest = MessageDigest.getInstance("SHA-256")
        file.inputStream().use { input ->
            val buffer = ByteArray(1 shl 16)
            while (true) {
                val read = input.read(buffer)
                if (read < 0) break
                digest.update(buffer, 0, read)
            }
        }
        return digest.digest().joinToString("") { "%02x".format(it) }
    }

    private fun untarXz(input: InputStream, dest: File) {
        TarArchiveInputStream(XZCompressorInputStream(input.buffered())).use { tar ->
            while (true) {
                val entry = tar.nextEntry ?: break
                val target = File(dest, entry.name).canonicalFile
                if (!target.path.startsWith(dest.canonicalPath)) throw IOException("Unsafe entry ${entry.name}")
                when {
                    entry.isSymbolicLink -> {
                        target.parentFile?.mkdirs()
                        target.delete()
                        Os.symlink(entry.linkName, target.absolutePath)
                    }
                    entry.isDirectory -> target.mkdirs()
                    else -> {
                        target.parentFile?.mkdirs()
                        target.outputStream().use { tar.copyTo(it) }
                        if (entry.mode and "100".toInt(8) != 0) target.setExecutable(true, false)
                    }
                }
            }
        }
    }
}
