/// Mouse cursor appearance.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Cursor {
    Normal,
    Invisible,
    Wait,
    Crosshair,
    Text,
    Hand,
    ArrowN,
    ArrowS,
    ArrowE,
    ArrowW,
    ArrowNE,
    ArrowNW,
    ArrowSE,
    ArrowSW,
    ResizeNS,
    ResizeEW,
    ResizeNESW,
    ResizeNWSE,
    Move,
}
