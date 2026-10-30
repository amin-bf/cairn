package dev.cairn.app

import android.content.Context
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.cairn.design.CairnTheme
import dev.cairn.design.pageArgb
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
 * The end-of-session leech pointer on the emulator (#230, ADR-0010 §6, ADR-0050 §4): a sitting on the
 * `crossing` fixture pushes a card over the floor, and the pointer takes the entrance's place until it
 * is answered. Held in Review's ViewModel only, so a rotation (the screen leaving and re-entering
 * composition over the same ViewModel) keeps it.
 */
@RunWith(AndroidJUnit4::class)
class LeechPointerTest {
    @get:Rule
    val rule = createComposeRule()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "pointer-${System.nanoTime()}")
    private val owner = CollectionOwner(context, File(root, "files"), File(root, "no_backup"))

    @After
    fun tearDown() {
        root.deleteRecursively()
    }

    private fun fixture(name: String) = runBlocking { owner.use { it.installFixture(name, System.currentTimeMillis()) } }

    private fun exists(tag: String) = rule.onAllNodes(hasTestTag(tag), useUnmergedTree = true).fetchSemanticsNodes().isNotEmpty()

    private fun waitFor(tag: String) = rule.waitUntil(10_000) { exists(tag) }

    private fun waitGone(tag: String) = rule.waitUntil(10_000) { !exists(tag) }

    private fun textOf(tag: String): String =
        rule.onNodeWithTag(tag).fetchSemanticsNode().config[SemanticsProperties.Text].joinToString("") { it.text }

    // Review alone, with its model kept outside the composition so a test can leave and re-enter.
    private val inComposition: MutableState<Boolean> = mutableStateOf(true)

    private fun showReview() {
        val model = ReviewViewModel(owner)
        rule.setContent {
            CairnTheme(dark = false) { if (inComposition.value) ReviewScreen(model) }
        }
        waitFor("start-all")
    }

    private fun showApp() {
        rule.setContent { CairnTheme(dark = false) { CairnNavHost(owner, keyboardUp = false) } }
        waitFor("start-all")
    }

    /** A sitting of one card that is forgotten: the `crossing` card goes over the floor. */
    private fun forgetTheCrossingCard() {
        rule.onNodeWithTag("start-all").performClick()
        waitFor("card")
        rule.onNodeWithTag("card").performClick()
        waitFor("grades")
        rule.onNodeWithTag("grade-forgot").performScrollTo().performClick()
    }

    @Test
    fun a_sitting_that_pushes_a_card_over_the_floor_ends_on_the_pointer_with_the_count() {
        fixture("crossing")
        showReview()
        forgetTheCrossingCard()
        waitFor("leech-pointer")
        rule.onNodeWithTag("leech-pointer").assertTextEquals("1 card is costing you a lot. Take a look?")
        rule.onNodeWithTag("pointer-show").assertTextEquals("Show me")
        rule.onNodeWithTag("pointer-dismiss").assertTextEquals("Not now")
        // In place of the entrance, not beside it.
        assertFalse(exists("leech-entrance"))
        assertFalse(exists("start-all"))
    }

    @Test
    fun not_now_lands_on_the_entrance_and_the_pointer_does_not_come_back() {
        fixture("crossing")
        showApp()
        forgetTheCrossingCard()
        waitFor("pointer-dismiss")
        rule.onNodeWithTag("pointer-dismiss").performClick()
        waitFor("leech-entrance")
        assertFalse(exists("leech-pointer"))
        assertFalse(exists("leech-screen"))
        // Leaving and coming back to Review does not bring it back either.
        rule.onNodeWithTag("destination-notes").performClick()
        rule.onNodeWithTag("destination-review").performClick()
        waitFor("leech-entrance")
        assertFalse(exists("leech-pointer"))
    }

    @Test
    fun show_me_lands_on_the_leech_screen_and_the_pointer_does_not_come_back() {
        fixture("crossing")
        showApp()
        forgetTheCrossingCard()
        waitFor("pointer-show")
        rule.onNodeWithTag("pointer-show").performClick()
        waitFor("leech-screen")
        assertFalse(exists("leech-pointer"))
        rule.onNodeWithTag("back-to-review").performClick()
        waitFor("leech-entrance")
        assertFalse(exists("leech-pointer"))
    }

    @Test
    fun the_pointer_survives_a_rotation() {
        fixture("crossing")
        showReview()
        forgetTheCrossingCard()
        waitFor("leech-pointer")
        inComposition.value = false
        waitGone("leech-pointer")
        inComposition.value = true
        waitFor("leech-pointer")
        rule.onNodeWithTag("leech-pointer").assertTextEquals("1 card is costing you a lot. Take a look?")
    }

    @Test
    fun a_sitting_in_which_nothing_crosses_ends_on_the_entrance() {
        fixture("crossing")
        showReview()
        rule.onNodeWithTag("start-all").performClick()
        waitFor("card")
        rule.onNodeWithTag("card").performClick()
        waitFor("grades")
        rule.onNodeWithTag("grade-good").performScrollTo().performClick()
        waitGone("progress")
        waitFor("state-caught-up")
        assertFalse(exists("leech-pointer"))
    }

    @Test
    fun many_cards_are_counted_in_the_plural() {
        showPointer(3)
        assertEquals("3 cards are costing you a lot. Take a look?", textOf("leech-pointer"))
    }

    private fun showPointer(crossed: Int, dark: MutableState<Boolean> = mutableStateOf(false)) {
        rule.setContent { CairnTheme(dark = dark.value) { LeechPointer(crossed, onShow = {}, onDismiss = {}) } }
        waitFor("leech-pointer")
    }

    @Test
    fun the_pointer_is_captured_in_both_themes() {
        val dir = context.getExternalFilesDir("leech-shots")!!
        val dark: MutableState<Boolean> = mutableStateOf(false)
        showPointer(2, dark)
        val pages = mutableListOf<Int>()
        for (isDark in listOf(false, true)) {
            dark.value = isDark
            rule.waitForIdle()
            val bitmap = rule.onRoot().captureToImage().asAndroidBitmap()
            FileOutputStream(File(dir, "pointer-${if (isDark) "dark" else "light"}.png")).use {
                bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it)
            }
            assertEquals("page colour, dark=$isDark", pageArgb(isDark), bitmap.getPixel(2, bitmap.height / 2))
            assertTrue(
                "not blank, dark=$isDark",
                (0 until bitmap.height step 5).any { y -> (0 until bitmap.width step 5).any { x -> bitmap.getPixel(x, y) != pageArgb(isDark) } },
            )
            pages += bitmap.getPixel(2, 2)
        }
        assertNotEquals("looks the same in both themes", pages[0], pages[1])
    }
}
