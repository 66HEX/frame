//! Keep GPUI and Frame's canvas in sync with the desktop's motion preference.

use gpui::App;
#[cfg(all(not(test), target_os = "linux"))]
use gpui::AppContext;

// A failed read leaves the last known preference intact (including on portal
// restart). Polling is independent of rendering and never redraws unchanged UI.
fn apply_preference(preference: Option<bool>, cx: &mut App) {
    if let Some(reduce_motion) = preference {
        cx.set_reduce_motion(reduce_motion);
    }
}

#[cfg(not(test))]
pub(super) fn observe(cx: &mut App) {
    #[cfg(not(target_os = "linux"))]
    apply_preference(read_preference(), cx);

    cx.spawn(async move |cx| {
        loop {
            #[cfg(target_os = "linux")]
            let preference = cx.background_spawn(async { read_preference() }).await;
            #[cfg(not(target_os = "linux"))]
            let preference = read_preference();
            cx.update(|cx| apply_preference(preference, cx));
            let interval = if preference.is_some() { 1 } else { 5 };
            cx.background_executor()
                .timer(std::time::Duration::from_secs(interval))
                .await;
        }
    })
    .detach();
}

#[cfg(all(not(test), target_os = "macos"))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "Keep the platform reader contract; other platforms can fail to read the setting."
)]
fn read_preference() -> Option<bool> {
    Some(objc2_app_kit::NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion())
}

#[cfg(all(not(test), target_os = "windows"))]
fn read_preference() -> Option<bool> {
    use windows::Win32::UI::WindowsAndMessaging::{
        SPI_GETCLIENTAREAANIMATION, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
    };
    let mut enabled = windows::core::BOOL::default();
    // SAFETY: GETCLIENTAREAANIMATION writes exactly one BOOL to the valid output
    // pointer. This query does not change any Windows setting.
    unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some(std::ptr::from_mut(&mut enabled).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    }
    .ok()
    .map(|()| !enabled.as_bool())
}

#[cfg(all(not(test), target_os = "linux"))]
fn read_preference() -> Option<bool> {
    use ashpd::desktop::settings::{ReducedMotion, Settings};
    // D-Bus may be absent or unresponsive. Run off the UI thread with a bounded
    // wait; async-io drives the portal even though GPUI uses its own executor.
    smol::block_on(smol::future::or(
        async {
            let settings = Settings::new().await.ok()?;
            let preference = settings.reduced_motion().await.ok()?;
            Some(preference == ReducedMotion::ReducedMotion)
        },
        async {
            smol::Timer::after(std::time::Duration::from_secs(2)).await;
            None
        },
    ))
}

#[cfg(all(
    not(test),
    not(any(target_os = "macos", target_os = "windows", target_os = "linux"))
))]
fn read_preference() -> Option<bool> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preference_changes_apply_live_and_read_failure_preserves_the_last_value() {
        let cx = gpui::TestAppContext::single();
        cx.update(|cx| {
            assert!(!cx.reduce_motion());
            apply_preference(Some(true), cx);
            assert!(cx.reduce_motion());
            apply_preference(None, cx);
            assert!(cx.reduce_motion());
            apply_preference(Some(false), cx);
            assert!(!cx.reduce_motion());
        });
    }
}
