package dev.cairn.app

import android.os.SystemClock
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.SavedStateHandle
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.cairn.design.wrapAsBlank
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.CardPaneFacts
import dev.cairn.ffi.DeckFact
import dev.cairn.ffi.EditingHandle
import dev.cairn.ffi.FieldValue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

/** What the editor shows besides the text itself: the heading's fact, the dropdowns and the field list. */
sealed interface EditorScreenState {
    data object Loading : EditorScreenState

    data class Field(val name: String, val multiline: Boolean)

    data class Ready(
        val isNew: Boolean,
        val kinds: List<String>,
        val kind: String,
        val decks: List<DeckFact>,
        val deck: String?,
        val fields: List<Field>,
    ) : EditorScreenState

    data object Failed : EditorScreenState
}

/** The route's arguments: a stored note, or the draft's kind and deck. */
internal const val ARG_NOTE = "note"
internal const val ARG_KIND = "kind"
internal const val ARG_DECK = "deck"

/** The kind a new note starts as. */
internal const val NEW_KIND = "basic"

/**
 * The editor's state (ADR-0048, ADR-0021 §7). The text being typed is Compose's own: [values] holds
 * each field's buffer and is written on the main thread, in the same call that Compose's field makes,
 * so the field's composition is never round-tripped through the binding. Every call into the binding
 * is queued here and run one at a time, in order, off the main thread; whether a field needs writing,
 * when a draft is born and when the idle window falls due are the editing handle's rules.
 *
 * The route carries the note id, or the draft's kind and deck. Once a draft is born its id is put in
 * [saved] too, so a process death returns to the same note rather than to a fresh draft.
 */
