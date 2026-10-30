package dev.cairn.app

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.ThemeChoice
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

/** What Settings shows: the rate field as typed, the stored appearance, and the block that last refused a write. */
sealed interface SettingsScreenState {
    data object Loading : SettingsScreenState

    data class Ready(
        val rate: String,
        val choice: ThemeChoice,
        val rateRefused: Boolean = false,
        val appearanceRefused: Boolean = false,
    ) : SettingsScreenState

    data object Failed : SettingsScreenState
}

/**
 * Settings' state (ADR-0042, ADR-0051). The rate's clamp, the stored forms and what a stored theme
 * means are the binding's (ADR-0046 §1); this holds what was typed, commits it when the person
 * finishes, and reads the block again after a refusal. A write to Appearance updates [appearance], so
 * the whole app changes theme at once.
 */
class SettingsViewModel(
    private val owner: CollectionAccess,
    private val appearance: AppearanceState,
) : ViewModel() {
    private val _state = MutableStateFlow<SettingsScreenState>(SettingsScreenState.Loading)
    val state: StateFlow<SettingsScreenState> = _state

    /** Whether the field holds something the person typed that was not yet committed; a read leaves it alone. */
    private var editing = false

    init {
        refresh()
    }

    /** Read the page again; a field the person is mid-way through is not overwritten. */
    fun refresh() {
        viewModelScope.launch { read(rateRefused = false, appearanceRefused = false) }
    }

    private suspend fun read(rateRefused: Boolean, appearanceRefused: Boolean, rereadRate: Boolean = false) {
        try {
            val (rate, choice) = owner.use { it.newCardRate() to it.themeChoice() }
            val shown = (_state.value as? SettingsScreenState.Ready)?.rate
            val keep = editing && !rereadRate && shown != null
            if (rereadRate) editing = false
            _state.value = SettingsScreenState.Ready(
                rate = if (keep) shown!! else rate.toString(),
                choice = choice,
                rateRefused = rateRefused,
                appearanceRefused = appearanceRefused,
            )
        } catch (_: CairnException) {
            _state.value = SettingsScreenState.Failed
        }
    }

    /** The field changed: what is typed is kept as typed. */
    fun editRate(text: String) {
        val ready = _state.value as? SettingsScreenState.Ready ?: return
        editing = true
        _state.value = ready.copy(rate = text, rateRefused = false)
    }

    /** *Done*, or the field losing focus: commit what is typed if it parses, and show the value stored. */
    fun commitRate() {
        val ready = _state.value as? SettingsScreenState.Ready ?: return
        val typed = ready.rate.trim()
        // Digits too many for the binding's integer are still a number above the limit: the clamp answers it.
        val rate = typed.toUIntOrNull() ?: if (typed.isNotEmpty() && typed.all { it in '0'..'9' }) UInt.MAX_VALUE else return
        viewModelScope.launch {
            try {
                val stored = owner.use { it.setNewCardRate(rate) }
                editing = false
                _state.value = (_state.value as? SettingsScreenState.Ready)?.copy(rate = stored.toString(), rateRefused = false)
                    ?: return@launch
            } catch (_: CairnException) {
                read(rateRefused = true, appearanceRefused = false, rereadRate = true)
            }
        }
    }

    /** A choice of appearance: written, then applied to the whole app. */
    fun chooseAppearance(choice: ThemeChoice) {
        viewModelScope.launch {
            try {
                owner.use { it.setThemeChoice(choice) }
                appearance.choice = choice
                (_state.value as? SettingsScreenState.Ready)?.let {
                    _state.value = it.copy(choice = choice, appearanceRefused = false)
                }
            } catch (_: CairnException) {
                read(rateRefused = false, appearanceRefused = true)
            }
        }
    }
}
