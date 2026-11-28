//! One-shot reuse of committed output settings, with source-bound state kept per file.

use crate::app::components::frame_text_button;
use crate::app::inspector::shared::settings_hint_text;
use crate::app::primitives::{ButtonVariant, color};
use crate::app::root::FrameRoot;
use crate::file_queue::FileItem;
use crate::settings::{
    ConversionConfig, PresetDefinition, SourceKind, SourceMetadata, apply_preset,
    audio_settings_error, container_supports_audio, create_custom_preset, is_audio_only_container,
    is_audio_stream_codec_allowed_for_container, is_container_compatible_for_stream_copy,
    is_gif_container, is_image_container, is_video_stream_codec_allowed_for_container,
    preset_is_compatible, source_kind_for,
};
use crate::source_metadata::MetadataStatus;
use crate::theme;
use gpui::prelude::*;
use gpui::{Context, PromptButton, PromptLevel, Window, div};
use std::collections::HashSet;

const DONOR_LOADING: &str = "Wait for this file to finish loading.";
const DONOR_ERROR: &str = "Choose a file that loaded successfully.";
const DONOR_LOCKED: &str = "Choose a pending file to copy settings from.";
const TARGETS_LOADING: &str = "Wait for other files to finish loading.";
const NO_TARGETS: &str = "No other pending files can use these settings.";
const UPDATER: &str = "Installing update and restarting...";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum ApplyAllAvailability {
    Hidden,
    Disabled(&'static str),
    Enabled,
}

#[derive(Clone)]
struct FileIdentity {
    id: String,
    path: String,
}

impl From<&FileItem> for FileIdentity {
    fn from(file: &FileItem) -> Self {
        Self {
            id: file.id.clone(),
            path: file.path.clone(),
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum BatchScope {
    Selected,
    All,
}

#[derive(Clone)]
struct ApplyAllSnapshot {
    donor: Option<FileIdentity>,
    preset: PresetDefinition,
    targets: Vec<FileIdentity>,
    scope: BatchScope,
    skipped: usize,
}

// These necessary conditions avoid cloning large configurations for obvious rejects.
// A passing prefilter is never sufficient: the normalized candidate remains the oracle.
#[derive(Clone, Copy, Eq, PartialEq)]
enum OutputKind {
    Audio,
    Image,
    Gif,
    Video,
}

const CODEC_MEMO_LIMIT: usize = 32;

#[derive(Default)]
struct CodecVerdicts<'a> {
    entries: Vec<(&'a str, bool)>,
}

impl<'a> CodecVerdicts<'a> {
    fn allowed(&mut self, codec: &'a str, check: impl FnOnce() -> bool) -> bool {
        if let Some((_, allowed)) = self.entries.iter().find(|(key, _)| *key == codec) {
            return *allowed;
        }
        let allowed = check();
        if self.entries.len() < CODEC_MEMO_LIMIT {
            self.entries.push((codec, allowed));
        }
        allowed
    }
}

struct CandidatePolicy<'a> {
    donor: &'a ConversionConfig,
    kind: OutputKind,
    supports_audio: bool,
    video_verdicts: CodecVerdicts<'a>,
    audio_verdicts: CodecVerdicts<'a>,
}

impl<'a> CandidatePolicy<'a> {
    fn new(donor: &'a ConversionConfig) -> Self {
        Self {
            donor,
            kind: if is_audio_only_container(&donor.container) {
                OutputKind::Audio
            } else if is_image_container(&donor.container) {
                OutputKind::Image
            } else if is_gif_container(&donor.container) {
                OutputKind::Gif
            } else {
                OutputKind::Video
            },
            supports_audio: container_supports_audio(&donor.container),
            video_verdicts: CodecVerdicts::default(),
            audio_verdicts: CodecVerdicts::default(),
        }
    }

    fn may_apply(&mut self, target: &ConversionConfig, metadata: &'a SourceMetadata) -> bool {
        let container = self.donor.container.as_str();
        let kind = source_kind_for(Some(metadata));
        if (kind == SourceKind::Audio && self.kind != OutputKind::Audio)
            || (kind == SourceKind::Image
                && !matches!(self.kind, OutputKind::Image | OutputKind::Gif))
        {
            return false;
        }
        let keeps_audio = kind != SourceKind::Image && self.supports_audio;
        if keeps_audio
            && !target.selected_audio_tracks.iter().all(|index| {
                metadata
                    .audio_tracks
                    .iter()
                    .any(|track| track.index == *index)
            })
        {
            return false;
        }
        let mode = self.donor.processing_mode;
        if (!mode.copies_video() && !mode.copies_audio())
            || kind == SourceKind::Image
            || self.kind == OutputKind::Gif
        {
            return true;
        }
        if self.kind == OutputKind::Image {
            return !mode.copies_video();
        }
        if self.kind == OutputKind::Audio {
            if mode.copies_audio() && target.selected_audio_tracks.is_empty() {
                return false;
            }
        } else if mode.copies_video()
            && !metadata.video_codec.as_deref().is_some_and(|codec| {
                self.video_verdicts.allowed(codec, || {
                    is_video_stream_codec_allowed_for_container(container, codec)
                })
            })
        {
            return false;
        }
        !mode.copies_audio()
            || !keeps_audio
            || metadata
                .audio_tracks
                .iter()
                .filter(|track| target.selected_audio_tracks.contains(&track.index))
                .all(|track| {
                    self.audio_verdicts.allowed(&track.codec, || {
                        is_audio_stream_codec_allowed_for_container(container, &track.codec)
                    })
                })
    }
}

fn candidate_for(
    preset: &PresetDefinition,
    target: &ConversionConfig,
    metadata: &SourceMetadata,
) -> Option<ConversionConfig> {
    #[cfg(test)]
    CANDIDATES.with(|count| count.set(count.get() + 1));
    if !preset_is_compatible(preset, Some(metadata)) {
        return None;
    }
    let mut candidate = target.clone();
    apply_preset(&mut candidate, preset, Some(metadata));
    let indices_exist = candidate.selected_audio_tracks.iter().all(|index| {
        metadata
            .audio_tracks
            .iter()
            .any(|track| track.index == *index)
    }) && candidate.selected_subtitle_tracks.iter().all(|index| {
        metadata
            .subtitle_tracks
            .iter()
            .any(|track| track.index == *index)
    });
    (indices_exist
        && audio_settings_error(&candidate, Some(metadata)).is_none()
        && is_container_compatible_for_stream_copy(
            &candidate,
            Some(metadata),
            &candidate.container,
        ))
    .then_some(candidate)
}

impl FrameRoot {
    fn apply_all_donor(&self) -> Result<&FileItem, &'static str> {
        if self.update_installation_in_progress() {
            return Err(UPDATER);
        }
        let donor = self.file_queue.selected_file().ok_or(DONOR_ERROR)?;
        self.check_apply_all_donor(donor)?;
        Ok(donor)
    }

    fn check_apply_all_donor(&self, donor: &FileItem) -> Result<(), &'static str> {
        if !donor.status.is_actionable_for_conversion() {
            return Err(DONOR_LOCKED);
        }
        match self.source_metadata.status_for(&donor.id) {
            MetadataStatus::Idle | MetadataStatus::Loading => Err(DONOR_LOADING),
            MetadataStatus::Error => Err(DONOR_ERROR),
            MetadataStatus::Ready => self
                .source_metadata
                .ready_metadata_for(&donor.id)
                .map(|_| ())
                .ok_or(DONOR_ERROR),
        }
    }

    pub(in crate::app) fn apply_all_availability(&self) -> ApplyAllAvailability {
        let donor = match self.apply_all_donor() {
            Ok(donor) => donor,
            Err(reason) => return ApplyAllAvailability::Disabled(reason),
        };
        let mut policy = CandidatePolicy::new(&donor.config);
        let mut loading = false;
        let available = self.file_queue.files().iter().any(|target| {
            if target.id == donor.id || !target.status.is_actionable_for_conversion() {
                return false;
            }
            let Some(metadata) = self.source_metadata.ready_metadata_for(&target.id) else {
                loading |= matches!(
                    self.source_metadata.status_for(&target.id),
                    MetadataStatus::Idle | MetadataStatus::Loading
                );
                return false;
            };
            if !policy.may_apply(&target.config, metadata) {
                return false;
            }
            let preset = create_custom_preset(String::new(), "Output settings", &donor.config);
            candidate_for(&preset, &target.config, metadata).is_some()
        });
        if available {
            ApplyAllAvailability::Enabled
        } else {
            ApplyAllAvailability::Disabled(if loading { TARGETS_LOADING } else { NO_TARGETS })
        }
    }

    fn snapshot_apply_all(&self) -> Option<ApplyAllSnapshot> {
        let donor = self.apply_all_donor().ok()?;
        let preset = create_custom_preset(String::new(), "Output settings", &donor.config);
        let snapshot = self.snapshot_batch(&preset, Some(donor));
        (!snapshot.targets.is_empty()).then_some(snapshot)
    }

    fn snapshot_batch(
        &self,
        preset: &PresetDefinition,
        donor: Option<&FileItem>,
    ) -> ApplyAllSnapshot {
        let mut policy = CandidatePolicy::new(&preset.config);
        let mut considered = 0;
        let targets = self
            .file_queue
            .files()
            .iter()
            .filter_map(|target| {
                if donor.is_some_and(|donor| target.id == donor.id) {
                    return None;
                }
                considered += 1;
                if !target.status.is_actionable_for_conversion() {
                    return None;
                }
                let metadata = self.source_metadata.ready_metadata_for(&target.id)?;
                (policy.may_apply(&target.config, metadata)
                    && candidate_for(preset, &target.config, metadata).is_some())
                .then(|| FileIdentity::from(target))
            })
            .collect::<Vec<_>>();
        ApplyAllSnapshot {
            donor: donor.map(FileIdentity::from),
            preset: preset.clone(),
            skipped: considered - targets.len(),
            targets,
            scope: BatchScope::All,
        }
    }

    fn selected_batch_snapshot(&self, all: &ApplyAllSnapshot) -> ApplyAllSnapshot {
        let selected: HashSet<_> = self
            .file_queue
            .files()
            .iter()
            .filter(|file| {
                file.is_selected_for_conversion
                    && all.donor.as_ref().is_none_or(|donor| donor.id != file.id)
            })
            .map(|file| file.id.as_str())
            .collect();
        let targets: Vec<_> = all
            .targets
            .iter()
            .filter(|target| selected.contains(target.id.as_str()))
            .cloned()
            .collect();
        let skipped = selected.len() - targets.len();
        ApplyAllSnapshot {
            targets,
            scope: BatchScope::Selected,
            skipped,
            donor: all.donor.clone(),
            preset: all.preset.clone(),
        }
    }

    fn apply_all_snapshot(&mut self, snapshot: &ApplyAllSnapshot) -> bool {
        if self.update_installation_in_progress() {
            return false;
        }
        if let Some(identity) = &snapshot.donor {
            let Some(donor) = self.file_queue.file_by_id(&identity.id) else {
                return false;
            };
            if donor.path != identity.path || self.check_apply_all_donor(donor).is_err() {
                return false;
            }
        }
        self.prune_applied_presets();
        let identities: HashSet<_> = snapshot
            .targets
            .iter()
            .map(|target| (target.id.as_str(), target.path.as_str()))
            .collect();
        let mut policy = CandidatePolicy::new(&snapshot.preset.config);
        let mut changed = false;
        let named_preset = self
            .presets
            .iter()
            .any(|preset| preset.id == snapshot.preset.id);
        for target in self.file_queue.files_mut() {
            if snapshot
                .donor
                .as_ref()
                .is_some_and(|donor| target.id == donor.id)
                || !target.status.is_actionable_for_conversion()
                || (snapshot.scope == BatchScope::Selected && !target.is_selected_for_conversion)
                || !identities.contains(&(target.id.as_str(), target.path.as_str()))
            {
                continue;
            }
            let Some(metadata) = self.source_metadata.ready_metadata_for(&target.id) else {
                continue;
            };
            if !policy.may_apply(&target.config, metadata) {
                continue;
            }
            let Some(candidate) = candidate_for(&snapshot.preset, &target.config, metadata) else {
                continue;
            };
            if candidate != target.config {
                target.config = candidate;
                changed = true;
            }
            if named_preset {
                let previous = self.settings_ui.applied_presets.insert(
                    target.id.clone(),
                    super::state::AppliedPreset {
                        path: target.path.clone(),
                        preset_id: snapshot.preset.id.clone(),
                    },
                );
                changed |= previous.is_none_or(|previous| {
                    previous.path != target.path || previous.preset_id != snapshot.preset.id
                });
            } else {
                changed |= self
                    .settings_ui
                    .applied_presets
                    .remove(&target.id)
                    .is_some();
            }
        }
        changed
    }

    fn confirm_apply_all_settings(&self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(snapshot) = self.snapshot_apply_all() else {
            return;
        };
        self.confirm_batch_snapshot(snapshot, window, cx);
    }

    pub(in crate::app) fn confirm_preset_batch(
        &self,
        preset: &PresetDefinition,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.update_installation_in_progress() {
            return;
        }
        self.confirm_batch_snapshot(self.snapshot_batch(preset, None), window, cx);
    }

    #[cfg(test)]
    pub(in crate::app) fn apply_preset_to_all_pending(
        &mut self,
        preset: &PresetDefinition,
    ) -> bool {
        let snapshot = self.snapshot_batch(preset, None);
        self.apply_all_snapshot(&snapshot)
    }

    fn confirm_batch_snapshot(
        &self,
        all: ApplyAllSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if all.targets.is_empty() {
            return;
        }
        let selected = self.selected_batch_snapshot(&all);
        let describe = |snapshot: &ApplyAllSnapshot| {
            let count = snapshot.targets.len();
            let noun = if count == 1 { "file" } else { "files" };
            format!("{count} {noun}; {} skipped.", snapshot.skipped)
        };
        let detail = format!(
            "Replace output settings.\nSelected for conversion: {}\nAll files: {}",
            describe(&selected),
            describe(&all),
        );
        let title = if all.donor.is_some() {
            "Apply output settings?".to_string()
        } else {
            format!("Apply \"{}\"?", all.preset.name)
        };
        let mut choices = Vec::new();
        if !selected.targets.is_empty() {
            choices.push((
                format!("Apply to selected ({})", selected.targets.len()),
                selected,
            ));
        }
        choices.push((format!("Apply to all ({})", all.targets.len()), all));
        let mut buttons: Vec<_> = choices
            .iter()
            .map(|(label, _)| PromptButton::ok(label.clone()))
            .collect();
        buttons.push(PromptButton::cancel("Cancel"));
        let receiver = window.prompt(PromptLevel::Warning, &title, Some(&detail), &buttons, cx);
        cx.spawn(async move |this, cx| {
            let Ok(answer) = receiver.await else {
                return;
            };
            let Some((_, snapshot)) = choices.into_iter().nth(answer) else {
                return;
            };
            this.update(cx, |root, cx| {
                if root.apply_all_snapshot(&snapshot) {
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }
}

pub(super) fn apply_all_control(
    availability: ApplyAllAvailability,
    palette: &'static theme::ThemePalette,
    window: &mut Window,
    cx: &mut Context<FrameRoot>,
) -> gpui::Div {
    let reason = match availability {
        ApplyAllAvailability::Hidden => return div(),
        ApplyAllAvailability::Disabled(reason) => Some(reason),
        ApplyAllAvailability::Enabled => None,
    };
    let enabled = reason.is_none();
    div()
        .p_4()
        .border_b_1()
        .border_color(color(palette.border_subtle))
        .flex()
        .flex_col()
        .gap_2()
        .child(
            frame_text_button(
                "settings-apply-all",
                "Apply to files…",
                if palette.is_light() {
                    ButtonVariant::Accent
                } else {
                    ButtonVariant::Secondary
                },
                false,
                enabled,
                palette,
                window,
                cx,
            )
            .w_full()
            .when_some(reason, gpui::StatefulInteractiveElement::aria_description)
            .on_click(cx.listener(move |root, _, window, cx| {
                cx.stop_propagation();
                if enabled {
                    root.confirm_apply_all_settings(window, cx);
                }
            })),
        )
        .when_some(reason, |body, reason| {
            body.child(
                settings_hint_text(reason, palette)
                    .id("settings-apply-all-reason")
                    .debug_selector(|| "settings-apply-all-reason".into()),
            )
        })
}

#[cfg(test)]
thread_local! { static CANDIDATES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

#[cfg(test)]
mod tests;
