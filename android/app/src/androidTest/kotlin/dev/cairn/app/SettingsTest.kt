package dev.cairn.app

import android.content.Context
import android.view.KeyEvent
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.hasTestTag
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performImeAction
import androidx.compose.ui.test.performTextClearance
import androidx.compose.ui.test.performTextReplacement
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import dev.cairn.design.CairnTheme
import dev.cairn.design.pageArgb
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.CollectionHandle
import dev.cairn.ffi.ThemeChoice
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
import java.util.concurrent.atomic.AtomicInteger

/**
 * Settings in the destinations row (#234, ADR-0042, ADR-0051): the new-card rate and Appearance,
 * against a real store in the app's cache directory.
 */
@RunWith(AndroidJUnit4::class)
class SettingsTest {
    @get:Rule
    val rule = createAndroidComposeRule<ComponentActivity>()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "settings-${System.nanoTime()}")
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

    private fun storedRate() = runBlocking { owner.use { it.newCardRate() } }

    private fun storedTheme() = runBlocking { owner.use { it.themeChoice() } }

    private fun exists(tag: String) = rule.onAllNodes(hasTestTag(tag)).fetchSemanticsNodes().isNotEmpty()

    private fun waitFor(tag: String) = rule.waitUntil(10_000) { exists(tag) }

    private fun textOf(tag: String): String =
        rule.onNodeWithTag(tag).fetchSemanticsNode().config[SemanticsProperties.EditableText].text

    private fun selected(tag: String): Boolean =
        rule.onNodeWithTag(tag).fetchSemanticsNode().config[SemanticsProperties.Selected]

    private fun showApp(
        access: CollectionAccess = owner,
        initial: ThemeChoice = ThemeChoice.LIGHT,
        settingsTickMs: Long = SETTINGS_TICK_MS,
        // Review shows its start button only when something is due; the history fixture's cards are not.
        ready: String = "start-all",
    ): AppearanceState {
        runBlocking { owner.use { it.setThemeChoice(initial) } }
        val appearance = AppearanceState(initial)
        rule.setContent {
            CairnAppearance(appearance) { CairnNavHost(access, keyboardUp = false, settingsTickMs = settingsTickMs) }
        }
        waitFor(ready)
        return appearance
    }

    private fun openSettings() {
        rule.onNodeWithTag("destination-settings").performClick()
        waitFor("settings-screen")
        waitFor("rate-field")
    }

    private fun leaveTheField() {
        // A tap on the page outside the field: the field gives up focus, as a person's tap elsewhere does.
        rule.onNodeWithTag("settings-heading").performClick()
        rule.waitForIdle()
    }

    private fun typeRate(text: String) {
        rule.onNodeWithTag("rate-field").performTextReplacement(text)
    }

    @Test
    fun settings_is_the_third_destination_and_back_returns_to_review() {
        showApp()
        rule.onNodeWithTag("destination-settings").assertTextEquals("Settings")
        openSettings()
        assertTrue(selected("destination-settings"))
        rule.onNodeWithTag("destination-review").let {
            assertEquals(false, it.fetchSemanticsNode().config[SemanticsProperties.Selected])
        }
        InstrumentationRegistry.getInstrumentation().sendKeyDownUpSync(KeyEvent.KEYCODE_BACK)
        waitFor("start-all")
        assertEquals(true, selected("destination-review"))
    }

    @Test
    fun the_page_has_two_groups_with_their_captions_and_no_files_or_sync_block() {
        showApp()
        openSettings()
        rule.onNodeWithTag("settings-heading").assertTextEquals("Settings")
        rule.onNodeWithTag("group-studying").assertTextEquals("Studying")
        rule.onNodeWithTag("group-studying-caption").assertTextEquals("Follows your collection to every device.")
        rule.onNodeWithTag("group-device").assertTextEquals("This device")
        rule.onNodeWithTag("group-device-caption").assertTextEquals("Stays on this machine.")
        rule.onNodeWithTag("label-New cards a day").assertTextEquals("New cards a day")
        rule.onNodeWithTag("rate-consequence")
            .assertTextEquals("The only limit in the app. Set it to zero to clear a backlog, then turn it back on.")
        rule.onNodeWithTag("appearance-note")
            .assertTextEquals("This machine only — your appearance does not follow the collection to another device.")
        for (absent in listOf("Files", "Open a file", "sync", "Sync")) {
            assertEquals("nothing on the page says '$absent'", 0, rule.onAllNodesWithText(absent, substring = true).fetchSemanticsNodes().size)
        }
    }

    @Test
    fun the_rate_shows_the_stored_value() {
        runBlocking { owner.use { it.setNewCardRate(17u) } }
        showApp()
        openSettings()
        assertEquals("17", textOf("rate-field"))
    }

    @Test
    fun the_rate_commits_on_done_and_the_field_shows_the_value_stored() {
        showApp()
        openSettings()
        typeRate("12")
        rule.onNodeWithTag("rate-field").performImeAction()
        rule.waitUntil(10_000) { storedRate() == 12u }
        assertEquals("12", textOf("rate-field"))
    }

    @Test
    fun the_rate_commits_when_the_field_is_left() {
        showApp()
        openSettings()
        rule.onNodeWithTag("rate-field").performClick()
        typeRate("23")
        leaveTheField()
        rule.waitUntil(10_000) { storedRate() == 23u }
    }

    @Test
    fun a_rate_above_the_limit_shows_the_clamp_back() {
        showApp()
        openSettings()
        typeRate("123456")
        rule.onNodeWithTag("rate-field").performImeAction()
        rule.waitUntil(10_000) { textOf("rate-field") == "9999" }
        assertEquals(9999u, storedRate())
    }

    @Test
    fun a_number_too_long_for_the_binding_is_clamped_like_any_other_above_the_limit() {
        showApp()
        openSettings()
        typeRate("99999999999999999999")
        rule.onNodeWithTag("rate-field").performImeAction()
        rule.waitUntil(10_000) { textOf("rate-field") == "9999" }
        assertEquals(9999u, storedRate())
    }

    @Test
    fun zero_is_kept() {
        showApp()
        openSettings()
        typeRate("0")
        rule.onNodeWithTag("rate-field").performImeAction()
        rule.waitUntil(10_000) { storedRate() == 0u }
        assertEquals("0", textOf("rate-field"))
    }

    @Test
    fun an_unfinished_field_is_left_as_typed_and_nothing_is_written() {
        showApp()
        openSettings()
        rule.onNodeWithTag("rate-field").performTextClearance()
        rule.onNodeWithTag("rate-field").performImeAction()
        rule.waitForIdle()
        assertEquals("", textOf("rate-field"))
        assertEquals(5u, storedRate())
        typeRate("1x")
        leaveTheField()
        assertEquals("1x", textOf("rate-field"))
        assertEquals(5u, storedRate())
    }

    @Test
    fun a_refused_rate_write_is_said_in_its_block_and_the_block_is_read_again() {
        val access = Refusing(owner)
        showApp(access)
        openSettings()
        // The page's first read was call 1 (and later ones may follow); refuse the very next one.
        rule.waitForIdle()
        access.failAt = access.calls.get() + 1
        typeRate("40")
        rule.onNodeWithTag("rate-field").performImeAction()
        waitFor("rate-refused")
        rule.onNodeWithTag("rate-refused").assertTextEquals("The new-card rate could not be saved.")
        rule.waitUntil(10_000) { textOf("rate-field") == "5" }
        assertEquals(5u, storedRate())
        assertTrue(!exists("appearance-refused"))
    }

    @Test
    fun appearance_marks_the_current_choice_and_switches_the_theme_at_once() {
        val appearance = showApp(initial = ThemeChoice.LIGHT)
        openSettings()
        assertEquals(true, selected("appearance-light"))
        assertEquals(false, selected("appearance-dark"))
        val before = rule.onRoot().captureToImage().asAndroidBitmap().getPixel(2, 400)
        assertEquals(pageArgb(false), before)
        rule.onNodeWithTag("appearance-dark").performClick()
        rule.waitUntil(10_000) { appearance.choice == ThemeChoice.DARK }
        rule.waitForIdle()
        assertEquals(true, selected("appearance-dark"))
        assertEquals(pageArgb(true), rule.onRoot().captureToImage().asAndroidBitmap().getPixel(2, 400))
        assertEquals(ThemeChoice.DARK, storedTheme())
    }

    @Test
    fun the_chosen_appearance_is_stored_and_a_fresh_state_reads_it_back() {
        showApp(initial = ThemeChoice.SYSTEM)
        openSettings()
        rule.onNodeWithTag("appearance-light").performClick()
        rule.waitUntil(10_000) { storedTheme() == ThemeChoice.LIGHT }
        // A relaunch reads the stored choice before its first frame.
        val relaunched = AppearanceState(storedTheme())
        assertEquals(ThemeChoice.LIGHT, relaunched.choice)
    }

    @Test
    fun a_refused_appearance_write_is_said_in_its_block_and_the_choice_does_not_change() {
        val access = Refusing(owner)
        val appearance = showApp(access, initial = ThemeChoice.LIGHT)
        openSettings()
        rule.waitForIdle()
        access.failAt = access.calls.get() + 1
        rule.onNodeWithTag("appearance-dark").performClick()
        waitFor("appearance-refused")
        rule.onNodeWithTag("appearance-refused").assertTextEquals("The appearance could not be saved.")
        assertEquals(ThemeChoice.LIGHT, appearance.choice)
        assertTrue(!exists("rate-refused"))
    }

    @Test
    fun the_page_is_captured_in_both_themes() {
        val dir = context.getExternalFilesDir("settings-shots")!!
        val appearance = showApp(initial = ThemeChoice.LIGHT)
        openSettings()
        val pages = mutableListOf<Int>()
        for (choice in listOf(ThemeChoice.LIGHT, ThemeChoice.DARK)) {
            appearance.choice = choice
            rule.waitForIdle()
            rule.onNodeWithTag("settings-screen").assertIsDisplayed()
            val bitmap = rule.onRoot().captureToImage().asAndroidBitmap()
            FileOutputStream(File(dir, "settings-${choice.name.lowercase()}.png")).use {
                bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it)
            }
            assertEquals(pageArgb(choice == ThemeChoice.DARK), bitmap.getPixel(2, bitmap.height / 2))
            pages += bitmap.getPixel(2, 2)
        }
        assertNotEquals(pages[0], pages[1])
    }

    // --- Optimise (#236, ADR-0014): over the fixture with plenty of history, so a run fits a new vector.

    private val nudgeAtRest = "Using the standard parameters. You've reviewed 225 times."
    private val nudgeAfterRun = "Fitted over 225 reviews. You've reviewed 0 times since."
    private val updated = "Parameters updated. Due dates have been recalculated."

    private fun withHistory() {
        runBlocking { owner.use { it.installFixture("history", System.currentTimeMillis()) } }
    }

    private fun text(tag: String): String =
        rule.onNodeWithTag(tag).fetchSemanticsNode().config[SemanticsProperties.Text].joinToString("") { it.text }

    @Test
    fun optimise_sits_under_the_scheduler_label_with_the_nudge_beneath() {
        withHistory()
        showApp(ready = "destination-settings")
        openSettings()
        waitFor("nudge-text")
        rule.onNodeWithTag("label-Scheduler").assertTextEquals("Scheduler")
        rule.onNodeWithTag("optimise-button").assertTextEquals("Optimise")
        assertEquals(nudgeAtRest, text("nudge-text"))
        assertTrue(!exists("optimise-done") && !exists("optimise-cancel"))
    }

    @Test
    fun the_nudge_of_a_collection_with_one_review_is_singular_and_a_big_one_is_grouped() {
        // 225 is the history fixture; the singular and the grouping are the egui words' own cases.
        val single = SettingsScreenState.Ready("5", ThemeChoice.LIGHT, nudge = Nudge.Standard(1))
        val fitted = SettingsScreenState.Ready("5", ThemeChoice.LIGHT, nudge = Nudge.Fitted(1, 1))
        val big = SettingsScreenState.Ready("5", ThemeChoice.LIGHT, nudge = Nudge.Fitted(3120, 1234567))
        val said = mutableListOf<String>()
        var shown by mutableStateOf(single)
        rule.setContent { CairnTheme(dark = false) { SettingsPage(shown, {}, {}, {}, {}, {}) } }
        for (state in listOf(single, fitted, big)) {
            shown = state
            rule.waitForIdle()
            said += text("nudge-text")
        }
        assertEquals(
            listOf(
                "Using the standard parameters. You've reviewed 1 time.",
                "Fitted over 1 review. You've reviewed 1 time since.",
                "Fitted over 3,120 reviews. You've reviewed 1,234,567 times since.",
            ),
            said,
        )
    }

    @Test
    fun optimise_then_cancel_returns_to_rest_and_writes_nothing() {
        withHistory()
        // No tick lands before Cancel is pressed, so the run is in flight when it is.
        showApp(settingsTickMs = 60_000, ready = "destination-settings")
        openSettings()
        waitFor("optimise-button")
        rule.onNodeWithTag("optimise-button").performClick()
        waitFor("optimise-cancel")
        rule.onNodeWithTag("optimise-cancel").assertTextEquals("Cancel")
        assertTrue("the button is replaced while the run is in flight", !exists("optimise-button"))
        assertTrue(exists("optimise-preparing") || exists("optimise-progress"))
        rule.onNodeWithTag("optimise-cancel").performClick()
        waitFor("optimise-button")
        assertEquals(nudgeAtRest, text("nudge-text"))
        assertTrue(!exists("optimise-done"))
    }

    @Test
    fun optimise_runs_to_the_completion_sentence_and_the_nudge_reads_fitted() {
        withHistory()
        showApp(ready = "destination-settings")
        openSettings()
        waitFor("optimise-button")
        rule.onNodeWithTag("optimise-button").performClick()
        waitFor("optimise-cancel")
        waitFor("optimise-done")
        rule.onNodeWithTag("optimise-done").assertTextEquals(updated)
        waitFor("optimise-button")
        rule.waitUntil(10_000) { text("nudge-text") == nudgeAfterRun }
        // The sentence stays until the next run starts.
        rule.onNodeWithTag("optimise-button").performClick()
        rule.waitUntil(10_000) { !exists("optimise-done") }
    }

    @Test
    fun settings_mid_run_and_after_a_run_are_captured_in_both_themes() {
        val dir = context.getExternalFilesDir("settings-shots")!!
        val states = mapOf(
            "mid-run" to SettingsScreenState.Ready("5", ThemeChoice.LIGHT, nudge = Nudge.Standard(225), run = OptimiseRun.Training(40, 100)),
            "preparing" to SettingsScreenState.Ready("5", ThemeChoice.LIGHT, nudge = Nudge.Standard(225), run = OptimiseRun.Preparing),
            "after-run" to SettingsScreenState.Ready("5", ThemeChoice.LIGHT, nudge = Nudge.Fitted(225, 0), updated = true),
        )
        var dark by mutableStateOf(false)
        var shown by mutableStateOf(states.getValue("mid-run"))
        rule.setContent { CairnTheme(dark) { SettingsPage(shown, {}, {}, {}, {}, {}) } }
        for ((name, state) in states) {
            for (isDark in listOf(false, true)) {
                dark = isDark
                shown = state
                rule.waitForIdle()
                rule.onNodeWithTag("settings-screen").assertIsDisplayed()
                val bitmap = rule.onRoot().captureToImage().asAndroidBitmap()
                FileOutputStream(File(dir, "optimise-$name-${if (isDark) "dark" else "light"}.png")).use {
                    bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it)
                }
                assertEquals(pageArgb(isDark), bitmap.getPixel(2, bitmap.height / 2))
            }
        }
        // What each state shows: the bar and Cancel mid-run; the sentence under the nudge after.
        shown = states.getValue("mid-run")
        rule.waitForIdle()
        assertTrue(exists("optimise-progress") && exists("optimise-cancel") && !exists("optimise-button"))
        shown = states.getValue("after-run")
        rule.waitForIdle()
        assertEquals(updated, text("optimise-done"))
    }

    @Test
    fun two_quick_requests_to_optimise_start_one_run() {
        withHistory()
        val started = AtomicInteger()
        val counting = object : CollectionAccess {
            override suspend fun <T> use(block: (CollectionHandle) -> T): T =
                owner.use(block).also { if (it is dev.cairn.ffi.OptimiseHandle) started.incrementAndGet() }
        }
        val model = SettingsViewModel(counting, AppearanceState(ThemeChoice.LIGHT))
        rule.runOnUiThread {
            model.optimise()
            model.optimise()
        }
        rule.waitUntil(10_000) { (model.state.value as? SettingsScreenState.Ready)?.run != OptimiseRun.Idle }
        runBlocking { model.cancelOptimise(); kotlinx.coroutines.delay(300) }
        assertEquals(1, started.get())
    }
}
