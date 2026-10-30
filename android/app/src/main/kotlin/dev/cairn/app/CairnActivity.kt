package dev.cairn.app

import android.os.Bundle
import android.view.ViewTreeObserver
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.Composable
import androidx.lifecycle.lifecycleScope
import dev.cairn.design.CairnTheme
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.ThemeChoice
import kotlinx.coroutines.launch

/** Whether the screen is dark: `Light` and `Dark` are fixed, `System` follows the platform. */
fun isDark(choice: ThemeChoice, systemDark: Boolean): Boolean =
    when (choice) {
        ThemeChoice.SYSTEM -> systemDark
        ThemeChoice.LIGHT -> false
        ThemeChoice.DARK -> true
    }

/**
 * The base of every screen of the app. It reads the stored appearance choice through the binding
 * before anything is drawn: the window's first draw is held back until the choice is known, so a
 * handset set to dark never shows a light frame first (ADR-0036 §3). The choice is only read here;
 * changing it is the Settings slice's.
 */
abstract class CairnActivity : ComponentActivity() {
    private var ready = false

    @Composable
    protected abstract fun Screen()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val content = window.decorView
        content.viewTreeObserver.addOnPreDrawListener(
            object : ViewTreeObserver.OnPreDrawListener {
                override fun onPreDraw(): Boolean {
                    if (!ready) return false
                    content.viewTreeObserver.removeOnPreDrawListener(this)
                    return true
                }
            },
        )
        lifecycleScope.launch {
            // A failed read follows the platform rather than leaving the window blank.
            val choice = try {
                (application as CairnApplication).collection.use { it.themeChoice() }
            } catch (_: CairnException) {
                ThemeChoice.SYSTEM
            }
            setContent {
                CairnTheme(dark = isDark(choice, isSystemInDarkTheme())) { Screen() }
            }
            ready = true
        }
    }
}
