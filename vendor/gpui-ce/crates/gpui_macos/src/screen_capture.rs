use crate::display::screen_id;
use anyhow::{Result, anyhow};
use block2::RcBlock;
use collections::HashMap;
use core_foundation::base::TCFType;
use core_graphics::display::{
    CGDirectDisplayID, CGDisplayCopyDisplayMode, CGDisplayModeGetPixelHeight,
    CGDisplayModeGetPixelWidth, CGDisplayModeRelease,
};
use core_video::pixel_buffer::kCVPixelFormatType_420YpCbCr8BiPlanarFullRange;
use futures::channel::oneshot;
use gpui::{
    DevicePixels, ForegroundExecutor, ScreenCaptureFrame, ScreenCaptureSource, ScreenCaptureStream,
    SharedString, SourceMetadata, size,
};
use media::core_media::{CMSampleBuffer, CMSampleBufferRef};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{AnyThread, DefinedClass, MainThreadMarker, define_class, msg_send};
use objc2_app_kit::NSScreen;
use objc2_core_media::CMSampleBuffer as ObjcCMSampleBuffer;
use objc2_foundation::{NSArray, NSError, NSObject, NSObjectProtocol};
use objc2_screen_capture_kit::{
    SCContentFilter, SCDisplay, SCShareableContent, SCStream, SCStreamConfiguration,
    SCStreamDelegate, SCStreamOutput, SCStreamOutputType, SCWindow,
};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
pub struct MacScreenCaptureSource {
    sc_display: Retained<SCDisplay>,
    meta: Option<ScreenMeta>,
}

pub struct MacScreenCaptureStream {
    sc_stream: Retained<SCStream>,
    sc_stream_output: Retained<StreamOutput>,
    meta: SourceMetadata,
}

const SCREEN_CAPTURE_QUEUE_DEPTH: isize = 3;

impl ScreenCaptureSource for MacScreenCaptureSource {
    fn metadata(&self) -> Result<SourceMetadata> {
        let display_id = unsafe { self.sc_display.displayID() };
        let display_mode_ref = unsafe { CGDisplayCopyDisplayMode(display_id) };
        anyhow::ensure!(
            !display_mode_ref.is_null(),
            "display {display_id} no longer has an active display mode"
        );
        let size = unsafe {
            let width = CGDisplayModeGetPixelWidth(display_mode_ref);
            let height = CGDisplayModeGetPixelHeight(display_mode_ref);
            CGDisplayModeRelease(display_mode_ref);
            size(DevicePixels(width as i32), DevicePixels(height as i32))
        };
        let (label, is_main) = self
            .meta
            .clone()
            .map(|meta| (meta.label, meta.is_main))
            .unzip();

        Ok(SourceMetadata {
            id: display_id as u64,
            label,
            is_main,
            resolution: size,
        })
    }

    fn stream(
        &self,
        _foreground_executor: &ForegroundExecutor,
        frame_callback: Box<dyn Fn(ScreenCaptureFrame) + Send>,
    ) -> oneshot::Receiver<Result<Box<dyn ScreenCaptureStream>>> {
        let (tx, rx) = oneshot::channel();
        let meta = match self.metadata() {
            Ok(meta) => meta,
            Err(error) => {
                let _ = tx.send(Err(error));
                return rx;
            }
        };

        unsafe {
            let excluded_windows = NSArray::<SCWindow>::new();
            let filter = SCContentFilter::initWithDisplay_excludingWindows(
                SCContentFilter::alloc(),
                &self.sc_display,
                &excluded_windows,
            );
            let configuration = SCStreamConfiguration::new();
            configuration.setScalesToFit(true);
            configuration.setPixelFormat(kCVPixelFormatType_420YpCbCr8BiPlanarFullRange);
            configuration.setQueueDepth(SCREEN_CAPTURE_QUEUE_DEPTH);
            configuration.setWidth(meta.resolution.width.0 as usize);
            configuration.setHeight(meta.resolution.height.0 as usize);

            let delegate = StreamDelegate::new();
            let output = StreamOutput::new(frame_callback);
            let stream = SCStream::initWithFilter_configuration_delegate(
                SCStream::alloc(),
                &filter,
                &configuration,
                Some(ProtocolObject::from_ref(&*delegate)),
            );

            if let Err(error) = stream.addStreamOutput_type_sampleHandlerQueue_error(
                ProtocolObject::from_ref(&*output),
                SCStreamOutputType::Screen,
                None,
            ) {
                let _ = tx.send(Err(anyhow!(
                    "failed to add stream output {}",
                    error.localizedDescription()
                )));
                return rx;
            }

            let tx = Rc::new(RefCell::new(Some(tx)));
            let stream_for_completion = stream.clone();
            let output_for_completion = output.clone();
            let completion = RcBlock::new(move |error: *mut NSError| {
                let result = if let Some(error) = error.as_ref() {
                    Err(anyhow!(
                        "failed to start screen capture stream {}",
                        error.localizedDescription()
                    ))
                } else {
                    Ok(Box::new(MacScreenCaptureStream {
                        meta: meta.clone(),
                        sc_stream: stream_for_completion.clone(),
                        sc_stream_output: output_for_completion.clone(),
                    }) as Box<dyn ScreenCaptureStream>)
                };
                if let Some(tx) = tx.borrow_mut().take() {
                    tx.send(result).ok();
                }
            });
            stream.startCaptureWithCompletionHandler(Some(&completion));
        }
        rx
    }
}

