package com.wanderstest3.pojarush

import android.Manifest
import android.app.Application
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.os.Environment
import okhttp3.OkHttpClient
import java.io.File
import java.util.concurrent.TimeUnit

class PojarushApp : Application() {
    override fun onCreate() {
        super.onCreate()
        Paths.init(this)
        Accounts.init(this)
        com.wanderstest3.pojarush.ui.UiPrefs.load(this)
    }
}

object Paths {
    const val FOLDER = "Pojarush_Test3"
    lateinit var privateRoot: File
        private set
    lateinit var publicRoot: File
        private set
    private lateinit var appExternal: File
    val sharedRoot: File get() = File(Environment.getExternalStorageDirectory(), FOLDER)
    lateinit var cache: File
        private set
    lateinit var nativeDir: File
        private set

    val runtimes: File get() = File(privateRoot, "runtimes")

    fun init(app: Application) {
        privateRoot = File(app.filesDir, "pojarush")
        appExternal = app.getExternalFilesDir(null) ?: File(app.filesDir, "public")
        cache = app.cacheDir
        nativeDir = File(app.applicationInfo.nativeLibraryDir)
        privateRoot.mkdirs()
        publicRoot = choose(app)
        publicRoot.mkdirs()
    }

    fun hasStorageAccess(context: Context): Boolean =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) Environment.isExternalStorageManager()
        else context.checkSelfPermission(Manifest.permission.WRITE_EXTERNAL_STORAGE) == PackageManager.PERMISSION_GRANTED

    private fun choose(context: Context): File =
        if (hasStorageAccess(context) && (sharedRoot.isDirectory || sharedRoot.mkdirs())) sharedRoot else appExternal

    fun refresh(context: Context): Boolean {
        val next = choose(context)
        if (next == publicRoot) return false
        if (next == sharedRoot) migrate(appExternal, sharedRoot)
        publicRoot = next
        return true
    }

    private fun migrate(from: File, to: File) {
        for (name in listOf("horizon", "controls")) {
            val src = File(from, name)
            val dst = File(to, name)
            if (!src.isDirectory) continue
            runCatching {
                src.walkTopDown().forEach { f ->
                    val target = File(dst, f.relativeTo(src).path)
                    if (f.isDirectory) target.mkdirs()
                    else if (!target.exists()) f.copyTo(target)
                }
                src.deleteRecursively()
            }
        }
    }
}

object Http {
    val client: OkHttpClient by lazy {
        OkHttpClient.Builder()
            .connectTimeout(30, TimeUnit.SECONDS)
            .readTimeout(60, TimeUnit.SECONDS)
            .build()
    }
}
