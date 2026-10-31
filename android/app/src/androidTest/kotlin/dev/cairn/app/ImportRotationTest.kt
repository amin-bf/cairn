package dev.cairn.app

import android.app.Activity
import android.content.Context
import android.content.Intent
import androidx.activity.result.ActivityResultRegistry
import androidx.activity.result.ActivityResultRegistryOwner
import androidx.activity.result.contract.ActivityResultContract
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createEmptyComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.core.app.ActivityOptionsCompat
import androidx.test.core.app.ActivityScenario
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.cairn.ffi.benchFiles
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File

/** The preview through a recreation of the activity (#242): the model holds the file, so the plan survives a rotation. */
@RunWith(AndroidJUnit4::class)
class ImportRotationTest {
    @get:Rule
    val rule = createEmptyComposeRule()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "import-rotation-${System.nanoTime()}")
    private val owner = CollectionOwner(context, File(root, "files"), File(root, "no_backup"))

    @After
    fun tearDown() {
        RotatingActivity.access = null
        RotatingActivity.registry = null
        BenchProvider.files.clear()
        root.deleteRecursively()
    }

    private fun exists(tag: String) = rule.onAllNodes(hasTestTag(tag)).fetchSemanticsNodes().isNotEmpty()

    private fun waitFor(tag: String) = rule.waitUntil(10_000) { exists(tag) }

    private fun text(tag: String): String =
        rule.onNodeWithTag(tag).fetchSemanticsNode().config[SemanticsProperties.Text].joinToString("") { it.text }

    @Test
    fun the_preview_survives_a_rotation_and_a_second_one_goes_back_to_settings() {
        val update = benchFiles()[0]
        BenchProvider.files[update.name] = update.bytes
        runBlocking { owner.use { it.installFixture("decks", System.currentTimeMillis()) } }
        RotatingActivity.access = owner
        RotatingActivity.registry = object : ActivityResultRegistryOwner {
            override val activityResultRegistry = object : ActivityResultRegistry() {
                override fun <I, O> onLaunch(requestCode: Int, contract: ActivityResultContract<I, O>, input: I, options: ActivityOptionsCompat?) {
                    dispatchResult(requestCode, Activity.RESULT_OK, Intent().setData(BenchProvider.uri(update.name)))
                }
            }
        }
        ActivityScenario.launch(RotatingActivity::class.java).use { scenario ->
            waitFor("destination-settings")
            rule.onNodeWithTag("destination-settings").performClick()
            waitFor("open-file")
            rule.onNodeWithTag("open-file").performClick()
            waitFor("import-screen")
            assertEquals("by Marjan Rahimi · CC BY-SA 4.0", text("import-claim-0"))
            scenario.recreate()
            waitFor("import-screen")
            waitFor("import-deck-0")
            assertEquals(update.name, text("import-heading"))
            assertEquals("French A1", text("import-deck-0"))
            assertEquals(true, exists("gate-import"))
        }
    }
}
