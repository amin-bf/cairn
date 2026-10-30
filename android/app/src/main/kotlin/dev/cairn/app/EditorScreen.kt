package dev.cairn.app

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusDirection
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.repeatOnLifecycle
import androidx.compose.ui.semantics.semantics
import dev.cairn.design.Cairn
import dev.cairn.design.CairnCard
import dev.cairn.design.CairnControl
import dev.cairn.design.CairnDestinationsRow
import dev.cairn.design.CairnLeftRule
import dev.cairn.design.CairnOutlineEntry
import dev.cairn.design.CardRun
import dev.cairn.design.Destination
import dev.cairn.design.tokens.Surface
import dev.cairn.ffi.CardPaneFacts
import dev.cairn.ffi.DormantFact
import dev.cairn.ffi.DormantName
import dev.cairn.ffi.PaneEntry
import dev.cairn.ffi.PaneState
import dev.cairn.ffi.Run
import dev.cairn.ffi.RunStyle
import dev.cairn.design.CairnDropdown
import dev.cairn.design.CairnFieldLabel
import dev.cairn.design.CairnLabeledField
import dev.cairn.design.CairnPage
import dev.cairn.design.CairnText
import dev.cairn.design.CairnTextAction
import dev.cairn.design.CairnTextField
import dev.cairn.design.ReachColumn
import dev.cairn.design.Tier
import dev.cairn.design.gap
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext

/** How often the idle check asks, so a pause of two seconds settles within a fraction of one more. */
private const val IDLE_CHECK_MS = 500L

/**
 * The editor (ADR-0040, ADR-0048): the form alone for now. [onDone] leaves, [onNewNote] opens a fresh
 * draft in place of this one. Every way out settles all first, the system Back included, and so does
 * the activity stopping (ADR-0021 §7).
 */
@Composable
fun EditorScreen(
    model: EditorViewModel,
    onDone: () -> Unit,
    onNewNote: (kind: String, deck: String?) -> Unit,
) {
    val state by model.state.collectAsStateWithLifecycle()
    BackHandler { model.settleAll(onDone) }
    LifecycleEventEffect(Lifecycle.Event.ON_STOP) { model.settleAll() }
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    LaunchedEffect(model, lifecycle) {
        lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) {
            while (true) {
                // Wall-clock time, off the composition's own clock: the idle window is real seconds.
                withContext(Dispatchers.Default) { delay(IDLE_CHECK_MS) }
                model.tick()
            }
        }
    }
    when (val s = state) {
        is EditorScreenState.Loading -> CairnPage {}
        is EditorScreenState.Ready -> EditorForm(model, s, onDone, onNewNote)
        is EditorScreenState.Failed ->
            CairnPage { CairnText(stringResource(R.string.review_unavailable), Modifier.testTag("editor-failed")) }
    }
}

