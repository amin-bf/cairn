package dev.cairn.app

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.LeechListFacts
import dev.cairn.ffi.LeechRowFact
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

/** A row of the leech screen: one card of one note. */
data class LeechRowKey(val noteId: String, val slot: UShort)

/** The write a row asked for: a refusal says which, so each is worded as it deserves. */
enum class LeechWrite { Suspend, Unsuspend, Delete }

/** The row a refused write came from, and which write it was: the refusal is said in that row. */
data class LeechRefusal(val row: LeechRowKey, val write: LeechWrite)

/** The row a fact belongs to. */
val LeechRowFact.key: LeechRowKey get() = LeechRowKey(noteId, slot)

/** What the leech screen shows: the two sections as facts, and the write the store last refused. */
sealed interface LeechScreenState {
    data object Loading : LeechScreenState

    data class Ready(
        val facts: LeechListFacts,
        val refusal: LeechRefusal? = null,
        /** The row whose *Delete* is asking for confirmation, if any (ADR-0050 §2). */
        val confirming: LeechRowKey? = null,
    ) : LeechScreenState

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

    /** The row whose *Delete* is asking; kept here so a rotation keeps the question (ADR-0050 §2). */
    private var confirming: LeechRowKey? = null
    private var deleting = false

    init {
        refresh()
    }

    /** Read the list again, keeping a refusal said in a row. */
    fun refresh() {
        viewModelScope.launch { read(null) }
    }

    private suspend fun read(refusal: LeechRefusal?) {
        _state.value = try {
            val facts = owner.use { it.leechList(wallClock()) }
            // A question belongs to an active row; once the row is gone or suspended it is not asked again.
            if (facts.active.none { it.key == confirming }) confirming = null
            LeechScreenState.Ready(facts, refusal, confirming)
        } catch (_: CairnException) {
            LeechScreenState.Failed
        }
    }

    /** *Suspend* on a row: the card stops coming up and moves to the suspended section. */
    fun suspend(row: LeechRowFact) = write(row, LeechWrite.Suspend)

    /** *Unsuspend* on a row: the card returns to review, and to the ranked list if it still qualifies. */
    fun unsuspend(row: LeechRowFact) = write(row, LeechWrite.Unsuspend)

    /** *Delete* on a row: ask first, in that row, and write nothing (ADR-0050 §2). */
    fun askDelete(row: LeechRowFact) {
        confirming = row.key
        refresh()
    }

    /** *Keep it*, or leaving the row for the editor: the question and any refusal go, and the note stays. */
    fun keep() {
        confirming = null
        refresh()
    }

    /** *Delete*, confirmed: the note and every card of it go through the operation; there is no undelete. */
    fun delete(row: LeechRowFact) = write(row, LeechWrite.Delete)

    private fun write(row: LeechRowFact, write: LeechWrite) {
        viewModelScope.launch {
            // A second tap on *Delete* while the first is in flight would delete a note that is already gone.
            if (write == LeechWrite.Delete) {
                if (deleting || confirming != row.key) return@launch
                deleting = true
            } else {
                confirming = null
            }
            val refusal = try {
                owner.use {
                    when (write) {
                        LeechWrite.Suspend -> it.suspendCard(row.noteId, row.slot)
                        LeechWrite.Unsuspend -> it.unsuspendCard(row.noteId, row.slot)
                        LeechWrite.Delete -> it.deleteNote(row.noteId)
                    }
                }
                if (write == LeechWrite.Delete) confirming = null
                null
            } catch (_: CairnException) {
                // A refused delete keeps its question in the row, so the person can try again or keep the note.
                LeechRefusal(row.key, write)
            } finally {
                deleting = false
            }
            read(refusal)
        }
    }
}
