package dev.cairn.app

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class PlaceholderTest {
    @get:Rule
    val rule = createAndroidComposeRule<MainActivity>()

    @Test
    fun the_screen_shows_the_due_count_read_through_the_binding_and_no_call_ran_on_the_main_thread() {
        rule.waitUntil(timeoutMillis = 10_000) {
            rule.onAllNodes(androidx.compose.ui.test.hasTestTag("due-count")).fetchSemanticsNodes().isNotEmpty()
        }
        rule.onNodeWithTag("due-count").assertIsDisplayed()

        val app = ApplicationProvider.getApplicationContext<CairnApplication>()
        assertEquals(
            "StrictMode saw work of ours on the main thread:\n${app.strictModeViolations.joinToString("\n---\n")}",
            emptyList<String>(),
            app.strictModeViolations.toList(),
        )
    }
}