class EditorViewModel(
    private val owner: CollectionAccess,
    private val saved: SavedStateHandle,
    private val monotonicClock: () -> Long = SystemClock::elapsedRealtime,
) : ViewModel() {
    private val _state = MutableStateFlow<EditorScreenState>(EditorScreenState.Loading)
    val state: StateFlow<EditorScreenState> = _state

    /** Each field's buffer by name: its text, selection and composition are Compose's own. */
    private val buffers = mutableStateMapOf<String, TextFieldValue>()

    /** Each field's text by name. */
    val values: Map<String, String> get() = buffers.mapValues { it.value.text }

    fun buffer(field: String): TextFieldValue = buffers[field] ?: TextFieldValue()

    /** What the Cards pane and the warning draw: the note's cards as the store holds them. */
    var pane by mutableStateOf<CardPaneFacts?>(null)
        private set

    /** Whether the Cards pane shows rather than the form (ADR-0048 §7). */
    var showingCards by mutableStateOf(false)
        private set

    /** Whether a typing pause was pending at the last tick, so the tick that settles it re-reads the pane. */
    private var settlePending = false

    /** The name being typed for *New deck*. */
    var newDeck by mutableStateOf("")
        private set

    private var editing: EditingHandle? = null
    private val queue = Channel<suspend () -> Unit>(Channel.UNLIMITED)

    init {
        viewModelScope.launch {
            for (job in queue) {
                try {
                    job()
                } catch (_: CairnException) {
                    _state.value = EditorScreenState.Failed
                }
            }
        }
        enqueue {
            val handle = owner.use { collection ->
                val note = saved.get<String>(ARG_NOTE)
                if (note != null) {
                    collection.editNote(note)
                } else {
                    collection.newDraft(saved.get<String>(ARG_KIND) ?: NEW_KIND, saved.get<String>(ARG_DECK))
                }
            }
            editing = handle
            show(handle, reload = true)
        }
    }

    private fun enqueue(job: suspend () -> Unit) {
        queue.trySend(job)
    }

    /**
     * Read what the screen draws from the handle. The buffers are taken from the store only when
     * [reload] says the fields are new (the first read, a kind change); otherwise what is typed stays.
     */
    private suspend fun show(handle: EditingHandle, reload: Boolean = false) {
        val (ready, stored) = owner.use {
            val fields = handle.fields()
            EditorScreenState.Ready(
                isNew = handle.isNew(),
                kinds = handle.kindOptions(),
                kind = handle.kind(),
                decks = handle.decks(),
                deck = handle.deck(),
                fields = fields.map { EditorScreenState.Field(it.name, it.multiline) },
            ) to fields
        }
        if (reload) {
            buffers.clear()
            stored.forEach { buffers[it.name] = TextFieldValue(it.value) }
        }
        refreshPane(handle)
        _state.value = ready
    }

    private fun persistNoteId(note: String?) {
        if (note != null) saved[ARG_NOTE] = note
    }

    private fun snapshot() = buffers.map { FieldValue(it.key, it.value.text) }

    /** Read the pane from the store again, after something wrote to it. */
    private suspend fun refreshPane(handle: EditingHandle) {
        pane = owner.use { handle.cardPane() }
    }

    /** A field changed: show it. A change of text restarts the idle window; a move of the caret does not. */
    fun typed(field: String, value: TextFieldValue) {
        val changed = buffers[field]?.text != value.text
        buffers[field] = value
        if (!changed) return
        val now = monotonicClock().toULong()
        enqueue { editing?.let { h -> owner.use { h.touched(field, value.text, now) } } }
    }

    /** A field lost focus: commit it. */
    fun blur(field: String) {
        val text = buffers[field]?.text ?: return
        enqueue {
            editing?.let { h ->
                owner.use { h.commitField(field, text) }
                persistNoteId(owner.use { h.noteId() })
                refreshPane(h)
            }
        }
    }

    /** The ticking check: settle once the typing has paused, then read the pane the settling changed. */
    fun tick() {
        val now = monotonicClock().toULong()
        enqueue {
            editing?.let { h ->
                val stillWaiting = owner.use { h.settleIfIdle(now) }
                persistNoteId(owner.use { h.noteId() })
                if (stillWaiting != null) {
                    settlePending = true
                } else if (settlePending) {
                    settlePending = false
                    refreshPane(h)
                }
            }
        }
    }

    /** Settle every field, then [then]. Used on stopping, and by every way out. */
    fun settleAll(then: () -> Unit = {}) {
        val snapshot = snapshot()
        enqueue {
            settle(snapshot)
            then()
        }
    }

    /**
     * Settle every field and return once it is done, failed or not: a file arriving from another app
     * waits on it before the preview takes the screen (ADR-0021 §7, ADR-0052 §2).
     */
    suspend fun settleAllAndWait() {
        val done = CompletableDeferred<Unit>()
        val snapshot = snapshot()
        enqueue {
            try {
                settle(snapshot)
            } finally {
                done.complete(Unit)
            }
        }
        done.await()
    }

    private suspend fun settle(snapshot: List<FieldValue>) {
        editing?.let { h -> persistNoteId(owner.use { h.settleAll(snapshot) }) }
    }

    /** `Write | Cards`: either way it settles all, and the Cards pane then shows what the store holds (ADR-0048 §7). */
    fun showCards(cards: Boolean) {
        showingCards = cards
        val snapshot = snapshot()
        enqueue {
            editing?.let { h ->
                persistNoteId(owner.use { h.settleAll(snapshot) })
                refreshPane(h)
            }
        }
    }

    /**
     * *Blank it* (ADR-0012 §3): wrap the field's selection as `{{n::…}}`, [n] one above the highest ever
     * used, which the binding answers because the text alone cannot see a dormant blank. A selection
     * that is only a caret makes nothing.
     */
    fun blankIt(field: String) {
        val before = buffers[field] ?: return
        if (before.selection.collapsed) return
        enqueue {
            val h = editing ?: return@enqueue
            val n = owner.use { h.nextBlankNumber(before.text) }
            // Typing that arrived meanwhile wins: the selection no longer points into this text.
            val now = buffers[field] ?: return@enqueue
            if (now.text != before.text) return@enqueue
            val (wrapped, caret) = wrapAsBlank(now.text, now.selection.start, now.selection.end, n.toInt())
            buffers[field] = TextFieldValue(wrapped, TextRange(caret))
            owner.use { h.touched(field, wrapped, monotonicClock().toULong()) }
        }
    }

    /** *New note*: settle this one, then hand the kind and deck forward to a fresh draft. */
    fun newNote(open: (kind: String, deck: String?) -> Unit) {
        val snapshot = snapshot()
        enqueue {
            val h = editing ?: return@enqueue
            owner.use { h.settleAll(snapshot) }
            val (kind, deck) = owner.use { h.kind() to h.deck() }
            open(kind, deck)
        }
    }

    /** A kind was chosen. A stored note is written; the fields rebuild, carrying the shared names. */
    fun chooseKind(kind: String) {
        val snapshot = snapshot()
        enqueue {
            val h = editing ?: return@enqueue
            owner.use {
                h.settleAll(snapshot)
                h.switchKind(kind)
            }
            persistNoteId(owner.use { h.noteId() })
            show(h, reload = true)
        }
    }

    /** A deck was chosen, or none. A stored note is written; a draft holds it until it is born. */
    fun chooseDeck(deck: String?) {
        enqueue {
            val h = editing ?: return@enqueue
            owner.use { h.setDeck(deck) }
            show(h)
        }
    }

    fun typeNewDeck(text: String) {
        newDeck = text
    }

    /** *New deck*: make it and file the note under it. A blank name makes nothing. */
    fun createDeck() {
        val name = newDeck.trim()
        if (name.isEmpty()) return
        newDeck = ""
        enqueue {
            val h = editing ?: return@enqueue
            // A name the collection makes nothing of leaves the note's deck as it was.
            owner.use { collection -> collection.createDeck(name)?.let { h.setDeck(it) } }
            show(h)
        }
    }
}
