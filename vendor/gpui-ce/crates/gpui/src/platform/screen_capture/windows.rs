use super::*;
use crate::{DevicePixels, WindowsScreenCaptureFrame};
use windows_capture::monitor::Monitor;

pub(super) fn screen_sources(
    foreground_executor: &ForegroundExecutor,
) -> oneshot::Receiver<Result<Vec<Rc<dyn ScreenCaptureSource>>>> {
    let (sources_tx, sources_rx) = oneshot::channel();
    std::thread::spawn(|| {
        let sources = Monitor::enumerate().map(|monitors| {
            let primary = Monitor::primary().ok();
            monitors
                .into_iter()
                .enumerate()
                .filter_map(
                    |(index, monitor)| match CaptureSource::new(monitor, index) {
                        Ok(mut source) => {
                            source.metadata.is_main = primary.map(|primary| primary == monitor);
                            Some(source)
                        }
                        Err(error) => {
                            log::warn!("ignoring an unavailable Windows monitor: {error}");
                            None
                        }
                    },
                )
                .collect()
        });
        sources_tx.send(sources.map_err(anyhow::Error::from)).ok();
    });
    to_dyn_screen_capture_sources(sources_rx, foreground_executor)
}

struct CaptureSource {
    monitor: Monitor,
    metadata: SourceMetadata,
}

impl CaptureSource {
    fn new(
        monitor: Monitor,
        index: usize,
    ) -> std::result::Result<Self, windows_capture::monitor::Error> {
        let width = monitor.width()?;
        let height = monitor.height()?;
        let label = monitor
            .name()
            .or_else(|_| monitor.device_name())
            .ok()
            .map(Into::into);
        Ok(Self {
            monitor,
            metadata: SourceMetadata {
                resolution: crate::size(DevicePixels(width as i32), DevicePixels(height as i32)),
                label,
                is_main: None,
                id: index as u64 + 1,
            },
        })
    }
}

impl ScreenCaptureSource for CaptureSource {
    fn metadata(&self) -> Result<SourceMetadata> {
        Ok(self.metadata.clone())
    }

    fn stream(
        &self,
        foreground_executor: &ForegroundExecutor,
        frame_callback: Box<dyn Fn(ScreenCaptureFrame) + Send>,
    ) -> oneshot::Receiver<Result<Box<dyn ScreenCaptureStream>>> {
        capture_monitor(
            self.monitor,
            &self.metadata,
            foreground_executor,
            frame_callback,
        )
    }
}

fn capture_monitor(
    monitor: Monitor,
    metadata: &SourceMetadata,
    foreground_executor: &ForegroundExecutor,
    frame_callback: Box<dyn Fn(ScreenCaptureFrame) + Send>,
) -> oneshot::Receiver<Result<Box<dyn ScreenCaptureStream>>> {
    use windows_capture::{
        capture::GraphicsCaptureApiHandler as _,
        settings::{
            ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
            MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
        },
    };

    let (stream_tx, stream_rx) = oneshot::channel();
    let settings = Settings::new(
        monitor,
        CursorCaptureSettings::WithCursor,
        DrawBorderSettings::Default,
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Bgra8,
        frame_callback,
    );
    let stream = CaptureHandler::start_free_threaded(settings)
        .map(|capture| CaptureStream {
            capture: Some(capture),
            metadata: metadata.clone(),
        })
        .map_err(anyhow::Error::from);
    stream_tx.send(stream).ok();
    to_dyn_screen_capture_stream(stream_rx, foreground_executor)
}

struct CaptureHandler {
    frame_callback: Box<dyn Fn(ScreenCaptureFrame) + Send>,
}

impl windows_capture::capture::GraphicsCaptureApiHandler for CaptureHandler {
    type Flags = Box<dyn Fn(ScreenCaptureFrame) + Send>;
    type Error = anyhow::Error;

    fn new(
        context: windows_capture::capture::Context<Self::Flags>,
    ) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            frame_callback: context.flags,
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut windows_capture::frame::Frame,
        _control: windows_capture::graphics_capture_api::InternalCaptureControl,
    ) -> std::result::Result<(), Self::Error> {
        let texture = unsafe { frame.as_raw_texture() }.clone();
        let frame = WindowsScreenCaptureFrame::new(
            texture,
            crate::size(
                DevicePixels(frame.width() as i32),
                DevicePixels(frame.height() as i32),
            ),
            frame.timestamp().Duration.max(0) as u64,
        );
        (self.frame_callback)(ScreenCaptureFrame(frame));
        Ok(())
    }

    fn on_closed(&mut self) -> std::result::Result<(), Self::Error> {
        Ok(())
    }
}

struct CaptureStream {
    capture: Option<windows_capture::capture::CaptureControl<CaptureHandler, anyhow::Error>>,
    metadata: SourceMetadata,
}

impl ScreenCaptureStream for CaptureStream {
    fn metadata(&self) -> Result<SourceMetadata> {
        Ok(self.metadata.clone())
    }
}

impl Drop for CaptureStream {
    fn drop(&mut self) {
        if let Some(capture) = self.capture.take()
            && let Err(error) = capture.stop()
        {
            log::error!("failed to stop Windows screen capture: {error}");
        }
    }
}
