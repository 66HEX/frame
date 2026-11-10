use crate::app::input::constants::{TEXT_INPUT_BLINK_INTERVAL, TEXT_INPUT_BLINK_PAUSE};
use crate::app::root::FrameRoot;
use gpui::Context;

impl FrameRoot {
    pub(in crate::app) const fn next_text_input_cursor_epoch(&mut self) -> usize {
        self.text_input_ui.cursor_epoch += 1;
        self.text_input_ui.cursor_epoch
    }

    pub(in crate::app) fn start_text_input_cursor(&mut self, cx: &mut Context<Self>) {
        self.text_input_ui.cursor_paused = false;
        self.blink_text_input_cursor(self.text_input_ui.cursor_epoch, cx);
    }

    pub(in crate::app) const fn stop_text_input_cursor(&mut self) {
        self.text_input_ui.active = None;
        self.text_input_ui.cursor_paused = false;
        self.text_input_ui.cursor_visible = false;
        self.next_text_input_cursor_epoch();
    }

    pub(in crate::app) fn pause_text_input_cursor(&mut self, cx: &mut Context<Self>) {
        self.text_input_ui.cursor_paused = true;
        self.text_input_ui.cursor_visible = true;
        cx.notify();

        let epoch = self.next_text_input_cursor_epoch();
        self.text_input_ui.cursor_task = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(TEXT_INPUT_BLINK_PAUSE).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |root, cx| {
                    root.text_input_ui.cursor_paused = false;
                    root.blink_text_input_cursor(epoch, cx);
                });
            }
        });
    }

    pub(in crate::app) fn blink_text_input_cursor(&mut self, epoch: usize, cx: &mut Context<Self>) {
        if self.text_input_ui.active.is_none() {
            self.text_input_ui.cursor_visible = false;
            return;
        }
        if self.text_input_ui.cursor_paused || epoch != self.text_input_ui.cursor_epoch {
            self.text_input_ui.cursor_visible = true;
            return;
        }

        self.text_input_ui.cursor_visible = !self.text_input_ui.cursor_visible;
        cx.notify();

        let next_epoch = self.next_text_input_cursor_epoch();
        self.text_input_ui.cursor_task = cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(TEXT_INPUT_BLINK_INTERVAL)
                .await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |root, cx| {
                    root.blink_text_input_cursor(next_epoch, cx);
                });
            }
        });
    }
}
