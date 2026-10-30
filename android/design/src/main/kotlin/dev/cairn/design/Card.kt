package dev.cairn.design

import androidx.compose.animation.core.Easing
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.wrapContentHeight
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.SemanticsPropertyKey
import androidx.compose.ui.semantics.SemanticsPropertyReceiver
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextMeasurer
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.cairn.design.tokens.Motion
import dev.cairn.design.tokens.Surface
import dev.cairn.design.tokens.Typography

/** A stretch of a card face in one style. Code wins over bold; italic composes with either. */
data class CardRun(val text: String, val bold: Boolean = false, val italic: Boolean = false, val code: Boolean = false)

/** The type tiers a card face may take, largest first. The last is the floor (ADR-0033 §4). */
private val CardTiers = floatArrayOf(Typography.DISPLAY, Typography.HEADING, Typography.BODY)

/** The type size the card chose, in sp, for a test to read; nothing in the app reads it. */
val CardTierKey = SemanticsPropertyKey<Float>("CairnCardTier")
private var SemanticsPropertyReceiver.cardTier by CardTierKey

private val RevealEasing = Easing { Motion.easing(it) }

/** The runs as one string, every glyph in the face that has it and each run in its own style. */
internal fun FaceSet.annotate(runs: List<CardRun>): AnnotatedString = buildAnnotatedString {
    for (run in runs) {
        val family = when {
            run.code -> Family.Monospace
            run.bold -> Family.Bold
            else -> Family.Proportional
        }
        val start = length
        append(resolve(run.text, family))
        if (run.italic) addStyle(SpanStyle(fontStyle = FontStyle.Italic), start, length)
    }
}

private fun faceStyle(size: Float, color: Color) =
    TextStyle(color = color, fontSize = size.sp, textDirection = TextDirection.Content)

/** How tall [text] lays out at [size] in [width] pixels. */
private fun TextMeasurer.heightAt(text: AnnotatedString, size: Float, width: Int, density: Density): Float =
    measure(text, faceStyle(size, Color.Unspecified), constraints = Constraints(maxWidth = width), density = density)
        .size.height.toFloat()

/**
 * The card (ADR-0033): one surface with two faces, the whole of it the only target and the tap on it
 * the only cause of the reveal (ADR-0006 §3). [onReveal] is told of every tap, and the caller decides
 * what it does.
 *
 * **The type tier is chosen once, against the whole card** — prompt and answer both — so it cannot step
 * down at the reveal, and **the card's rect never changes**: it is as tall for the question as for the
 * answer. The reveal is the answer half *opening* over [Motion.DURATION], cubic-out (ADR-0037 §2–§4);
 * nothing slides, and the answer is not composed at all until it begins to open.
 *
 * The box [badge] sits in the corner opposite the one the prompt begins at — top-left for a
 * right-to-left prompt ([rtl]), top-right otherwise — and the prompt's direction holds for the card's
 * life, so it never changes sides (ADR-0033 §5, ADR-0030 §4). It arrives with the answer.
 *
 * A new card is a new composition (key it by the card): the animation starts where it should and the
 * next card appears without a fade.
 *
 * [minHeight] is the card's floor: the review card's by default; a list of cards passes
 * [Surface.FIT], so each is only as tall as its content (the type tier is still chosen against the
 * review card's budget).
 */
