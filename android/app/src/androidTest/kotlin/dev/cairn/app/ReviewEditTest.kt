package dev.cairn.app

import android.content.Context
import androidx.activity.OnBackPressedDispatcher
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performTextReplacement
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.cairn.design.CairnTheme
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

/**
 * *Edit note* from Review and the return to the sitting (ADR-0029, ADR-0048 §6): the action only after
 * the reveal, a round trip through the editor that comes back to the same card with the edit showing
 * and still revealed, and an edit that stops the card generating moving the sitting on with no grade.
 */
@RunWith(AndroidJUnit4::class)
class ReviewEditTest {
    @get:Rule
    val rule = createComposeRule()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "review-edit-${System.nanoTime()}")
    private val owner = CollectionOwner(context, File(root, "files"), File(root, "no_backup"))
    private var dispatcher: OnBackPressedDispatcher? = null

    @After
    fun tearDown() {
        root.deleteRecursively()
    }

    private fun fixture(name: String) = runBlocking { owner.use { it.installFixture(name, System.currentTimeMillis()) } }

    private fun leaf(tag: String) = rule.onNodeWithTag(tag, useUnmergedTree = true)

    private fun exists(tag: String) = rule.onAllNodes(hasTestTag(tag), useUnmergedTree = true).fetchSemanticsNodes().isNotEmpty()

    private fun waitFor(tag: String) = rule.waitUntil(10_000) { exists(tag) }

    private fun textOf(tag: String): String =
        leaf(tag).fetchSemanticsNode().config[SemanticsProperties.Text].joinToString("") { it.text }

    private fun showApp() {
        rule.setContent {
            dispatcher = androidx.activity.compose.LocalOnBackPressedDispatcherOwner.current!!.onBackPressedDispatcher
            CairnTheme(dark = false) { CairnNavHost(owner, keyboardUp = false) }
        }
        waitFor("start-all")
    }

    private fun begin(tag: String) {
        rule.onNodeWithTag(tag).performClick()
        waitFor("progress")
        waitFor("card")
    }

    private fun reveal() {
        rule.onNodeWithTag("card").performClick()
        waitFor("grades")
    }

    @Test
    fun edit_note_is_absent_before_the_reveal_and_present_after_it() {
        fixture("backlog")
        showApp()
        begin("shorter-5")
        assertFalse(exists("edit-note"))
        reveal()
        waitFor("edit-note")
        rule.onNodeWithTag("edit-note").assertTextEquals("Edit note")
    }

    @Test
    fun editing_the_prompt_and_pressing_done_returns_to_the_same_card_edited_and_still_revealed() {
        fixture("backlog")
        showApp()
        begin("shorter-5")
        val before = textOf("prompt")
        val progress = textOf("progress")
        reveal()
        rule.onNodeWithTag("edit-note").performScrollTo().performClick()
        waitFor("editor-heading")
        // Both faces are fields of the note: marking each means the prompt changes whichever face it is.
        for (field in listOf("Front", "Back")) {
            rule.onNodeWithTag("field-$field").performTextReplacement("EDITED")
        }
        rule.waitForIdle()
        rule.onNodeWithTag("done").performScrollTo().performClick()
        waitFor("card")
        waitFor("grades")
        assertTrue(textOf("prompt"), textOf("prompt").contains("EDITED"))
        assertNotEquals(before, textOf("prompt"))
        assertEquals("the same card: nothing was graded", progress, textOf("progress"))
        assertFalse(exists("editor-heading"))
    }

    @Test
    fun the_system_back_from_the_editor_returns_to_the_sitting_the_same_way() {
        fixture("backlog")
        showApp()
        begin("shorter-5")
        val progress = textOf("progress")
        reveal()
        rule.onNodeWithTag("edit-note").performScrollTo().performClick()
        waitFor("editor-heading")
        // Typing settles on its own; the system Back pops without Done and the edit still shows.
        for (field in listOf("Front", "Back")) {
            rule.onNodeWithTag("field-$field").performTextReplacement("EDITED")
        }
        rule.waitForIdle()
        rule.runOnUiThread { dispatcher!!.onBackPressed() }
        waitFor("grades")
        assertEquals(progress, textOf("progress"))
        assertTrue(textOf("prompt"), textOf("prompt").contains("EDITED"))
    }

    @Test
    fun an_edit_that_stops_the_card_generating_returns_to_the_next_card_unrevealed_and_uncounted() {
        fixture("cloze")
        showApp()
        begin("start-all")
        val progress = textOf("progress")
        val prompt = textOf("prompt")
        reveal()
        rule.onNodeWithTag("edit-note").performScrollTo().performClick()
        waitFor("editor-heading")
        // The cloze text loses every blank, so none of this note's cards generates any more.
        rule.onNodeWithTag("field-Text").performTextReplacement("no blank any more")
        rule.waitForIdle()
        rule.onNodeWithTag("done").performScrollTo().performClick()
        // The fixture holds several cloze notes: the next one is on screen, unrevealed, nothing counted.
        rule.waitUntil(10_000) { exists("card") && !exists("editor-heading") && textOf("prompt") != prompt }
        assertEquals("no grading counted", progress, textOf("progress"))
        assertFalse("the next card starts unrevealed", exists("grades"))
        assertFalse(textOf("prompt"), textOf("prompt").contains("no blank any more"))
    }
}
