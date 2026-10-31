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
import androidx.compose.ui.test.performScrollTo
import androidx.core.app.ActivityOptionsCompat
import androidx.test.core.app.ActivityScenario
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.util.concurrent.atomic.AtomicInteger

/** *Back up…* through a recreation of the activity (#249): the report survives it, and so does the instant behind the dialog. */
@RunWith(AndroidJUnit4::class)
class BackupRotationTest {
    @get:Rule
    val rule = createEmptyComposeRule()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "backup-rotation-${System.nanoTime()}")
    private val owner = CollectionOwner(context, File(root, "files"), File(root, "no_backup"))

    /** The save dialog, answering with [NAME] at once, or holding the answer back until [answer]. */
    private class Dialog(val holding: Boolean) : ActivityResultRegistryOwner {
        val pending = AtomicInteger(-1)

        /** The intents started through the registry other than the save dialog: Share's chooser. */
        val started: MutableList<Intent> = java.util.concurrent.CopyOnWriteArrayList()

        override val activityResultRegistry = object : ActivityResultRegistry() {
            override fun <I, O> onLaunch(requestCode: Int, contract: ActivityResultContract<I, O>, input: I, options: ActivityOptionsCompat?) {
                if (input is Intent) started += input else if (holding) pending.set(requestCode) else answer(requestCode)
            }
        }

        fun answer(requestCode: Int = pending.get()) {
            activityResultRegistry.dispatchResult(requestCode, Activity.RESULT_OK, Intent().setData(BenchProvider.uri(NAME)))
        }
    }

    @Before
    fun withHistory() {
        BenchProvider.files.clear()
        runBlocking { owner.use { it.installFixture("history", System.currentTimeMillis()) } }
        RotatingActivity.access = owner
    }

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

    private fun backUp() {
        waitFor("destination-settings")
        rule.onNodeWithTag("destination-settings").performClick()
        waitFor("back-up")
        rule.onNodeWithTag("back-up").performScrollTo().performClick()
    }

    @Test
    fun the_report_survives_a_rotation() {
        RotatingActivity.registry = Dialog(holding = false)
        ActivityScenario.launch(RotatingActivity::class.java).use { scenario ->
            backUp()
            waitFor("backup-report")
            val report = text("backup-report")
            scenario.recreate()
            waitFor("settings-screen")
            waitFor("backup-report")
            assertEquals(report, text("backup-report"))
        }
    }

    @Test
    fun share_survives_a_rotation_with_the_report_and_still_hands_on_the_document() {
        val dialog = Dialog(holding = false)
        RotatingActivity.registry = dialog
        ActivityScenario.launch(RotatingActivity::class.java).use { scenario ->
            backUp()
            waitFor("backup-share")
            scenario.recreate()
            waitFor("settings-screen")
            waitFor("backup-share")
            assertTrue("nothing was started by the turn", dialog.started.isEmpty())
            rule.onNodeWithTag("backup-share").performScrollTo().performClick()
            rule.waitUntil(10_000) { dialog.started.isNotEmpty() }
            assertEquals(BenchProvider.uri(NAME), dialog.started.single().clipData!!.getItemAt(0).uri)
        }
    }

    @Test
    fun a_rotation_behind_the_dialog_keeps_the_instant() {
        val dialog = Dialog(holding = true)
        RotatingActivity.registry = dialog
        ActivityScenario.launch(RotatingActivity::class.java).use { scenario ->
            val before = System.currentTimeMillis()
            backUp()
            rule.waitUntil(10_000) { dialog.pending.get() >= 0 }
            val launched = System.currentTimeMillis()
            Thread.sleep(50)
            scenario.recreate()
            waitFor("back-up")
            scenario.onActivity { dialog.answer() }
            waitFor("backup-report")
            rule.waitUntil(10_000) { BenchProvider.files.containsKey(NAME) }
            val created = isoMillis(manifestOf(BenchProvider.files.getValue(NAME)).getString("created"))
            assertTrue("$created in [$before, $launched]", created in before..launched)
        }
    }

    private companion object {
        const val NAME = "Rotated backup.ccoll"
    }
}