@Composable
private fun EditorForm(
    model: EditorViewModel,
    state: EditorScreenState.Ready,
    onDone: () -> Unit,
    onNewNote: (kind: String, deck: String?) -> Unit,
) {
    val unfiled = stringResource(R.string.notes_unfiled)
    val deckChoices = listOf<String?>(null) + state.decks.map { it.id }
    val deckLabels = listOf(unfiled) + state.decks.map { it.name }
    val deckName = state.decks.firstOrNull { it.id == state.deck }?.name ?: unfiled
    CairnPage {
        BoxWithConstraints(Modifier.testTag("page-room")) {
            val room = maxHeight
            Column(Modifier.fillMaxWidth().verticalScroll(rememberScrollState())) {
                ReachColumn(
                    room = room,
                    floor = gap(3),
                    top = {
                        Column(Modifier.padding(top = gap(2))) {
                            CairnText(
                                stringResource(if (state.isNew) R.string.editor_heading_new else R.string.editor_heading_edit),
                                Modifier.testTag("editor-heading"),
                                tier = Tier.Heading,
                            )
                            Spacer(Modifier.height(gap(2)))
                            // The panes always fold on a phone (ADR-0048 §7): one toggle, and a tap settles all.
                            CairnDestinationsRow(
                                listOf(
                                    Destination(stringResource(R.string.editor_pane_write), !model.showingCards, "pane-write") { model.showCards(false) },
                                    Destination(stringResource(R.string.editor_pane_cards), model.showingCards, "pane-cards") { model.showCards(true) },
                                ),
                            )
                            Spacer(Modifier.height(gap(2)))
                            if (model.showingCards) {
                                CardsPane(model.pane)
                            } else {
                                CairnFieldLabel(stringResource(R.string.editor_kind))
                                CairnDropdown(state.kind, state.kinds, onSelect = { model.chooseKind(state.kinds[it]) }, tag = "kind")
                                Spacer(Modifier.height(gap(2)))
                                CairnFieldLabel(stringResource(R.string.notes_deck))
                                CairnDropdown(deckName, deckLabels, onSelect = { model.chooseDeck(deckChoices[it]) }, tag = "deck")
                                Spacer(Modifier.height(gap(2)))
                                Row(horizontalArrangement = Arrangement.spacedBy(gap(1)), verticalAlignment = Alignment.CenterVertically) {
                                    CairnControl(stringResource(R.string.notes_new_deck), onClick = model::createDeck, modifier = Modifier.testTag("new-deck"))
                                    CairnTextField(model.newDeck, model::typeNewDeck, Modifier.weight(1f).testTag("new-deck-name"))
                                }
                                Spacer(Modifier.height(gap(2)))
                                model.pane?.warning?.takeIf { it.isNotEmpty() }?.let {
                                    DormancyWarning(it)
                                    Spacer(Modifier.height(gap(2)))
                                }
                                Fields(model, state)
                            }
                        }
                    },
                    reach = {
                        Column(Modifier.fillMaxWidth()) {
                            CairnTextAction(
                                stringResource(R.string.editor_new_note),
                                onClick = { model.newNote(onNewNote) },
                                modifier = Modifier.testTag("new-note"),
                            )
                            Spacer(Modifier.height(gap(1)))
                            CairnControl(
                                stringResource(R.string.editor_done),
                                onClick = { model.settleAll(onDone) },
                                modifier = Modifier.fillMaxWidth().testTag("done"),
                                primary = true,
                            )
                        }
                    },
                )
            }
        }
    }
}

/**
 * The fields in kind order, each laid out by its own first strong character. The keyboard's action key
 * is *Next* in a single-line field and closes the keyboard on the last one; it commits nothing. A
 * multi-line field's key types a new line (ADR-0048 §3).
 */
@Composable
private fun Fields(model: EditorViewModel, state: EditorScreenState.Ready) {
    val focus = LocalFocusManager.current
    val keyboard = LocalSoftwareKeyboardController.current
    state.fields.forEachIndexed { place, field ->
        val last = place == state.fields.lastIndex
        var focused by remember(field.name) { mutableStateOf(false) }
        CairnLabeledField(
            label = field.name,
            value = model.buffer(field.name),
            onValueChange = { model.typed(field.name, it) },
            modifier = Modifier
                .testTag("field-${field.name}")
                .onFocusChanged {
                    // Losing focus is the blur that commits; gaining it commits nothing.
                    if (focused && !it.isFocused) model.blur(field.name)
                    focused = it.isFocused
                },
            keyboardOptions = KeyboardOptions(
                imeAction = when {
                    field.multiline -> ImeAction.Default
                    last -> ImeAction.Done
                    else -> ImeAction.Next
                },
            ),
            keyboardActions = KeyboardActions(
                onNext = { focus.moveFocus(FocusDirection.Next) },
                onDone = { keyboard?.hide() },
            ),
            singleLine = !field.multiline,
        )
        // The multi-line field is the cloze Text, and it carries *Blank it*: enabled while text is selected.
        if (field.multiline) {
            Spacer(Modifier.height(gap(1)))
            CairnControl(
                stringResource(R.string.editor_blank_it),
                onClick = { model.blankIt(field.name) },
                modifier = Modifier.fillMaxWidth().testTag("blank-it"),
                enabled = !model.buffer(field.name).selection.collapsed,
            )
        }
        Spacer(Modifier.height(gap(2)))
    }
}

