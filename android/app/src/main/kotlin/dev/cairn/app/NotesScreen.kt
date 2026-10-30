package dev.cairn.app

import android.text.BidiFormatter
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.items
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.cairn.design.Cairn
import dev.cairn.design.CairnConfirmRow
import dev.cairn.design.CairnControl
import dev.cairn.design.CairnInertRow
import dev.cairn.design.CairnQuietTarget
import dev.cairn.design.RowAction
import dev.cairn.design.RowIcon
import dev.cairn.ffi.NoteRowFact
import dev.cairn.design.CairnFieldLabel
import dev.cairn.design.CairnDropdown
import dev.cairn.design.CairnListRow
import dev.cairn.design.CairnPage
import dev.cairn.design.CairnRule
import dev.cairn.design.CairnText
import dev.cairn.design.CairnTextField
import dev.cairn.design.Tier
import dev.cairn.design.gap
import dev.cairn.design.rememberKeyboardUp
import dev.cairn.design.tokens.Frame
import dev.cairn.ffi.DeckSelector

/** Notes: the note list, its deck filter and search (ADR-0021 §2, §9; ADR-0039). */
@Composable
fun NotesScreen(model: NotesViewModel, onOpen: (String) -> Unit = {}, onCreate: (deck: String?) -> Unit = {}) {
    val state by model.state.collectAsStateWithLifecycle()
    // Coming back from the editor, the list is read again: a note was born, changed or filed.
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { model.refresh() }
    when (val s = state) {
        is NotesScreenState.Loading -> CairnPage {}
        is NotesScreenState.Ready -> NoteList(
            s,
            onSearch = model::setSearch,
            onSelect = model::selectDeck,
            onNewDeck = model::setNewDeck,
            onCreateDeck = model::createDeck,
            onAskDelete = model::askDeleteDeck,
            onKeep = model::keepDeck,
            onDelete = model::deleteDeck,
            rows = RowCallbacks(
                onAskDelete = model::askDeleteNote,
                onKeep = model::keepNote,
                onDelete = model::deleteNote,
                onMove = model::startMove,
                onCancelMove = model::cancelMove,
                onPlace = model::placeAt,
            ),
            onOpen = onOpen,
            onCreate = onCreate,
        )
        is NotesScreenState.Failed ->
            CairnPage { CairnText(stringResource(R.string.review_unavailable), Modifier.testTag("notes-failed")) }
    }
}

/** What a row's actions and the placement state do: each takes the note (or gap) it was pressed on. */
class RowCallbacks(
    val onAskDelete: (String) -> Unit,
    val onKeep: () -> Unit,
    val onDelete: () -> Unit,
    val onMove: (String) -> Unit,
    val onCancelMove: () -> Unit,
    val onPlace: (Int) -> Unit,
)

