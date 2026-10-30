package dev.cairn.app

import android.content.Context
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.onNodeWithText
import dev.cairn.design.pageArgb
import androidx.compose.ui.test.assertHeightIsAtLeast
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import androidx.compose.ui.unit.dp
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.cairn.design.CairnTheme
import dev.cairn.ffi.Grade
import dev.cairn.ffi.ReviewFacts
import dev.cairn.ffi.ReviewStateKind
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
 * Review's entrance on the emulator, each state reached through a fixture installed by name into a
 * collection of its own (never the app's), and drawn the way the activity draws it.
 */
@RunWith(AndroidJUnit4::class)
class ReviewEntranceTest {
    @get:Rule
    val rule = createComposeRule()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "entrance-${System.nanoTime()}")
    private val owner = CollectionOwner(context, File(root, "files"), File(root, "no_backup"))

    @After
    fun tearDown() {
        root.deleteRecursively()
    }

    private fun fixture(name: String) = runBlocking { owner.use { it.installFixture(name, System.currentTimeMillis()) } }

    private fun show(dark: MutableState<Boolean> = mutableStateOf(false)) {
        val model = ReviewViewModel(owner)
        rule.setContent { CairnTheme(dark = dark.value) { ReviewScreen(model) } }
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("start-all") or hasTestTag("state-caught-up") or hasTestTag("state-empty")).fetchSemanticsNodes().isNotEmpty() }
    }

    @Test
    fun a_collection_with_nothing_due_says_it_is_caught_up_under_the_mark() {
        fixture("caught-up")
        show()
        rule.onNodeWithTag("state-caught-up").assertIsDisplayed()
        rule.onNodeWithTag("caught-up-mark").assertIsDisplayed()
        rule.onNodeWithText("All caught up.").assertIsDisplayed()
        rule.onNodeWithText("Nothing is due right now.").assertIsDisplayed()
        rule.onNodeWithTag("start-all").assertDoesNotExist()
        rule.onNodeWithTag("shorter-sittings").assertDoesNotExist()
    }

    @Test
    fun a_backlog_is_framed_and_offers_the_shorter_sittings() {
        fixture("backlog")
        show()
        rule.onNodeWithTag("state-due").assertTextEquals("Plenty due — pick a comfortable size, the rest will keep.")
        rule.onNodeWithTag("start-all").assertTextEquals("Start — all 25")
        rule.onNodeWithText("or a shorter sitting:").assertIsDisplayed()
        for (n in listOf(5, 10, 20)) rule.onNodeWithTag("shorter-$n").assertTextEquals("$n")
    }

    @Test
    fun a_fresh_deck_is_worded_and_has_no_shorter_line_when_none_is_shorter() {
        fixture("persian")
        show()
        rule.onNodeWithTag("state-new-deck").assertTextEquals("A fresh deck. These cards are new — start whenever you like.")
        rule.onNodeWithTag("start-all").assertTextEquals("Start — all 5")
        rule.onNodeWithTag("shorter-sittings").assertDoesNotExist()
    }

    @Test
    fun nothing_due_with_new_cards_left_is_worded_in_the_singular_and_the_plural() {
        fixture("persian")
        // One card reviewed leaves the rest new and nothing due: the state of a collection with history.
        runBlocking {
            owner.use { handle ->
                val now = System.currentTimeMillis()
                handle.startSitting(1u, 0uL, now).use {
                    it.card(now, 1uL)
                    it.reveal()
                    it.grade(Grade.GOOD, 2uL, now)
                }
            }
        }
        show()
        rule.onNodeWithTag("state-new-only").assertTextEquals("Nothing due right now. 4 new cards, whenever you like.")
        rule.onNodeWithTag("start-all").assertTextEquals("Start — all 4")
    }

    @Test
    fun the_other_wordings_are_exact() {
        val cases = listOf(
            facts(ReviewStateKind.Empty, 0u, 0u) to ("state-empty" to "No cards yet. Add a note to start reviewing."),
            facts(ReviewStateKind.NewOnly(1u), 0u, 1u) to ("state-new-only" to "Nothing due right now. One new card, whenever you like."),
            facts(ReviewStateKind.Due(3u, 2u, false), 3u, 2u) to ("state-due" to "3 due, plus 2 new. Pick a size."),
            facts(ReviewStateKind.Due(3u, 0u, false), 3u, 0u) to ("state-due" to "3 due. Pick a size."),
        )
        val current = mutableStateOf(cases[0].first)
        rule.setContent { CairnTheme(dark = false) { ReviewEntrance(current.value, onStart = {}) } }
        for ((f, expected) in cases) {
            current.value = f
            rule.waitForIdle()
            rule.onNodeWithTag(expected.first).assertTextEquals(expected.second)
        }
        current.value = facts(ReviewStateKind.NewOnly(1u), 0u, 1u)
        rule.waitForIdle()
        rule.onNodeWithTag("start-all").assertTextEquals("Start — the one card")
    }

    private fun facts(state: ReviewStateKind, due: UInt, new: UInt) =
        ReviewFacts(state, due, new, due + new, false, emptyList(), 0u, 0u)

    @Test
    fun every_control_is_at_least_36_dp_tall() {
        fixture("backlog")
        show()
        rule.onNodeWithTag("start-all").assertHeightIsAtLeast(36.dp)
        for (n in listOf(5, 10, 20)) rule.onNodeWithTag("shorter-$n").assertHeightIsAtLeast(36.dp)
    }

    @Test
    fun starting_opens_a_sitting_of_the_count_chosen() {
        fixture("backlog")
        show()
        rule.onNodeWithTag("shorter-10").performClick()
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("sitting-stub")).fetchSemanticsNodes().isNotEmpty() }
        rule.onNodeWithTag("sitting-stub").assertTextEquals("A sitting of 10")
    }

    @Test
    fun no_call_into_the_binding_ran_on_the_main_thread() {
        fixture("backlog")
        show()
        rule.onNodeWithTag("start-all").performClick()
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("sitting-stub")).fetchSemanticsNodes().isNotEmpty() }
        val app = ApplicationProvider.getApplicationContext<CairnApplication>()
        assertEquals(
            "StrictMode saw work of ours on the main thread:\n${app.strictModeViolations.joinToString("\n---\n")}",
            emptyList<String>(),
            app.strictModeViolations.toList(),
        )
    }

    /**
     * Each state is captured in both themes. The image is kept in the app's external files directory
     * for looking at (never committed), and checked for what the theme decides: the page colour fills
     * the frame, and the two themes differ.
     */
    private fun capturedInBothThemes(name: String) {
        val dir = context.getExternalFilesDir("entrance-shots")!!
        fixture(name)
        val dark = mutableStateOf(false)
        show(dark)
        val pages = mutableListOf<Int>()
        for (isDark in listOf(false, true)) {
            dark.value = isDark
            rule.waitForIdle()
            val bitmap = rule.onRoot().captureToImage().asAndroidBitmap()
            FileOutputStream(File(dir, "$name-${if (isDark) "dark" else "light"}.png")).use {
                bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it)
            }
            // The page colour fills the frame beside the column, whatever the state draws inside it.
            assertEquals("$name page colour, dark=$isDark", pageArgb(isDark), bitmap.getPixel(2, bitmap.height / 2))
            // Something is drawn on it: not every sampled pixel is the page.
            val drawn = (0 until bitmap.height step 5).any { y ->
                (0 until bitmap.width step 5).any { x -> bitmap.getPixel(x, y) != pageArgb(isDark) }
            }
            assertTrue("$name is not blank, dark=$isDark", drawn)
            pages += bitmap.getPixel(2, 2)
        }
        assertNotEquals("$name looks the same in both themes", pages[0], pages[1])
    }

    @Test
    fun the_caught_up_floor_is_captured_in_both_themes() = capturedInBothThemes("caught-up")

    @Test
    fun a_backlog_is_captured_in_both_themes() = capturedInBothThemes("backlog")

    @Test
    fun a_fresh_deck_is_captured_in_both_themes() = capturedInBothThemes("persian")
}
