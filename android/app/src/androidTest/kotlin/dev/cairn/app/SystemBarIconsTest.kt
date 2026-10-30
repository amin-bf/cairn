package dev.cairn.app

import androidx.activity.ComponentActivity
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
