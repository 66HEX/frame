#include "hardware.h"
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <libavutil/avstring.h>
#include <libavfilter/avfilter.h>
#include <libavutil/pixdesc.h>
#include <libavutil/intreadwrite.h>
#ifdef __linux__
#include <glob.h>
#endif
#ifdef _WIN32
#define COBJMACROS
#include <windows.h>
#include <dxgi.h>
#include <libavutil/hwcontext_d3d11va.h>
#endif

static int vaapi(const AVCodec *codec) {
    return av_match_name(codec->name, "h264_vaapi,hevc_vaapi,av1_vaapi");
}

enum AVPixelFormat frame_surface_samples(const AVCodec *codec, const char *requested,
                                         enum AVPixelFormat input) {
    if (!vaapi(codec) && strcmp(codec->name, "hevc_amf"))
        return AV_PIX_FMT_NONE;
    if (requested) {
        if (!strcmp(requested, "nv12") || !strcmp(requested, "yuv420p"))
            return AV_PIX_FMT_NV12;
        if (codec->id != AV_CODEC_ID_H264 &&
            (!strcmp(requested, "p010le") || !strcmp(requested, "yuv420p10le")))
            return AV_PIX_FMT_P010LE;
        return AV_PIX_FMT_NONE;
    }
    const AVPixFmtDescriptor *desc = av_pix_fmt_desc_get(input);
    return codec->id != AV_CODEC_ID_H264 && desc && desc->comp[0].depth > 8
               ? AV_PIX_FMT_P010LE : AV_PIX_FMT_NV12;
}

const char *frame_filter_mapping(const char *backend, const char *graph, int transpose_available) {
    int cuda = !strcmp(backend, "cuda"), va = !strcmp(backend, "vaapi");
    if (!cuda && !va && strcmp(backend, "d3d11va") && strcmp(backend, "videotoolbox"))
        return NULL;
    if (!graph || !strcmp(graph, "null"))
        return "null";
    if (!transpose_available || (!cuda && !va))
        return NULL;
    static const struct { const char *host, *cuda, *va; } transforms[] = {
        {"transpose=1", "transpose_cuda=clock", "transpose_vaapi=clock"},
        {"transpose=2", "transpose_cuda=cclock", "transpose_vaapi=cclock"},
        {"transpose=1,transpose=1", "transpose_cuda=reversal", "transpose_vaapi=reversal"},
        {"hflip", "transpose_cuda=hflip", "transpose_vaapi=hflip"},
        {"vflip", "transpose_cuda=vflip", "transpose_vaapi=vflip"},
    };
    for (size_t i = 0; i < sizeof(transforms) / sizeof(*transforms); ++i)
        if (!strcmp(graph, transforms[i].host))
            return cuda ? transforms[i].cuda : transforms[i].va;
    // Keep mixed graphs, exact edge padding, overlays and subtitle rasterization
    // together on the host. Substituting only one filter can change the result.
    return NULL;
}

const char *frame_surface_graph(const AVCodec *codec, const char *backend, const char *graph) {
    if (!backend)
        return NULL;
    enum AVHWDeviceType hw = av_hwdevice_find_type_by_name(backend);
    const AVCodecHWConfig *config;
    for (int i = 0; (config = avcodec_get_hw_config(codec, i)); ++i) {
        if (config->device_type != hw || !(config->methods & AV_CODEC_HW_CONFIG_METHOD_HW_FRAMES_CTX))
            continue;
        const char *transpose = hw == AV_HWDEVICE_TYPE_CUDA ? "transpose_cuda" : "transpose_vaapi";
        return frame_filter_mapping(backend, graph, avfilter_get_by_name(transpose) != NULL);
    }
    return NULL;
}

