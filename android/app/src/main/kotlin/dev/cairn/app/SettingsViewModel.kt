package dev.cairn.app

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dev.cairn.ffi.CairnException
import dev.cairn.ffi.OptimisationNudge
import dev.cairn.ffi.OptimiseFinish
import dev.cairn.ffi.OptimiseHandle
import dev.cairn.ffi.OptimisePhase
import dev.cairn.ffi.ThemeChoice
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

/** The fact under the Scheduler label (ADR-0014 §2): counts only; the sentence is the string resources'. */
sealed interface Nudge {
    data class Standard(val reviewsTotal: Long) : Nudge

    data class Fitted(val fittedOver: Long, val reviewsSince: Long) : Nudge
}

/** Where an Optimise run is, as the page draws it (ADR-0014 §4). */
sealed interface OptimiseRun {
    data object Idle : OptimiseRun

    /** The uncancellable lead-in: an indeterminate indicator. */
    data object Preparing : OptimiseRun

    data class Training(val current: Long, val total: Long) : OptimiseRun
}

/**
 * What Settings shows: the rate field as typed, the stored appearance, the block that last refused a
 * write, the Scheduler's nudge, the Optimise run, and whether a run has just written (the completion
 * sentence stays until the next run starts).
 */
sealed interface SettingsScreenState {
    data object Loading : SettingsScreenState

    data class Ready(
        val rate: String,
        val choice: ThemeChoice,
        val rateRefused: Boolean = false,
        val appearanceRefused: Boolean = false,
        val nudge: Nudge? = null,
        val run: OptimiseRun = OptimiseRun.Idle,
        val updated: Boolean = false,
    ) : SettingsScreenState

    data object Failed : SettingsScreenState
}

private fun OptimisationNudge.toNudge(): Nudge = when (this) {
    is OptimisationNudge.Standard -> Nudge.Standard(reviewsTotal.toLong())
    is OptimisationNudge.Fitted -> Nudge.Fitted(fittedOver.toLong(), reviewsSince.toLong())
}

/**
 * Settings' state (ADR-0042, ADR-0051). The rate's clamp, the stored forms and what a stored theme
 * means are the binding's (ADR-0046 §1); this holds what was typed, commits it when the person
 * finishes, and reads the block again after a refusal. A write to Appearance updates [appearance], so
 * the whole app changes theme at once.
 *
 * It also holds the Optimise run (ADR-0014, client-stack rule 10): the handle lives here, so a run outlives a
 * rotation and a switch of destination. [tick] polls it; the screen calls that on a short interval
 * while its lifecycle is started. No service, no wakelock, no scheduled work: a run that nobody
 * watches is simply not seen to finish until the screen is back.
 */
class SettingsViewModel(
    private val owner: CollectionAccess,
    private val appearance: AppearanceState,
    private val nowWallMs: () -> Long = System::currentTimeMillis,
) : ViewModel() {
    private val _state = MutableStateFlow<SettingsScreenState>(SettingsScreenState.Loading)
    val state: StateFlow<SettingsScreenState> = _state

    /** Whether the field holds something the person typed that was not yet committed; a read leaves it alone. */
    private var editing = false

    /** The run in flight, if any. Touched from the main thread and from [tick]'s coroutine, one at a time. */
    private var run: OptimiseHandle? = null

    /** Whether a run is being started, so a second request before the first has a handle starts nothing. */
    private var starting = false

    /** Whether a [tick] is under way, so a slow poll is not overtaken by the next one. */
    private var ticking = false

    init {
        refresh()
    }

    /** Read the page again; a field the person is mid-way through is not overwritten, nor is a run. */
    fun refresh() {
        viewModelScope.launch { read(rateRefused = false, appearanceRefused = false) }
    }

    private suspend fun read(rateRefused: Boolean, appearanceRefused: Boolean, rereadRate: Boolean = false) {
        try {
            val (rate, choice, nudge) = owner.use { Triple(it.newCardRate(), it.themeChoice(), it.optimisationNudge().toNudge()) }
            val before = _state.value as? SettingsScreenState.Ready
            val shown = before?.rate
            val keep = editing && !rereadRate && shown != null
            if (rereadRate) editing = false
            _state.value = SettingsScreenState.Ready(
                rate = if (keep) shown!! else rate.toString(),
                choice = choice,
                rateRefused = rateRefused,
                appearanceRefused = appearanceRefused,
                nudge = nudge,
                run = before?.run ?: OptimiseRun.Idle,
                updated = before?.updated ?: false,
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

    /** *Optimise*: start a run from the foreground (client-stack rule 10). The completion sentence clears. */
    fun optimise() {
        if (run != null || starting) return
        starting = true
        viewModelScope.launch {
            try {
                run = owner.use { it.startOptimise() }
                (_state.value as? SettingsScreenState.Ready)?.let {
                    _state.value = it.copy(run = OptimiseRun.Preparing, updated = false)
                }
            } catch (_: CairnException) {
                // The run could not start: the page stays at rest, as it was.
            } finally {
                starting = false
            }
        }
    }

    /** *Cancel*: honoured once training is under way; the next poll finds the run over and nothing written. */
    fun cancelOptimise() {
        val handle = run ?: return
        viewModelScope.launch {
            owner.use { handle.cancel() }
            tick()
        }
    }

    /**
     * One poll of the run: it finishes (and a written vector is read back into the nudge), or it shows
     * where it is. A poll that fails leaves the run to be polled again.
     */
    suspend fun tick() {
        val handle = run ?: return
        if (ticking) return
        ticking = true
        try {
            val (finish, phase) = owner.use { handle.finish(nowWallMs()) to handle.phase() }
            when (finish) {
                OptimiseFinish.STILL_RUNNING -> setRun(
                    when (phase) {
                        is OptimisePhase.Training -> OptimiseRun.Training(phase.current.toLong(), phase.total.toLong())
                        else -> OptimiseRun.Preparing
                    },
                )
                OptimiseFinish.WRITTEN -> {
                    run = null
                    setRun(OptimiseRun.Idle, updated = true)
                    read(rateRefused = false, appearanceRefused = false)
                }
                OptimiseFinish.NOTHING_WRITTEN -> {
                    run = null
                    setRun(OptimiseRun.Idle)
                }
            }
        } catch (_: CairnException) {
            // Polled again on the next tick.
        } finally {
            ticking = false
        }
    }

    private fun setRun(run: OptimiseRun, updated: Boolean? = null) {
        (_state.value as? SettingsScreenState.Ready)?.let {
            _state.value = it.copy(run = run, updated = updated ?: it.updated)
        }
    }
}
