package dev.cairn.app

import android.content.Context
import android.view.inputmethod.EditorInfo
import androidx.activity.ComponentActivity
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.getUnclippedBoundsInRoot
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.AndroidComposeTestRule
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performImeAction
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.test.performTextInput
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.SavedStateHandle
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.cairn.design.CairnTheme
import dev.cairn.design.pageArgb
import dev.cairn.ffi.DeckSelector
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.io.FileOutputStream

/**
 * The editor on the emulator (ADR-0040, ADR-0048): each state reached through a fixture installed by
 * name into a collection of its own, the text typed the way a keyboard types it, and the store read
 * back to see what reached it.
 */
@RunWith(AndroidJUnit4::class)
class EditorTest {
    @get:Rule
    val rule: AndroidComposeTestRule<*, ComponentActivity> = createAndroidComposeRule<ComponentActivity>()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "editor-${System.nanoTime()}")
    private val owner = CollectionOwner(context, File(root, "files"), File(root, "no_backup"))

    @Volatile private var left = false
    @Volatile private var opened: Pair<String, String?>? = null

    @After
    fun tearDown() {
        root.deleteRecursively()
    }

    private fun fixture(name: String) = runBlocking { owner.use { it.installFixture(name, System.currentTimeMillis()) } }

    private fun rows(selector: DeckSelector = DeckSelector.All) =
        runBlocking { owner.use { it.noteList(selector, "").rows } }

    private fun previews() = rows().map { it.preview }

    private fun decks() = runBlocking { owner.use { it.noteList(DeckSelector.All, "").decks } }

    private fun storedFront(id: String): String = runBlocking {
        owner.use { h -> h.editNote(id).fields().first { it.name == "Front" }.value }
    }

    private fun storedKind(id: String): String = runBlocking { owner.use { it.editNote(id).kind() } }

    private fun show(args: Map<String, Any?>, dark: MutableState<Boolean> = mutableStateOf(false)): EditorViewModel {
        val model = EditorViewModel(owner, SavedStateHandle(args))
        rule.setContent {
            CairnTheme(dark = dark.value) {
                EditorScreen(model, onDone = { left = true }, onNewNote = { kind, deck -> opened = kind to deck })
            }
        }
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("editor-heading")).fetchSemanticsNodes().isNotEmpty() }
        return model
    }

    private fun draft(kind: String = "basic", deck: String? = null) =
        mapOf(ARG_KIND to kind, ARG_DECK to deck)

    private fun waitFor(what: String, condition: () -> Boolean) {
        val until = System.currentTimeMillis() + 10_000
        while (!condition()) {
            if (System.currentTimeMillis() > until) throw AssertionError("never happened: $what (notes: ${previews()})")
            Thread.sleep(50)
        }
    }

    private fun type(field: String, text: String) {
        rule.onNodeWithTag("field-$field").performTextInput(text)
        rule.waitForIdle()
    }

    /** Text committed through the focused field's input connection, as a keyboard commits it. */
    private fun commitThroughKeyboard(field: String, text: String) {
        rule.onNodeWithTag("field-$field").performClick()
        rule.waitForIdle()
        rule.runOnUiThread {
            val view = rule.activity.window.decorView.findFocus()
            val connection = view.onCreateInputConnection(EditorInfo())
            assertTrue(connection.commitText(text, 1))
        }
        rule.waitForIdle()
    }

    private val scenario get() = rule.activityRule as androidx.test.ext.junit.rules.ActivityScenarioRule<*>

    private val persian = "سلام دنیا"

    @Test
    fun the_heading_says_new_for_a_draft() {
        fixture("empty")
        show(draft())
        rule.onNodeWithTag("editor-heading").assertTextEquals("New note")
    }

    @Test
    fun the_heading_says_edit_for_a_stored_note() {
        fixture("decks")
        show(mapOf(ARG_NOTE to rows().first().noteId))
        rule.onNodeWithTag("editor-heading").assertTextEquals("Edit note")
    }

    @Test
    fun leaving_a_field_stores_the_text_and_reopening_the_note_shows_it() {
        fixture("empty")
        show(draft())
        type("Front", "le chat")
        // Nothing has been left yet and two seconds have not passed.
        assertEquals(emptyList<String>(), previews())
        rule.onNodeWithTag("field-Back").performClick()
        waitFor("blur commit") { previews() == listOf("le chat") }
        val id = rows().single().noteId
        assertEquals("le chat", storedFront(id))
    }

    @Test
    fun pausing_two_seconds_settles_the_field_while_the_caret_is_still_in_it() {
        fixture("empty")
        show(draft())
        type("Front", "la plume")
        rule.onNodeWithTag("field-Front").assertIsFocused()
        waitFor("idle settle") { previews() == listOf("la plume") }
    }

    @Test
    fun backgrounding_the_activity_settles_all() {
        fixture("empty")
        show(draft())
        type("Front", "la fenêtre")
        scenario.scenario.moveToState(Lifecycle.State.CREATED)
        waitFor("settle on stop") { previews() == listOf("la fenêtre") }
    }

    @Test
    fun an_untouched_new_note_leaves_nothing_behind() {
        fixture("empty")
        show(draft())
        rule.onNodeWithTag("field-Front").performClick()
        rule.onNodeWithTag("done").performClick()
        waitFor("done left") { left }
        assertEquals(0L, runBlocking { owner.use { it.noteCount() } }.toLong())
    }

    @Test
    fun persian_committed_through_the_input_connection_is_stored_and_drawn_right_to_left() {
        fixture("empty")
        show(draft())
        val labelBefore = rule.onNodeWithTag("label-Front").getUnclippedBoundsInRoot()
        commitThroughKeyboard("Front", persian)
        rule.onNodeWithTag("field-Front").assertTextEquals(persian)
        val labelAfter = rule.onNodeWithTag("label-Front").getUnclippedBoundsInRoot()
        val field = rule.onNodeWithTag("field-Front").getUnclippedBoundsInRoot()
        // The label moved to the right edge, where the Persian text starts, and the other field's did not.
        assertTrue("label ${labelAfter.left} should be right of ${labelBefore.left}", labelAfter.left > labelBefore.left + 20.dp)
        assertTrue(field.right - labelAfter.right < 16.dp)
        assertEquals(
            rule.onNodeWithTag("label-Back").getUnclippedBoundsInRoot().left,
            labelBefore.left,
        )
        rule.onNodeWithTag("done").performClick()
        waitFor("persian stored") { previews() == listOf(persian) }
        assertEquals(persian, storedFront(rows().single().noteId))
    }

    @Test
    fun next_moves_focus_and_the_last_fields_key_commits_nothing() {
        fixture("empty")
        show(draft())
        rule.onNodeWithTag("field-Front").performClick()
        rule.onNodeWithTag("field-Front").performImeAction()
        rule.onNodeWithTag("field-Back").assertIsFocused()
        rule.onNodeWithTag("field-Back").performImeAction()
        rule.waitForIdle()
        rule.onNodeWithTag("field-Back").assertIsFocused()
        Thread.sleep(300)
        assertEquals(emptyList<String>(), previews())
    }

    @Test
    fun new_note_settles_and_hands_the_kind_and_deck_to_a_fresh_draft() {
        fixture("decks")
        val deck = decks().first().id
        val before = previews().size
        show(draft(deck = deck))
        type("Front", "un nouveau mot")
        rule.onNodeWithTag("new-note").performClick()
        waitFor("new note opened") { opened != null }
        assertEquals("basic" to deck, opened)
        assertTrue("un nouveau mot" in previews())
        assertEquals(before + 1, previews().size)
        assertEquals(deck, rows().first { it.preview == "un nouveau mot" }.deckId)
    }

    @Test
    fun done_settles_and_leaves() {
        fixture("empty")
        show(draft())
        type("Front", "la rue")
        // The first typing of a run brings the keyboard up and shifts the layout under a touch at the
        // old place; the semantic click goes to the control itself.
        rule.onNodeWithTag("done").performSemanticsAction(SemanticsActions.OnClick)
        waitFor("left") { left }
        assertEquals(listOf("la rue"), previews())
    }

    @Test
    fun the_system_back_does_the_same() {
        fixture("empty")
        show(draft())
        type("Front", "le pont")
        rule.runOnUiThread { rule.activity.onBackPressedDispatcher.onBackPressed() }
        waitFor("left") { left }
        assertEquals(listOf("le pont"), previews())
    }

    @Test
    fun changing_kind_and_deck_on_a_stored_note_writes_them_at_once() {
        fixture("decks")
        val row = rows().first()
        show(mapOf(ARG_NOTE to row.noteId))
        rule.onNodeWithTag("kind").performClick()
        rule.onNodeWithTag("kind-option-1").performClick()
        waitFor("kind written") { storedKind(row.noteId) != "basic" }
        rule.onNodeWithTag("deck").performClick()
        rule.onNodeWithTag("deck-option-0").performClick()
        waitFor("deck cleared") { rows().first { it.noteId == row.noteId }.deckId == null }
    }

    @Test
    fun on_a_draft_the_deck_applies_when_the_note_is_born() {
        fixture("decks")
        val deck = decks().first()
        show(draft())
        rule.onNodeWithTag("deck").performClick()
        rule.onNodeWithTag("deck-option-1").performClick()
        rule.waitForIdle()
        Thread.sleep(300)
        assertFalse("nothing is stored before a field has text", "x-draft" in previews())
        type("Front", "x-draft")
        rule.onNodeWithTag("done").performClick()
        waitFor("born") { "x-draft" in previews() }
        assertEquals(deck.id, rows().first { it.preview == "x-draft" }.deckId)
    }

    @Test
    fun a_born_draft_returns_as_the_same_note_after_a_process_death() {
        fixture("empty")
        val saved = SavedStateHandle(draft())
        val first = EditorViewModel(owner, saved)
        rule.setContent { CairnTheme(dark = false) { EditorScreen(first, {}, { _, _ -> }) } }
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("editor-heading")).fetchSemanticsNodes().isNotEmpty() }
        type("Front", "un")
        rule.onNodeWithTag("done").performClick()
        waitFor("born") { previews() == listOf("un") && saved.get<String>(ARG_NOTE) != null }
        // The route's arguments come back; the born note's id is in the saved state.
        val after = EditorViewModel(owner, SavedStateHandle(saved.keys().associateWith { saved.get<Any>(it) }))
        waitFor("reopened") { after.values["Front"] == "un" }
        assertEquals(1, previews().size)
    }

    // --- both-theme captures, kept in the app's external files directory for looking at (never committed) ---

    private fun capturedInBothThemes(name: String, fixture: String, args: () -> Map<String, Any?>, drive: () -> Unit = {}) {
        val dir = context.getExternalFilesDir("editor-shots")!!
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
    fun a_new_note_is_captured_in_both_themes() = capturedInBothThemes("new", "empty", { draft() })

    @Test
    fun an_edited_note_is_captured_in_both_themes() = capturedInBothThemes("edit", "decks", { mapOf(ARG_NOTE to rows().first().noteId) })

    @Test
    fun a_persian_note_is_captured_in_both_themes() = capturedInBothThemes(
        "persian",
        "decks",
        { mapOf(ARG_NOTE to rows().first { dev.cairn.design.isRightToLeft(it.preview) }.noteId) },
    ) {
        rule.onNodeWithTag("label-Front").assertExists()
    }

    @Test
    fun a_persian_draft_typed_through_the_keyboard_is_captured_in_both_themes() = capturedInBothThemes(
        "persian-typed",
        "empty",
        { draft() },
    ) { commitThroughKeyboard("Front", persian) }
}
