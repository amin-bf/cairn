package dev.cairn.app

import android.content.ContentProvider
import android.content.ContentValues
import android.database.Cursor
import android.database.MatrixCursor
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.provider.OpenableColumns
import java.io.File

/**
 * A debug-only provider that serves bytes the way a document provider does: a `DISPLAY_NAME` for a
 * query and a stream for a read. A URI with `?nameless` answers a query with no row, as a share that
 * carries no name does (ADR-0024 §1), and one with `?fails` cannot be opened at all. One with `?slow`
 * takes a while to open, so a test can turn the phone mid-read; every opening is counted.
 */
class BenchProvider : ContentProvider() {
    override fun onCreate() = true

    override fun query(uri: Uri, projection: Array<String>?, selection: String?, args: Array<String>?, sort: String?): Cursor? {
        if (uri.getQueryParameter("nameless") != null) return MatrixCursor(arrayOf(OpenableColumns.DISPLAY_NAME))
        val cursor = MatrixCursor(arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE))
        val name = uri.lastPathSegment ?: return cursor
        cursor.addRow(arrayOf(name, files[name]?.size?.toLong() ?: 0L))
        return cursor
    }

    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor {
        if (uri.getQueryParameter("fails") != null) throw java.io.FileNotFoundException("refused")
        val name = uri.lastPathSegment ?: throw java.io.FileNotFoundException("no name")
        val bytes = files[name] ?: throw java.io.FileNotFoundException(name)
        opened.computeIfAbsent(name) { java.util.concurrent.atomic.AtomicInteger() }.incrementAndGet()
        if (uri.getQueryParameter("slow") != null) Thread.sleep(2_000)
        val file = File.createTempFile("bench", ".bin", context!!.cacheDir).apply { writeBytes(bytes) }
        return ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY).also { file.delete() }
    }

    override fun getType(uri: Uri): String? = null

    override fun insert(uri: Uri, values: ContentValues?): Uri? = null

    override fun delete(uri: Uri, selection: String?, args: Array<String>?) = 0

    override fun update(uri: Uri, values: ContentValues?, selection: String?, args: Array<String>?) = 0

    companion object {
        /** The bytes behind each name, set by the test that serves them. It runs in the app's process, so the tests share this map. */
        val files = java.util.concurrent.ConcurrentHashMap<String, ByteArray>()

        /** How often each name was opened for reading, since the test cleared it. */
        val opened = java.util.concurrent.ConcurrentHashMap<String, java.util.concurrent.atomic.AtomicInteger>()

        fun openings(name: String): Int = opened[name]?.get() ?: 0

        fun uri(name: String, query: String? = null): Uri =
            Uri.Builder().scheme("content").authority("dev.cairn.app.test.bench").appendPath(name).encodedQuery(query).build()
    }
}
