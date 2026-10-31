//! The page's one scroll area.
//!
//! Every destination is drawn inside a single vertical scroll area below the pinned nav row, so the
//! offset that area holds is application state: a screen that **replaces** the page rather than
//! continuing it has to put it back at the top.

/// The scroll area every destination is drawn in, and the offset a replacing screen forces on it.
#[derive(Debug, Default)]
pub struct Page {
    /// Set for exactly one frame, to pin the offset *before* layout.
    forced_scroll: Option<f32>,
}

impl Page {
    /// The scroll area for this frame, carrying the forced offset when there is one.
    ///
    /// The offset is **set**, not requested: `scroll_to_rect` and friends land a frame later, so the
    /// first frame of the new page would be drawn at the old offset.
    pub fn scroll_area(&mut self) -> egui::ScrollArea {
        let area = egui::ScrollArea::vertical().auto_shrink([false, false]);
        match self.forced_scroll.take() {
            Some(offset) => area.vertical_scroll_offset(offset),
            None => area,
        }
    }

    /// Put the next frame's scroll area at its top.
    ///
    /// For a screen that **replaces** the page rather than continuing it — the import preview (#167).
    /// Every screen is drawn in the one scroll area, so without this a row opened halfway down
    /// Settings opens its preview halfway down too, with the file's name and its header already
    /// scrolled away above the viewport.
    pub fn scroll_to_top(&mut self) {
        self.forced_scroll = Some(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The forced offset lasts one frame: held after it was set, gone once a frame has taken it, so
    /// the person's own scrolling is never fought on the frames after.
    #[test]
    fn the_top_is_forced_for_one_frame_only() {
        let mut page = Page::default();
        assert_eq!(page.forced_scroll, None);

        page.scroll_to_top();
        assert_eq!(page.forced_scroll, Some(0.0));

        let _ = page.scroll_area();
        assert_eq!(page.forced_scroll, None);
    }
}
