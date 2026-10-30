package dev.cairn.app

import android.content.Context
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.cairn.ffi.ThemeChoice
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File

/**
 * The other side of the variable (#237, ADR-0051 §1): a build made without `CAIRN_DEVELOPMENT=1`
 * draws no development control and does not hold the block's code at all. Compiled only without it.
 */
@RunWith(AndroidJUnit4::class)
class NoDevelopmentBlockTest {
    @get:Rule
    val rule = createComposeRule()

    private val context: Context = ApplicationProvider.getApplicationContext()
    private val root = File(context.cacheDir, "shipping-${System.nanoTime()}")

    @After
    fun tearDown() {
        root.deleteRecursively()
    }

    @Test
    fun no_development_control_is_drawn_on_settings() {
        val owner = CollectionOwner(context, File(root, "files"), File(root, "no_backup"))
        rule.setContent { CairnAppearance(AppearanceState(ThemeChoice.LIGHT)) { CairnNavHost(owner, keyboardUp = false) } }
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("start-all")).fetchSemanticsNodes().isNotEmpty() }
        rule.onNodeWithTag("destination-settings").performClick()
        rule.waitUntil(10_000) { rule.onAllNodes(hasTestTag("rate-field")).fetchSemanticsNodes().isNotEmpty() }
        for (tag in listOf("development-heading", "reset-control", "bench-sentence")) {
            assertEquals(tag, 0, rule.onAllNodes(hasTestTag(tag)).fetchSemanticsNodes().size)
        }
        for (words in listOf("Development", "Reset the collection", "fixture")) {
            assertEquals(words, 0, rule.onAllNodesWithText(words, substring = true, ignoreCase = true).fetchSemanticsNodes().size)
        }
    }

    @Test
    fun the_blocks_class_is_not_in_the_apk() {
        for (name in listOf("dev.cairn.app.DevelopmentViewModel", "dev.cairn.app.DevelopmentBench")) {
            try {
                Class.forName(name)
                fail("$name must not be in a build made without the variable")
            } catch (_: ClassNotFoundException) {
                assertTrue(true)
            }
        }
    }
}
