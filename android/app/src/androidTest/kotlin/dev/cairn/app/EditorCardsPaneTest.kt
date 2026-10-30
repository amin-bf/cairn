package dev.cairn.app

import android.content.Context
import androidx.activity.ComponentActivity
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.getUnclippedBoundsInRoot
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.AndroidComposeTestRule
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.test.performTextInputSelection
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.text.TextRange
import androidx.lifecycle.SavedStateHandle
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.cairn.design.CairnTheme
import dev.cairn.design.isRightToLeft
import dev.cairn.design.pageArgb
import dev.cairn.ffi.DeckSelector
import dev.cairn.ffi.PaneState
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.io.FileOutputStream

/**
 * The editor's other half on the emulator (ADR-0048 §5, §7): the `Write | Cards` toggle, the Cards
 * pane, the dormancy warning above the fields and *Blank it*. Each pane state is reached through a
 * fixture installed by name, and each is captured in both themes.
 */
@RunWith(AndroidJUnit4::class)
class EditorCardsPaneTest {
    @get:Rule
    val rule: AndroidComposeTestRule<*, ComponentActivity> = createAndroidComposeRule<ComponentActivity>()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "cards-pane-${System.nanoTime()}")
    private val owner = CollectionOwner(context, File(root, "files"), File(root, "no_backup"))

    @After
    fun tearDown() {
        root.deleteRecursively()
    }

    private fun fixture(name: String) = runBlocking { owner.use { it.installFixture(name, System.currentTimeMillis()) } }

    private fun rows() = runBlocking { owner.use { it.noteList(DeckSelector.All, "").rows } }

    private fun storedFront(id: String): String = runBlocking {
        owner.use { h -> h.editNote(id).fields().first { it.name == "Front" }.value }
    }

    /** The first note whose pane is in [state], or whose pane holds a dormant entry when [dormant]. */
    private fun noteWith(state: PaneState, dormant: Boolean? = null): String = runBlocking {
        owner.use { h ->
            rows().first {
                val pane = h.editNote(it.noteId).cardPane()
                pane.state == state && (dormant == null || pane.warning.isNotEmpty() == dormant)
            }.noteId
        }
    }

    private fun show(args: Map<String, Any?>, dark: MutableState<Boolean> = mutableStateOf(false)): EditorViewModel {
        val model = EditorViewModel(owner, SavedStateHandle(args))
        rule.setContent {
            CairnTheme(dark = dark.value) {
                EditorScreen(model, onDone = {}, onNewNote = { _, _ -> })
            }
        }
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("editor-heading")).fetchSemanticsNodes().isNotEmpty() }
        return model
    }

    private fun edit(note: String) = mapOf(ARG_NOTE to note)

    private fun draft(kind: String = "basic") = mapOf(ARG_KIND to kind, ARG_DECK to null)

    private fun showCards() {
        rule.onNodeWithTag("pane-cards").performClick()
        rule.waitForIdle()
    }

    /** The view model queues its work off the main thread, so a result is waited for rather than assumed. */
    private fun awaitText(tag: String, text: String) {
        rule.waitUntil(10_000) {
            val config = rule.onNodeWithTag(tag).fetchSemanticsNode().config
            config.contains(SemanticsProperties.EditableText) && config[SemanticsProperties.EditableText].text == text
        }
        rule.onNodeWithTag(tag).assertTextEquals(text)
    }

    private fun waitForTag(tag: String) {
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag(tag), useUnmergedTree = true).fetchSemanticsNodes().isNotEmpty() }
    }

    @Test
    fun the_write_pane_shows_first_and_the_toggle_switches_to_the_cards_pane_and_back() {
        fixture("decks")
        show(edit(rows().first().noteId))
        rule.onNodeWithTag("field-Front").assertExists()
        rule.onNodeWithTag("pane-cards").assertExists()
        showCards()
        rule.onNodeWithTag("field-Front").assertDoesNotExist()
        waitForTag("card")
        rule.onNodeWithTag("pane-write").performClick()
        rule.onNodeWithTag("field-Front").assertExists()
    }

    @Test
    fun tapping_the_toggle_settles_all_so_the_pane_shows_what_was_typed() {
        fixture("empty")
        show(draft())
        rule.onNodeWithTag("field-Front").performTextInput("chien")
        rule.onNodeWithTag("field-Back").performTextInput("dog")
        showCards()
        waitForTag("card")
        rule.onNodeWithTag("prompt", useUnmergedTree = true).assertTextEquals("chien")
        assertEquals("chien", storedFront(rows().single().noteId))
    }

    @Test
    fun the_cards_pane_draws_the_card_review_draws_with_a_persian_prompts_badge_on_the_left() {
        fixture("persian")
        val persian = rows().first { isRightToLeft(it.preview) }
        show(edit(persian.noteId))
        showCards()
        waitForTag("card")
        rule.onNodeWithTag("prompt", useUnmergedTree = true).assertTextContains(persian.preview, substring = true)
        val card = rule.onNodeWithTag("card").getUnclippedBoundsInRoot()
        val badge = rule.onNodeWithTag("badge", useUnmergedTree = true).getUnclippedBoundsInRoot()
        assertTrue("the badge sits left of the card's middle", badge.left < card.left + (card.right - card.left) / 2)
        rule.onNodeWithTag("badge", useUnmergedTree = true).assertTextEquals("new")
    }

    @Test
    fun a_latin_prompts_badge_is_on_the_right() {
        fixture("decks")
        show(edit(rows().first { !isRightToLeft(it.preview) }.noteId))
        showCards()
        waitForTag("badge")
        val card = rule.onNodeWithTag("card").getUnclippedBoundsInRoot()
        val badge = rule.onNodeWithTag("badge", useUnmergedTree = true).getUnclippedBoundsInRoot()
        assertTrue(badge.left > card.left + (card.right - card.left) / 2)
    }

    @Test
    fun on_the_dormant_fixture_the_warning_names_the_dormant_cards_above_the_fields() {
        fixture("dormant")
        show(edit(noteWith(PaneState.CARDS, dormant = true)))
        waitForTag("dormancy-warning")
        rule.onNodeWithTag("dormancy-warning").assertTextContains("This edit made cards dormant:", substring = true)
        rule.onNodeWithTag("dormancy-warning").assertTextContains("blank 2 · dormant · 2 reviews kept", substring = true)
        rule.onNodeWithTag("dormancy-warning").assertTextContains(
            "Nothing is deleted — the reviews stay in the log and reattach if the content returns.",
            substring = true,
        )
        val warning = rule.onNodeWithTag("dormancy-warning").getUnclippedBoundsInRoot()
        val firstField = rule.onNodeWithTag("label-Text").getUnclippedBoundsInRoot()
        assertTrue("the warning is above the fields", warning.bottom <= firstField.top)
    }

    @Test
    fun a_note_with_nothing_dormant_shows_no_warning() {
        fixture("decks")
        show(edit(rows().first().noteId))
        rule.onNodeWithTag("dormancy-warning").assertDoesNotExist()
    }

    @Test
    fun the_dormant_fixture_draws_outlined_entries_in_raw_slot_order() {
        fixture("dormant")
        show(edit(noteWith(PaneState.CARDS, dormant = true)))
        showCards()
        waitForTag("dormant-entry")
        rule.onAllNodesWithTag("card").assertCountEquals(1)
        rule.onAllNodesWithTag("dormant-entry").assertCountEquals(1)
        rule.onNodeWithTag("dormant-entry").assertTextEquals("blank 2 · dormant · 2 reviews kept")
        val live = rule.onNodeWithTag("card").getUnclippedBoundsInRoot()
        val dormant = rule.onNodeWithTag("dormant-entry").getUnclippedBoundsInRoot()
        assertTrue("blank 1 sorts above blank 2", live.top < dormant.top)
    }

    @Test
    fun an_entry_no_kind_declares_is_named_by_its_bare_slot() {
        fixture("dormant")
        val stranger = runBlocking {
            owner.use { h ->
                rows().first { r -> h.editNote(r.noteId).cardPane().warning.any { it.slot == 7u } }.noteId
            }
        }
        show(edit(stranger))
        showCards()
        waitForTag("dormant-entry")
        rule.onNodeWithTag("dormant-entry").assertTextEquals("card 7 · dormant · 2 reviews kept")
    }

    @Test
    fun a_pane_with_nothing_live_says_so_and_names_each_dormant_entry_by_its_roles() {
        fixture("dormant")
        show(edit(noteWith(PaneState.NO_LIVE_CARDS)))
        showCards()
        waitForTag("cards-no-live")
        rule.onNodeWithTag("cards-no-live").assertTextEquals("This note currently generates no cards.")
        rule.onAllNodesWithTag("card").assertCountEquals(0)
        rule.onAllNodesWithTag("dormant-entry").assertCountEquals(2)
        rule.onAllNodesWithTag("dormant-entry")[0].assertTextEquals("Term → Meaning · dormant · 2 reviews kept")
        rule.onAllNodesWithTag("dormant-entry")[1].assertTextEquals("Meaning → Term · dormant · 2 reviews kept")
    }

    @Test
    fun a_new_note_has_no_cards_yet() {
        fixture("empty")
        show(draft())
        showCards()
        waitForTag("cards-empty")
        rule.onNodeWithTag("cards-empty").assertTextEquals("No cards yet.")
    }

    @Test
    fun blank_it_exists_only_under_the_cloze_text_field() {
        fixture("empty")
        show(draft("basic"))
        rule.onNodeWithTag("blank-it").assertDoesNotExist()
        rule.onNodeWithTag("kind").performClick()
        val cloze = runBlocking { owner.use { it.newDraft("basic", null).kindOptions().indexOf("cloze") } }
        rule.onNodeWithTag("kind-option-$cloze").performClick()
        waitForTag("blank-it")
    }

    @Test
    fun blank_it_is_disabled_with_no_selection_and_wraps_a_touch_selection_as_the_first_blank() {
        fixture("empty")
        show(draft("cloze"))
        rule.onNodeWithTag("field-Text").performTextInput("Le chat mange")
        rule.onNodeWithTag("blank-it").assertIsNotEnabled()
        rule.onNodeWithTag("field-Text").performTextInputSelection(TextRange(3, 7))
        rule.onNodeWithTag("blank-it").assertIsEnabled()
        rule.onNodeWithTag("blank-it").performClick()
        rule.waitForIdle()
        awaitText("field-Text", "Le {{1::chat}} mange")
        rule.onNodeWithTag("blank-it").assertIsNotEnabled()
    }

    @Test
    fun blank_it_numbers_one_above_the_highest_ever_used_a_dormant_blank_included() {
        fixture("dormant")
        val pruned = noteWith(PaneState.CARDS, dormant = true)
        show(edit(pruned))
        val text = runBlocking { owner.use { h -> h.editNote(pruned).fields().first { it.name == "Text" }.value } }
        val at = text.indexOf("pré")
        rule.onNodeWithTag("field-Text").performTextInputSelection(TextRange(at, at + 3))
        // Under the warning the button sits lower, behind the keyboard the selection raised.
        rule.onNodeWithTag("blank-it").performScrollTo().performClick()
        rule.waitForIdle()
        awaitText("field-Text", text.replace("pré", "{{3::pré}}"))
    }

    @Test
    fun a_wrapped_blank_is_stored_and_makes_a_live_card() {
        fixture("empty")
        show(draft("cloze"))
        rule.onNodeWithTag("field-Text").performTextInput("Le chat mange")
        rule.onNodeWithTag("field-Text").performTextInputSelection(TextRange(3, 7))
        rule.onNodeWithTag("blank-it").performClick()
        showCards()
        waitForTag("card")
        rule.onNodeWithTag("prompt", useUnmergedTree = true).assertTextContains("Le [", substring = true)
    }

    private fun captured(name: String, fixture: String, args: () -> Map<String, Any?>, drive: () -> Unit = {}) {
        val dir = context.getExternalFilesDir("cards-pane-shots")!!
        fixture(fixture)
        val dark = mutableStateOf(false)
        show(args(), dark)
        drive()
        val pages = mutableListOf<Int>()
        for (isDark in listOf(false, true)) {
            dark.value = isDark
            rule.waitForIdle()
            val bitmap = rule.onRoot().captureToImage().asAndroidBitmap()
            FileOutputStream(File(dir, "$name-${if (isDark) "dark" else "light"}.png")).use {
                bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it)
            }
            assertEquals("$name page colour, dark=$isDark", pageArgb(isDark), bitmap.getPixel(2, bitmap.height / 2))
            pages += bitmap.getPixel(2, 2)
        }
        assertNotEquals("$name looks the same in both themes", pages[0], pages[1])
    }

    @Test
    fun the_write_pane_with_the_warning_is_captured_in_both_themes() =
        captured("write-warning", "dormant", { edit(noteWith(PaneState.CARDS, dormant = true)) }) {
            waitForTag("dormancy-warning")
        }

    @Test
    fun the_cards_pane_is_captured_in_both_themes() =
        captured("cards", "decks", { edit(rows().first { !isRightToLeft(it.preview) }.noteId) }) {
            showCards()
            waitForTag("card")
        }

    @Test
    fun a_persian_cards_pane_is_captured_in_both_themes() =
        captured("cards-persian", "persian", { edit(rows().first { isRightToLeft(it.preview) }.noteId) }) {
            showCards()
            waitForTag("card")
        }

    @Test
    fun the_outlined_dormant_entries_are_captured_in_both_themes() =
        captured("cards-dormant", "dormant", { edit(noteWith(PaneState.CARDS, dormant = true)) }) {
            showCards()
            waitForTag("dormant-entry")
        }

    @Test
    fun a_pane_with_nothing_live_is_captured_in_both_themes() =
        captured("cards-no-live", "dormant", { edit(noteWith(PaneState.NO_LIVE_CARDS)) }) {
            showCards()
            waitForTag("cards-no-live")
        }

    @Test
    fun the_empty_cards_pane_is_captured_in_both_themes() =
        captured("cards-empty", "empty", { draft() }) {
            showCards()
            waitForTag("cards-empty")
        }

    @Test
    fun the_cloze_field_with_blank_it_is_captured_in_both_themes() =
        captured("cloze", "empty", { draft("cloze") }) {
            rule.onNodeWithTag("field-Text").performTextInput("Le chat mange")
            rule.onNodeWithTag("field-Text").performTextInputSelection(TextRange(3, 7))
            rule.waitForIdle()
        }
}
