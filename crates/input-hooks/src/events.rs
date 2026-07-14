#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtfInputEvent {
    DoubleTapShift,
    DoubleTapCtrl,
    ShiftScroll(i32),
    CursorMoved(i32, i32),
}