/** The dormant entry's one line, worded *kept* (ADR-0018 §3): its name, *dormant*, its history. */
@Composable
private fun dormantLine(entry: DormantFact): String {
    val name = when (val n = entry.name) {
        is DormantName.Roles -> stringResource(R.string.editor_dormant_roles, n.prompt.joinToString(", "), n.answer.joinToString(", "))
        is DormantName.Blank -> stringResource(R.string.editor_dormant_blank, n.number.toInt())
        is DormantName.Slot -> stringResource(R.string.editor_dormant_slot, n.slot.toInt())
    }
    return stringResource(R.string.editor_dormant_history, name, entry.reviews.toInt())
}

/**
 * The ambient warning above the fields (ADR-0012 §5, ADR-0025 §4): the dormant names at body, the
 * reassurance as the aside, inside a left rule. Recomputed from what the store holds, never a modal.
 */
@Composable
private fun DormancyWarning(dormant: List<DormantFact>) {
    CairnLeftRule(Modifier.testTag("dormancy-warning").semantics(mergeDescendants = true) {}) {
        CairnText(stringResource(R.string.editor_dormant_lead))
        Spacer(Modifier.height(gap(1)))
        for (entry in dormant) CairnText(dormantLine(entry))
        Spacer(Modifier.height(gap(1)))
        CairnText(stringResource(R.string.editor_dormant_undo), tier = Tier.Small, color = Cairn.colors.weak)
    }
}

private fun List<Run>.toCardRuns(): List<CardRun> = map {
    CardRun(
        it.text,
        bold = it.style == RunStyle.BOLD || it.style == RunStyle.BOLD_ITALIC,
        italic = it.style == RunStyle.ITALIC || it.style == RunStyle.BOLD_ITALIC,
        code = it.style == RunStyle.CODE,
    )
}

/**
 * The Cards pane (ADR-0040 §3, §5; ADR-0018): a preview of what the note will ask. Entries in raw slot
 * order, live and dormant alike. A live one is the design module's card, fully open, with its box
 * badge; a dormant one is an outline at the card's footprint carrying its name and kept reviews.
 */
@Composable
private fun CardsPane(pane: CardPaneFacts?) {
    CairnFieldLabel(stringResource(R.string.editor_pane_cards))
    Spacer(Modifier.height(gap(1)))
    // Not read yet: say nothing rather than a false "No cards yet."
    if (pane == null) return
    if (pane.entries.isEmpty()) {
        CairnText(stringResource(R.string.editor_cards_none), Modifier.testTag("cards-empty"))
        return
    }
    for (entry in pane.entries) {
        when (entry) {
            is PaneEntry.Live -> {
                val card = entry.card
                CairnCard(
                    prompt = card.prompt.toCardRuns(),
                    answer = card.answer.toCardRuns(),
                    badge = if (card.isNew) stringResource(R.string.review_badge_new) else stringResource(R.string.review_badge_box, card.boxNumber.toInt()),
                    rtl = card.rtl,
                    revealed = true,
                    onReveal = {},
                    minHeight = Surface.FIT,
                )
            }
            is PaneEntry.Dormant -> CairnOutlineEntry(dormantLine(entry.entry), Modifier.testTag("dormant-entry"))
        }
        Spacer(Modifier.height(gap(2)))
    }
    if (pane.state == PaneState.NO_LIVE_CARDS) {
        CairnText(stringResource(R.string.editor_cards_no_live), Modifier.testTag("cards-no-live"), tier = Tier.Small, color = Cairn.colors.weak)
    }
}
