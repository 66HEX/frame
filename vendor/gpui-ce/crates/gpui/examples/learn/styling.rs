//! Styling Patterns Example
//!
//! This example demonstrates different styling approaches in GPUI:
//!
//! 1. Interactive states - hover, active, focus, focus_visible
//! 2. Conditional styling - when, when_some, map
//! 3. Theming patterns - using Colors for consistent styling

use gpui::colors::Colors;
use gpui::{
    App, Bounds, Context, FocusHandle, Hsla, KeyBinding, Menu, MenuItem, Render, Rgba, Window,
    WindowBounds, WindowOptions, actions, div, prelude::*, px, rgb, size,
};

actions!(styling_example, [Quit, Tab, TabPrev]);

// Interactive States Example

fn interactive_button(
    id: impl Into<gpui::ElementId>,
    label: &'static str,
    colors: &Colors,
) -> impl IntoElement {
    let accent = colors.selected;
    let accent_hover = colors.selected;
    let accent_active = colors.selected;
    let text = colors.selected_text;

    div()
        .id(id)
        .px_4()
        .py_2()
        .rounded_md()
        .cursor_pointer()
        .bg(accent)
        .text_color(text)
        .text_sm()
        .hover(move |style| style.bg(accent_hover))
        .active(move |style| style.bg(accent_active))
        .child(label)
}

fn focus_button(
    id: impl Into<gpui::ElementId>,
    label: &'static str,
    focus_handle: &FocusHandle,
    colors: &Colors,
) -> impl IntoElement {
    let surface = colors.container;
    let surface_hover = colors.selected;
    let text = colors.text;
    let accent = colors.selected;
    let focus_ring: Rgba = rgb(0x60a5fa);

    div()
        .id(id)
        .track_focus(focus_handle)
        .px_4()
        .py_2()
        .rounded_md()
        .cursor_pointer()
        .bg(surface)
        .text_color(text)
        .text_sm()
        .border_2()
        .border_color(gpui::transparent_black())
        .hover(move |style| style.bg(surface_hover))
        .focus(move |style| style.border_color(accent))
        .focus_visible(move |style| style.border_color(focus_ring).shadow_sm())
        .child(label)
}

fn interactive_states_section(colors: &Colors) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .text_xs()
                .text_color(colors.disabled)
                .child("hover() / active() - Mouse interaction states"),
        )
        .child(
            div()
                .flex()
                .gap_2()
                .child(interactive_button("btn-1", "Hover me", colors))
                .child(interactive_button("btn-2", "Click me", colors)),
        )
}

// Conditional Styling Example

fn status_badge(status: &'static str, variant: StatusVariant, colors: &Colors) -> impl IntoElement {
    let (bg, text): (Rgba, Rgba) = match variant {
        StatusVariant::Success => (rgb(0x388e3c), colors.selected_text),
        StatusVariant::Warning => (rgb(0xf9a825), rgb(0x000000)),
        StatusVariant::Error => (rgb(0xd32f2f), colors.selected_text),
        StatusVariant::Neutral => (colors.container, colors.text),
    };

    div()
        .px_2()
        .py_0p5()
        .rounded_full()
        .text_xs()
        .bg(bg)
        .text_color(text)
        .child(status)
}

#[derive(Clone, Copy)]
enum StatusVariant {
    Success,
    Warning,
    Error,
    Neutral,
}

fn list_item(
    id: impl Into<gpui::ElementId>,
    label: &'static str,
    is_selected: bool,
    is_disabled: bool,
    colors: &Colors,
) -> impl IntoElement {
    let surface = colors.container;
    let surface_hover = colors.selected;
    let text = colors.text;
    let text_muted = colors.disabled;
    let accent = colors.selected;

    div()
        .id(id)
        .px_3()
        .py_2()
        .rounded_md()
        .text_sm()
        .cursor_pointer()
        .border_1()
        .border_color(gpui::transparent_black())
        .when(is_disabled, |el| {
            el.opacity(0.5)
                .cursor_not_allowed()
                .bg(surface)
                .text_color(text_muted)
        })
        .when(!is_disabled && is_selected, move |el| {
            let accent_bg: Hsla = accent.into();
            el.bg(accent_bg.opacity(0.2))
                .border_color(accent)
                .text_color(text)
        })
        .when(!is_disabled && !is_selected, move |el| {
            el.bg(surface)
                .text_color(text)
                .hover(move |style| style.bg(surface_hover))
        })
        .child(label)
}

fn conditional_section(colors: &Colors) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .text_xs()
                .text_color(colors.disabled)
                .child("when() - Apply styles conditionally"),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(list_item("item-1", "Normal item", false, false, colors))
                .child(list_item("item-2", "Selected item", true, false, colors))
                .child(list_item("item-3", "Disabled item", false, true, colors)),
        )
        .child(
            div()
                .text_xs()
                .text_color(colors.disabled)
                .mt_2()
                .child("Status badges with variant-based styling"),
        )
        .child(
            div()
                .flex()
                .gap_2()
                .child(status_badge("Success", StatusVariant::Success, colors))
                .child(status_badge("Warning", StatusVariant::Warning, colors))
                .child(status_badge("Error", StatusVariant::Error, colors))
                .child(status_badge("Neutral", StatusVariant::Neutral, colors)),
        )
}

// Group Hover Example

