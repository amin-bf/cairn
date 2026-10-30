package dev.cairn.app

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.DeckSelector
import dev.cairn.ffi.NoteListFacts
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

/** What Notes shows: the list as facts, with the controls that narrow it. */
sealed interface NotesScreenState {
    data object Loading : NotesScreenState

    /** Which write the collection refused, shown where the person did it. */
    enum class Refusal { CreateDeck, DeleteDeck }

    /**
     * The list under [selector] and [search], the [newDeck] name being typed, and — when a deck's
     * delete has been asked for and not yet confirmed — how many notes it would take
     * ([confirmingCount], the binding's count, never counted here).
     */
    data class Ready(
        val facts: NoteListFacts,
        val selector: DeckSelector,
        val search: String,
        val newDeck: String,
        val confirmingCount: Long?,
        val refusal: Refusal? = null,
    ) : NotesScreenState

    data object Failed : NotesScreenState
}

/**
 * Notes' state. Every call goes through the owner (off the main thread); this holds the three things
 * the person has set — the deck filter, the search text and the new deck's name — and asks the binding
 * for the list under them. Which rows match, whether a caption applies and whether the collection is
 * empty are the binding's facts (ADR-0046 §1).
 */
class NotesViewModel(private val owner: CollectionAccess) : ViewModel() {
    private val _state = MutableStateFlow<NotesScreenState>(NotesScreenState.Loading)
    val state: StateFlow<NotesScreenState> = _state

    private var selector: DeckSelector = DeckSelector.All
    private var search = ""
    private var newDeck = ""
    private var confirming: String? = null
    private var refusal: NotesScreenState.Refusal? = null
    private var reading: Job? = null

    init {
        refresh()
    }

    /** Ask for the list again under what is set now; a read still under way is dropped, the later one wins. */
    fun refresh() {
        reading?.cancel()
        reading = viewModelScope.launch {
            _state.value = try {
                val (facts, count) = owner.use { handle ->
                    val facts = handle.noteList(selector, search)
                    facts to confirming?.let { handle.notesInDeck(it).toLong() }
                }
                NotesScreenState.Ready(facts, selector, search, newDeck, count, refusal)
            } catch (_: CairnException) {
                NotesScreenState.Failed
            }
        }
    }

    /** The search text changed. It is shown at once; the list follows. */
    fun setSearch(text: String) {
        search = text
        (_state.value as? NotesScreenState.Ready)?.let { _state.value = it.copy(search = text) }
        refresh()
    }

    /** A filter value was chosen: *All decks*, *Unfiled* or a deck. Asking to delete another deck is dropped. */
    fun selectDeck(choice: DeckSelector) {
        selector = choice
        confirming = null
        refusal = null
        refresh()
    }

    fun setNewDeck(text: String) {
        newDeck = text
        refusal = null
        (_state.value as? NotesScreenState.Ready)?.let { _state.value = it.copy(newDeck = text) }
    }

    /**
     * *New deck*: create it and filter to it — it was made to be used (ADR-0021 §9). A blank name makes
     * nothing: that is the operation's rule, and it answers with no id. A refusal is shown in place.
     */
    fun createDeck() {
        viewModelScope.launch {
            try {
                val id = owner.use { it.createDeck(newDeck) } ?: return@launch
                newDeck = ""
                selector = DeckSelector.Deck(id)
                confirming = null
                refusal = null
            } catch (_: CairnException) {
                refusal = NotesScreenState.Refusal.CreateDeck
            }
            refresh()
        }
    }

    /** *Delete deck*: ask first, naming how many notes would go with it. */
    fun askDeleteDeck() {
        val deck = (selector as? DeckSelector.Deck)?.id ?: return
        confirming = deck
        refusal = null
        refresh()
    }

    /** *Keep it*. */
    fun keepDeck() {
        confirming = null
        refusal = null
        refresh()
    }

    /** *Delete the deck and its notes*: confirmed. The filter returns to *All decks*. */
    fun deleteDeck() {
        val deck = confirming ?: return
        viewModelScope.launch {
            try {
                owner.use { it.deleteDeck(deck) }
                confirming = null
                selector = DeckSelector.All
                refusal = null
            } catch (_: CairnException) {
                // The warning stays up, so the person can try again or keep the deck.
                refusal = NotesScreenState.Refusal.DeleteDeck
            }
            refresh()
        }
    }
}
