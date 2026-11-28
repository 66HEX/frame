#ifndef FRAME_HARDWARE_H
#define FRAME_HARDWARE_H

#include <libavcodec/avcodec.h>
#include <libavutil/hwcontext.h>

/* Frame's filter graphs operate on host samples. The encoder can either consume
 * those samples directly or require a surface on the selected device. */
typedef struct {
    enum AVHWDeviceType type;
    AVBufferRef *device;
    char device_name[80];
    const char *decoder;
    const char *fallback;
    enum AVPixelFormat sample_format;
    int upload;
} FrameHardwareRoute;

int frame_hardware_route(FrameHardwareRoute *route, const AVCodec *encoder,
                         enum AVPixelFormat samples, const char *requested_device);
int frame_prepare_surface(AVCodecContext *ctx, const FrameHardwareRoute *route);
int frame_send_video_frame(AVCodecContext *ctx, AVFrame *samples);
enum AVPixelFormat frame_surface_samples(const AVCodec *codec, const char *requested,
                                         enum AVPixelFormat input);
int frame_check_encoder(const char *name, int allow_sw, const char *sample_format,
                        const uint8_t *input);
#endif
