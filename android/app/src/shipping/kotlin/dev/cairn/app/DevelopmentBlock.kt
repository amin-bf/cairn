package dev.cairn.app

import androidx.compose.runtime.Composable

/**
 * The Development block in a build made without `CAIRN_DEVELOPMENT=1`: nothing. The real block, its
 * model and its strings are a source set of the same shape that is added only to a development build
 * (ADR-0051 §1), so none of it is in this APK, hidden or otherwise.
 */
@Composable
@Suppress("UNUSED_PARAMETER")
fun DevelopmentBlock(access: CollectionAccess, onRestart: () -> Unit) {}
