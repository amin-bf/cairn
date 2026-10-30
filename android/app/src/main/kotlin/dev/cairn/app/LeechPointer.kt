package dev.cairn.app

import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import dev.cairn.design.CairnControl
import dev.cairn.design.CairnPage
import dev.cairn.design.CairnText
import dev.cairn.design.ReachColumn
import dev.cairn.design.Tier
import dev.cairn.design.gap

/**
 * The end-of-session pointer (ADR-0010 §6): shown in the entrance's place when a sitting ended with
 * [crossed] cards over the leech floor. Both answers are primary and full width; what each does is the
 * caller's.
 */
@Composable
fun LeechPointer(crossed: Int, onShow: () -> Unit, onDismiss: () -> Unit, modifier: Modifier = Modifier) {
    CairnPage(modifier) {
        BoxWithConstraints {
            val room = maxHeight
            Column(Modifier.fillMaxWidth().verticalScroll(rememberScrollState())) {
                ReachColumn(
                    room = room,
                    floor = gap(3),
                    top = {
                        Column(Modifier.padding(top = gap(2))) {
                            CairnText(stringResource(R.string.review_heading), tier = Tier.Heading)
                            Spacer(Modifier.height(gap(2)))
                            CairnText(
                                pluralStringResource(R.plurals.leeches_pointer, crossed, crossed),
                                Modifier.testTag("leech-pointer"),
                            )
                        }
                    },
                    reach = {
                        Column {
                            CairnControl(
                                stringResource(R.string.leeches_pointer_show),
                                onClick = onShow,
                                primary = true,
                                modifier = Modifier.fillMaxWidth().testTag("pointer-show"),
                            )
                            Spacer(Modifier.height(gap(2)))
                            CairnControl(
                                stringResource(R.string.leeches_pointer_dismiss),
                                onClick = onDismiss,
                                primary = true,
                                modifier = Modifier.fillMaxWidth().testTag("pointer-dismiss"),
                            )
                        }
                    },
                )
            }
        }
    }
}
