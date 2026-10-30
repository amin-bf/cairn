package dev.cairn.app

import android.content.Context
import androidx.activity.OnBackPressedDispatcher
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.cairn.design.CairnTheme
import dev.cairn.design.pageArgb
import dev.cairn.ffi.DeckSelector
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
 * The note list on the emulator: each state reached through a fixture installed by name into a
 * collection of its own (never the app's), drawn the way the activity draws it.
 */
@RunWith(AndroidJUnit4::class)
class NoteListTest {
    @get:Rule
    val rule = createComposeRule()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "notes-${System.nanoTime()}")
    private val owner = CollectionOwner(context, File(root, "files"), File(root, "no_backup"))

    @After
    fun tearDown() {
        root.deleteRecursively()
    }

    private fun fixture(name: String) = runBlocking { owner.use { it.installFixture(name, System.currentTimeMillis()) } }

    private fun facts(selector: DeckSelector = DeckSelector.All, search: String = "") =
        runBlocking { owner.use { it.noteList(selector, search) } }

    private fun showNotes(
        dark: MutableState<Boolean> = mutableStateOf(false),
        onOpen: (String) -> Unit = {},
        onCreate: (String?) -> Unit = {},
    ) {
        val model = NotesViewModel(owner)
        rule.setContent { CairnTheme(dark = dark.value) { NotesScreen(model, onOpen, onCreate) } }
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("note-row") or hasTestTag("notes-empty")).fetchSemanticsNodes().isNotEmpty() }
    }

    private fun waitForText(text: String) =
        rule.waitUntil(10_000) { rule.onAllNodes(androidx.compose.ui.test.hasText(text)).fetchSemanticsNodes().isNotEmpty() }

    /** Open the filter and take its option at [place]: 0 is *All decks*, 1 *Unfiled*, then the decks in id order. */
    private fun chooseDeck(place: Int) {
        rule.onNodeWithTag("deck-filter").performClick()
        rule.onNodeWithTag("deck-filter-option-$place").performClick()
    }

    private val unfiled = 1
    private val francais = 2
    private val persian = 3

    @Test
    fun rows_come_in_list_order_and_carry_their_deck_under_all_decks() {
        fixture("decks")
        showNotes()
        rule.onAllNodesWithTag("note-row")[0].assertIsDisplayed()
        rule.onNodeWithText("le carrefour").assertIsDisplayed()
        rule.onAllNodesWithText("Français")[0].assertIsDisplayed()
        // The list never shows a position, a box or a due count.
        rule.onNodeWithText("box", substring = true).assertDoesNotExist()
    }

    @Test
    fun a_collection_without_decks_captions_no_row() {
        fixture("caught-up")
        showNotes()
        rule.onNodeWithText("Unfiled").assertDoesNotExist()
    }

    @Test
    fun narrowing_to_a_deck_drops_the_caption_and_unfiled_shows_the_notes_with_none() {
        fixture("decks")
        showNotes()
        chooseDeck(unfiled)
        waitForText("le colporteur")
        rule.onNodeWithText("la girouette").assertIsDisplayed()
        rule.onNodeWithText("le carrefour").assertDoesNotExist()
        // Under a filter the rows no longer repeat what the filter says.
        assertEquals(3, rule.onAllNodesWithTag("note-row").fetchSemanticsNodes().size)
        rule.onAllNodesWithText("Français").assertCountEquals(0)
    }

    @Test
    fun a_persian_search_finds_notes_through_the_arabic_letters_a_keyboard_may_type() {
        fixture("decks")
        showNotes()
        // Arabic kaf (U+0643) for the Persian kaf the notes hold (ADR-0049).
        rule.onNodeWithTag("notes-search").performTextInput("كتاب")
        waitForText("کتابخانه")
        rule.onNodeWithText("کتاب").assertIsDisplayed()
        assertEquals(2, rule.onAllNodesWithTag("note-row").fetchSemanticsNodes().size)
    }

    @Test
    fun tapping_a_row_opens_that_note_and_create_note_carries_the_filtered_deck() {
        fixture("decks")
        var opened: String? = null
        val created = mutableListOf<String?>()
        showNotes(onOpen = { opened = it }, onCreate = { created += it })
        rule.onAllNodesWithTag("note-row")[0].performClick()
        assertEquals(facts().rows[0].noteId, opened)
        rule.onNodeWithTag("create-note").performClick()
        chooseDeck(francais)
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("note-row")).fetchSemanticsNodes().size == 3 }
        rule.onNodeWithTag("create-note").performClick()
        val deck = facts().decks.first { it.name == "Français" }.id
        assertEquals(listOf(null, deck), created)
    }

    @Test
    fun a_search_that_matches_nothing_says_so_and_an_empty_collection_says_that() {
        fixture("decks")
        showNotes()
        rule.onNodeWithTag("notes-search").performTextInput("zzzz")
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("notes-no-match")).fetchSemanticsNodes().isNotEmpty() }
        rule.onNodeWithTag("notes-no-match").assertTextEquals("No notes match.")
        rule.onNodeWithTag("notes-search").assertIsDisplayed()
    }

    @Test
    fun an_empty_collection_says_what_to_do_about_it() {
        fixture("empty")
        showNotes()
        rule.onNodeWithTag("notes-empty").assertTextEquals("Nothing here yet — create a deck, import one, or set up sync")
        rule.onNodeWithTag("notes-search").assertDoesNotExist()
    }

    @Test
    fun a_new_deck_is_created_and_filtered_to() {
        fixture("decks")
        showNotes()
        rule.onNodeWithTag("new-deck-name").performTextInput("Géographie")
        rule.onNodeWithTag("new-deck").performClick()
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("notes-no-match")).fetchSemanticsNodes().isNotEmpty() }
        rule.onNodeWithTag("deck-filter").assertTextEquals("Géographie")
        assertTrue(facts().decks.any { it.name == "Géographie" })
    }

    @Test
    fun a_blank_deck_name_makes_no_deck() {
        fixture("decks")
        showNotes()
        rule.onNodeWithTag("new-deck-name").performTextInput("   ")
        rule.onNodeWithTag("new-deck").performClick()
        rule.waitForIdle()
        assertEquals(4, facts().decks.size)
    }

    @Test
    fun deleting_a_deck_asks_with_the_count_and_keeping_it_leaves_everything() {
        fixture("decks")
        showNotes()
        chooseDeck(francais)
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("delete-deck")).fetchSemanticsNodes().isNotEmpty() }
        rule.onNodeWithTag("delete-deck").performClick()
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("delete-deck-warning")).fetchSemanticsNodes().isNotEmpty() }
        rule.onNodeWithTag("delete-deck-warning")
            .assertTextEquals("Delete Français? Its 10 notes are deleted with it, and cannot be undeleted.")
        rule.onNodeWithTag("keep-deck").performClick()
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("delete-deck")).fetchSemanticsNodes().isNotEmpty() }
        assertEquals(25, facts().rows.size)
    }

    @Test
    fun confirming_deletes_the_deck_and_its_notes_and_returns_to_all_decks() {
        fixture("decks")
        showNotes()
        chooseDeck(francais)
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("delete-deck")).fetchSemanticsNodes().isNotEmpty() }
        rule.onNodeWithTag("delete-deck").performClick()
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("delete-deck-confirm")).fetchSemanticsNodes().isNotEmpty() }
        rule.onNodeWithTag("delete-deck-confirm").performClick()
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("delete-deck")).fetchSemanticsNodes().isEmpty() }
        rule.onNodeWithTag("deck-filter").assertTextEquals("All decks")
        assertEquals(15, facts().rows.size)
        assertEquals(3, facts().decks.size)
    }

    @Test
    fun a_deck_with_no_notes_says_so_when_asked_to_delete() {
        fixture("decks")
        showNotes()
        rule.onNodeWithTag("new-deck-name").performTextInput("Vide")
        rule.onNodeWithTag("new-deck").performClick()
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("delete-deck")).fetchSemanticsNodes().isNotEmpty() }
        rule.onNodeWithTag("delete-deck").performClick()
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("delete-deck-warning")).fetchSemanticsNodes().isNotEmpty() }
        rule.onNodeWithTag("delete-deck-warning").assertTextEquals("Delete Vide? It has no notes in it.")
    }

    @Test
    fun a_right_to_left_deck_name_in_the_english_warning_is_isolated() {
        fixture("decks")
        showNotes()
        chooseDeck(persian)
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("delete-deck")).fetchSemanticsNodes().isNotEmpty() }
        rule.onNodeWithTag("delete-deck").performClick()
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("delete-deck-warning")).fetchSemanticsNodes().isNotEmpty() }
        val said = rule.onNodeWithTag("delete-deck-warning").fetchSemanticsNode().config[androidx.compose.ui.semantics.SemanticsProperties.Text].joinToString("")
        assertTrue(said, said.contains("\u202Bفارسی\u202C"))
    }

    @Test
    fun the_destinations_row_moves_between_review_and_notes_and_back_returns_to_review() {
        fixture("decks")
        var dispatcher: OnBackPressedDispatcher? = null
        rule.setContent {
            dispatcher = androidx.activity.compose.LocalOnBackPressedDispatcherOwner.current!!.onBackPressedDispatcher
            CairnTheme(dark = false) { CairnNavHost(owner, keyboardUp = false) }
        }
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("start-all") or hasTestTag("state-caught-up")).fetchSemanticsNodes().isNotEmpty() }
        rule.onNodeWithTag("destination-review").assertIsSelected()
        rule.onNodeWithTag("destination-notes").performClick()
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("note-row")).fetchSemanticsNodes().isNotEmpty() }
        rule.onNodeWithTag("destination-notes").assertIsSelected()
        rule.runOnUiThread { dispatcher!!.onBackPressed() }
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("start-all") or hasTestTag("state-caught-up")).fetchSemanticsNodes().isNotEmpty() }
        rule.onNodeWithTag("destination-review").assertIsSelected()
    }

    @Test
    fun the_destinations_row_is_absent_while_the_keyboard_is_up() {
        fixture("decks")
        val up = mutableStateOf(false)
        rule.setContent {
            CairnTheme(dark = false) { dev.cairn.design.CairnShell(listOf(dev.cairn.design.Destination("Review", true, "d") {}), keyboardUp = up.value) {} }
        }
        rule.onNodeWithTag("destinations").assertIsDisplayed()
        up.value = true
        rule.waitForIdle()
        rule.onNodeWithTag("destinations").assertDoesNotExist()
    }

    @Test
    fun no_call_into_the_binding_ran_on_the_main_thread() {
        fixture("decks")
        showNotes()
        rule.onNodeWithTag("notes-search").performTextInput("a")
        rule.waitForIdle()
        val app = ApplicationProvider.getApplicationContext<CairnApplication>()
        assertEquals(
            "StrictMode saw work of ours on the main thread:\n${app.strictModeViolations.joinToString("\n---\n")}",
            emptyList<String>(),
            app.strictModeViolations.toList(),
        )
    }

    /** Kept in the app's external files directory for looking at (never committed), and checked for what the theme decides. */
    private fun capturedInBothThemes(name: String, drive: () -> Unit = {}) {
        val dir = context.getExternalFilesDir("notes-shots")!!
        val dark = mutableStateOf(false)
        val wanted = if (name == "empty") "empty" else "decks"
        fixture(wanted)
        showNotes(dark)
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
    fun the_list_is_captured_in_both_themes() = capturedInBothThemes("list")

    @Test
    fun a_filtered_list_is_captured_in_both_themes() = capturedInBothThemes("filtered") {
        chooseDeck(francais)
        waitForText("Delete deck")
    }

    @Test
    fun a_persian_row_is_captured_in_both_themes() = capturedInBothThemes("persian") {
        rule.onNodeWithTag("notes-search").performTextInput("پنجره")
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("note-row")).fetchSemanticsNodes().size == 1 }
    }

    @Test
    fun no_match_is_captured_in_both_themes() = capturedInBothThemes("no-match") {
        rule.onNodeWithTag("notes-search").performTextInput("zzzz")
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("notes-no-match")).fetchSemanticsNodes().isNotEmpty() }
    }

    @Test
    fun the_empty_state_is_captured_in_both_themes() = capturedInBothThemes("empty")

    @Test
    fun the_delete_warning_is_captured_in_both_themes() = capturedInBothThemes("delete-warning") {
        chooseDeck(francais)
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("delete-deck")).fetchSemanticsNodes().isNotEmpty() }
        rule.onNodeWithTag("delete-deck").performClick()
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("delete-deck-warning")).fetchSemanticsNodes().isNotEmpty() }
    }
}
