package com.wanderstest3.pojarush.ui

import android.database.Cursor
import android.database.MatrixCursor
import android.os.CancellationSignal
import android.os.ParcelFileDescriptor
import android.provider.DocumentsContract.Document
import android.provider.DocumentsContract.Root
import android.provider.DocumentsProvider
import android.webkit.MimeTypeMap
import com.wanderstest3.pojarush.BuildConfig
import com.wanderstest3.pojarush.Paths
import com.wanderstest3.pojarush.R
import java.io.File
import java.io.FileNotFoundException

class PojarushDocuments : DocumentsProvider() {
    private val rootColumns = arrayOf(Root.COLUMN_ROOT_ID, Root.COLUMN_FLAGS, Root.COLUMN_ICON, Root.COLUMN_TITLE, Root.COLUMN_SUMMARY, Root.COLUMN_DOCUMENT_ID, Root.COLUMN_AVAILABLE_BYTES)
    private val docColumns = arrayOf(Document.COLUMN_DOCUMENT_ID, Document.COLUMN_DISPLAY_NAME, Document.COLUMN_MIME_TYPE, Document.COLUMN_FLAGS, Document.COLUMN_SIZE, Document.COLUMN_LAST_MODIFIED)

    override fun onCreate(): Boolean = true

    private fun base(): File {
        if (runCatching { Paths.publicRoot }.isFailure) context?.let { Paths.init(it.applicationContext as android.app.Application) }
        return Paths.publicRoot.canonicalFile
    }

    private fun fileOf(id: String): File {
        val root = base()
        val f = if (id == ROOT) root else File(root, id.removePrefix("$ROOT/")).canonicalFile
        if (!f.path.startsWith(root.path)) throw FileNotFoundException(id)
        return f
    }

    private fun idOf(f: File): String {
        val root = base()
        val c = f.canonicalFile
        return if (c == root) ROOT else "$ROOT/" + c.relativeTo(root).path
    }

    private fun mime(f: File): String =
        if (f.isDirectory) Document.MIME_TYPE_DIR
        else MimeTypeMap.getSingleton().getMimeTypeFromExtension(f.extension.lowercase()) ?: "application/octet-stream"

    private fun row(c: MatrixCursor, f: File) {
        var flags = Document.FLAG_SUPPORTS_DELETE or Document.FLAG_SUPPORTS_RENAME
        flags = if (f.isDirectory) flags or Document.FLAG_DIR_SUPPORTS_CREATE else flags or Document.FLAG_SUPPORTS_WRITE
        c.newRow()
            .add(Document.COLUMN_DOCUMENT_ID, idOf(f))
            .add(Document.COLUMN_DISPLAY_NAME, if (f.canonicalFile == base()) Paths.FOLDER else f.name)
            .add(Document.COLUMN_MIME_TYPE, mime(f))
            .add(Document.COLUMN_FLAGS, flags)
            .add(Document.COLUMN_SIZE, if (f.isFile) f.length() else null)
            .add(Document.COLUMN_LAST_MODIFIED, f.lastModified())
    }

    override fun queryRoots(projection: Array<out String>?): Cursor {
        val c = MatrixCursor(projection ?: rootColumns)
        val root = base()
        c.newRow()
            .add(Root.COLUMN_ROOT_ID, ROOT)
            .add(Root.COLUMN_FLAGS, Root.FLAG_SUPPORTS_CREATE or Root.FLAG_SUPPORTS_IS_CHILD or Root.FLAG_LOCAL_ONLY)
            .add(Root.COLUMN_ICON, R.mipmap.ic_launcher)
            .add(Root.COLUMN_TITLE, context?.getString(R.string.app_name) ?: Paths.FOLDER)
            .add(Root.COLUMN_SUMMARY, Paths.FOLDER)
            .add(Root.COLUMN_DOCUMENT_ID, ROOT)
            .add(Root.COLUMN_AVAILABLE_BYTES, root.freeSpace)
        return c
    }

    override fun queryDocument(documentId: String, projection: Array<out String>?): Cursor =
        MatrixCursor(projection ?: docColumns).also { row(it, fileOf(documentId)) }

    override fun queryChildDocuments(parentDocumentId: String, projection: Array<out String>?, sortOrder: String?): Cursor {
        val c = MatrixCursor(projection ?: docColumns)
        fileOf(parentDocumentId).listFiles().orEmpty().sortedBy { it.name.lowercase() }.forEach { row(c, it) }
        return c
    }

    override fun isChildDocument(parentDocumentId: String, documentId: String): Boolean =
        runCatching { fileOf(documentId).path.startsWith(fileOf(parentDocumentId).path + "/") }.getOrDefault(false)

    override fun openDocument(documentId: String, mode: String, signal: CancellationSignal?): ParcelFileDescriptor =
        ParcelFileDescriptor.open(fileOf(documentId), ParcelFileDescriptor.parseMode(mode))

    override fun createDocument(parentDocumentId: String, mimeType: String, displayName: String): String {
        val dir = fileOf(parentDocumentId)
        var f = File(dir, displayName)
        var n = 1
        while (f.exists()) f = File(dir, "${displayName.substringBeforeLast('.')} ($n)" + displayName.substringAfterLast('.', "").let { if (it.isEmpty()) "" else ".$it" }).also { n++ }
        val ok = if (mimeType == Document.MIME_TYPE_DIR) f.mkdirs() else f.createNewFile()
        if (!ok) throw FileNotFoundException(displayName)
        return idOf(f)
    }

    override fun deleteDocument(documentId: String) {
        val f = fileOf(documentId)
        if (f.canonicalFile == base() || !f.deleteRecursively()) throw FileNotFoundException(documentId)
    }

    override fun renameDocument(documentId: String, displayName: String): String {
        val f = fileOf(documentId)
        val to = File(f.parentFile, displayName)
        if (!f.renameTo(to)) throw FileNotFoundException(displayName)
        return idOf(to)
    }

    companion object {
        const val ROOT = "pojarush"
        val AUTHORITY: String get() = BuildConfig.APPLICATION_ID + ".documents"
        fun documentId(dir: File): String? = runCatching {
            val root = Paths.publicRoot.canonicalFile
            val c = dir.canonicalFile
            when {
                c == root -> ROOT
                c.path.startsWith(root.path + "/") -> "$ROOT/" + c.relativeTo(root).path
                else -> null
            }
        }.getOrNull()
    }
}
