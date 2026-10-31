package dev.cairn.app

import android.content.ContentResolver
import android.net.Uri
import android.provider.OpenableColumns
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.ByteArrayOutputStream

/**
 * How much of a document is read: 32 MiB (#241, *yours to overturn*). It bounds memory against a file
 * the person didn't mean to pick; past it the head is handed over and marked incomplete.
 */
const val READ_CEILING_BYTES = 32 * 1024 * 1024

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
 */
suspend fun readDocument(resolver: ContentResolver, uri: Uri, ceiling: Int = READ_CEILING_BYTES): DocumentRead =
    withContext(Dispatchers.IO) {
        val name = displayName(resolver, uri)
        try {
            val input = resolver.openInputStream(uri) ?: return@withContext DocumentRead(name, ByteArray(0), false)
            input.use {
                val out = ByteArrayOutputStream()
                val buffer = ByteArray(64 * 1024)
                var left = ceiling
                while (left > 0) {
                    val n = it.read(buffer, 0, minOf(buffer.size, left))
                    if (n < 0) return@withContext DocumentRead(name, out.toByteArray(), true)
                    out.write(buffer, 0, n)
                    left -= n
                }
                // The ceiling is reached: the read is whole only if nothing follows it.
                DocumentRead(name, out.toByteArray(), complete = it.read() < 0)
            }
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            DocumentRead(name, ByteArray(0), false)
        }
    }

private fun displayName(resolver: ContentResolver, uri: Uri): String? =
    try {
        resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
            val column = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            if (column >= 0 && cursor.moveToFirst()) cursor.getString(column) else null
        }
    } catch (_: Exception) {
        null
    }