@Composable
fun NoteList(
    state: NotesScreenState.Ready,
    onSearch: (String) -> Unit,
    onSelect: (DeckSelector) -> Unit,
    onNewDeck: (String) -> Unit,
    onCreateDeck: () -> Unit,
    onAskDelete: () -> Unit,
    onKeep: () -> Unit,
    onDelete: () -> Unit,
    rows: RowCallbacks,
    modifier: Modifier = Modifier,
    onOpen: (String) -> Unit = {},
    onCreate: (deck: String?) -> Unit = {},
) {
    val facts = state.facts
    CairnPage(modifier) {
        Column(Modifier.fillMaxSize()) {
            LazyColumn(Modifier.fillMaxWidth().weight(1f).padding(top = gap(2))) {
                item {
                    CairnText(stringResource(R.string.notes_heading), tier = Tier.Heading)
                    Spacer(Modifier.height(gap(2)))
                }
                // The empty state is the empty *collection*, not a filter that matched nothing.
                if (!facts.hasNotes) {
                    item {
                        CairnText(stringResource(R.string.notes_empty), Modifier.testTag("notes-empty"))
                        Spacer(Modifier.height(gap(2)))
                        // The sentence's first verb is *create a deck*, so the control for it is here too
                        // (ADR-0021 §2 puts a surface behind each of the three). Once a deck exists the
                        // whole deck surface returns with it, so the new deck can be seen and deleted.
                        if (facts.decks.isEmpty()) {
                            NewDeck(state, onNewDeck, onCreateDeck)
                        } else {
                            DeckControls(state, onSelect, onNewDeck, onCreateDeck, onAskDelete, onKeep, onDelete)
                        }
                    }
                    return@LazyColumn
                }
                item {
                    DeckControls(state, onSelect, onNewDeck, onCreateDeck, onAskDelete, onKeep, onDelete)
                    Spacer(Modifier.height(gap(2)))
                    CairnFieldLabel(stringResource(R.string.notes_search))
                    CairnTextField(state.search, onSearch, Modifier.testTag("notes-search"))
                    Spacer(Modifier.height(gap(2)))
                    CairnRule()
                    Spacer(Modifier.height(gap(2)))
                }
                if (facts.rows.isEmpty()) {
                    item { CairnText(stringResource(R.string.notes_no_match), Modifier.testTag("notes-no-match")) }
                    return@LazyColumn
                }
                val held = facts.rows.firstOrNull { it.noteId == state.moving }
                if (held != null) {
                    placement(facts.rows, held, rows)
                    return@LazyColumn
                }
                items(facts.rows, key = { it.noteId }) { row ->
                    Spacer(Modifier.height(gap(1)))
                    if (row.noteId == state.confirmingNote) {
                        DeleteQuestion(row.preview, rows)
                    } else {
                        val move = stringResource(R.string.notes_move)
                        val delete = stringResource(R.string.notes_delete)
                        CairnListRow(
                            title = row.preview,
                            caption = if (facts.captioned) row.deckName ?: stringResource(R.string.notes_unfiled) else null,
                            modifier = Modifier.testTag("note-row"),
                            actions = listOf(
                                RowAction(RowIcon.Move, move, "row-move") { rows.onMove(row.noteId) },
                                RowAction(RowIcon.Delete, delete, "row-delete") { rows.onAskDelete(row.noteId) },
                            ),
                            onClick = { onOpen(row.noteId) },
                        )
                    }
                }
            }
            // Pinned outside the scroll on the reach line (ADR-0039 §8) while the keyboard is down, so
            // the list keeps the room while one types: a new note is a `basic` draft filed to the deck
            // the list is filtered to.
            if (!rememberKeyboardUp()) {
                CairnControl(
                    stringResource(R.string.notes_create),
                    onClick = { onCreate((state.selector as? DeckSelector.Deck)?.id) },
                    modifier = Modifier.fillMaxWidth().testTag("create-note"),
                    primary = true,
                )
                Spacer(Modifier.height(Frame.REACH_LINE.dp))
            }
        }
    }
}

/** *Delete ⟨preview⟩? It cannot be undeleted.* in the note's own row, the preview bidi-isolated (ADR-0048 §4). */
@Composable
private fun DeleteQuestion(preview: String, rows: RowCallbacks) {
    CairnConfirmRow(
        question = stringResource(R.string.notes_delete_note_question, BidiFormatter.getInstance().unicodeWrap(preview)),
        confirm = stringResource(R.string.notes_delete),
        keep = stringResource(R.string.notes_keep_it),
        onConfirm = rows.onDelete,
        onKeep = rows.onKeep,
        questionModifier = Modifier.testTag("row-delete-warning"),
        confirmModifier = Modifier.testTag("row-delete-confirm"),
        keepModifier = Modifier.testTag("row-keep"),
    )
}

/**
 * The placement state (ADR-0039 §7): *Cancel move*, *Placing* over the held note, then a quiet
 * *Place here* in every gap between the other visible notes, which are real rows that offer nothing.
 */
private fun LazyListScope.placement(all: List<NoteRowFact>, held: NoteRowFact, rows: RowCallbacks) {
    val others = all.filter { it.noteId != held.noteId }
    item {
        CairnControl(stringResource(R.string.notes_cancel_move), onClick = rows.onCancelMove, modifier = Modifier.fillMaxWidth().testTag("cancel-move"))
        Spacer(Modifier.height(gap(2)))
        CairnFieldLabel(stringResource(R.string.notes_placing))
        Spacer(Modifier.height(gap(1)))
        CairnInertRow(held.preview, Modifier.testTag("held-row"), held = true)
        Spacer(Modifier.height(gap(2)))
    }
    for (at in 0..others.size) {
        item(key = "gap-$at") {
            CairnQuietTarget(stringResource(R.string.notes_place_here), onClick = { rows.onPlace(at) }, modifier = Modifier.testTag("place-here"))
        }
        others.getOrNull(at)?.let { row ->
            item(key = row.noteId) {
                Spacer(Modifier.height(gap(1)))
                CairnInertRow(row.preview, Modifier.testTag("inert-row"))
                Spacer(Modifier.height(gap(1)))
            }
        }
    }
}

