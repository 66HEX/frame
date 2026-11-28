// Return the identity paths of the six loaded FFmpeg libraries. Paths are
// NUL-delimited so Unix filenames can contain newlines and non-UTF-8 bytes.
#ifndef _WIN32
#define _GNU_SOURCE
#include <dlfcn.h>
#else
#include <windows.h>
#include <fcntl.h>
#include <io.h>
#endif
#include <stdio.h>
#include <string.h>
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavfilter/avfilter.h>
#include <libavutil/avutil.h>
#include <libswresample/swresample.h>
#include <libswscale/swscale.h>

int frame_library_paths(void)
{
#ifdef _WIN32
#define DLL(name, version) #name "-" AV_STRINGIFY(version) ".dll"
    const char *names[] = {DLL(avcodec, LIBAVCODEC_VERSION_MAJOR),
        DLL(avformat, LIBAVFORMAT_VERSION_MAJOR), DLL(avfilter, LIBAVFILTER_VERSION_MAJOR),
        DLL(avutil, LIBAVUTIL_VERSION_MAJOR), DLL(swscale, LIBSWSCALE_VERSION_MAJOR),
        DLL(swresample, LIBSWRESAMPLE_VERSION_MAJOR)};
    if (_setmode(_fileno(stdout), _O_BINARY) < 0) return -1;
    for (size_t i = 0; i < sizeof(names) / sizeof(*names); i++) {
        wchar_t wide[32768];
        HMODULE module = GetModuleHandleA(names[i]);
        if (!module) return -1;
        DWORD length = GetModuleFileNameW(module, wide, 32768);
        if (!length || length >= 32768) return -1;
        char *utf8 = av_malloc((size_t)length * 4 + 1);
        if (!utf8) return -1;
        int bytes = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, wide, (int)length + 1,
            utf8, (int)length * 4 + 1, NULL, NULL);
        int ok = bytes > 0 && fwrite(utf8, 1, bytes, stdout) == (size_t)bytes;
        av_free(utf8);
        if (!ok) return -1;
    }
#else
    unsigned (*libraries[])(void) = {avcodec_version, avformat_version, avfilter_version,
                                     avutil_version, swscale_version, swresample_version};
    for (size_t i = 0; i < sizeof(libraries) / sizeof(*libraries); i++) {
        Dl_info info;
        if (!dladdr((const void *)libraries[i], &info) || !info.dli_fname) return -1;
        size_t size = strlen(info.dli_fname) + 1;
        if (fwrite(info.dli_fname, 1, size, stdout) != size) return -1;
    }
#endif
    return fflush(stdout);
}

int frame_library_versions(void)
{
    return printf("FFmpeg %s (avformat %u, avcodec %u, avfilter %u)\n",
        av_version_info(), avformat_version(), avcodec_version(), avfilter_version()) < 0 ? -1 : 0;
}
