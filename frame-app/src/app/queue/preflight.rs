use crate::app::root::FrameRoot;
use crate::conversion_runner::ConversionProcessController;
use crate::conversion_runner::video_route::{needs_video_route, plan_task_video};
use crate::file_queue::FileStatus;
use frame_core::args::VideoRoute;
use frame_core::events::ConversionEvent;
use frame_core::types::ConversionTask;
use gpui::{AppContext, Context, PromptButton, PromptLevel, Window};
use std::collections::HashMap;
use std::path::Path;

#[derive(Default)]
struct PlannedBatch {
    tasks: Vec<ConversionTask>,
    routes: HashMap<String, VideoRoute>,
    messages: Vec<ConversionEvent>,
    fallbacks: HashMap<String, String>,
}

fn exports_jpeg_xl(task: &ConversionTask) -> bool {
    task.config.container.eq_ignore_ascii_case("jxl")
}

fn jpeg_xl_preflight(
    available: Option<bool>,
    probe: impl FnOnce() -> Result<bool, String>,
) -> Result<(), String> {
    match available.map_or_else(probe, Ok) {
        Ok(true) => Ok(()),
        Ok(false) => Err("JPEG XL encoding is unavailable in this media runtime.".into()),
        Err(reason) => Err(format!(
            "Could not check JPEG XL encoder availability: {reason}"
        )),
    }
}

impl PlannedBatch {
    fn prepare(
        tasks: Vec<ConversionTask>,
        controller: &ConversionProcessController,
        jpeg_xl_available: Option<bool>,
    ) -> Self {
        let jpeg_xl_error = if tasks.iter().any(exports_jpeg_xl) {
            jpeg_xl_preflight(jpeg_xl_available, || {
                crate::capabilities::detect_available_encoders()
                    .map(|encoders| encoders.libjxl)
                    .map_err(|error| error.to_string())
            })
            .err()
        } else {
            None
        };
        let mut batch = Self::default();
        for mut task in tasks {
            let id = task.id.clone();
            let result = jpeg_xl_error
                .as_ref()
                .filter(|_| exports_jpeg_xl(&task))
                .map_or_else(
                    || {
                        plan_task_video(&mut task, || controller.is_cancelled(&id))
                            .map_err(|error| error.to_string())
                    },
                    |error| Err(error.clone()),
                );
            if controller.is_cancelled(&id) {
                batch.messages.push(ConversionEvent::cancelled(id));
                continue;
            }
            match result {
                Ok(Some(route)) => {
                    let summary = format!(
                        "Decode: {}. Filters: {}. Encoder: {} ({} on {}).",
                        route.decoder, route.filters, route.encoder, route.format, route.device
                    );
                    batch.messages.push(ConversionEvent::log(
                        &id,
                        format!("[INFO] Video preflight: {summary}"),
                    ));
                    if let Some(reason) = &route.fallback {
                        let name = Path::new(&task.file_path)
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy();
                        batch
                            .messages
                            .push(ConversionEvent::log(&id, format!("[WARN] {reason}")));
                        batch
                            .fallbacks
                            .insert(id.clone(), format!("{name}: {reason}\n{summary}"));
                    }
                    batch.routes.insert(id, route);
                }
                Ok(None) => {}
                Err(error) => {
                    batch.messages.push(ConversionEvent::error(id, error));
                    continue;
                }
            }
            batch.tasks.push(task);
        }
        batch
    }
}

impl FrameRoot {
    pub(super) fn preflight_conversion_batch(
        &mut self,
        tasks: Vec<ConversionTask>,
        window: &Window,
        cx: &Context<Self>,
    ) {
        self.conversion_batch_generation = self.conversion_batch_generation.wrapping_add(1);
        let generation = self.conversion_batch_generation;
        let jpeg_xl_available = (self.hardware_capabilities.inventory
            == crate::capabilities::EncoderInventory::Complete)
            .then_some(self.available_encoders.libjxl);
        if tasks.is_empty() {
            return;
        }
        if !tasks.iter().any(needs_video_route)
            && (jpeg_xl_available == Some(true) || !tasks.iter().any(exports_jpeg_xl))
        {
            self.spawn_conversion_batch(tasks, HashMap::new(), cx);
            return;
        }
        let controller = self.conversion_processes.clone();
        cx.spawn_in(window, async move |this, cx| {
            let batch = cx
                .background_spawn(async move {
                    PlannedBatch::prepare(tasks, &controller, jpeg_xl_available)
                })
                .await;
            this.update_in(cx, |root, window, cx| {
                root.confirm_planned_batch(generation, batch, window, cx);
            })
            .ok();
        })
        .detach();
    }

    fn confirm_planned_batch(
        &mut self,
        generation: u64,
        mut batch: PlannedBatch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for event in batch.messages.drain(..) {
            if self.preflight_task_is_queued(generation, event.id()) {
                self.apply_batch_event(generation, event);
            }
        }
        batch
            .tasks
            .retain(|task| self.preflight_task_is_queued(generation, &task.id));
        cx.notify();
        if batch.tasks.is_empty() {
            return;
        }

        let details: Vec<_> = batch
            .tasks
            .iter()
            .filter_map(|task| batch.fallbacks.remove(&task.id))
            .collect();
        if details.is_empty() {
            self.spawn_conversion_batch(batch.tasks, batch.routes, cx);
            return;
        }

        let decision = window.prompt(
            PromptLevel::Warning,
            "Use the available video settings?",
            Some(&details.join("\n\n")),
            &[
                PromptButton::ok("Continue export"),
                PromptButton::cancel("Cancel batch"),
            ],
            cx,
        );
        cx.spawn(async move |this, cx| {
            let approved = decision.await == Ok(0);
            this.update(cx, |root, cx| {
                batch
                    .tasks
                    .retain(|task| root.preflight_task_is_queued(generation, &task.id));
                if approved && !batch.tasks.is_empty() {
                    root.spawn_conversion_batch(batch.tasks, batch.routes, cx);
                } else {
                    for task in batch.tasks {
                        root.apply_batch_event(generation, ConversionEvent::cancelled(task.id));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    // Both preflight and the confirmation dialog can outlive an individual queue entry.
    fn preflight_task_is_queued(&mut self, generation: u64, id: &str) -> bool {
        let Some(file) = self.file_queue.file_by_id(id) else {
            let _ = self.conversion_processes.take_cancelled(id);
            return false;
        };
        if self.conversion_batch_generation != generation {
            return false;
        }
        if file.status == FileStatus::Cancelling {
            let _ = self.conversion_processes.take_cancelled(id);
            self.apply_batch_event(generation, ConversionEvent::cancelled(id));
            return false;
        }
        file.status == FileStatus::Queued
    }
}

#[cfg(test)]
mod tests {
    use super::jpeg_xl_preflight;

    #[test]
    fn jpeg_xl_preflight_checks_an_unknown_inventory_and_preserves_the_failure_reason() {
        for available in [false, true] {
            let cached = jpeg_xl_preflight(Some(available), || panic!("unexpected encoder rescan"));
            assert_eq!(cached.is_ok(), available);
            assert_eq!(jpeg_xl_preflight(None, || Ok(available)), cached);
        }
        let error = jpeg_xl_preflight(None, || Err("worker timed out".into())).unwrap_err();
        assert!(error.contains("worker timed out"));
    }
}

#[cfg(test)]
mod confirmation_tests;
