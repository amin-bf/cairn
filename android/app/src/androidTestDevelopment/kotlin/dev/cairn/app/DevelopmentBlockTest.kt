package dev.cairn.app

import android.content.Context
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.assertIsDisplayed
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
import dev.cairn.design.pageArgb
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.CollectionHandle
import dev.cairn.ffi.ThemeChoice
import dev.cairn.ffi.fixtures
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.io.FileOutputStream

/**
 * The Development block (#237, ADR-0051 §1, §6), compiled only into a build made with
 * `CAIRN_DEVELOPMENT=1`: reset and the fixture bench, each asking first in its row, and the app
 * starting afresh after either.
 */
@RunWith(AndroidJUnit4::class)
class DevelopmentBlockTest {
    @get:Rule
    val rule = createComposeRule()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "development-${System.nanoTime()}")
    private val owner = CollectionOwner(context, File(root, "files"), File(root, "no_backup"))

    @Before
    fun clearTheBench() {
        // The bench's last answer outlives the screens, so a test starts without the last test's.
        DevelopmentBench.said = null
    }

    @After
    fun tearDown() {
        root.deleteRecursively()
    }

    private fun exists(tag: String) = rule.onAllNodes(hasTestTag(tag)).fetchSemanticsNodes().isNotEmpty()

    private fun waitFor(tag: String) = rule.waitUntil(10_000) { exists(tag) }

    private fun notes() = runBlocking { owner.use { it.noteCount() } }

    private fun selected(tag: String): Boolean =
        rule.onNodeWithTag(tag).fetchSemanticsNode().config[SemanticsProperties.Selected]

    private fun showApp(initial: ThemeChoice = ThemeChoice.LIGHT): AppearanceState {
        runBlocking { owner.use { it.setThemeChoice(initial) } }
        val appearance = AppearanceState(initial)
        rule.setContent { CairnAppearance(appearance) { CairnNavHost(owner, keyboardUp = false) } }
        waitFor("start-all")
        return appearance
    }

    private fun openSettings() {
        rule.onNodeWithTag("destination-settings").performClick()
        waitFor("settings-screen")
    }

    private fun press(tag: String) {
        rule.onNodeWithTag(tag).performScrollTo().performClick()
    }

    @Test
    fun the_block_closes_settings_under_its_own_heading_with_the_two_texts() {
        showApp()
        openSettings()
        rule.onNodeWithTag("development-heading").performScrollTo().assertTextEquals("Development")
        rule.onNodeWithTag("reset-sentence").performScrollTo().assertTextEquals(
            "Development control — returns this device to a first launch, seed and all.",
        )
        rule.onNodeWithTag("reset-control").performScrollTo().assertTextEquals("Reset the collection")
        rule.onNodeWithTag("bench-sentence").performScrollTo().assertTextEquals(
            "Development control — replaces the collection with a pre-made one, to reach a screen the shipping seed cannot.",
        )
        // The block follows the page's last group, so nothing of it is above Appearance.
        val appearance = rule.onNodeWithTag("appearance-note").fetchSemanticsNode().boundsInRoot
        val heading = rule.onNodeWithTag("development-heading").fetchSemanticsNode().boundsInRoot
        assertTrue(heading.top > appearance.bottom)
        // Neither sentence carries the egui build's sync clause: Android has no sync.
        val reset = rule.onNodeWithTag("reset-sentence").fetchSemanticsNode().config[SemanticsProperties.Text].joinToString { it.text }
        assertTrue(!reset.contains("sync", ignoreCase = true))
    }

    @Test
    fun the_bench_has_one_control_per_fixture_by_key() {
        showApp()
        openSettings()
        for (fixture in fixtures()) {
            rule.onNodeWithTag("fixture-${fixture.key}").performScrollTo().assertTextEquals(fixture.key)
        }
    }

    @Test
    fun reset_asks_first_and_keeping_it_changes_nothing() {
        showApp()
        runBlocking { owner.use { it.installFixture("backlog", System.currentTimeMillis()) } }
        val before = notes()
        openSettings()
        press("reset-control")
        rule.onNodeWithTag("reset-question").assertTextEquals("Reset the collection to a first launch?")
        rule.onNodeWithTag("reset-confirm").assertTextEquals("Reset")
        rule.onNodeWithTag("reset-keep").assertTextEquals("Keep it")
        assertEquals(before, notes())
        press("reset-keep")
        assertTrue(!exists("reset-question"))
        assertTrue(exists("reset-control"))
        assertEquals(before, notes())
        assertTrue(exists("settings-screen"))
    }

    @Test
    fun a_confirmed_reset_leaves_a_first_launch_and_the_app_starts_afresh_on_review() {
        showApp()
        // A sitting is under way, and the collection is not the seed.
        rule.onNodeWithTag("start-all").performClick()
        waitFor("progress")
        runBlocking { owner.use { it.installFixture("backlog", System.currentTimeMillis()) } }
        val seeded = runBlocking {
            val handle = CollectionHandle.open(File(root, "seed/files").absolutePath, File(root, "seed/no_backup").absolutePath)
            handle.use { it.noteCount() }
        }
        assertTrue(notes() != seeded)
        openSettings()
        press("reset-control")
        press("reset-confirm")
        waitFor("start-all")
        assertEquals(seeded, notes())
        assertTrue("no sitting of the old collection survives", !exists("progress"))
        assertTrue("the back stack is Review", selected("destination-review"))
        assertTrue(!exists("settings-screen"))
    }

    @Test
    fun a_fixture_asks_first_and_keeping_it_changes_nothing() {
        showApp()
        val before = notes()
        openSettings()
        press("fixture-backlog")
        rule.onNodeWithTag("fixture-question").assertTextEquals("Replace the collection with backlog?")
        rule.onNodeWithTag("fixture-confirm").assertTextEquals("Replace")
        rule.onNodeWithTag("fixture-keep").assertTextEquals("Keep it")
        assertEquals(before, notes())
        press("fixture-keep")
        assertTrue(!exists("fixture-question"))
        assertEquals(before, notes())
        assertTrue(!exists("bench-result"))
    }

    @Test
    fun a_confirmed_fixture_replaces_the_collection_starts_afresh_and_the_bench_says_what_it_reached() {
        showApp()
        val before = notes()
        openSettings()
        press("fixture-backlog")
        press("fixture-confirm")
        waitFor("start-all")
        assertTrue(notes() != before)
        assertTrue(selected("destination-review"))
        // The bench's answer stays until the next press, and is the install's own account.
        openSettings()
        waitFor("bench-result")
        val said = rule.onNodeWithTag("bench-result").performScrollTo().fetchSemanticsNode()
            .config[SemanticsProperties.Text].joinToString { it.text }
        assertTrue(said, said.contains("cards") && said.contains("due"))
        // A new press clears it.
        press("fixture-caught-up")
        assertTrue(!exists("bench-result"))
    }

    @Test
    fun the_appearance_is_read_again_from_the_new_collection() {
        val appearance = showApp(initial = ThemeChoice.DARK)
        openSettings()
        press("reset-control")
        press("reset-confirm")
        waitFor("start-all")
        rule.waitUntil(10_000) { appearance.choice == ThemeChoice.SYSTEM }
        rule.waitForIdle()
        // The system's own setting decides what the emulator shows, so only the choice is asserted.
        assertEquals(ThemeChoice.SYSTEM, runBlocking { owner.use { it.themeChoice() } })
    }

    @Test
    fun a_refused_install_is_said_by_the_bench_and_the_page_stays() {
        val refusing = object : CollectionAccess {
            @Volatile
            var refuse = false

            override suspend fun <T> use(block: (CollectionHandle) -> T): T {
                if (refuse) throw CairnException.Failed("no space left")
                return owner.use(block)
            }
        }
        runBlocking { owner.use { it.setThemeChoice(ThemeChoice.LIGHT) } }
        rule.setContent { CairnAppearance(AppearanceState(ThemeChoice.LIGHT)) { CairnNavHost(refusing, keyboardUp = false) } }
        waitFor("start-all")
        openSettings()
        rule.waitForIdle()
        val before = notes()
        refusing.refuse = true
        press("fixture-backlog")
        press("fixture-confirm")
        waitFor("bench-result")
        val said = rule.onNodeWithTag("bench-result").performScrollTo().fetchSemanticsNode()
            .config[SemanticsProperties.Text].joinToString { it.text }
        assertTrue(said, said.contains("no space left"))
        assertTrue("nothing was replaced, so nothing starts afresh", exists("settings-screen"))
        assertTrue(!exists("fixture-question"))
        refusing.refuse = false
        assertEquals(before, notes())
    }

    @Test
    fun the_block_is_captured() {
        val dir = context.getExternalFilesDir("settings-shots")!!
        val appearance = showApp(initial = ThemeChoice.LIGHT)
        openSettings()
        for (choice in listOf(ThemeChoice.LIGHT, ThemeChoice.DARK)) {
            appearance.choice = choice
            rule.waitForIdle()
            rule.onNodeWithTag("development-heading").performScrollTo().assertIsDisplayed()
            val bitmap = rule.onRoot().captureToImage().asAndroidBitmap()
            FileOutputStream(File(dir, "development-${choice.name.lowercase()}.png")).use {
                bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it)
            }
            assertEquals(pageArgb(choice == ThemeChoice.DARK), bitmap.getPixel(2, bitmap.height / 2))
        }
    }
}