fn card_with_group_hover(
    id: impl Into<gpui::ElementId>,
    title: &'static str,
    description: &'static str,
    colors: &Colors,
) -> impl IntoElement {
    let surface = colors.container;
    let border = colors.border;
    let accent = colors.selected;
    let text = colors.text;
    let text_muted = colors.disabled;

    div()
        .id(id)
        .group("card")
        .p_4()
        .rounded_lg()
        .bg(surface)
        .border_1()
        .border_color(border)
        .cursor_pointer()
        .hover(move |style| style.border_color(accent))
        .child(
            div()
                .flex()
                .justify_between()
                .items_center()
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(text)
                        .child(title),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(text_muted)
                        .opacity(0.)
                        .group_hover("card", |style| style.opacity(1.))
                        .child("→"),
                ),
        )
        .child(
            div()
                .mt_1()
                .text_xs()
                .text_color(text_muted)
                .child(description),
        )
}

fn group_hover_section(colors: &Colors) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .text_xs()
                .text_color(colors.disabled)
                .child("group() / group_hover() - Parent hover affects children"),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(card_with_group_hover(
                    "card-1",
                    "Documents",
                    "View and manage your documents",
                    colors,
                ))
                .child(card_with_group_hover(
                    "card-2",
                    "Settings",
                    "Configure application settings",
                    colors,
                )),
        )
}

// Main Application View

struct StylingExample {
    focus_handle: FocusHandle,
    buttons: Vec<FocusHandle>,
}

impl StylingExample {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle, cx);

        let buttons = vec![
            cx.focus_handle().tab_index(1).tab_stop(true),
            cx.focus_handle().tab_index(2).tab_stop(true),
            cx.focus_handle().tab_index(3).tab_stop(true),
        ];

        Self {
            focus_handle,
            buttons,
        }
    }

    fn on_tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        window.focus_next(cx);
    }

    fn on_tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        window.focus_prev(cx);
    }
}

impl Render for StylingExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Colors::for_appearance(window);

        div()
            .id("app")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_tab))
            .on_action(cx.listener(Self::on_tab_prev))
            .size_full()
            .p_6()
            .bg(colors.background)
            .overflow_scroll()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_6()
                    .max_w(px(500.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(colors.text)
                                    .child("Styling Patterns"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(colors.disabled)
                                    .child("Interactive states, conditional styling, and theming"),
                            ),
                    )
                    .child(section(
                        &colors,
                        "Interactive States",
                        interactive_states_section(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Focus States (Tab to navigate)",
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(colors.disabled)
                                    .child("focus() / focus_visible() - Keyboard navigation"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(focus_button(
                                        "focus-1",
                                        "Button 1",
                                        &self.buttons[0],
                                        &colors,
                                    ))
                                    .child(focus_button(
                                        "focus-2",
                                        "Button 2",
                                        &self.buttons[1],
                                        &colors,
                                    ))
                                    .child(focus_button(
                                        "focus-3",
                                        "Button 3",
                                        &self.buttons[2],
                                        &colors,
                                    )),
                            ),
                    ))
                    .child(section(
                        &colors,
                        "Conditional Styling",
                        conditional_section(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Group Hover",
                        group_hover_section(&colors),
                    ))
                    .child(section(
                        &colors,
                        "Default Colors",
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(colors.disabled)
                                    .child("Using Colors::for_appearance() for consistent theming"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap_2()
                                    .child(color_swatch(&colors, "background", colors.background))
                                    .child(color_swatch(&colors, "container", colors.container))
                                    .child(color_swatch(&colors, "selected", colors.selected))
                                    .child(color_swatch(&colors, "success", rgb(0x388e3c)))
                                    .child(color_swatch(&colors, "warning", rgb(0xf9a825)))
                                    .child(color_swatch(&colors, "error", rgb(0xd32f2f)))
                                    .child(color_swatch(&colors, "border", colors.border)),
                            ),
                    )),
            )
    }
}

fn section(colors: &Colors, title: &'static str, content: impl IntoElement) -> impl IntoElement {
    let surface: Hsla = colors.container.into();
    let border: Hsla = colors.border.into();

    div()
        .flex()
        .flex_col()
        .gap_3()
        .p_4()
        .bg(surface.opacity(0.3))
        .rounded_lg()
        .border_1()
        .border_color(border.opacity(0.5))
        .child(
            div()
                .text_sm()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(colors.text)
                .child(title),
        )
        .child(content)
}

fn color_swatch(colors: &Colors, name: &'static str, color: Rgba) -> impl IntoElement {
    let text_muted = colors.disabled;

    div()
        .flex()
        .flex_col()
        .items_center()
        .gap_1()
        .child(
            div()
                .size_8()
                .rounded_md()
                .bg(color)
                .border_1()
                .border_color(gpui::white().opacity(0.2)),
        )
        .child(div().text_xs().text_color(text_muted).child(name))
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        cx.activate(true);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("tab", Tab, None),
            KeyBinding::new("shift-tab", TabPrev, None),
        ]);
        cx.set_menus(vec![Menu {
            name: "Styling".into(),
            items: vec![MenuItem::action("Quit", Quit)],
            disabled: false,
        }]);
        cx.on_window_closed(|cx, _window_id| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let bounds = Bounds::centered(None, size(px(550.), px(800.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| StylingExample::new(window, cx)),
        )
        .expect("Failed to open window");
    });
}
