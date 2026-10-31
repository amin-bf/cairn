package dev.cairn.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.LocalActivityResultRegistryOwner
import androidx.activity.compose.setContent
import androidx.activity.result.ActivityResultRegistryOwner
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import dev.cairn.ffi.ThemeChoice

/**
 * The app's navigation over a collection a test hands it, with its content set in onCreate as the real
 * activity's is, so that a recreation restores the back stack and keeps the models (SettingsRotationTest).
 */
class RotatingActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val appearance = AppearanceState(ThemeChoice.LIGHT)
        setContent {
            val content = @Composable { CairnAppearance(appearance) { CairnNavHost(access!!, keyboardUp = false) } }
            // A test may stand in for the system's document picker (ImportRotationTest).
            val stub = registry
            if (stub == null) content() else CompositionLocalProvider(LocalActivityResultRegistryOwner provides stub) { content() }
        }
    }

    companion object {
        @Volatile
        var access: CollectionAccess? = null

        @Volatile
        var registry: ActivityResultRegistryOwner? = null
    }
}
