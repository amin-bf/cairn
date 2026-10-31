package dev.cairn.app

import android.app.Application
import android.content.Intent
import android.net.Uri
import androidx.core.content.IntentCompat
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import dev.cairn.ffi.FileArrival
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

/**
 * A file another app handed over, not yet read: how it came and where to read it. Once read it is the
 * inbound file the binding holds. Two arrivals of one URI are two.
 */
class ArrivingFile(val arrival: FileArrival, val uri: Uri)

/** An arriving file once read, waiting for the preview. Two reads of one file are two. */
class ReadFile(val arrival: FileArrival, val document: DocumentRead)

/**
 * The file an intent carries (ADR-0024 §2): `VIEW` takes the data URI, `SEND` takes `EXTRA_STREAM` and
 * falls back to the first `ClipData` item. Anything else carries none.
 */
fun arrivingFrom(intent: Intent?): ArrivingFile? =
    when (intent?.action) {
        Intent.ACTION_VIEW -> intent.data?.let { ArrivingFile(FileArrival.OPENED, it) }
        Intent.ACTION_SEND ->
            (IntentCompat.getParcelableExtra(intent, Intent.EXTRA_STREAM, Uri::class.java) ?: intent.clipData?.takeIf { it.itemCount > 0 }?.getItemAt(0)?.uri)
                ?.let { ArrivingFile(FileArrival.SHARED, it) }
        else -> null
    }

/**
 * The files arriving from other apps. Each is read here, in the activity's store, so a rotation
 * mid-read neither drops it nor reads it twice (ADR-0052), and the newest read not yet at the preview
 * is held. A newer arrival cancels an older one still being read: the newest file is the one the
 * person just chose (#241).
 */
class IncomingFiles(app: Application) : AndroidViewModel(app) {
    private val resolver = app.contentResolver
    private val ceiling = readCeiling(app)
    private val _pending = MutableStateFlow<ReadFile?>(null)
    val pending: StateFlow<ReadFile?> = _pending
    private var reading: Job? = null

    fun arrive(file: ArrivingFile) {
        reading?.cancel()
        reading = viewModelScope.launch { _pending.value = ReadFile(file.arrival, readDocument(resolver, file.uri, ceiling)) }
    }

    /** [file] has reached the preview; a newer one that came meanwhile stays pending. */
    fun handled(file: ReadFile) {
        _pending.compareAndSet(file, null)
    }
}
