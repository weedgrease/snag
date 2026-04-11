pub mod alert_form;
pub mod confirm;
pub mod ebay_setup;
pub mod listing_detail;

/// Returned by a dialog's key handler to signal its desired lifecycle transition.
///
/// `Continue` — still open; `Cancel` — close without action; `Submit(T)` — close and apply result.
pub enum DialogResult<T> {
    Continue,
    Cancel,
    Submit(T),
}
