package dev.cairn.app

import android.content.Context
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.cairn.design.CairnTheme
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.CollectionHandle
import dev.cairn.ffi.DeckSelector
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.util.concurrent.atomic.AtomicInteger

/**
 * A collection that refuses a deck write, and the empty collection's own *New deck*: the list keeps
 * its screen and says so where the person acted, rather than turning into the failure page.
 */
@RunWith(AndroidJUnit4::class)
class NoteListRefusalTest {
    @get:Rule
    val rule = createComposeRule()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "refusal-${System.nanoTime()}")
    private val owner = CollectionOwner(context, File(root, "files"), File(root, "no_backup"))

    /** The real collection, except that the call numbered [failAt] is refused the way a store fault is. */
    private class Refusing(private val real: CollectionOwner) : CollectionAccess {
        val calls = AtomicInteger()

        @Volatile
        var failAt = -1

        override suspend fun <T> use(block: (CollectionHandle) -> T): T {
            if (calls.incrementAndGet() == failAt) throw CairnException.Failed("refused")
            return real.use(block)
        }
    }

    @After
    fun tearDown() {
        root.deleteRecursively()
    }

    private fun fixture(name: String) = runBlocking { owner.use { it.installFixture(name, System.currentTimeMillis()) } }

    private fun facts() = runBlocking { owner.use { it.noteList(DeckSelector.All, "") } }

    private fun waitFor(tag: String) =
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag(tag)).fetchSemanticsNodes().isNotEmpty() }

    private fun show(access: CollectionAccess) {
        rule.setContent { CairnTheme(dark = false) { NotesScreen(NotesViewModel(access)) } }
        waitFor("note-row")
    }

    @Test
    fun a_refused_deck_create_is_said_in_place_and_the_list_stays() {
        fixture("decks")
        val access = Refusing(owner)
        show(access)
        rule.onNodeWithTag("new-deck-name").performTextInput("Géographie")
        rule.waitForIdle()
        access.failAt = access.calls.get() + 1
        rule.onNodeWithTag("new-deck").performClick()
        waitFor("new-deck-refused")
        rule.onNodeWithTag("new-deck-refused").assertTextEquals("The deck could not be created.")
        rule.onAllNodesWithTag("note-row")[0].assertIsDisplayed()
        rule.onNodeWithTag("new-deck-name").assertIsDisplayed()
        assertEquals(4, facts().decks.size)
        // The next press goes through, and a create that succeeds clears the refusal.
        rule.onNodeWithTag("new-deck").performClick()
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("new-deck-refused")).fetchSemanticsNodes().isEmpty() }
        rule.waitUntil(10_000) { facts().decks.size == 5 }
    }

    @Test
    fun a_refused_deck_delete_is_said_in_place_and_the_warning_and_list_stay() {
        fixture("decks")
        val access = Refusing(owner)
        show(access)
        rule.onNodeWithTag("deck-filter").performClick()
        rule.onNodeWithTag("deck-filter-option-2").performClick()
        waitFor("delete-deck")
        rule.onNodeWithTag("delete-deck").performClick()
        waitFor("delete-deck-confirm")
        rule.waitForIdle()
        access.failAt = access.calls.get() + 1
        rule.onNodeWithTag("delete-deck-confirm").performClick()
        waitFor("delete-deck-refused")
        rule.onNodeWithTag("delete-deck-refused").assertTextEquals("The deck could not be deleted.")
        rule.onNodeWithTag("delete-deck-warning").assertIsDisplayed()
        assertEquals(4, facts().decks.size)
        assertEquals(25, facts().rows.size)
    }

    @Test
    fun the_empty_collection_offers_new_deck_and_a_made_deck_shows_the_deck_controls() {
        fixture("empty")
        rule.setContent { CairnTheme(dark = false) { NotesScreen(NotesViewModel(owner)) } }
        waitFor("notes-empty")
        rule.onNodeWithTag("new-deck").assertIsDisplayed()
        rule.onNodeWithTag("deck-filter").assertDoesNotExist()
        rule.onNodeWithTag("new-deck-name").performTextInput("Géographie")
        rule.onNodeWithTag("new-deck").performClick()
        waitFor("deck-filter")
        rule.onNodeWithTag("deck-filter").assertTextEquals("Géographie")
        rule.onNodeWithTag("notes-empty").assertIsDisplayed()
        assertTrue(facts().decks.any { it.name == "Géographie" })
    }
}