int frame_prepare_surface(AVCodecContext *ctx, const FrameHardwareRoute *route) {
    if (!route->device)
        return 0;
    ctx->hw_device_ctx = av_buffer_ref(route->device);
    if (!ctx->hw_device_ctx)
        return AVERROR(ENOMEM);
    if (!route->upload)
        return 0;
    ctx->hw_frames_ctx = av_hwframe_ctx_alloc(route->device);
    if (!ctx->hw_frames_ctx)
        return AVERROR(ENOMEM);
    AVHWFramesContext *pool = (AVHWFramesContext *)ctx->hw_frames_ctx->data;
    pool->format = AV_PIX_FMT_VAAPI;
    pool->sw_format = ctx->pix_fmt;
    pool->width = ctx->width;
    pool->height = ctx->height;
    ctx->pix_fmt = pool->format;
    return av_hwframe_ctx_init(ctx->hw_frames_ctx);
}

int frame_send_video_frame(AVCodecContext *ctx, AVFrame *samples) {
    if (!ctx->hw_frames_ctx)
        return avcodec_send_frame(ctx, samples);
    if (samples->hw_frames_ctx) {
        if (samples->hw_frames_ctx->data != ctx->hw_frames_ctx->data) {
            fprintf(stderr, "The source surface pool changed after preflight. Retry with CPU decoding.\n");
            return AVERROR_INPUT_CHANGED;
        }
        return avcodec_send_frame(ctx, samples);
    }
    AVFrame *surface = av_frame_alloc();
    if (!surface)
        return AVERROR(ENOMEM);
    int err = av_hwframe_get_buffer(ctx->hw_frames_ctx, surface, 0);
    if (err >= 0)
        err = av_hwframe_transfer_data(surface, samples, 0);
    if (err >= 0)
        err = av_frame_copy_props(surface, samples);
    if (err >= 0)
        err = avcodec_send_frame(ctx, surface);
    av_frame_free(&surface);
    return err;
}

/* The probe protocol always carries a planar 8-bit test image. Convert it to
 * the actual sample format being proved before sending it to the encoder. */
static void write_probe_plane(AVFrame *f, int plane, const uint8_t *input) {
    int source_size = plane ? 64 : 128;
    int width = plane ? f->width / 2 : f->width;
    int height = plane ? f->height / 2 : f->height;
    int interleaved = f->format == AV_PIX_FMT_NV12 || f->format == AV_PIX_FMT_P010LE;
    int deep = f->format == AV_PIX_FMT_P010LE;
    int dst_plane = interleaved && plane ? 1 : plane;
    for (int y = 0; y < height; ++y) {
        uint8_t *row = f->data[dst_plane] + y * f->linesize[dst_plane];
        for (int x = 0; x < width; ++x) {
            int pos = interleaved && plane ? 2 * x + plane - 1 : x;
            uint8_t value = input[(y % source_size) * source_size + x % source_size];
            if (deep)
                AV_WL16(row + 2 * pos, ((uint16_t)value) << 8);
            else
                row[pos] = value;
        }
    }
}

static int trial(const AVCodec *codec, const FrameHardwareRoute *route, int allow_sw,
                 const uint8_t *input) {
    AVCodecContext *ctx = avcodec_alloc_context3(codec);
    AVPacket *pkt = av_packet_alloc();
    AVFrame *frame = av_frame_alloc();
    AVDictionary *options = NULL;
    int err = AVERROR(ENOMEM);
    if (!ctx || !pkt || !frame)
        goto done;
    // Some devices reject tiny pictures even for a supported codec/profile.
    ctx->width = 320;
    ctx->height = 256;
    ctx->time_base = (AVRational){1, 30};
    ctx->framerate = (AVRational){30, 1};
    ctx->bit_rate = 1000000;
    ctx->pix_fmt = route->sample_format;
    if (strstr(codec->name, "_videotoolbox")) {
        av_dict_set(&options, "allow_sw", allow_sw ? "1" : "0", 0);
        av_dict_set(&options, "require_sw", "0", 0);
    }
    if ((err = frame_prepare_surface(ctx, route)) < 0 ||
        (err = avcodec_open2(ctx, codec, &options)) < 0)
        goto done;
    if (av_dict_count(options)) {
        err = AVERROR_OPTION_NOT_FOUND;
        goto done;
    }
    frame->width = ctx->width;
    frame->height = ctx->height;
    frame->format = route->sample_format;
    frame->pts = 0;
    if ((err = av_frame_get_buffer(frame, 32)) < 0)
        goto done;
    write_probe_plane(frame, 0, input);
    write_probe_plane(frame, 1, input + 128 * 128);
    write_probe_plane(frame, 2, input + 128 * 128 + 64 * 64);
    if ((err = frame_send_video_frame(ctx, frame)) < 0)
        goto done;
    err = avcodec_receive_packet(ctx, pkt);
    if (err == AVERROR(EAGAIN) && (err = avcodec_send_frame(ctx, NULL)) >= 0)
        err = avcodec_receive_packet(ctx, pkt);
    if (err >= 0 && pkt->size <= 0)
        err = AVERROR_INVALIDDATA;
done:
    av_dict_free(&options);
    avcodec_free_context(&ctx);
    av_packet_free(&pkt);
    av_frame_free(&frame);
    return err;
}

