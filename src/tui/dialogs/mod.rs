pub mod alert_form;
pub mod confirm;
pub mod listing_detail;

pub enum DialogResult<T> {
    Continue,
    Cancel,
    Submit(T),
}