@Composable
fun CairnCard(
    prompt: List<CardRun>,
    answer: List<CardRun>,
    badge: String,
    rtl: Boolean,
    revealed: Boolean,
    onReveal: () -> Unit,
    modifier: Modifier = Modifier,
    minHeight: Float = Surface.REVIEW_HEIGHT,
) {
    val colors = Cairn.colors
    val faces = LocalFaces.current
    val density = LocalDensity.current
    val measurer = rememberTextMeasurer()
    val promptText = remember(prompt, faces) { faces.annotate(prompt) }
    val answerText = remember(answer, faces) { faces.annotate(answer) }
    val badgeText = remember(badge, faces) { faces.resolve(badge, Family.Proportional) }
    val t by animateFloatAsState(
        if (revealed) 1f else 0f,
        tween((Motion.DURATION * 1000).toInt(), easing = RevealEasing),
        label = "reveal",
    )
    val pad = gap(2)
    val shape = RoundedCornerShape(Surface.RADIUS.dp)

    // The card is laid out left to right whatever the locale: the badge's corner is a physical one.
    CompositionLocalProvider(LocalLayoutDirection provides LayoutDirection.Ltr) {
        BoxWithConstraints(modifier.fillMaxWidth()) {
            val innerWidth = with(density) { (maxWidth - pad * 2).roundToPx() }.coerceAtLeast(1)
            val budget = with(density) { (Surface.REVIEW_HEIGHT.dp - pad * 2).toPx() }
            val floor = with(density) { (minHeight.dp - pad * 2).toPx() }
            val badgeLine = with(density) { (Typography.SMALL * 1.4f).sp.toPx() }
            val faceGap = with(density) { gap(3).toPx() }
            val answerBlock = faceGap * 2 + with(density) { 1.dp.toPx() }

            // Chosen against the whole card, so it is the same before and after the reveal.
            val plan = remember(promptText, answerText, innerWidth, density.density, density.fontScale) {
                var size = CardTiers[0]
                var promptHeight = 0f
                var content = 0f
                var answerHeight = 0f
                for ((i, tier) in CardTiers.withIndex()) {
                    size = tier
                    promptHeight = measurer.heightAt(promptText, tier, innerWidth, density)
                    answerHeight = measurer.heightAt(answerText, tier, innerWidth, density)
                    content = promptHeight + answerBlock + answerHeight + badgeLine
                    if (content <= budget || i == CardTiers.lastIndex) break
                }
                CardPlan(size, promptHeight, answerHeight, content)
            }
            val inner = maxOf(plan.content, floor)
            val shut = plan.prompt
            // Shut, the prompt is centred as though it were alone; open, it sits above the answer.
            val closed = ((inner - shut) / 2 - badgeLine).coerceAtLeast(0f)
            val open = (inner - plan.content) / 2
            val lead = closed + (open - closed) * t
            val opened = answerBlock * t + plan.answer * t

            Box(
                Modifier
                    .fillMaxWidth()
                    .clip(shape)
                    .background(colors.card)
                    .border(BorderStroke(1.dp, colors.separator), shape)
                    .testTag("card")
                    .semantics { cardTier = plan.size }
                    .clickable(onClick = onReveal)
                    .padding(pad),
            ) {
                Column(Modifier.fillMaxWidth().height(with(density) { inner.toDp() })) {
                    Row(
                        Modifier.fillMaxWidth().height(with(density) { badgeLine.toDp() }).alpha(t),
                        horizontalArrangement = if (rtl) Arrangement.Start else Arrangement.End,
                    ) {
                        if (t > 0f) {
                            BasicText(badgeText, Modifier.testTag("badge"), style = faceStyle(Typography.SMALL, colors.weak))
                        }
                    }
                    Spacer(Modifier.height(with(density) { lead.toDp() }))
                    Face(promptText, plan.size, colors.body, Modifier.testTag("prompt"))
                    if (t > 0f) {
                        Box(
                            Modifier
                                .fillMaxWidth()
                                .height(with(density) { opened.toDp() })
                                .clipToBounds(),
                        ) {
                            // Drawn at its final position under a clip that grows: an opening, not a slide.
                            Column(Modifier.fillMaxWidth().wrapContentHeight(align = Alignment.Top, unbounded = true)) {
                                Spacer(Modifier.height(gap(3)))
                                Box(Modifier.fillMaxWidth().height(1.dp), contentAlignment = Alignment.Center) {
                                    Box(Modifier.fillMaxWidth(0.25f).height(1.dp).background(colors.cardDivider))
                                }
                                Spacer(Modifier.height(gap(3)))
                                Face(answerText, plan.size, colors.body, Modifier.testTag("answer"))
                            }
                        }
                    }
                }
            }
        }
    }
}

private class CardPlan(val size: Float, val prompt: Float, val answer: Float, val content: Float)

/**
 * One face: centred when it fits a line, the full column when it wraps (so the text lands at its own
 * start, which a centred paragraph would not).
 */
@Composable
private fun Face(text: AnnotatedString, size: Float, color: Color, modifier: Modifier) {
    Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.TopCenter) {
        BasicText(text, modifier, style = faceStyle(size, color))
    }
}