#ifdef _WIN32
static int amd_adapter_name(AVBufferRef *ref, char *name, size_t size) {
    AVHWDeviceContext *hw = (AVHWDeviceContext *)ref->data;
    AVD3D11VADeviceContext *d3d = hw->hwctx;
    // IID_IDXGIDevice, kept local to avoid a dependency on the DXGUID import library.
    static const GUID iid = {0x54ec77fa, 0x1377, 0x44e6, {0x8c,0x32,0x88,0xfd,0x5f,0x44,0xc8,0x4c}};
    IDXGIDevice *dxgi = NULL;
    IDXGIAdapter *adapter = NULL;
    DXGI_ADAPTER_DESC desc;
    HRESULT hr = ID3D11Device_QueryInterface(d3d->device, &iid, (void **)&dxgi);
    if (SUCCEEDED(hr))
        hr = IDXGIDevice_GetAdapter(dxgi, &adapter);
    if (SUCCEEDED(hr))
        hr = IDXGIAdapter_GetDesc(adapter, &desc);
    if (adapter)
        IDXGIAdapter_Release(adapter);
    if (dxgi)
        IDXGIDevice_Release(dxgi);
    if (FAILED(hr) || desc.VendorId != 0x1002)
        return AVERROR(ENODEV);
    snprintf(name, size, "amd-%08lx-%08lx", (unsigned long)desc.AdapterLuid.HighPart,
             (unsigned long)desc.AdapterLuid.LowPart);
    return 0;
}

static int select_radeon(FrameHardwareRoute *r, const AVCodec *codec, const char *pin,
                         int allow_sw, const uint8_t *input) {
    static const GUID factory_iid = {0x7b7166ec,0x21c7,0x44ae,{0xb2,0x1a,0xc9,0xae,0x32,0x1a,0xe3,0x69}};
    IDXGIFactory *factory = NULL;
    if (FAILED(CreateDXGIFactory(&factory_iid, (void **)&factory)))
        return AVERROR(ENODEV);
    int err = AVERROR(ENODEV);
    for (UINT i = 0;; ++i) {
        IDXGIAdapter *adapter = NULL;
        DXGI_ADAPTER_DESC desc;
        if (FAILED(IDXGIFactory_EnumAdapters(factory, i, &adapter)))
            break;
        HRESULT hr = IDXGIAdapter_GetDesc(adapter, &desc);
        IDXGIAdapter_Release(adapter);
        if (FAILED(hr) || desc.VendorId != 0x1002)
            continue;
        char name[80];
        snprintf(name, sizeof(name), "amd-%08lx-%08lx", (unsigned long)desc.AdapterLuid.HighPart,
                 (unsigned long)desc.AdapterLuid.LowPart);
        if (pin && strcmp(pin, name))
            continue;
        char index[16];
        snprintf(index, sizeof(index), "%u", i);
        err = av_hwdevice_ctx_create(&r->device, r->type, index, NULL, 0);
        if (err >= 0)
            err = amd_adapter_name(r->device, r->device_name, sizeof(r->device_name));
        // DXGI adapter indices can change while an external GPU is removed.
        if (err >= 0 && strcmp(name, r->device_name))
            err = AVERROR(ENODEV);
        if (err >= 0)
            err = trial(codec, r, allow_sw, input);
        if (err >= 0)
            break;
        av_buffer_unref(&r->device);
    }
    IDXGIFactory_Release(factory);
    return err;
}
#endif

