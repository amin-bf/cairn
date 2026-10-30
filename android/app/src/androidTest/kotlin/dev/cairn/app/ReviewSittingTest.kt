package dev.cairn.app

import android.content.Context
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.click
import androidx.compose.ui.test.getBoundsInRoot
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.height
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.requiredSize
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.performScrollTo
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.cairn.design.CairnTheme
import dev.cairn.design.CardTierKey
import dev.cairn.design.pageArgb
import dev.cairn.ffi.IntervalPreview
import dev.cairn.ffi.Run
import dev.cairn.ffi.RunStyle
import dev.cairn.ffi.ShownCard
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
 * A Review sitting on the emulator, from the first card to the return to the entrance. States are
 * reached through fixtures installed by name into a collection of the test's own, and drawn the way
 * the activity draws them; the clocks are the test's, so the ten-minute check-in is reached by
 * passing a later monotonic time rather than by waiting.
 */
@RunWith(AndroidJUnit4::class)
class ReviewSittingTest {
    @get:Rule
    val rule = createComposeRule()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "sitting-${System.nanoTime()}")
    private val owner = CollectionOwner(context, File(root, "files"), File(root, "no_backup"))
    private var mono = 1_000L

    @After
    fun tearDown() {
        root.deleteRecursively()
    }

    private fun fixture(name: String) = runBlocking { owner.use { it.installFixture(name, System.currentTimeMillis()) } }

    // The card is one merged node; its faces and badge are read from the tree beneath it.
    private fun leaf(tag: String) = rule.onNodeWithTag(tag, useUnmergedTree = true)

    private fun exists(tag: String) = rule.onAllNodes(hasTestTag(tag), useUnmergedTree = true).fetchSemanticsNodes().isNotEmpty()

    private fun waitFor(tag: String) = rule.waitUntil(10_000) { exists(tag) }

    private fun waitGone(tag: String) = rule.waitUntil(10_000) { !exists(tag) }

    // The screen's model, swappable so a test can stand in for a relaunch.
    private lateinit var current: MutableState<ReviewViewModel>

    private fun show(dark: MutableState<Boolean> = mutableStateOf(false)): ReviewViewModel {
        val model = ReviewViewModel(owner, monotonicClock = { mono })
        current = mutableStateOf(model)
        rule.setContent { CairnTheme(dark = dark.value) { ReviewScreen(current.value) } }
        waitFor("start-all")
        return model
    }

    private fun begin(tag: String = "start-all") {
        rule.onNodeWithTag(tag).performClick()
        waitFor("progress")
        waitFor("card")
    }

    private fun reveal() {
        rule.onNodeWithTag("card").performClick()
        waitFor("grades")
    }

    private fun textOf(tag: String): String =
        leaf(tag).fetchSemanticsNode().config[SemanticsProperties.Text].joinToString("") { it.text }

    private fun progress(): String = textOf("progress")

    private fun grade(tag: String) {
        val before = progress()
        // A short screen scrolls the cluster into reach, as a thumb would.
        rule.onNodeWithTag(tag).performScrollTo().performClick()
        rule.waitUntil(10_000) { !exists("progress") || progress() != before }
    }

    @Test
    fun no_grade_control_exists_before_the_reveal_and_a_tap_where_they_will_be_records_nothing() {
        fixture("backlog")
        show()
        begin("shorter-5")
        rule.onNodeWithTag("progress").assertTextEquals("0 of 5")
        assertFalse(exists("grades"))
        assertFalse(exists("grade-good"))
        // Tap where the cluster will be: straight under the card, where each of its controls will sit.
        val cardBottom = rule.onNodeWithTag("card").getBoundsInRoot().bottom
        rule.onRoot().performTouchInput {
            for (below in listOf(30.dp, 80.dp, 130.dp)) click(Offset(width / 2f, (cardBottom + below).toPx()))
        }
        rule.waitForIdle()
        rule.onNodeWithTag("progress").assertTextEquals("0 of 5")
        assertFalse(exists("grades"))
        assertFalse(exists("answer"))
    }

    @Test
    fun the_answer_is_not_on_screen_until_the_card_is_tapped() {
        fixture("backlog")
        show()
        begin("shorter-5")
        assertFalse(exists("answer"))
        assertFalse(exists("badge"))
        reveal()
        waitFor("answer")
        waitFor("badge")
        leaf("answer").assertIsDisplayed()
    }

    @Test
    fun a_forgotten_card_returns_in_the_same_sitting_and_progress_moves_on_every_grade() {
        // One card is due and the sitting is asked for five: the only card there is can come back.
        fixture("crossing")
        val model = show()
        model.start(5)
        waitFor("card")
        rule.onNodeWithTag("progress").assertTextEquals("0 of 5")
        val prompt = promptText()
        reveal()
        grade("grade-forgot")
        rule.onNodeWithTag("progress").assertTextEquals("1 of 5")
        // The lapse is shown again, shut, and the progress line moves on its grade too.
        waitFor("card")
        assertEquals(prompt, promptText())
        assertFalse("the re-show starts shut", exists("grades"))
        reveal()
        grade("grade-easy")
        // The lapse took the card over the leech floor, so the pointer stands in the entrance's place
        // (#230); answering it shows the entrance, caught up.
        waitFor("leech-pointer")
        rule.onNodeWithTag("pointer-dismiss").performClick()
        waitFor("state-caught-up")
    }

    private fun promptText(): String = textOf("prompt")

    @Test
    fun reaching_the_count_returns_straight_to_the_entrance() {
        fixture("backlog")
        show()
        begin("shorter-5")
        repeat(5) {
            reveal()
            grade("grade-easy")
        }
        waitFor("start-all")
        rule.onNodeWithTag("state-due").assertIsDisplayed()
        assertFalse(exists("progress"))
    }

    @Test
    fun an_emptied_queue_returns_to_the_entrance_before_the_count() {
        fixture("crossing")
        show()
        begin()
        reveal()
        grade("grade-easy")
        waitFor("state-caught-up")
    }

    @Test
    fun the_checkpoint_appears_after_ten_minutes_and_keep_going_dismisses_it_for_the_sitting() {
        fixture("backlog")
        val model = show()
        begin("shorter-10")
        assertFalse(exists("checkpoint"))
        mono += 10 * 60 * 1000L + 1
        model.tick()
        waitFor("checkpoint")
        rule.onNodeWithTag("checkpoint").assertIsDisplayed()
        // It never hides the card, which stays gradeable.
        rule.onNodeWithTag("card").assertIsDisplayed()
        reveal()
        grade("grade-good")
        waitFor("checkpoint")
        rule.onNodeWithTag("keep-going").performClick()
        waitGone("checkpoint")
        mono += 30 * 60 * 1000L
        model.tick()
        rule.waitForIdle()
        reveal()
        grade("grade-good")
        assertFalse("keep going holds for the sitting", exists("checkpoint"))
    }

    @Test
    fun finish_here_ends_the_sitting_at_the_entrance() {
        fixture("backlog")
        val model = show()
        begin("shorter-10")
        mono += 10 * 60 * 1000L + 1
        model.tick()
        waitFor("checkpoint")
        rule.onNodeWithTag("finish-here").performClick()
        waitFor("start-all")
        assertFalse(exists("progress"))
    }

    @Test
    fun a_relaunch_mid_sitting_shows_the_entrance_with_what_is_still_due() {
        fixture("backlog")
        show()
        begin("shorter-5")
        reveal()
        grade("grade-easy")
        // The process dies: the sitting was held only in memory. A new model over the same collection
        // is the relaunch.
        current.value = ReviewViewModel(owner, monotonicClock = { mono })
        waitFor("start-all")
        rule.onNodeWithTag("state-due").assertTextEquals("Plenty due — pick a comfortable size, the rest will keep.")
        rule.onNodeWithTag("start-all").assertTextEquals("Start — all 24")
    }

    @Test
    fun the_cloze_card_keeps_one_type_tier_and_one_rect_across_the_reveal() {
        fixture("cloze")
        show()
        begin()
        val tierBefore = rule.onNodeWithTag("card").fetchSemanticsNode().config[CardTierKey]
        val rectBefore = rule.onNodeWithTag("card").getBoundsInRoot()
        reveal()
        rule.waitForIdle()
        val tierAfter = rule.onNodeWithTag("card").fetchSemanticsNode().config[CardTierKey]
        val rectAfter = rule.onNodeWithTag("card").getBoundsInRoot()
        assertEquals(tierBefore, tierAfter, 0f)
        assertEquals(rectBefore, rectAfter)
        assertTrue("a cloze paragraph steps down from display", tierBefore < 40f)
    }

    private fun shown(prompt: String, rtl: Boolean, revealed: Boolean = true, isNew: Boolean = false) = ShownCard(
        noteId = "n",
        ordinal = 0u,
        prompt = listOf(Run(prompt, RunStyle.PLAIN)),
        answer = listOf(Run("answer", RunStyle.PLAIN), Run(" bold", RunStyle.BOLD), Run(" italic", RunStyle.ITALIC), Run(" code", RunStyle.CODE)),
        rtl = rtl,
        isNew = isNew,
        boxNumber = 3u,
        previews = IntervalPreview(1u, 2u, 5u, 9u),
        revealed = revealed,
    )

    // A sitting drawn from a card built here, for what no fixture's first card is guaranteed to be.
    private val drawnCard = mutableStateOf(shown("chien", rtl = false))
    private val drawnCheckpoint = mutableStateOf(false)
    private val drawnDark = mutableStateOf(false)
    private var contentSet = false

    private fun draw(card: ShownCard) {
        drawnCard.value = card
        if (!contentSet) {
            contentSet = true
            rule.setContent {
                CairnTheme(dark = drawnDark.value) {
                    ReviewSitting(10, 3, drawnCard.value, drawnCheckpoint.value, {}, {}, {}, {})
                }
            }
        }
        rule.waitForIdle()
    }

    @Test
    fun a_persian_prompt_puts_the_badge_top_left_and_a_french_one_top_right() {
        draw(shown("سلام", rtl = true))
        var card = rule.onNodeWithTag("card").getBoundsInRoot()
        var badge = leaf("badge").getBoundsInRoot()
        assertTrue("persian badge is on the left: $badge in $card", badge.left - card.left < card.right - badge.right)
        leaf("badge").assertTextEquals("box 3")

        draw(shown("chien", rtl = false))
        card = rule.onNodeWithTag("card").getBoundsInRoot()
        badge = leaf("badge").getBoundsInRoot()
        assertTrue("french badge is on the right: $badge in $card", card.right - badge.right < badge.left - card.left)
    }

    @Test
    fun a_card_with_no_history_reads_new() {
        draw(shown("chien", rtl = false, isNew = true))
        leaf("badge").assertTextEquals("new")
    }

    @Test
    fun the_grade_cluster_sits_on_the_reach_line_with_every_control_36_dp_tall() {
        // The emulator is 640 dp tall: the heading, a 300 dp card and the cluster leave no slack above
        // a reach line of 165 dp there, so the page is given the height of a handset (a Pixel's is about
        // 900 dp) and the reach line is measured in that.
        rule.setContent {
            CairnTheme(dark = false) {
                Box(Modifier.requiredSize(411.dp, 900.dp)) {
                    ReviewSitting(10, 3, shown("chien", rtl = false), false, {}, {}, {}, {})
                }
            }
        }
        rule.waitForIdle()
        val room = rule.onNodeWithTag("page-room").getBoundsInRoot()
        val grades = rule.onNodeWithTag("grades").getBoundsInRoot()
        assertEquals("the cluster's bottom is 165 dp above the page's bottom", (room.bottom - 165.dp).value, grades.bottom.value, 0.6f)
        for (tag in listOf("grade-forgot", "grade-barely", "grade-good", "grade-easy")) {
            assertTrue("$tag is at least 36 dp", rule.onNodeWithTag(tag).getBoundsInRoot().height >= 36.dp)
        }
        rule.onNodeWithTag("grade-forgot").assertTextEquals("Forgot", "1d")
        rule.onNodeWithTag("grade-easy").assertTextEquals("Easy", "9d")
    }

    /**
     * The screen is kept in the app's external files directory for looking at (never committed); what is
     * asserted is what the theme and the state decide: the page colour fills the frame.
     */
    private fun capture(name: String, dark: Boolean): android.graphics.Bitmap {
        val dir = context.getExternalFilesDir("sitting-shots")!!
        val bitmap = rule.onRoot().captureToImage().asAndroidBitmap()
        FileOutputStream(File(dir, "$name-${if (dark) "dark" else "light"}.png")).use {
            bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it)
        }
        assertEquals("$name page colour, dark=$dark", pageArgb(dark), bitmap.getPixel(2, bitmap.height / 2))
        return bitmap
    }

    /** The question, the revealed card and the checkpoint, each in both themes. */
    @Test
    fun the_question_the_revealed_card_and_the_checkpoint_are_captured_in_both_themes() {
        val pages = mutableListOf<Int>()
        for (isDark in listOf(false, true)) {
            drawnDark.value = isDark
            drawnCheckpoint.value = false
            draw(shown("Qu'est-ce que c'est?", rtl = false, revealed = false))
            val question = capture("question", isDark)

            draw(shown("Qu'est-ce que c'est?", rtl = false, revealed = true))
            assertTrue(exists("grades"))
            val revealed = capture("revealed", isDark)
            assertFalse("the revealed screen differs from the question", question.sameAs(revealed))

            drawnCheckpoint.value = true
            rule.waitForIdle()
            val checkpoint = capture("checkpoint", isDark)
            assertTrue("the card is still there under the check-in", exists("card"))
            assertFalse("the check-in is drawn", revealed.sameAs(checkpoint))
            pages += question.getPixel(2, 2)
        }
        assertNotEquals("the two themes look the same", pages[0], pages[1])
    }
}
