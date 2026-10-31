package dev.cairn.design

import androidx.compose.runtime.Composable
import androidx.compose.foundation.layout.Column
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onChildren
import androidx.compose.ui.test.onFirst
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.text.TextLayoutResult
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.cairn.design.tokens.DarkPalette
import dev.cairn.design.tokens.Faces
import dev.cairn.design.tokens.LightPalette
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class DesignTest {
    @get:Rule
    val rule = createComposeRule()

    private val faces = FaceSet(ApplicationProvider.getApplicationContext())

    // Places in the family's list: Ubuntu / Hack, then Noto Arabic, DejaVu, Cairn Icons.
    private fun owner(text: String, family: Family = Family.Proportional) = faces.owners(text, family).single()

    @Test
    fun every_face_the_lists_name_is_bundled_and_loads() {
        for (family in Family.entries) {
            assertEquals(
                when (family) {
                    Family.Proportional -> Faces.PROPORTIONAL
                    Family.Monospace -> Faces.MONOSPACE
                    Family.Bold -> Faces.BOLD
                }.split(',').size,
                faces.faces(family).size,
            )
        }
    }

    @Test
    fun a_persian_letter_reaches_noto_arabic_before_dejavus_and_latin_stays_in_the_body_face() {
        assertEquals("French reaches the body face", 0, owner("é"))
        assertEquals("a Persian-only letter reaches Noto Sans Arabic", 1, owner("گ"))
        assertEquals("an IPA symbol reaches DejaVu", 2, owner("ɸ"))
        assertEquals("the mark reaches the icon face, last", 3, owner(""))
    }

    @Test
    fun the_monospace_family_has_the_same_fallbacks_and_bold_reaches_its_own_cuts() {
        assertEquals(0, owner("e", Family.Monospace))
        assertEquals(2, owner("گ", Family.Monospace))
        assertEquals(4, owner("", Family.Monospace))
        assertEquals("bold Persian is Noto Arabic Bold", 0, owner("گ", Family.Bold))
        assertEquals("bold Latin is DejaVu Bold", 1, owner("a", Family.Bold))
        assertEquals("the mark has no bold cut and reaches the regular", 2, owner("", Family.Bold))
    }

    @Test
    fun every_letter_of_mi_ravam_is_drawn_by_noto_arabic_whatever_the_joiner_between_them_does() {
        val text = "می‌روم"
        val owners = faces.owners(text, Family.Proportional)
        text.forEachIndexed { i, c -> if (c != '‌') assertEquals("letter $i", 1, owners[i]) }
    }

    @Test
    fun a_mixed_line_is_split_into_runs_by_face() {
        val spans = faces.resolve("chat گربه", Family.Proportional).spanStyles
        assertTrue("French and Persian are drawn in different faces", spans.size >= 2)
    }

    @Composable
    private fun Probe(onColors: (CairnColors) -> Unit) {
        onColors(Cairn.colors)
    }

    private fun colorsOf(dark: Boolean): CairnColors {
        var seen: CairnColors? = null
        rule.setContent { CairnTheme(dark) { Probe { seen = it } } }
        rule.waitForIdle()
        return seen!!
    }

    @Test
    fun the_theme_carries_each_palettes_roles() {
        val dark = colorsOf(dark = true)
        assertEquals(Color(DarkPalette.PAGE), dark.page)
        assertEquals(Color(DarkPalette.CARD), dark.card)
        assertTrue(dark.dark)
    }

    @Test
    fun the_light_theme_carries_the_light_roles() {
        val light = colorsOf(dark = false)
        assertEquals(Color(LightPalette.PAGE), light.page)
        assertEquals(Color(LightPalette.BODY), light.body)
        assertTrue(!light.dark)
    }

    /** The colour a text node is laid out in, read from its own layout. */
    private fun inkOf(tag: String): Color {
        val layouts = mutableListOf<TextLayoutResult>()
        rule.onNode(hasTestTag(tag), useUnmergedTree = true).onChildren().onFirst()
            .performSemanticsAction(SemanticsActions.GetTextLayoutResult) { it(layouts) }
        return layouts.single().layoutInput.style.color
    }

    @Test
    fun a_disabled_text_action_cannot_be_pressed_and_speaks_in_the_weak_ink() {
        var colors: CairnColors? = null
        var presses = 0
        rule.setContent {
            CairnTheme(false) {
                Probe { colors = it }
                Column {
                    CairnTextAction("Cancel", { presses++ }, Modifier.testTag("on"))
                    CairnTextAction("Cancel", { presses++ }, Modifier.testTag("off"), enabled = false)
                }
            }
        }
        rule.onNodeWithTag("on").assertIsEnabled()
        rule.onNodeWithTag("off").assertIsNotEnabled().performClick()
        assertEquals(0, presses)
        assertEquals(colors!!.link, inkOf("on"))
        assertEquals(colors!!.weak, inkOf("off"))
    }

    private fun specimenShowsEverything(dark: Boolean) {
        rule.setContent { CairnTheme(dark) { Specimen() } }
        for (tag in listOf("heading", "french", "persian", "primary", "ordinary", "mark")) {
            rule.onNodeWithTag("specimen-$tag").assertIsDisplayed()
        }
    }

    @Test
    fun the_dark_specimen_shows_heading_french_persian_both_controls_and_the_mark() = specimenShowsEverything(true)

    @Test
    fun the_light_specimen_shows_heading_french_persian_both_controls_and_the_mark() = specimenShowsEverything(false)
}