static int select_device(FrameHardwareRoute *r, const AVCodec *codec, const char *pin,
                         int allow_sw, const uint8_t *input) {
#if !defined(__linux__) && !defined(_WIN32)
    (void)allow_sw;
    (void)input;
#endif
    if (vaapi(codec)) {
        r->type = AV_HWDEVICE_TYPE_VAAPI;
        r->upload = 1;
#ifdef __linux__
        glob_t nodes = {0};
        int err = AVERROR(ENODEV);
        if (glob("/dev/dri/renderD[0-9]*", 0, NULL, &nodes) == 0) {
            for (size_t i = 0; i < nodes.gl_pathc; ++i) {
                const char *node = nodes.gl_pathv[i];
                if (pin && strcmp(node, pin))
                    continue;
                if (strlen(node) >= sizeof(r->device_name) ||
                    strspn(node + strlen("/dev/dri/renderD"), "0123456789") !=
                        strlen(node + strlen("/dev/dri/renderD")))
                    continue;
                err = av_hwdevice_ctx_create(&r->device, r->type, node, NULL, 0);
                if (err >= 0)
                    err = trial(codec, r, allow_sw, input);
                if (err >= 0) {
                    av_strlcpy(r->device_name, node, sizeof(r->device_name));
                    break;
                }
                av_buffer_unref(&r->device);
            }
        }
        globfree(&nodes);
        return err;
#else
        return AVERROR(ENOSYS);
#endif
    }
    if (!strcmp(codec->name, "hevc_amf")) {
        r->type = AV_HWDEVICE_TYPE_D3D11VA;
#ifdef _WIN32
        return select_radeon(r, codec, pin, allow_sw, input);
#else
        return AVERROR(ENOSYS);
#endif
    }
    if (pin && strcmp(pin, "default"))
        return AVERROR(ENODEV);
    av_strlcpy(r->device_name, "default", sizeof(r->device_name));
    return 0;
}

int frame_hardware_route(FrameHardwareRoute *r, const AVCodec *encoder,
                         enum AVPixelFormat samples, const char *requested_device) {
    static const uint8_t pattern[128 * 128 * 3 / 2] = {0};
    if ((vaapi(encoder) || !strcmp(encoder->name, "hevc_amf")) && samples == AV_PIX_FMT_NONE)
        return AVERROR(EINVAL);
    r->sample_format = samples;
    r->decoder = "cpu";
    r->filters = "cpu";
    return select_device(r, encoder, requested_device, 0, pattern);
}

int frame_check_encoder(const char *name, int allow_sw, const char *sample_format,
                        const uint8_t *input) {
    const AVCodec *codec = avcodec_find_encoder_by_name(name);
    if (!codec)
        return AVERROR_ENCODER_NOT_FOUND;
    FrameHardwareRoute route = {0};
    if (vaapi(codec) || !strcmp(name, "hevc_amf"))
        route.sample_format = frame_surface_samples(codec, sample_format, AV_PIX_FMT_YUV420P);
    else
        route.sample_format = sample_format ? av_get_pix_fmt(sample_format) : AV_PIX_FMT_YUV420P;
    if (route.sample_format != AV_PIX_FMT_NV12 && route.sample_format != AV_PIX_FMT_P010LE &&
        route.sample_format != AV_PIX_FMT_YUV420P)
        return AVERROR(EINVAL);
    int err = select_device(&route, codec, NULL, allow_sw, input);
    if (err >= 0 && !route.device)
        err = trial(codec, &route, allow_sw, input);
    if (err >= 0)
        fprintf(stderr, "Encoder %s: device=%s samples=%s\n", name, route.device_name,
                av_get_pix_fmt_name(route.sample_format));
    av_buffer_unref(&route.device);
    return err;
}
