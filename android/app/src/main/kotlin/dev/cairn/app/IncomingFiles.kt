package dev.cairn.app

import android.content.Intent
import android.net.Uri
import androidx.core.content.IntentCompat
import androidx.lifecycle.ViewModel
import dev.cairn.ffi.FileArrival
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

/**
 * A file another app handed over, not yet read: how it came and where to read it. Once read it is the
 * inbound file the binding holds. Two arrivals of one URI are two.
 */
class ArrivingFile(val arrival: FileArrival, val uri: Uri)

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
 * The files arriving from other apps, of which the newest not yet at the preview is held, by the
 * activity's store so a rotation mid-read neither drops it nor reads it twice. A newer arrival replaces
 * an older one still pending: the newest file is the one the person just chose (#241).
 */
class IncomingFiles : ViewModel() {
    private val _pending = MutableStateFlow<ArrivingFile?>(null)
    val pending: StateFlow<ArrivingFile?> = _pending

    fun arrive(file: ArrivingFile) {
        _pending.value = file
    }

    /** [file] has reached the preview; a newer one that came meanwhile stays pending. */
    fun handled(file: ArrivingFile) {
        _pending.compareAndSet(file, null)
    }
}
