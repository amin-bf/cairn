package dev.cairn.app

import android.text.BidiFormatter
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.cairn.design.CairnControl
import dev.cairn.design.CairnFieldLabel
import dev.cairn.design.CairnDropdown
import dev.cairn.design.CairnListRow
import dev.cairn.design.CairnPage
import dev.cairn.design.CairnRule
import dev.cairn.design.CairnText
import dev.cairn.design.CairnTextField
import dev.cairn.design.Tier
import dev.cairn.design.tokens.Spacing
import dev.cairn.ffi.DeckSelector

private fun gap(units: Int): Dp = (Spacing.UNIT * units).dp

/** Notes: the note list, its deck filter and search (ADR-0021 §2, §9; ADR-0039). */
@Composable
fun NotesScreen(model: NotesViewModel) {
    val state by model.state.collectAsStateWithLifecycle()
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
        )
        is NotesScreenState.Failed ->
            CairnPage { CairnText(stringResource(R.string.review_unavailable), Modifier.testTag("notes-failed")) }
    }
}

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
    modifier: Modifier = Modifier,
) {
    val facts = state.facts
    CairnPage(modifier) {
        LazyColumn(Modifier.fillMaxWidth().padding(top = gap(2))) {
            item {
                CairnText(stringResource(R.string.notes_heading), tier = Tier.Heading)
                Spacer(Modifier.height(gap(2)))
            }
            // The empty state is the empty *collection*, not a filter that matched nothing.
            if (!facts.hasNotes) {
                item { CairnText(stringResource(R.string.notes_empty), Modifier.testTag("notes-empty")) }
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
            items(facts.rows, key = { it.noteId }) { row ->
                Spacer(Modifier.height(gap(1)))
                CairnListRow(
                    title = row.preview,
                    caption = if (facts.captioned) row.deckName ?: stringResource(R.string.notes_unfiled) else null,
                    modifier = Modifier.testTag("note-row"),
                )
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
    Row(horizontalArrangement = Arrangement.spacedBy(gap(1)), verticalAlignment = Alignment.CenterVertically) {
        CairnControl(stringResource(R.string.notes_new_deck), onClick = onCreateDeck, modifier = Modifier.testTag("new-deck"))
        CairnTextField(state.newDeck, onNewDeck, Modifier.weight(1f).testTag("new-deck-name"))
    }

    // Delete exists only for the deck the filter names, and asks first, with the count.
    if (state.selector !is DeckSelector.Deck) return
    Spacer(Modifier.height(gap(1)))
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
