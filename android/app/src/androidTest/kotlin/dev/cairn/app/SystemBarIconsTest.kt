package dev.cairn.app

import android.content.res.Configuration
import androidx.activity.ComponentActivity
import androidx.test.core.app.ActivityScenario
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.core.view.WindowCompat
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * The system bars' icons follow the theme (ADR-0045 §7): dark icons on the light page, light icons
 * on the dark one, and a switch while the app is open changes them too (#223).
 */
@RunWith(AndroidJUnit4::class)
class SystemBarIconsTest {
    @get:Rule
    val rule = createAndroidComposeRule<ComponentActivity>()

    private fun iconsAreDark(): Boolean {
        val window = rule.activity.window
        val controller = WindowCompat.getInsetsController(window, window.decorView)
        return controller.isAppearanceLightStatusBars && controller.isAppearanceLightNavigationBars
    }

    /** The real activity wires it: with no stored choice it follows the platform's theme. */
    @Test
    fun the_app_activity_applies_the_icons_for_the_platform_theme() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            var systemDark = false
            var lightIcons = false
            // The content is composed only once the stored choice is read, so poll from here.
            val deadline = System.currentTimeMillis() + 30_000
            do {
                scenario.onActivity { activity ->
                    systemDark = activity.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK ==
                        Configuration.UI_MODE_NIGHT_YES
                    lightIcons = WindowCompat.getInsetsController(activity.window, activity.window.decorView)
                        .isAppearanceLightStatusBars
                }
                if (lightIcons == !systemDark) break
                Thread.sleep(100)
            } while (System.currentTimeMillis() < deadline)
            assertEquals(!systemDark, lightIcons)
        }
    }

    @Test
    fun the_icons_are_dark_in_light_and_light_in_dark_and_follow_a_switch() {
        val dark = mutableStateOf(false)
        rule.setContent { SystemBarIcons(dark.value) }
        rule.waitForIdle()
        assertEquals("light theme", true, iconsAreDark())
        dark.value = true
        rule.waitForIdle()
        assertEquals("dark theme", false, iconsAreDark())
        dark.value = false
        rule.waitForIdle()
        assertEquals("light again", true, iconsAreDark())
    }
}