/** The deck filter, *New deck* beside the name being typed, and the filtered deck's delete (ADR-0021 §9). */
@Composable
private fun DeckControls(
    state: NotesScreenState.Ready,
    onSelect: (DeckSelector) -> Unit,
    onNewDeck: (String) -> Unit,
    onCreateDeck: () -> Unit,
    onAskDelete: () -> Unit,
    onKeep: () -> Unit,
    onDelete: () -> Unit,
) {
    val allDecks = stringResource(R.string.notes_all_decks)
    val unfiled = stringResource(R.string.notes_unfiled)
    val decks = state.facts.decks
    val choices = listOf<DeckSelector>(DeckSelector.All, DeckSelector.Unfiled) + decks.map { DeckSelector.Deck(it.id) }
    val labels = listOf(allDecks, unfiled) + decks.map { it.name }
    // A filter naming a deck the collection no longer holds reads as *Unfiled*, like a dangling reference.
    val selectedName = when (val s = state.selector) {
        is DeckSelector.All -> allDecks
        is DeckSelector.Unfiled -> unfiled
        is DeckSelector.Deck -> decks.firstOrNull { it.id == s.id }?.name ?: unfiled
    }

    CairnFieldLabel(stringResource(R.string.notes_deck))
    CairnDropdown(selectedName, labels, onSelect = { onSelect(choices[it]) }, tag = "deck-filter")
    Spacer(Modifier.height(gap(1)))
    NewDeck(state, onNewDeck, onCreateDeck)

    // Delete exists only for the deck the filter names, and asks first, with the count.
    if (state.selector !is DeckSelector.Deck) return
    Spacer(Modifier.height(gap(1)))
    if (state.refusal == NotesScreenState.Refusal.DeleteDeck) {
        CairnText(stringResource(R.string.notes_delete_deck_refused), Modifier.testTag("delete-deck-refused"), color = Cairn.colors.error)
        Spacer(Modifier.height(gap(1)))
    }
    val count = state.confirmingCount
    if (count == null) {
        CairnControl(stringResource(R.string.notes_delete_deck), onClick = onAskDelete, modifier = Modifier.testTag("delete-deck"))
    } else {
        val name = BidiFormatter.getInstance().unicodeWrap(selectedName)
        val warning = when (count) {
            0L -> stringResource(R.string.notes_delete_deck_empty, name)
            else -> pluralStringResource(R.plurals.notes_delete_deck_warning, count.toInt(), name, count.toInt())
        }
        CairnText(warning, Modifier.testTag("delete-deck-warning"))
        Spacer(Modifier.height(gap(1)))
        // Stacked: the confirming label is as long as a phone's width allows, and a row would crush the other.
        Column(verticalArrangement = Arrangement.spacedBy(gap(1))) {
            CairnControl(stringResource(R.string.notes_delete_deck_confirm), onClick = onDelete, modifier = Modifier.testTag("delete-deck-confirm"))
            CairnControl(stringResource(R.string.notes_keep_it), onClick = onKeep, modifier = Modifier.testTag("keep-deck"))
        }
    }
}

/** *New deck* beside the name being typed, and — when the collection refused it — the refusal under them. */
@Composable
private fun NewDeck(state: NotesScreenState.Ready, onNewDeck: (String) -> Unit, onCreateDeck: () -> Unit) {
    Row(horizontalArrangement = Arrangement.spacedBy(gap(1)), verticalAlignment = Alignment.CenterVertically) {
        CairnControl(stringResource(R.string.notes_new_deck), onClick = onCreateDeck, modifier = Modifier.testTag("new-deck"))
        CairnTextField(state.newDeck, onNewDeck, Modifier.weight(1f).testTag("new-deck-name"))
    }
    if (state.refusal == NotesScreenState.Refusal.CreateDeck) {
        Spacer(Modifier.height(gap(1)))
        CairnText(stringResource(R.string.notes_new_deck_refused), Modifier.testTag("new-deck-refused"), color = Cairn.colors.error)
    }
}
