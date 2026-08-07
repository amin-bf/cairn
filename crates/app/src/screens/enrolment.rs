//! The **Enrolment** destination: the enrolment screen's surface, reached from Settings.

use crate::{body, field_label, full_width_button, heading, sync};

/// The enrolment screen's surface (ADR-0015 §7, ADR-0019 §4): what it states *before* the grant. The
/// device flow, the credential file and the UserInfo fetch that would follow are the deferred network
/// mechanism (see `sync` and ADR-0013 §11); this screen owns the plain-words scope and the one-time
/// disclosure, which are decided and need no network to state.
pub(crate) fn enrolment_screen(ui: &mut egui::Ui, setting_up: &mut bool) {
    heading(ui, sync::SET_UP_SYNC);
    ui.add_space(8.0);

    // The scope, in plain words (ADR-0015 §7, ADR-0019 §4): the consent screen asks for two things.
    body(ui, sync::SCOPE_PLAIN_WORDS);
    ui.add_space(8.0);
    // The promise again — it appears at enrolment and in settings, and nowhere else (ADR-0015 §3).
    body(ui, sync::PROMISE);
    ui.add_space(8.0);
    // What leaves the device, stated once (ADR-0020 §7): not a status message, never promoted to a
    // resting surface.
    body(ui, sync::DISCLOSURE_CLAUSE);

    ui.add_space(12.0);
    // The device flow itself needs the network and a handset (ADR-0013 §11): the surface is settled
    // here, the grant is its own step. Stated plainly rather than offered as a control that cannot
    // complete.
    field_label(
        ui,
        "Granting access uses the device flow, which needs a network connection — not available in \
         this build.",
    );

    ui.add_space(8.0);
    if full_width_button(ui, "Back").clicked() {
        *setting_up = false;
    }
}