impl ScreenCaptureStream for MacScreenCaptureStream {
    fn metadata(&self) -> Result<SourceMetadata> {
        Ok(self.meta.clone())
    }
}

impl Drop for MacScreenCaptureStream {
    fn drop(&mut self) {
        unsafe {
            if let Err(error) = self.sc_stream.removeStreamOutput_type_error(
                ProtocolObject::from_ref(&*self.sc_stream_output),
                SCStreamOutputType::Screen,
            ) {
                log::error!(
                    "failed to remove screen stream output {}",
                    error.localizedDescription()
                );
            }

            let completion = RcBlock::new(move |error: *mut NSError| {
                if let Some(error) = error.as_ref() {
                    log::error!(
                        "failed to stop screen capture stream {}",
                        error.localizedDescription()
                    );
                }
            });
            self.sc_stream
                .stopCaptureWithCompletionHandler(Some(&completion));
        }
    }
}

#[derive(Clone)]
struct ScreenMeta {
    label: SharedString,
    // Is this the screen with menu bar?
    is_main: bool,
}

fn screen_id_to_human_label() -> HashMap<CGDirectDisplayID, ScreenMeta> {
    let Some(mtm) = MainThreadMarker::new() else {
        return HashMap::default();
    };

    let screens = NSScreen::screens(mtm);
    let mut map = HashMap::default();
    for i in 0..screens.count() {
        let screen = screens.objectAtIndex(i);
        let Some(screen_id) = screen_id(&screen) else {
            log::warn!("NSScreen device description is missing NSScreenNumber");
            continue;
        };
        let name = screen.localizedName();
        map.insert(
            screen_id,
            ScreenMeta {
                label: name.to_string().into(),
                is_main: i == 0,
            },
        );
    }
    map
}

pub(crate) fn get_sources() -> oneshot::Receiver<Result<Vec<Rc<dyn ScreenCaptureSource>>>> {
    let (tx, rx) = oneshot::channel();
    let tx = Rc::new(RefCell::new(Some(tx)));
    let screen_id_to_label = screen_id_to_human_label();
    let block = RcBlock::new(
        move |shareable_content: *mut SCShareableContent, error: *mut NSError| {
            let Some(tx) = tx.borrow_mut().take() else {
                return;
            };

            let result = if let Some(error) = unsafe { error.as_ref() } {
                Err(anyhow!(
                    "Screen share failed: {}",
                    error.localizedDescription()
                ))
            } else if let Some(shareable_content) = unsafe { shareable_content.as_ref() } {
                let displays = unsafe { shareable_content.displays() };
                let mut result = Vec::new();
                for i in 0..displays.count() {
                    let display = displays.objectAtIndex(i);
                    let display_id = unsafe { display.displayID() };
                    let meta = screen_id_to_label.get(&display_id).cloned();
                    result.push(Rc::new(MacScreenCaptureSource {
                        sc_display: display,
                        meta,
                    }) as Rc<dyn ScreenCaptureSource>);
                }
                Ok(result)
            } else {
                Err(anyhow!("Screen share failed without content or error"))
            };
            tx.send(result).ok();
        },
    );

    unsafe {
        SCShareableContent::getShareableContentExcludingDesktopWindows_onScreenWindowsOnly_completionHandler(
            true,
            true,
            &block,
        );
    }
    rx
}

struct StreamOutputIvars {
    callback: Box<dyn Fn(ScreenCaptureFrame) + Send>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "GPUIStreamDelegate"]
    #[ivars = ()]
    struct StreamDelegate;

    unsafe impl NSObjectProtocol for StreamDelegate {}
    unsafe impl SCStreamDelegate for StreamDelegate {}
);

impl StreamDelegate {
    fn new() -> Retained<Self> {
        let this = Self::alloc().set_ivars(());
        // SAFETY: NSObject's `init` is its designated initializer.
        unsafe { msg_send![super(this), init] }
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "GPUIStreamOutput"]
    #[ivars = StreamOutputIvars]
    struct StreamOutput;

    unsafe impl NSObjectProtocol for StreamOutput {}
    unsafe impl SCStreamOutput for StreamOutput {
        // The Rust name mirrors the generated protocol requirement exactly.
        #[allow(non_snake_case)]
        #[unsafe(method(stream:didOutputSampleBuffer:ofType:))]
        fn stream_didOutputSampleBuffer_ofType(
            &self,
            _stream: &SCStream,
            sample_buffer: &ObjcCMSampleBuffer,
            buffer_type: SCStreamOutputType,
        ) {
            if buffer_type != SCStreamOutputType::Screen {
                return;
            }

            // ScreenCaptureKit's CMSampleBuffer and gpui-media's CoreMedia
            // wrapper are both the same opaque CoreMedia reference. Keep the
            // conversion at this boundary while using the generated typed
            // ScreenCaptureKit protocol method above.
            let sample_buffer = sample_buffer as *const ObjcCMSampleBuffer as CMSampleBufferRef;
            let sample_buffer = unsafe { CMSampleBuffer::wrap_under_get_rule(sample_buffer) };
            if let Some(buffer) = sample_buffer.image_buffer() {
                (self.ivars().callback)(ScreenCaptureFrame(buffer));
            }
        }
    }
);

impl StreamOutput {
    fn new(callback: Box<dyn Fn(ScreenCaptureFrame) + Send>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(StreamOutputIvars { callback });
        // SAFETY: NSObject's `init` is its designated initializer.
        unsafe { msg_send![super(this), init] }
    }
}
