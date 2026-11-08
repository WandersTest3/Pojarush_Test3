package com.wanderstest3.pojarush.ui

import android.content.ActivityNotFoundException
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.StrictMode
import android.provider.DocumentsContract
import android.widget.Toast
import com.wanderstest3.pojarush.R
import java.io.File

object FolderOpener {
    private val ZARCHIVER = listOf("ru.zdevs.zarchiver.pro", "ru.zdevs.zarchiver")

    private fun installed(context: Context, pkg: String): Boolean =
        runCatching { context.packageManager.getPackageInfo(pkg, 0) }.isSuccess

    private fun withFileUris(block: () -> Unit) {
        val old = StrictMode.getVmPolicy()
        StrictMode.setVmPolicy(StrictMode.VmPolicy.Builder().build())
        try {
            block()
        } finally {
            StrictMode.setVmPolicy(old)
        }
    }

    private fun documentUri(dir: File): Uri? {
        val root = android.os.Environment.getExternalStorageDirectory().absolutePath
        val path = dir.absolutePath
        if (!path.startsWith(root)) return null
        val relative = path.removePrefix(root).trim('/')
        return DocumentsContract.buildDocumentUri("com.android.externalstorage.documents", "primary:$relative")
    }

    fun open(context: Context, dir: File) {
        dir.mkdirs()
        val file = Uri.fromFile(dir)
        for (pkg in ZARCHIVER.filter { installed(context, it) }) {
            val attempts = listOf(
                Intent(Intent.ACTION_VIEW).setDataAndType(file, "resource/folder"),
                Intent(Intent.ACTION_VIEW).setData(file),
            )
            for (intent in attempts) {
                intent.setPackage(pkg).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                var ok = false
                withFileUris { ok = runCatching { context.startActivity(intent) }.isSuccess }
                if (ok) return
            }
            context.packageManager.getLaunchIntentForPackage(pkg)?.let { launch ->
                Toast.makeText(context, context.getString(R.string.folder_path_toast, dir.absolutePath), Toast.LENGTH_LONG).show()
                runCatching { context.startActivity(launch.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) }.onSuccess { return }
            }
        }
        val doc = documentUri(dir)
        if (doc != null) {
            val intent = Intent(Intent.ACTION_VIEW)
                .setDataAndType(doc, DocumentsContract.Document.MIME_TYPE_DIR)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION)
            try {
                context.startActivity(intent)
                return
            } catch (_: ActivityNotFoundException) {
            } catch (_: SecurityException) {
            }
            runCatching {
                context.startActivity(
                    Intent(Intent.ACTION_VIEW).setDataAndType(doc, DocumentsContract.Document.MIME_TYPE_DIR)
                        .setComponent(ComponentName("com.google.android.documentsui", "com.android.documentsui.files.FilesActivity"))
                        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                )
            }.onSuccess { return }
        }
        Toast.makeText(context, context.getString(R.string.folder_path_toast, dir.absolutePath), Toast.LENGTH_LONG).show()
    }
}
