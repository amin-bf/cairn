package dev.cairn.app

import android.content.Context
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsNode
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.hasScrollAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollToIndex
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.unit.dp
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
 * The note list's two row actions on the emulator (ADR-0048 §4, ADR-0039 §7): *Delete* asking in
 * the row, and *Move* with its two-tap placement. The store is read back through the binding, so
 * what is asserted is what reached it.
 */
@RunWith(AndroidJUnit4::class)
class NoteRowActionsTest {
    @get:Rule
    val rule = createComposeRule()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "row-actions-${System.nanoTime()}")
    private val owner = CollectionOwner(context, File(root, "files"), File(root, "no_backup"))

    @After
    fun tearDown() {
        root.deleteRecursively()
    }

    private fun fixture(name: String) = runBlocking { owner.use { it.installFixture(name, System.currentTimeMillis()) } }

    private fun facts(selector: DeckSelector = DeckSelector.All, search: String = "") =
        runBlocking { owner.use { it.noteList(selector, search) } }

    private fun order(selector: DeckSelector = DeckSelector.All) = facts(selector).rows.map { it.noteId }

    private fun showNotes(dark: MutableState<Boolean> = mutableStateOf(false)) {
        val model = NotesViewModel(owner)
        rule.setContent { CairnTheme(dark = dark.value) { NotesScreen(model) } }
        waitFor("note-row")
    }

    private fun waitFor(tag: String) =
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag(tag)).fetchSemanticsNodes().isNotEmpty() }

    private fun waitGone(tag: String) =
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag(tag)).fetchSemanticsNodes().isEmpty() }

    private fun waitForText(text: String) =
        rule.waitUntil(10_000) { rule.onAllNodes(hasText(text)).fetchSemanticsNodes().isNotEmpty() }

    private fun count(tag: String) = rule.onAllNodes(hasTestTag(tag)).fetchSemanticsNodes().size

    private fun chooseDeck(place: Int) {
        rule.onNodeWithTag("deck-filter").performClick()
        rule.onNodeWithTag("deck-filter-option-$place").performClick()
    }

    private val unfiled = 1
    private val francais = 2

    /** Tap the last *Place here*, scrolling to it: the list is lazy, and the end gap is the last item. */
    private fun placeAtTheEnd(others: Int) {
        // The heading, the deck and search controls and the held note come first; then a target and a row per other note.
        val lastGap = 3 + 2 * others
        rule.onNode(hasScrollAction()).performScrollToIndex(lastGap)
        rule.waitForIdle()
        rule.onAllNodesWithTag("place-here").let { it[it.fetchSemanticsNodes().size - 1] }.performClick()
        waitGone("cancel-move")
    }

    private fun move(row: Int) {
        rule.onAllNodesWithTag("row-move")[row].performClick()
        waitFor("cancel-move")
    }

    @Test
    fun every_row_carries_a_move_and_a_delete_and_nothing_else_is_asked() {
        fixture("decks")
        showNotes()
        // The list is lazy, so only the rows on screen are composed; each of them carries both.
        assertTrue(count("row-move") > 0)
        assertEquals(count("note-row"), count("row-move"))
        assertEquals(count("note-row"), count("row-delete"))
        rule.onNodeWithTag("row-delete-confirm").assertDoesNotExist()
    }

    @Test
    fun the_actions_are_36_dp_squares_against_the_rows_right_edge() {
        fixture("decks")
        showNotes()
        val row = rule.onAllNodesWithTag("note-row")[0].fetchSemanticsNode().boundsInRoot
        val delete = rule.onAllNodesWithTag("row-delete")[0].fetchSemanticsNode().boundsInRoot
        val move = rule.onAllNodesWithTag("row-move")[0].fetchSemanticsNode().boundsInRoot
        val side = rule.density.run { 36.dp.toPx() }
        assertEquals(side, delete.width, 1f)
        assertEquals(side, delete.height, 1f)
        assertEquals(side, move.width, 1f)
        assertEquals(row.right, delete.right, 2f)
        assertEquals(move.right, delete.left, 1f)
    }

    @Test
    fun deleting_asks_in_the_row_naming_the_note() {
        fixture("decks")
        showNotes()
        val preview = facts().rows[0].preview
        rule.onAllNodesWithTag("row-delete")[0].performClick()
        waitFor("row-delete-confirm")
        rule.onNodeWithTag("row-delete-warning").assertIsDisplayed()
        val said = rule.onNodeWithTag("row-delete-warning").fetchSemanticsNode().config[SemanticsProperties.Text].joinToString("")
        assertTrue(said, said.endsWith("? It cannot be undeleted."))
        assertTrue(said, said.contains(preview))
        // Asking leaves the note where it is.
        assertEquals(25, facts().rows.size)
    }

    /** Scroll the list by [dy] pixels (positive moves the content up), as a drag would. */
    private fun scrollBy(dy: Float) {
        rule.onNode(hasScrollAction()).performSemanticsAction(SemanticsActions.ScrollBy) { it(0f, dy) }
        rule.waitForIdle()
    }

    private fun top(tag: String) = rule.onNodeWithTag(tag).fetchSemanticsNode().boundsInRoot.top
    private fun bottom(tag: String) = rule.onNodeWithTag(tag).fetchSemanticsNode().boundsInRoot.bottom

    /** The whole question — the sentence, *Delete* and *Keep it* — shows above the pinned *Create note*. */
    private fun assertQuestionAboveCreateNote() {
        waitFor("row-keep")
        rule.waitForIdle()
        for (tag in listOf("row-delete-warning", "row-delete-confirm", "row-keep")) {
            rule.onNodeWithTag(tag).assertIsDisplayed()
            assertTrue("$tag clears Create note", bottom(tag) <= top("create-note"))
        }
    }

    /** A row's laid-out extent: `boundsInRoot` is cut to what is visible, which is what these tests ask about. */
    private fun SemanticsNode.laidOutTop() = positionInRoot.y
    private fun SemanticsNode.laidOutBottom() = positionInRoot.y + size.height

    @Test
    fun asking_on_the_last_whole_row_brings_keep_it_in_above_create_note() {
        fixture("decks")
        showNotes()
        rule.onNode(hasScrollAction()).performScrollToIndex(8)
        rule.waitForIdle()
        val band = top("create-note")
        val last = rule.onAllNodesWithTag("note-row").fetchSemanticsNodes().indexOfLast { it.laidOutBottom() <= band }
        rule.onAllNodesWithTag("row-delete")[last].performClick()
        assertQuestionAboveCreateNote()
    }

    @Test
    fun asking_on_a_row_half_hidden_under_the_band_brings_keep_it_in_above_create_note() {
        fixture("decks")
        showNotes()
        rule.onNode(hasScrollAction()).performScrollToIndex(8)
        rule.waitForIdle()
        // Put a row's centre two dp above the band, so its actions can be pressed and its lower half is cut.
        val band = top("create-note")
        val row = rule.onAllNodesWithTag("note-row").fetchSemanticsNodes().last { it.laidOutTop() < band }
        scrollBy((row.laidOutTop() + row.size.height / 2f) - (band - rule.density.run { 2.dp.toPx() }))
        val half = rule.onAllNodesWithTag("note-row").fetchSemanticsNodes().indexOfFirst { it.laidOutBottom() > band }
        assertTrue("a row is cut by the band", half >= 0)
        rule.onAllNodesWithTag("row-delete")[half].performClick()
        assertQuestionAboveCreateNote()
    }

    @Test
    fun keeping_it_leaves_the_note_in_the_list_and_the_store() {
        fixture("decks")
        showNotes()
        rule.onAllNodesWithTag("row-delete")[0].performClick()
        waitFor("row-keep")
        rule.onNodeWithTag("row-keep").performClick()
        waitGone("row-delete-confirm")
        rule.onNodeWithText(facts().rows[0].preview).assertIsDisplayed()
        assertEquals(25, facts().rows.size)
    }

    @Test
    fun confirming_removes_the_note_from_the_list_and_the_store() {
        fixture("decks")
        showNotes()
        val before = order()
        val preview = facts().rows[0].preview
        rule.onAllNodesWithTag("row-delete")[0].performClick()
        waitFor("row-delete-confirm")
        rule.onNodeWithTag("row-delete-confirm").performClick()
        waitGone("row-delete-confirm")
        rule.waitUntil(10_000) { rule.onAllNodes(hasText(preview)).fetchSemanticsNodes().isEmpty() }
        assertEquals(before.drop(1), order())
    }

    @Test
    fun a_right_to_left_preview_in_the_english_question_is_isolated() {
        fixture("decks")
        showNotes()
        rule.onNodeWithTag("notes-search").performTextInput("پنجره")
        rule.waitUntil(10_000) { count("note-row") == 1 }
        rule.onAllNodesWithTag("row-delete")[0].performClick()
        waitFor("row-delete-warning")
        val said = rule.onNodeWithTag("row-delete-warning").fetchSemanticsNode().config[SemanticsProperties.Text].joinToString("")
        assertTrue(said, said.contains("‫پنجره‬"))
    }

    @Test
    fun moving_shows_cancel_the_held_note_and_a_target_between_the_other_rows() {
        fixture("decks")
        showNotes()
        val moving = facts().rows[2]
        move(2)
        rule.onNodeWithText("Cancel move").assertIsDisplayed()
        rule.onNodeWithText("Placing").assertIsDisplayed()
        rule.onNodeWithTag("held-row").assertIsDisplayed()
        // Every other note is a row that offers nothing, with a target before it.
        assertTrue(count("place-here") > 1)
        assertTrue(count("inert-row") > 0)
        assertEquals(0, count("row-move"))
        assertEquals(0, count("row-delete"))
        assertEquals(moving.preview, rule.onNodeWithTag("held-row").fetchSemanticsNode().config[SemanticsProperties.Text].joinToString(""))
    }

    @Test
    fun cancel_move_returns_the_list_with_the_order_untouched() {
        fixture("decks")
        showNotes()
        val before = order()
        move(2)
        rule.onNodeWithTag("cancel-move").performClick()
        waitFor("row-move")
        assertEquals(0, count("cancel-move"))
        assertEquals(before, order())
    }

    @Test
    fun placing_between_two_rows_moves_the_note_there_and_leaves_the_others_in_order() {
        fixture("decks")
        showNotes()
        val before = order()
        val moving = before[0]
        move(0)
        // The gap after the second of the *other* notes, i.e. between them and the third.
        rule.onAllNodesWithTag("place-here")[2].performClick()
        waitGone("cancel-move")
        val others = before.drop(1)
        assertEquals(others.take(2) + moving + others.drop(2), order())
        waitFor("row-move")
    }

    @Test
    fun placing_at_the_first_and_the_last_gap_reaches_either_end() {
        fixture("decks")
        showNotes()
        val before = order()
        move(5)
        rule.onAllNodesWithTag("place-here")[0].performClick()
        waitGone("cancel-move")
        assertEquals(listOf(before[5]) + before.filterIndexed { i, _ -> i != 5 }, order())
        val now = order()
        move(0)
        placeAtTheEnd(now.size - 1)
        assertEquals(now.drop(1) + now[0], order())
    }

    @Test
    fun placing_under_a_filter_keeps_every_hidden_note_where_it_was() {
        fixture("decks")
        showNotes()
        chooseDeck(francais)
        waitForText("Delete deck")
        val whole = order()
        val french = order(facts().decks.first { it.name == "Français" }.let { DeckSelector.Deck(it.id) })
        val hiddenBefore = whole.filter { it !in french }
        move(0)
        // The last gap: the first French note goes to the end of the filtered list.
        placeAtTheEnd(french.size - 1)
        val after = order()
        assertEquals(french.drop(1) + french[0], after.filter { it in french })
        assertEquals(hiddenBefore, after.filter { it !in french })
    }

    @Test
    fun a_filter_that_hides_the_moving_note_ends_the_move() {
        fixture("decks")
        showNotes()
        val before = order()
        // The first row is a French note; narrowing to *Unfiled* hides it.
        move(0)
        chooseDeck(unfiled)
        waitGone("cancel-move")
        waitFor("row-move")
        assertEquals(0, count("place-here"))
        assertEquals(before, order())
    }

    @Test
    fun a_search_that_hides_the_moving_note_ends_the_move() {
        fixture("decks")
        showNotes()
        move(0)
        rule.onNodeWithTag("notes-search").performTextInput("zzzz")
        waitGone("cancel-move")
        waitFor("notes-no-match")
    }

    @Test
    fun starting_a_move_drops_an_open_delete_question() {
        fixture("decks")
        showNotes()
        rule.onAllNodesWithTag("row-delete")[0].performClick()
        waitFor("row-delete-confirm")
        rule.onAllNodesWithTag("row-move")[1].performClick()
        waitFor("cancel-move")
        assertEquals(0, count("row-delete-confirm"))
    }

    /** Kept in the app's external files directory for looking at (never committed), and checked for what the theme decides. */
    private fun capturedInBothThemes(name: String, drive: () -> Unit) {
        val dir = context.getExternalFilesDir("notes-shots")!!
        val dark = mutableStateOf(false)
        fixture("decks")
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
    fun the_row_actions_are_captured_in_both_themes() = capturedInBothThemes("row-actions") {
        waitFor("row-move")
    }

    @Test
    fun the_delete_question_is_captured_in_both_themes() = capturedInBothThemes("row-delete-question") {
        rule.onAllNodesWithTag("row-delete")[0].performClick()
        waitFor("row-delete-confirm")
    }

    @Test
    fun a_persian_delete_question_is_captured_in_both_themes() = capturedInBothThemes("row-delete-persian") {
        rule.onNodeWithTag("notes-search").performTextInput("پنجره")
        rule.waitUntil(10_000) { count("note-row") == 1 }
        rule.onAllNodesWithTag("row-delete")[0].performClick()
        waitFor("row-delete-confirm")
    }

    @Test
    fun the_placement_state_is_captured_in_both_themes() = capturedInBothThemes("placement") {
        move(2)
        waitFor("place-here")
    }
}
