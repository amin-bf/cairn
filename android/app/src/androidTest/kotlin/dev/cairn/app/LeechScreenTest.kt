package dev.cairn.app

import android.content.Context
import androidx.activity.OnBackPressedDispatcher
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.assertHeightIsAtLeast
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.unit.dp
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.cairn.design.CairnTheme
import dev.cairn.design.pageArgb
import dev.cairn.ffi.LeechCost
import dev.cairn.ffi.LeechListFacts
import dev.cairn.ffi.LeechRowFact
import dev.cairn.ffi.Run
import dev.cairn.ffi.RunStyle
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
 * The leech screen on the emulator (#228, ADR-0050): reached from Review's durable entrance through
 * the whole navigation host, each state reached through a fixture installed into a collection of its
 * own, and a suspension made through the binding where a state needs one.
 */
@RunWith(AndroidJUnit4::class)
class LeechScreenTest {
    @get:Rule
    val rule = createComposeRule()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "leech-${System.nanoTime()}")
    private val owner = CollectionOwner(context, File(root, "files"), File(root, "no_backup"))
    private var dispatcher: OnBackPressedDispatcher? = null

    @After
    fun tearDown() {
        root.deleteRecursively()
    }

    private fun fixture(name: String) = runBlocking { owner.use { it.installFixture(name, System.currentTimeMillis()) } }

    private fun list() = runBlocking { owner.use { it.leechList(System.currentTimeMillis()) } }

    private fun suspendThroughBinding(row: LeechRowFact) = runBlocking { owner.use { it.suspendCard(row.noteId, row.slot) } }

    // The merged tree: a control's words sit on its child, and only the merged node carries them.
    private fun leaf(tag: String) = rule.onNodeWithTag(tag)

    private fun exists(tag: String) = rule.onAllNodes(hasTestTag(tag), useUnmergedTree = true).fetchSemanticsNodes().isNotEmpty()

    private fun waitFor(tag: String) = rule.waitUntil(10_000) { exists(tag) }

    private fun waitGone(tag: String) = rule.waitUntil(10_000) { !exists(tag) }

    private fun textOf(tag: String): String =
        leaf(tag).fetchSemanticsNode().config[SemanticsProperties.Text].joinToString("") { it.text }

    private fun key(row: LeechRowFact) = "${row.noteId}-${row.slot}"

    private fun showApp() {
        rule.setContent {
            dispatcher = androidx.activity.compose.LocalOnBackPressedDispatcherOwner.current!!.onBackPressedDispatcher
            CairnTheme(dark = false) { CairnNavHost(owner, keyboardUp = false) }
        }
        rule.waitUntil(10_000) { exists("start-all") || exists("state-caught-up") || exists("state-empty") }
    }

    private fun openScreen() {
        showApp()
        rule.onNodeWithTag("leech-entrance").performScrollTo().performClick()
        waitFor("leech-screen")
    }

    // --- the entrance

    @Test
    fun the_entrance_names_the_leeches_and_is_a_full_width_primary_below_the_entrance() {
        fixture("due-with-leeches")
        showApp()
        waitFor("leech-entrance")
        leaf("leech-entrance").assertTextEquals("Leeches (3)")
        leaf("leech-entrance").assertHeightIsAtLeast(36.dp)
        // Below whatever Review shows: under the way in, not beside it.
        val start = leaf("start-all").fetchSemanticsNode().boundsInRoot
        val entrance = leaf("leech-entrance").fetchSemanticsNode().boundsInRoot
        assertTrue("entrance ${entrance.top} is below start ${start.bottom}", entrance.top >= start.bottom)
    }

    @Test
    fun a_suspended_leech_is_named_once_and_the_words_follow_the_counts() {
        fixture("leeches")
        suspendThroughBinding(list().active.first())
        showApp()
        waitFor("leech-entrance")
        leaf("leech-entrance").assertTextEquals("Leeches (2) · suspended (1)")
    }

    @Test
    fun only_suspended_cards_say_suspended() {
        fixture("leeches")
        for (row in list().active) suspendThroughBinding(row)
        showApp()
        waitFor("leech-entrance")
        leaf("leech-entrance").assertTextEquals("Suspended (3)")
    }

    @Test
    fun the_entrance_is_absent_when_there_is_nothing_to_list() {
        fixture("caught-up")
        showApp()
        rule.waitForIdle()
        assertFalse(exists("leech-entrance"))
    }

    // --- the screen

    @Test
    fun the_screen_ranks_the_leeches_under_the_statement_with_a_cost_line_and_worded_controls() {
        fixture("leeches")
        val rows = list().active
        assertEquals(3, rows.size)
        openScreen()
        leaf("leeches-statement").assertTextEquals("These keep catching you out — worst first.")
        assertFalse(exists("suspended-statement"))
        for (row in rows) {
            waitFor("leech-row-${key(row)}")
            val cost = row.cost!!
            val minutes = when (cost.answerMinutes) {
                0u -> "under a minute"
                1u -> "1 minute"
                else -> "${cost.answerMinutes} minutes"
            }
            leaf("leech-cost-${key(row)}").assertTextEquals("${cost.failureDays} bad days · ${cost.reviews} reviews · $minutes")
            leaf("leech-edit-${key(row)}").assertTextEquals("Edit")
            leaf("leech-suspend-${key(row)}").assertTextEquals("Suspend")
            leaf("leech-edit-${key(row)}").assertHeightIsAtLeast(36.dp)
            leaf("leech-suspend-${key(row)}").assertHeightIsAtLeast(36.dp)
        }
        // Worst first: the rows sit top to bottom in the order the list gave them.
        val tops = rows.map { leaf("leech-row-${key(it)}").fetchSemanticsNode().boundsInRoot.top }
        assertEquals(tops.sorted(), tops)
        leaf("back-to-review").assertTextEquals("Back to review")
    }

    @Test
    fun a_screen_with_suspended_cards_and_no_leeches_shows_only_the_suspended_section() {
        fixture("leeches")
        for (row in list().active) suspendThroughBinding(row)
        openScreen()
        leaf("suspended-statement").assertTextEquals("Suspended — not shown in review.")
        assertFalse(exists("leeches-statement"))
        for (row in list().suspended) {
            waitFor("leech-row-${key(row)}")
            leaf("leech-edit-${key(row)}").assertTextEquals("Edit")
            leaf("leech-unsuspend-${key(row)}").assertTextEquals("Unsuspend")
            assertFalse(exists("leech-cost-${key(row)}"))
        }
    }

    @Test
    fun a_screen_with_neither_says_nothing_is_costing_a_lot() {
        fixture("caught-up")
        val model = LeechViewModel(owner)
        rule.setContent { CairnTheme(dark = false) { LeechScreen(model, onEdit = {}, onBack = {}) } }
        waitFor("leeches-empty")
        leaf("leeches-empty").assertTextEquals("Nothing is costing you a lot right now.")
        assertFalse(exists("leeches-statement"))
        assertFalse(exists("suspended-statement"))
    }

    @Test
    fun suspend_and_unsuspend_move_a_row_between_the_sections_and_reach_the_store() {
        fixture("leeches")
        val row = list().active.first()
        openScreen()
        leaf("leech-suspend-${key(row)}").performScrollTo().performClick()
        waitFor("suspended-statement")
        waitFor("leech-unsuspend-${key(row)}")
        assertFalse(exists("leech-suspend-${key(row)}"))
        assertEquals(listOf(key(row)), list().suspended.map { key(it) })
        assertEquals(2, list().active.size)

        leaf("leech-unsuspend-${key(row)}").performScrollTo().performClick()
        waitFor("leech-suspend-${key(row)}")
        waitGone("suspended-statement")
        assertEquals(0, list().suspended.size)
        assertEquals(3, list().active.size)
    }

    /** The real collection, except that the next call after [refuse] is set is refused the way a store fault is. */
    private class Refusing(private val real: CollectionOwner) : CollectionAccess {
        @Volatile
        var refuse = false

        override suspend fun <T> use(block: (dev.cairn.ffi.CollectionHandle) -> T): T {
            if (refuse) {
                refuse = false
                throw dev.cairn.ffi.CairnException.Failed("refused")
            }
            return real.use(block)
        }
    }

    @Test
    fun a_refused_write_is_said_in_its_row_and_the_list_is_read_again() {
        fixture("leeches")
        val rows = list().active
        val row = rows.first()
        val access = Refusing(owner)
        val model = LeechViewModel(access)
        rule.setContent { CairnTheme(dark = false) { LeechScreen(model, onEdit = {}, onBack = {}) } }
        waitFor("leech-row-${key(row)}")
        access.refuse = true
        leaf("leech-suspend-${key(row)}").performScrollTo().performClick()
        waitFor("leech-refused-${key(row)}")
        leaf("leech-refused-${key(row)}").assertTextEquals("The card could not be suspended.")
        assertEquals("nothing was written", 0, list().suspended.size)
        // Said in that row only, and the list is still all there.
        assertFalse(exists("leech-refused-${key(rows.last())}"))
        for (r in rows) assertTrue(exists("leech-row-${key(r)}"))
    }

    // --- leaving and going on

    @Test
    fun edit_opens_the_editor_and_the_return_reads_the_list_again() {
        fixture("leeches")
        val rows = list().active
        openScreen()
        leaf("leech-edit-${key(rows.first())}").performScrollTo().performClick()
        waitFor("editor-heading")
        // While the editor is open another writer suspends a card; the return must show it.
        suspendThroughBinding(rows.last())
        rule.runOnUiThread { dispatcher!!.onBackPressed() }
        waitFor("leech-screen")
        waitFor("suspended-statement")
        waitFor("leech-unsuspend-${key(rows.last())}")
    }

    @Test
    fun back_to_review_and_the_system_back_return_to_review_whose_entrance_shows_the_new_counts() {
        fixture("due-with-leeches")
        val row = list().active.first()
        openScreen()
        leaf("leech-suspend-${key(row)}").performScrollTo().performClick()
        waitFor("suspended-statement")
        leaf("back-to-review").performClick()
        waitFor("start-all")
        waitFor("leech-entrance")
        assertFalse(exists("leech-screen"))
        rule.waitUntil(10_000) { textOf("leech-entrance") == "Leeches (2) · suspended (1)" }

        rule.onNodeWithTag("leech-entrance").performScrollTo().performClick()
        waitFor("leech-screen")
        leaf("leech-unsuspend-${key(row)}").performScrollTo().performClick()
        waitGone("suspended-statement")
        rule.runOnUiThread { dispatcher!!.onBackPressed() }
        waitFor("start-all")
        rule.waitUntil(10_000) { textOf("leech-entrance") == "Leeches (3)" }
    }

    @Test
    fun review_stays_marked_on_the_destinations_row_while_the_screen_shows() {
        fixture("leeches")
        openScreen()
        val selected = { tag: String -> leaf(tag).fetchSemanticsNode().config[SemanticsProperties.Selected] }
        assertEquals(true, selected("destination-review"))
        assertEquals(false, selected("destination-notes"))
    }

    // --- both themes

    private fun row(note: String, prompt: String, rtl: Boolean, cost: LeechCost?, generated: Boolean = true) = LeechRowFact(
        noteId = note,
        slot = 0u,
        prompt = if (generated) listOf(Run(prompt, RunStyle.PLAIN)) else emptyList(),
        rtl = rtl,
        generated = generated,
        notePreview = prompt,
        notePreviewRtl = rtl,
        cost = cost,
    )

    private fun capturedInBothThemes(name: String, facts: LeechListFacts) {
        val dir = context.getExternalFilesDir("leech-shots")!!
        val dark: MutableState<Boolean> = mutableStateOf(false)
        rule.setContent {
            CairnTheme(dark = dark.value) {
                LeechList(facts, refusal = null, onEdit = {}, onSuspend = {}, onUnsuspend = {}, onBack = {})
            }
        }
        val pages = mutableListOf<Int>()
        for (isDark in listOf(false, true)) {
            dark.value = isDark
            rule.waitForIdle()
            val bitmap = rule.onRoot().captureToImage().asAndroidBitmap()
            FileOutputStream(File(dir, "$name-${if (isDark) "dark" else "light"}.png")).use {
                bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it)
            }
            assertEquals("$name page colour, dark=$isDark", pageArgb(isDark), bitmap.getPixel(2, bitmap.height / 2))
            val drawn = (0 until bitmap.height step 5).any { y ->
                (0 until bitmap.width step 5).any { x -> bitmap.getPixel(x, y) != pageArgb(isDark) }
            }
            assertTrue("$name is not blank, dark=$isDark", drawn)
            pages += bitmap.getPixel(2, 2)
        }
        assertNotEquals("$name looks the same in both themes", pages[0], pages[1])
    }

    private val persian = "معنی این واژه چیست؟"

    @Test
    fun the_ranked_list_is_captured_in_both_themes() = capturedInBothThemes(
        "ranked",
        LeechListFacts(
            active = listOf(
                row("a", "néanmoins", false, LeechCost(6u, 12u, 4u)),
                row("b", persian, true, LeechCost(4u, 9u, 1u)),
                row("c", "", false, LeechCost(3u, 5u, 0u), generated = false),
            ),
            suspended = emptyList(),
        ),
    )

    @Test
    fun the_two_sections_are_captured_in_both_themes() = capturedInBothThemes(
        "sections",
        LeechListFacts(
            active = listOf(row("a", "néanmoins", false, LeechCost(6u, 12u, 4u))),
            suspended = listOf(row("b", persian, true, null), row("c", "", false, null, generated = false)),
        ),
    )

    @Test
    fun the_suspended_only_screen_is_captured_in_both_themes() = capturedInBothThemes(
        "suspended-only",
        LeechListFacts(active = emptyList(), suspended = listOf(row("b", persian, true, null))),
    )

    @Test
    fun the_empty_screen_is_captured_in_both_themes() =
        capturedInBothThemes("empty", LeechListFacts(active = emptyList(), suspended = emptyList()))

    @Test
    fun the_entrance_is_captured_in_both_themes() {
        // The entrance beside Review's own states is covered by the screen's capture of "backlog"
        // elsewhere; here the durable control itself is drawn with its words in both themes.
        val dark = mutableStateOf(false)
        fixture("due-with-leeches")
        val model = ReviewViewModel(owner)
        rule.setContent { CairnTheme(dark = dark.value) { ReviewScreen(model) } }
        waitFor("leech-entrance")
        val dir = context.getExternalFilesDir("leech-shots")!!
        val pages = mutableListOf<Int>()
        for (isDark in listOf(false, true)) {
            dark.value = isDark
            rule.waitForIdle()
            leaf("leech-entrance").assertIsDisplayed()
            val bitmap = rule.onRoot().captureToImage().asAndroidBitmap()
            FileOutputStream(File(dir, "entrance-${if (isDark) "dark" else "light"}.png")).use {
                bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it)
            }
            pages += bitmap.getPixel(2, 2)
        }
        assertNotEquals(pages[0], pages[1])
    }
}
