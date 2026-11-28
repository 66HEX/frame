use crate::app::root::FrameRoot;
use crate::conversion_runner::ConversionProcessController;
use crate::conversion_runner::video_route::{needs_video_route, plan_task_video};
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
    fallbacks: Vec<String>,
}

impl PlannedBatch {
    fn prepare(tasks: Vec<ConversionTask>, controller: &ConversionProcessController) -> Self {
        let mut batch = Self::default();
        for mut task in tasks {
            let id = task.id.clone();
            let result = plan_task_video(&mut task, || controller.is_cancelled(&id));
            if controller.is_cancelled(&id) {
                let _ = controller.take_cancelled(&id);
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
                        batch.messages.push(ConversionEvent::log(&id, format!("[WARN] {reason}")));
                        batch.fallbacks.push(format!("{name}: {reason}\n{summary}"));
                    }
                    batch.routes.insert(id, route);
                }
                Ok(None) => {}
                Err(error) => {
                    batch
                        .messages
                        .push(ConversionEvent::error(id, error.to_string()));
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
        mut tasks: Vec<ConversionTask>,
        window: &Window,
        cx: &Context<Self>,
    ) {
        self.conversion_batch_generation = self.conversion_batch_generation.wrapping_add(1);
        let generation = self.conversion_batch_generation;
        if !self.available_encoders.libjxl
            && self.hardware_capabilities.inventory
                == crate::capabilities::EncoderInventory::Complete
        {
            tasks.retain(|task| {
                if task.config.container == "jxl" {
                    self.apply_batch_event(
                        generation,
                        ConversionEvent::error(
                            &task.id,
                            "JPEG XL encoding is unavailable in this media runtime.",
                        ),
                    );
                    false
                } else {
                    true
                }
            });
        }
        if tasks.is_empty() {
            return;
        }
        if !tasks.iter().any(needs_video_route) {
            self.spawn_conversion_batch(tasks, HashMap::new(), cx);
            return;
        }
        let controller = self.conversion_processes.clone();
        cx.spawn_in(window, async move |this, cx| {
            let batch = cx
                .background_spawn(async move { PlannedBatch::prepare(tasks, &controller) })
                .await;
            let current = this
                .update(cx, |root, cx| {
                    if root.conversion_batch_generation != generation {
                        return false;
                    }
                    for event in batch.messages {
                        root.apply_batch_event(generation, event);
                    }
                    cx.notify();
                    true
                })
                .unwrap_or(false);
            if !current || batch.tasks.is_empty() {
                return;
            }
            let approved = batch.fallbacks.is_empty()
                || cx
                    .prompt(
                        PromptLevel::Warning,
                        "Use the available video settings?",
                        Some(&batch.fallbacks.join("\n\n")),
                        &[
                            PromptButton::ok("Continue export"),
                            PromptButton::cancel("Cancel batch"),
                        ],
                    )
                    .await
                    == Ok(0);
            this.update(cx, |root, cx| {
                if root.conversion_batch_generation != generation {
                    return;
                }
                if approved {
                    root.spawn_conversion_batch(batch.tasks, batch.routes, cx);
                } else {
                    for task in batch.tasks {
                        let _ = root.conversion_processes.take_cancelled(&task.id);
                        root.apply_batch_event(generation, ConversionEvent::cancelled(task.id));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}
