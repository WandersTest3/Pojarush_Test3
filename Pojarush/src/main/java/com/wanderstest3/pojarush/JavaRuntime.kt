package com.wanderstest3.pojarush

import android.content.Context
import android.system.Os
import org.apache.commons.compress.archivers.tar.TarArchiveInputStream
import org.apache.commons.compress.compressors.xz.XZCompressorInputStream
import java.io.File
import java.io.IOException
import java.io.InputStream

object JavaRuntime {
    const val MAJOR = 25
    private const val ASSETS = "runtimes/jre-25"

    val home: File get() = File(Paths.runtimes, "jre-$MAJOR")
    private val versionFile: File get() = File(home, ".pojarush-version")

    fun ensure(context: Context, progress: (String) -> Unit): File {
        val arch = if (android.os.Process.is64Bit()) "arm64" else "arm"
        val bundled = context.assets.open("$ASSETS/version").use { it.readBytes().decodeToString().trim() } + "-" + arch
        if (versionFile.isFile && versionFile.readText() == bundled && File(home, "lib/libjli.so").isFile) return home
        progress("Unpacking Java $MAJOR")
        home.deleteRecursively()
        home.mkdirs()
        try {
            context.assets.open("$ASSETS/universal.tar.xz").use { untarXz(it, home) }
            context.assets.open("$ASSETS/bin-$arch.tar.xz").use { untarXz(it, home) }
            if (!File(home, "lib/libjli.so").isFile) throw IOException("Java $MAJOR is incomplete after unpacking")
            versionFile.writeText(bundled)
        } catch (e: Exception) {
            home.deleteRecursively()
            throw e
        }
        return home
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
