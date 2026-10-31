package dev.cairn.app

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.cairn.ffi.CairnException
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
     * The file's name as it arrived, if it had one, and what importing it would do now. [opening]
     * counts the files opened, so a file replacing another is a new page, though its name is the same.
     */
    data class Preview(val name: String?, val preview: FilePreview, val opening: Int) : ImportScreenState

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
    private var opening = 0

    /** Hand the model a document that was read; the preview is derived before this returns. */
    suspend fun open(arrival: FileArrival, read: DocumentRead) {
        try {
            file = owner.use { it.openFile(arrival, read.name, read.bytes, read.complete) }
            name = read.name
            opening++
            derive()
        } catch (_: CairnException) {
            _state.value = ImportScreenState.Failed
        }
    }

    /** Derive the plan again against the collection as it stands. */
    fun refresh() {
        viewModelScope.launch { derive() }
    }

    /** Let the file go: *Cancel*, *Close* and Back all end here, with nothing written and nothing kept. */
    fun clear() {
        file = null
        name = null
        _state.value = ImportScreenState.Empty
    }

    private suspend fun derive() {
        val held = file ?: return
        try {
            val preview = owner.use { held.preview() }
            // A file let go while the plan was being derived stays let go.
            if (file === held) _state.value = ImportScreenState.Preview(name, preview, opening)
        } catch (_: CairnException) {
            _state.value = ImportScreenState.Failed
        }
    }
}
