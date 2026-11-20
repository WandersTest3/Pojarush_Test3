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

    private val DOCUMENTS_UI = listOf("com.google.android.documentsui", "com.android.documentsui")

    private fun tryStart(context: Context, intent: Intent): Boolean = try {
        context.startActivity(intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION))
        true
    } catch (_: ActivityNotFoundException) {
        false
    } catch (_: SecurityException) {
        false
    }

    private fun systemFiles(context: Context, uri: Uri): Boolean {
        for (pkg in DOCUMENTS_UI.filter { installed(context, it) }) {
            val view = Intent(Intent.ACTION_VIEW).setDataAndType(uri, DocumentsContract.Document.MIME_TYPE_DIR)
            if (tryStart(context, Intent(view).setComponent(ComponentName(pkg, "com.android.documentsui.files.FilesActivity")))) return true
            if (tryStart(context, Intent(view).setPackage(pkg))) return true
        }
        return tryStart(context, Intent(Intent.ACTION_VIEW).setDataAndType(uri, DocumentsContract.Document.MIME_TYPE_DIR))
    }

    fun open(context: Context, dir: File) {
        dir.mkdirs()
        val own = PojarushDocuments.documentId(dir)?.let { DocumentsContract.buildDocumentUri(PojarushDocuments.AUTHORITY, it) }
        val external = documentUri(dir)
        val external11 = android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.R && dir.absolutePath.contains("/Android/data/")
        if (external != null && !external11 && systemFiles(context, external)) return
        if (own != null && systemFiles(context, own)) return
        val file = Uri.fromFile(dir)
        for (pkg in ZARCHIVER.filter { installed(context, it) }) {
            for (intent in listOf(Intent(Intent.ACTION_VIEW).setDataAndType(file, "resource/folder"), Intent(Intent.ACTION_VIEW).setData(file))) {
                intent.setPackage(pkg).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                var ok = false
                withFileUris { ok = runCatching { context.startActivity(intent) }.isSuccess }
                if (ok) return
            }
        }
        Toast.makeText(context, context.getString(R.string.folder_path_toast, dir.absolutePath), Toast.LENGTH_LONG).show()
    }
}
