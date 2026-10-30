package dev.cairn.app

import android.content.Context
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createEmptyComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.test.core.app.ActivityScenario
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File

/** An Optimise run through a recreation of the activity (#236, ADR-0051 §4): the model holds the run. */
@RunWith(AndroidJUnit4::class)
class SettingsRotationTest {
    @get:Rule
    val rule = createEmptyComposeRule()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "rotation-${System.nanoTime()}")
    private val owner = CollectionOwner(context, File(root, "files"), File(root, "no_backup"))

    @After
    fun tearDown() {
        RotatingActivity.access = null
        root.deleteRecursively()
    }

    private fun exists(tag: String) = rule.onAllNodes(hasTestTag(tag)).fetchSemanticsNodes().isNotEmpty()

    private fun waitFor(tag: String) = rule.waitUntil(10_000) { exists(tag) }

    private fun text(tag: String): String =
        rule.onNodeWithTag(tag).fetchSemanticsNode().config[SemanticsProperties.Text].joinToString("") { it.text }

    @Test
    fun a_run_survives_a_rotation_and_finishes_into_the_new_screen() {
        runBlocking { owner.use { it.installFixture("history", System.currentTimeMillis()) } }
        RotatingActivity.access = owner
        ActivityScenario.launch(RotatingActivity::class.java).use { scenario ->
            waitFor("destination-settings")
            rule.onNodeWithTag("destination-settings").performClick()
            waitFor("optimise-button")
            rule.onNodeWithTag("optimise-button").performClick()
            waitFor("optimise-cancel")
            scenario.recreate()
            // The new screen is on Settings again and finds the run in the model the old one left.
            waitFor("settings-screen")
            assertTrue("the run is still in flight or has finished into the new screen", exists("optimise-cancel") || exists("optimise-done"))
            waitFor("optimise-done")
            rule.onNodeWithTag("optimise-done").assertTextEquals("Parameters updated. Due dates have been recalculated.")
            rule.waitUntil(10_000) { text("nudge-text") == "Fitted over 225 reviews. You've reviewed 0 times since." }
        }
    }
}
