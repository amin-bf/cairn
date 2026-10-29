//! The **Enrolment** destination: the enrolment screen's surface, reached from Settings.

use crate::{body, controls, field_label, frame, heading, spacing, sync};

/// The enrolment screen's surface (ADR-0015 §7, ADR-0019 §4): what it states *before* the grant. The
/// device flow, the credential file and the UserInfo fetch that would follow are the deferred network
/// mechanism (see `sync` and ADR-0013 §11); this screen owns the plain-words scope and the one-time
/// disclosure, which are decided and need no network to state.
pub(crate) fn enrolment_screen(ui: &mut egui::Ui, setting_up: &mut bool) {
    heading(ui, sync::SET_UP_SYNC);
    ui.add_space(spacing::gap(2));

    // The scope, in plain words (ADR-0015 §7, ADR-0019 §4): the consent screen asks for two things.
    body(ui, sync::SCOPE_PLAIN_WORDS);
    ui.add_space(spacing::gap(2));
    // The promise again — it appears at enrolment and in settings, and nowhere else (ADR-0015 §3).
    body(ui, sync::PROMISE);
    ui.add_space(spacing::gap(2));
    // What leaves the device, stated once (ADR-0020 §7): not a status message, never promoted to a
    // resting surface.
    body(ui, sync::DISCLOSURE_CLAUSE);

    ui.add_space(spacing::gap(3));
    // The device flow itself needs the network and a handset (ADR-0013 §11): the surface is settled
    // here, the grant is its own step. Stated plainly rather than offered as a control that cannot
    // complete.
    field_label(
        ui,
        "Granting access uses the device flow, which needs a network connection — not available in \
         this build.",
    );

    // **The way out sits on the reach line, as the primary** (#152, ADR-0042). This page has a
    // closing control where Settings has none, so the line is this sub-state's to take. Alone, the way
    // out is the primary, as the file surface's lone *Close* is (ADR-0041 §3); once the grant can
    // complete, a primary *continue* takes the line and *Back* becomes a text action beside it, as the
    // import gate draws *Cancel*.
    ui.add_space(frame::slack_above(
        frame::page_room(ui),
        controls::HEIGHT,
        spacing::gap(2),
    ));
    if controls::wide_primary(ui, "Back").clicked() {
        *setting_up = false;
    }
}
