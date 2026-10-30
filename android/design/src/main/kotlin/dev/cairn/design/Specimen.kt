package dev.cairn.design

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import dev.cairn.design.tokens.Frame

/**
 * The specimen: a heading, French and Persian body text, a primary and an ordinary control, and the
 * mark. It is how the faces, the type scale and the palette are looked at on a handset, and what the
 * design module's instrumented test draws.
 */
@Composable
fun Specimen(modifier: Modifier = Modifier) {
    val colors = Cairn.colors
    Column(
        modifier
            .fillMaxSize()
            .background(colors.page)
            .verticalScroll(rememberScrollState())
            .padding(Frame.PAGE_MARGIN.dp),
    ) {
        Column(
            Modifier.widthIn(max = Frame.MEASURE.dp).fillMaxWidth(),
            verticalArrangement = Arrangement.spacedBy(gap(1)),
        ) {
            CairnText("Cairn", Modifier.testTag("specimen-heading"), tier = Tier.Heading)
            CairnText("Le chien est dans la maison, et le chat dort sur le livre.", Modifier.testTag("specimen-french"))
            CairnText("سگ در خانه است و گربه روی کتاب می‌خوابد.", Modifier.fillMaxWidth().testTag("specimen-persian"))
            Row(horizontalArrangement = Arrangement.spacedBy(gap(1))) {
                CairnControl("Continue", onClick = {}, primary = true, modifier = Modifier.testTag("specimen-primary"))
                CairnControl("Cancel", onClick = {}, modifier = Modifier.testTag("specimen-ordinary"))
            }
            Mark(Modifier.testTag("specimen-mark"))
        }
    }
}
