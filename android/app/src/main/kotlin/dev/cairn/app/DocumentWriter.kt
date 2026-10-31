package dev.cairn.app

import android.content.ContentResolver
import android.net.Uri
import android.provider.DocumentsContract
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.IOException

/**
 * Where a backup's bytes go: the document the save dialog created (ADR-0053 §4). An interface so
 * Settings' model holds no `ContentResolver`; the app's is [ResolverDocuments].
 */
interface DocumentWriter {
    /** Write [bytes] over the document at [uri] and return the name the provider gave it, if any. Throws when the write fails. */
    suspend fun write(uri: Uri, bytes: ByteArray): String?

    /** Delete the document at [uri] if the provider allows it; a refusal is not an error. */
    suspend fun delete(uri: Uri)
}

/**
 * The documents as the system's providers keep them, off the main thread (ADR-0046 §5). The write
 * truncates (`"wt"`), so a document that held something is replaced, never appended to. No grant is
 * kept: the dialog's is used here and nowhere else.
 */
class ResolverDocuments(private val resolver: ContentResolver) : DocumentWriter {
    override suspend fun write(uri: Uri, bytes: ByteArray): String? = withContext(Dispatchers.IO) {
        val out = resolver.openOutputStream(uri, "wt") ?: throw IOException("the provider gave no stream")
        out.use { it.write(bytes) }
        displayName(resolver, uri)
    }

    override suspend fun delete(uri: Uri) {
        withContext(Dispatchers.IO) {
            try {
                DocumentsContract.deleteDocument(resolver, uri)
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                // The provider does not delete, or the document is gone already: nothing more to do.
            }
        }
    }
}
