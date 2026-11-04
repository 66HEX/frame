use crate::app::accessibility::{TabFocusGroup, focus_visible_ring};
use crate::app::layout::{TITLEBAR_ICON_SIZE, TITLEBAR_NAV_BUTTON_HEIGHT, TITLEBAR_SEGMENT_HEIGHT};
use crate::app::model::{ActiveView, FrameAppState, format_total_size};
use crate::app::motion::mix_color;
use crate::app::primitives::{
    ButtonVariant, apply_button_motion, button_colors, button_motion, color, icon_svg,
};
use crate::app::root::FrameRoot;
use crate::{assets, theme};
use gpui::prelude::Styled;
use gpui::prelude::*;
use gpui::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Window, div,
};

pub(in crate::app) fn titlebar_navigation(
    active_view: ActiveView,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Stateful<gpui::Div> {
    let focuses = TabFocusGroup::render(
        "main-tabs-focus",
        titlebar_tab_id(active_view).into(),
        [ActiveView::Workspace, ActiveView::Logs].map(|view| titlebar_tab_id(view).into()),
        window,
        cx,
    );
    div()
        .id("titlebar-main-view-tabs")
        .role(gpui::Role::TabList)
        .aria_label("Main view")
        .h(theme::ui_rem(TITLEBAR_SEGMENT_HEIGHT))
        .flex()
        .items_center()
        .gap_1()
        .rounded(theme::ui_rem(theme::RADIUS_MD))
        .bg(color(palette.canvas))
        .px(theme::ui_rem(3.0))
        .py(theme::ui_rem(2.0))
        .shadow(theme::shadows::recessed_control())
        .child(titlebar_segment(
            assets::ICON_LAYOUT_LIST,
            "Workspace",
            ActiveView::Workspace,
            &focuses,
            palette,
            window,
            cx,
        ))
        .child(titlebar_segment(
            assets::ICON_TERMINAL,
            "Logs",
            ActiveView::Logs,
            &focuses,
            palette,
            window,
            cx,
        ))
}

pub(in crate::app) fn titlebar_stats(
    state: FrameAppState,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap_4()
        .text_color(color(palette.text_muted))
        .child(titlebar_stat(
            assets::ICON_HARD_DRIVE,
            format!("Storage {}", format_total_size(state.total_size_bytes)),
            palette,
        ))
        .child(titlebar_stat(
            assets::ICON_FILE_VIDEO,
            format!("Items {}", state.file_count),
            palette,
        ))
}

pub(in crate::app) fn titlebar_stat(
    icon: &'static str,
    label: String,
    palette: &'static theme::ThemePalette,
) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(icon_svg(
            icon,
            TITLEBAR_ICON_SIZE,
            color(palette.text_muted),
        ))
        .child(theme::ui_text_owned(label))
}

pub(in crate::app) fn titlebar_segment(
    icon: &'static str,
    label: &'static str,
    view: ActiveView,
    focuses: &TabFocusGroup,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> impl IntoElement {
    let segment_id = titlebar_tab_id(view);
    let selected = focuses.selected(segment_id);
    let focus = focuses.handle(segment_id);
    let click_focus = focus.clone();
    let workspace_focus = focuses.handle("titlebar-workspace");
    let logs_focus = focuses.handle("titlebar-logs");
    let colors = button_colors(ButtonVariant::Secondary, selected, true, palette);
    let motion = button_motion(format!("{segment_id}-hover"), window, cx);
    let hover_progress = *motion.hover_transition.evaluate(window, cx);
    let background = if selected {
        mix_color(colors.background, colors.hover_background, hover_progress)
    } else {
        mix_color(palette.transparent, palette.fill_subtle, hover_progress)
    };
    let foreground = mix_color(
        if selected {
            palette.text_primary
        } else {
            palette.text_muted
        },
        palette.text_primary,
        hover_progress,
    );

    let button = div()
        .id(segment_id)
        .debug_selector(|| segment_id.to_string())
        .h(theme::ui_rem(TITLEBAR_NAV_BUTTON_HEIGHT))
        .role(gpui::Role::Tab)
        .aria_label(label)
        .aria_selected(selected)
        .track_focus(&focus)
        .tab_stop(selected)
        .focus_visible_style(move |style| focus_visible_ring(style, palette))
        .flex()
        .items_center()
        .gap_2()
        .rounded(theme::ui_rem(theme::RADIUS_SM))
        .group(segment_id)
        .px_2()
        .bg(background)
        .font_weight(theme::TEXT_WEIGHT_MEDIUM)
        .text_color(foreground)
        .when(selected, |this| {
            this.shadow(theme::shadows::CONTROL.shadows())
        })
        .hover_style(gpui::Styled::cursor_pointer)
        .active_style(move |style| style.bg(colors.active_fill()))
        .on_click(cx.listener(move |root, _: &ClickEvent, window, cx| {
            click_focus.focus(window, cx);
            if root.active_view != view {
                root.active_view = view;
                cx.notify();
            }
            cx.stop_propagation();
        }))
        .on_key_down(
            cx.listener(move |root, event: &gpui::KeyDownEvent, window, cx| {
                let Some(next_view) = titlebar_view_for_key(view, event.keystroke.key.as_str())
                else {
                    return;
                };
                match next_view {
                    ActiveView::Workspace => &workspace_focus,
                    ActiveView::Logs => &logs_focus,
                }
                .focus(window, cx);
                if root.active_view != next_view {
                    root.active_view = next_view;
                    cx.notify();
                }
                cx.stop_propagation();
            }),
        )
        .child(icon_svg(icon, TITLEBAR_ICON_SIZE, foreground))
        .child(theme::ui_text(label));

    apply_button_motion(button, motion, true)
}

pub(super) const fn titlebar_tab_id(view: ActiveView) -> &'static str {
    match view {
        ActiveView::Workspace => "titlebar-workspace",
        ActiveView::Logs => "titlebar-logs",
    }
}

pub(super) fn titlebar_view_for_key(current: ActiveView, key: &str) -> Option<ActiveView> {
    match key {
        "left" | "right" | "up" | "down" => Some(match current {
            ActiveView::Workspace => ActiveView::Logs,
            ActiveView::Logs => ActiveView::Workspace,
        }),
        "home" => Some(ActiveView::Workspace),
        "end" => Some(ActiveView::Logs),
        _ => None,
    }
}
