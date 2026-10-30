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
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.repeatOnLifecycle
import dev.cairn.design.CairnControl
import dev.cairn.design.CairnDropdown
import dev.cairn.design.CairnFieldLabel
import dev.cairn.design.CairnLabeledField
import dev.cairn.design.CairnPage
import dev.cairn.design.CairnText
import dev.cairn.design.CairnTextAction
import dev.cairn.design.CairnTextField
import dev.cairn.design.ReachColumn
import dev.cairn.design.Tier
import dev.cairn.design.tokens.Spacing
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext

private fun gap(units: Int): Dp = (Spacing.UNIT * units).dp

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
                            Fields(model, state)
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
            value = model.values[field.name].orEmpty(),
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
        Spacer(Modifier.height(gap(2)))
    }
}
