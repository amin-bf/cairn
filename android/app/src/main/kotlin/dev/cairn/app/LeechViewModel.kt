package dev.cairn.app

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.LeechListFacts
import dev.cairn.ffi.LeechRowFact
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

/** The row a refused write came from, and which write it was: the refusal is said in that row. */
data class LeechRefusal(val noteId: String, val slot: UShort, val suspending: Boolean)

/** What the leech screen shows: the two sections as facts, and the write the store last refused. */
sealed interface LeechScreenState {
    data object Loading : LeechScreenState

    data class Ready(val facts: LeechListFacts, val refusal: LeechRefusal? = null) : LeechScreenState

    data object Failed : LeechScreenState
}

/**
 * The leech screen's state (ADR-0050). The list, its ranking, the cost and the two suspension writes
 * are the binding's (ADR-0046 §1); this holds the answer and asks again whenever it may have gone
 * stale: after a write, and on resume from the editor.
 */
class LeechViewModel(
    private val owner: CollectionAccess,
    private val wallClock: () -> Long = System::currentTimeMillis,
) : ViewModel() {
    private val _state = MutableStateFlow<LeechScreenState>(LeechScreenState.Loading)
    val state: StateFlow<LeechScreenState> = _state

    init {
        refresh()
    }

    /** Read the list again, keeping a refusal said in a row. */
    fun refresh() {
        viewModelScope.launch { read(null) }
    }

    private suspend fun read(refusal: LeechRefusal?) {
        _state.value = try {
            LeechScreenState.Ready(owner.use { it.leechList(wallClock()) }, refusal)
        } catch (_: CairnException) {
            LeechScreenState.Failed
        }
    }

    /** *Suspend* on a row: the card stops coming up and moves to the suspended section. */
    fun suspend(row: LeechRowFact) = write(row, suspending = true)

    /** *Unsuspend* on a row: the card returns to review, and to the ranked list if it still qualifies. */
    fun unsuspend(row: LeechRowFact) = write(row, suspending = false)

    private fun write(row: LeechRowFact, suspending: Boolean) {
        viewModelScope.launch {
            val refusal = try {
                owner.use { if (suspending) it.suspendCard(row.noteId, row.slot) else it.unsuspendCard(row.noteId, row.slot) }
                null
            } catch (_: CairnException) {
                LeechRefusal(row.noteId, row.slot, suspending)
            }
            read(refusal)
        }
    }
}
