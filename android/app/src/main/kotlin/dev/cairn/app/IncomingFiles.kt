package dev.cairn.app

import android.app.Application
import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
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

/**
 * The arrival the app was last started by, kept across processes with the install it was started under.
 * The system replays a task's root intent after an upgrade, and the file it names is no longer granted
 * (#255): a start equal to the last one recorded, under an install since replaced, is that replay.
 * Only starts are recorded; a file reaching the running app is not, so it cannot hide the root intent.
 */
class HandledArrivals(private val prefs: SharedPreferences, private val installed: () -> Long) {
    /**
     * [file] is the replay of the last arrival handled under an earlier install. It is said once: the
     * record moves to this install, so the same file opened again is a new arrival.
     */
    fun isReplay(file: ArrivingFile): Boolean {
        val replay = prefs.getString(KEY, null) == key(file) && prefs.getLong(INSTALL, 0L) != installed()
        if (replay) prefs.edit().putLong(INSTALL, installed()).commit()
        return replay
    }

    // Committed, not applied: a process killed right after the start must not lose the record.
    fun handled(file: ArrivingFile) {
        prefs.edit().putString(KEY, key(file)).putLong(INSTALL, installed()).commit()
    }

    fun clear() {
        prefs.edit().clear().commit()
    }

    private fun key(file: ArrivingFile) = "${file.arrival}|${file.uri}"

    companion object {
        private const val KEY = "arrival"
        private const val INSTALL = "install"

        fun of(context: Context, installed: () -> Long = { installTime(context) }) =
            HandledArrivals(context.getSharedPreferences("handled_arrivals", Context.MODE_PRIVATE), installed)

        private fun installTime(context: Context): Long =
            context.packageManager.getPackageInfo(context.packageName, 0).lastUpdateTime
    }
}
