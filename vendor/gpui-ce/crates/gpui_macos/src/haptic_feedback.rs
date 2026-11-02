use gpui::HapticFeedbackStyle;
use objc2_app_kit::{
    NSHapticFeedbackManager, NSHapticFeedbackPattern, NSHapticFeedbackPerformanceTime,
    NSHapticFeedbackPerformer,
};

/// macOS haptic feedback using [`NSHapticFeedbackManager`].
///
/// Delivers transient taps through the Force Touch trackpad (macOS 10.11+).
/// Fire and forget. On machines without haptic hardware, calls are silently ignored by AppKit.
pub(crate) struct MacHaptics {
    supported: bool,
}

impl MacHaptics {
    pub fn new(headless: bool) -> Self {
        Self {
            supported: !headless,
        }
    }

    pub fn supported(&self) -> bool {
        self.supported
    }

    fn pattern_for_style(style: HapticFeedbackStyle) -> NSHapticFeedbackPattern {
        match style {
            HapticFeedbackStyle::Generic => NSHapticFeedbackPattern::Generic,
            HapticFeedbackStyle::Alignment => NSHapticFeedbackPattern::Alignment,
            HapticFeedbackStyle::LevelChange => NSHapticFeedbackPattern::LevelChange,
        }
    }

    pub fn play(&self, style: HapticFeedbackStyle) {
        if !self.supported {
            return;
        }

        let pattern = Self::pattern_for_style(style);

        // NSHapticFeedbackManager is available on macOS 10.11+; platform methods
        // are invoked on the main thread.
        let manager = NSHapticFeedbackManager::defaultPerformer();
        manager
            .performFeedbackPattern_performanceTime(pattern, NSHapticFeedbackPerformanceTime::Now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported() {
        let haptics = MacHaptics::new(false);
        assert!(haptics.supported());
    }

    #[test]
    fn test_headless_is_unsupported() {
        let haptics = MacHaptics::new(true);
        assert!(!haptics.supported());
    }

    #[test]
    fn test_style_to_pattern_mapping() {
        assert_eq!(
            MacHaptics::pattern_for_style(HapticFeedbackStyle::Generic),
            NSHapticFeedbackPattern::Generic
        );
        assert_eq!(
            MacHaptics::pattern_for_style(HapticFeedbackStyle::Alignment),
            NSHapticFeedbackPattern::Alignment
        );
        assert_eq!(
            MacHaptics::pattern_for_style(HapticFeedbackStyle::LevelChange),
            NSHapticFeedbackPattern::LevelChange
        );
    }
}
