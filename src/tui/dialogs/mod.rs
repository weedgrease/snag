pub mod alert_form;
pub mod confirm;

pub enum DialogResult<T> {
    Continue,
    Cancel,
    Submit(T),
}
