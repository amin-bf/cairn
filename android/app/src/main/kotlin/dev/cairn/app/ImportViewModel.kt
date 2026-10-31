package dev.cairn.app

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.FileApplied
import dev.cairn.ffi.FileArrival
import dev.cairn.ffi.FilePreview
import dev.cairn.ffi.InboundFileHandle
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

/** What the import screen shows (ADR-0022 §6): the file held in Rust, or nothing, or a fault reading it. */
sealed interface ImportScreenState {
    /** No file is held: after a process kill the preview is not restored (#241), and the app opens on Review. */
    data object Empty : ImportScreenState

    /**
     * The file's name as it arrived, if it had one, and what importing it would do now. [opened]
     * counts the files opened so far, so a file replacing another is a new page, though its name is the
     * same. While the write is [writing] the gate is disabled; [failure] is the store's account of a
     * refused write.
     */
    data class Preview(
        val name: String?,
        val preview: FilePreview,
        val opened: Int,
        val writing: Boolean = false,
        val failure: String? = null,
    ) : ImportScreenState

    /** Written: the note list is to open filtered to [filterTo], or unfiltered when the file held several decks. */
    data class Landed(val filterTo: String?) : ImportScreenState

    data object Failed : ImportScreenState
}

/**
 * The import preview's state (ADR-0052). It holds the inbound file handle — the bytes live in Rust —
 * so the preview survives a rotation, and it reads `preview()` when the screen starts and again on
 * resume, because the plan is derived on every read and a sync may have landed underneath it
 * (ADR-0022 §5). It is scoped to the activity, since the file arrives before the screen that shows it.
 */
class ImportViewModel(private val owner: CollectionAccess) : ViewModel() {
    private val _state = MutableStateFlow<ImportScreenState>(ImportScreenState.Empty)
    val state: StateFlow<ImportScreenState> = _state

    private var file: InboundFileHandle? = null
    private var name: String? = null
    private var opened = 0

    /** Hand the model a document that was read; the preview is derived before this returns. */
    suspend fun open(arrival: FileArrival, read: DocumentRead) {
        try {
            file = owner.use { it.openFile(arrival, read.name, read.bytes, read.complete) }
            name = read.name
            opened++
            derive(failure = null)
        } catch (_: CairnException) {
            _state.value = ImportScreenState.Failed
        }
    }

    /** Derive the plan again against the collection as it stands. */
    fun refresh() {
        viewModelScope.launch { derive(failure = (_state.value as? ImportScreenState.Preview)?.failure) }
    }

    /**
     * *Import*: the plan is derived again and written, off the main thread (ADR-0022 §5), and nothing
     * is said. The gate is disabled until the call returns. A refusal from a collection that changed
     * under the preview is the preview's refusal now, so the plan is read again and shows it; a write
     * the store refuses is said on the preview, the gate stays, and the plan is read again.
     */
    fun import() {
        val held = file ?: return
        val shown = _state.value as? ImportScreenState.Preview ?: return
        if (shown.writing) return
        _state.value = shown.copy(writing = true, failure = null)
        viewModelScope.launch {
            val applied = try {
                owner.use { held.apply() }
            } catch (e: CairnException) {
                derive(failure = (e as? CairnException.Failed)?.reason ?: e.message.orEmpty())
                return@launch
            }
            when (applied) {
                is FileApplied.Landed -> if (file === held) _state.value = ImportScreenState.Landed(applied.filterTo)
                is FileApplied.Refused -> derive(failure = null)
            }
        }
    }

    /** Whether a write is in flight: the gate, and Back with it, cannot be pressed until it returns. */
    val writing: Boolean get() = (_state.value as? ImportScreenState.Preview)?.writing == true

    /** Let the file go: *Cancel*, *Close* and Back all end here, with nothing written and nothing kept. */
    fun clear() {
        file = null
        name = null
        _state.value = ImportScreenState.Empty
    }

    /** Derive the plan again, with a refused write's [failure] said under it, or none. */
    private suspend fun derive(failure: String?) {
        val held = file ?: return
        try {
            val preview = owner.use { held.preview() }
            // A file let go while the plan was being derived stays let go.
            if (file === held) _state.value = ImportScreenState.Preview(name, preview, opened, failure = failure)
        } catch (_: CairnException) {
            if (file === held) _state.value = ImportScreenState.Failed
        }
    }
}
