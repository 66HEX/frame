use crate::types::{
    CancelledPayload, CompletedPayload, ErrorPayload, LogPayload, ProgressPayload, StartedPayload,
};

pub const CONVERSION_STARTED_EVENT: &str = "conversion-started";
pub const CONVERSION_PROGRESS_EVENT: &str = "conversion-progress";
pub const CONVERSION_COMPLETED_EVENT: &str = "conversion-completed";
pub const CONVERSION_ERROR_EVENT: &str = "conversion-error";
pub const CONVERSION_LOG_EVENT: &str = "conversion-log";
pub const CONVERSION_CANCELLED_EVENT: &str = "conversion-cancelled";

#[derive(Clone, Debug, PartialEq)]
pub enum ConversionEvent {
    Started(StartedPayload),
    Progress(ProgressPayload),
    Completed(CompletedPayload),
    Error(ErrorPayload),
    Log(LogPayload),
    Cancelled(CancelledPayload),
}

impl ConversionEvent {
    #[must_use]
    pub fn started(id: impl Into<String>) -> Self {
        Self::Started(StartedPayload { id: id.into() })
    }

    #[must_use]
    pub fn progress(id: impl Into<String>, progress: f64) -> Self {
        Self::Progress(ProgressPayload {
            id: id.into(),
            progress,
        })
    }

    #[must_use]
    pub fn completed(id: impl Into<String>, output_path: impl Into<String>) -> Self {
        Self::Completed(CompletedPayload {
            id: id.into(),
            output_path: output_path.into(),
        })
    }

    #[must_use]
    pub fn error(id: impl Into<String>, error: impl Into<String>) -> Self {
        Self::Error(ErrorPayload {
            id: id.into(),
            error: error.into(),
        })
    }

    #[must_use]
    pub fn log(id: impl Into<String>, line: impl Into<String>) -> Self {
        Self::Log(LogPayload {
            id: id.into(),
            line: line.into(),
        })
    }

    #[must_use]
    pub fn cancelled(id: impl Into<String>) -> Self {
        Self::Cancelled(CancelledPayload { id: id.into() })
    }

    #[must_use]
    pub const fn event_name(&self) -> &'static str {
        match self {
            Self::Started(_) => CONVERSION_STARTED_EVENT,
            Self::Progress(_) => CONVERSION_PROGRESS_EVENT,
            Self::Completed(_) => CONVERSION_COMPLETED_EVENT,
            Self::Error(_) => CONVERSION_ERROR_EVENT,
            Self::Log(_) => CONVERSION_LOG_EVENT,
            Self::Cancelled(_) => CONVERSION_CANCELLED_EVENT,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Started(payload) => &payload.id,
            Self::Progress(payload) => &payload.id,
            Self::Completed(payload) => &payload.id,
            Self::Error(payload) => &payload.id,
            Self::Log(payload) => &payload.id,
            Self::Cancelled(payload) => &payload.id,
        }
    }
}

pub trait ConversionEventSink {
    type Error;

    fn emit_conversion_event(&self, event: ConversionEvent) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default)]
    struct CollectingSink {
        events: RefCell<Vec<ConversionEvent>>,
    }

    impl ConversionEventSink for CollectingSink {
        type Error = ();

        fn emit_conversion_event(&self, event: ConversionEvent) -> Result<(), Self::Error> {
            self.events.borrow_mut().push(event);
            Ok(())
        }
    }

    #[test]
    fn conversion_event_event_name_matches_public_contract() {
        assert_eq!(
            ConversionEvent::progress("task-1", 42.0).event_name(),
            CONVERSION_PROGRESS_EVENT
        );
    }

    #[test]
    fn conversion_event_id_returns_wrapped_payload_id() {
        let event = ConversionEvent::completed("task-2", "/tmp/output.mp4");

        assert_eq!(event.id(), "task-2");
    }

    #[test]
    fn conversion_event_constructors_preserve_payload_data() {
        let event = ConversionEvent::error("task-3", "ffmpeg failed");

        assert_eq!(
            event,
            ConversionEvent::Error(ErrorPayload {
                id: "task-3".to_string(),
                error: "ffmpeg failed".to_string(),
            })
        );
    }

    #[test]
    fn conversion_event_sink_accepts_native_events() {
        let sink = CollectingSink::default();

        sink.emit_conversion_event(ConversionEvent::log("task-4", "[INFO] queued"))
            .expect("collecting sink should not fail");

        assert_eq!(
            sink.events.borrow().as_slice(),
            [ConversionEvent::log("task-4", "[INFO] queued")]
        );
    }
}
