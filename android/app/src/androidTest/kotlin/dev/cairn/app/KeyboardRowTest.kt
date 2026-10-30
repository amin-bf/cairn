package dev.cairn.app

import android.os.ParcelFileDescriptor
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowCompat
import androidx.core.view.ViewCompat
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertTrue
import org.junit.After
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * The destinations row against the real soft keyboard (ADR-0048 §1): the activity, the real input
 * method and the real insets — no `keyboardUp` parameter anywhere. Focusing the search field raises
 * the keyboard and the row goes; hiding the keyboard brings the row back.
 */
@RunWith(AndroidJUnit4::class)
class KeyboardRowTest {
    @get:Rule
    val rule = createAndroidComposeRule<MainActivity>()

    private val automation = InstrumentationRegistry.getInstrumentation().uiAutomation
    private var hardKeyboardSetting = "null"

    /** Runs a shell command and reads it to the end, so it has taken effect when this returns. */
    private fun shell(command: String): String =
        ParcelFileDescriptor.AutoCloseInputStream(automation.executeShellCommand(command)).use { it.readBytes().decodeToString().trim() }

    @Before
    fun letTheKeyboardShowOnAnEmulatorWithAHardwareKeyboard() {
        // An emulator reports a hardware keyboard, and the platform then keeps the soft one down.
        hardKeyboardSetting = shell("settings get secure show_ime_with_hard_keyboard")
        shell("settings put secure show_ime_with_hard_keyboard 1")
        val app = rule.activity.application as CairnApplication
        runBlocking { app.collection.use { it.installFixture("decks", System.currentTimeMillis()) } }
    }

    @After
    fun putTheKeyboardSettingBack() {
        if (hardKeyboardSetting == "null") shell("settings delete secure show_ime_with_hard_keyboard")
        else shell("settings put secure show_ime_with_hard_keyboard $hardKeyboardSetting")
    }

    private fun waitFor(tag: String) =
        rule.waitUntil(30_000) { rule.onAllNodes(hasTestTag(tag)).fetchSemanticsNodes().isNotEmpty() }

    private fun waitGone(tag: String) =
        rule.waitUntil(30_000) { rule.onAllNodes(hasTestTag(tag)).fetchSemanticsNodes().isEmpty() }

    private fun keyboardShown(): Boolean {
        var shown = false
        rule.runOnUiThread {
            shown = ViewCompat.getRootWindowInsets(rule.activity.window.decorView)
                ?.isVisible(WindowInsetsCompat.Type.ime()) == true
        }
        return shown
    }

    @Test
    fun the_row_leaves_with_the_real_keyboard_and_returns_when_it_goes() {
        waitFor("destinations")
        rule.onNodeWithTag("destination-notes").performClick()
        waitFor("notes-search")
        // The row is there while the keyboard is down.
        assertTrue("the keyboard starts down", !keyboardShown())
        rule.onNodeWithTag("destinations").assertExists()

        rule.onNodeWithTag("notes-search").performClick()
        rule.waitUntil(30_000) { keyboardShown() }
        waitGone("destinations")

        rule.runOnUiThread {
            WindowCompat.getInsetsController(rule.activity.window, rule.activity.window.decorView)
                .hide(WindowInsetsCompat.Type.ime())
        }
        rule.waitUntil(30_000) { !keyboardShown() }
        waitFor("destinations")
        rule.onNodeWithTag("destination-notes").assertExists()
    }
}
