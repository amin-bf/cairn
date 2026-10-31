package dev.cairn.app

import android.app.ActivityManager
import android.content.ContentResolver
import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.ByteArrayOutputStream

/** The read ceiling's floor: 32 MiB (#241), what every phone reads whatever its heap. */
const val READ_CEILING_FLOOR_BYTES = 32 * 1024 * 1024

/**
 * How much of a document is read on a phone whose `ActivityManager.memoryClass` is [memoryClassMb]: a
 * quarter of it, never below [READ_CEILING_FLOOR_BYTES] (spec #248, *yours to overturn*). It bounds
 * memory against a file the person didn't mean to pick, while a collection archive — which grows with
 * every review — still opens; past it the head is handed over and marked incomplete.
 */
fun readCeiling(memoryClassMb: Int): Int = maxOf(READ_CEILING_FLOOR_BYTES, memoryClassMb / 4 * 1024 * 1024)

/** The read ceiling on this phone. */
fun readCeiling(context: Context): Int =
    readCeiling((context.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager).memoryClass)

/**
 * A document read once. [complete] is false when the read stopped at the ceiling or failed; the bytes
 * are then the head that was read, and Rust decides what a head can say (ADR-0052).
 */
class DocumentRead(val name: String?, val bytes: ByteArray, val complete: Boolean)

/**
 * Read the document at [uri] once, off the main thread (ADR-0046 §5, ADR-0052): its display name from
 * `OpenableColumns.DISPLAY_NAME`, none when the provider gives none, and its bytes up to [ceiling].
 * No permission is asked for and no grant is kept — the picker's grant is used here and nowhere else.
 * A read the provider fails is an empty incomplete one, which the preview states as unreadable.
 *
 * The buffer is sized from `OpenableColumns.SIZE` when the provider states one, so a large archive is
 * neither grown by doubling nor copied once read; the size is a hint, and the stream decides.
 */
suspend fun readDocument(resolver: ContentResolver, uri: Uri, ceiling: Int): DocumentRead =
    withContext(Dispatchers.IO) {
        val (name, size) = described(resolver, uri)
        try {
            val input = resolver.openInputStream(uri) ?: return@withContext DocumentRead(name, ByteArray(0), false)
            input.use {
                val out = Held(size?.coerceIn(0L, ceiling.toLong())?.toInt() ?: 64 * 1024)
                val buffer = ByteArray(64 * 1024)
                var left = ceiling
                while (left > 0) {
                    val n = it.read(buffer, 0, minOf(buffer.size, left))
                    if (n < 0) return@withContext DocumentRead(name, out.bytes(), true)
                    out.write(buffer, 0, n)
                    left -= n
                }
                // The ceiling is reached: the read is whole only if nothing follows it.
                DocumentRead(name, out.bytes(), complete = it.read() < 0)
            }
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            DocumentRead(name, ByteArray(0), false)
        }
    }

/** A buffer that hands over its own array when it is exactly full, rather than a copy of it. */
private class Held(initial: Int) : ByteArrayOutputStream(initial) {
    fun bytes(): ByteArray = if (count == buf.size) buf else toByteArray()
}

/** The provider's `OpenableColumns.DISPLAY_NAME` for [uri], or none when it gives none. */
internal fun displayName(resolver: ContentResolver, uri: Uri): String? = described(resolver, uri).first

/** The provider's display name and `OpenableColumns.SIZE` for [uri], each none when it gives none. */
private fun described(resolver: ContentResolver, uri: Uri): Pair<String?, Long?> =
    try {
        resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE), null, null, null)?.use { cursor ->
            if (!cursor.moveToFirst()) return@use null to null
            val name = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME).takeIf { it >= 0 }?.let { cursor.getString(it) }
            val size = cursor.getColumnIndex(OpenableColumns.SIZE).takeIf { it >= 0 && !cursor.isNull(it) }?.let { cursor.getLong(it) }
            name to size
        } ?: (null to null)
    } catch (_: Exception) {
        null to null
    }
