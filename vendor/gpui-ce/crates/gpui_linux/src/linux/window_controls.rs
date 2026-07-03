use gpui::{DispatchEventResult, MouseButton, WindowControlArea};

pub(crate) fn should_start_window_move(
    button: MouseButton,
    result: DispatchEventResult,
    area: Option<WindowControlArea>,
    is_movable: bool,
    is_fullscreen: bool,
) -> bool {
    button == MouseButton::Left
        && result.propagate
        && !result.default_prevented
        && area == Some(WindowControlArea::Drag)
        && is_movable
        && !is_fullscreen
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uncancelled_result() -> DispatchEventResult {
        DispatchEventResult {
            propagate: true,
            default_prevented: false,
        }
    }

    #[test]
    fn allows_left_click_on_drag_area_for_movable_window() {
        assert!(should_start_window_move(
            MouseButton::Left,
            uncancelled_result(),
            Some(WindowControlArea::Drag),
            true,
            false,
        ));
    }

    #[test]
    fn rejects_cancelled_events() {
        assert!(!should_start_window_move(
            MouseButton::Left,
            DispatchEventResult {
                propagate: true,
                default_prevented: true,
            },
            Some(WindowControlArea::Drag),
            true,
            false,
        ));
        assert!(!should_start_window_move(
            MouseButton::Left,
            DispatchEventResult {
                propagate: false,
                default_prevented: false,
            },
            Some(WindowControlArea::Drag),
            true,
            false,
        ));
    }

    #[test]
    fn rejects_non_drag_or_non_left_clicks() {
        assert!(!should_start_window_move(
            MouseButton::Right,
            uncancelled_result(),
            Some(WindowControlArea::Drag),
            true,
            false,
        ));
        assert!(!should_start_window_move(
            MouseButton::Left,
            uncancelled_result(),
            Some(WindowControlArea::Close),
            true,
            false,
        ));
    }

    #[test]
    fn rejects_immovable_or_fullscreen_windows() {
        assert!(!should_start_window_move(
            MouseButton::Left,
            uncancelled_result(),
            Some(WindowControlArea::Drag),
            false,
            false,
        ));
        assert!(!should_start_window_move(
            MouseButton::Left,
            uncancelled_result(),
            Some(WindowControlArea::Drag),
            true,
            true,
        ));
    }
}
