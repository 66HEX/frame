#include "shared_input.h"
#include <errno.h>
#include <stdio.h>
#include <string.h>

enum { MEDIA_PAGE_BYTES = 256 * 1024, MEDIA_PAGE_COUNT = 4 };
typedef struct {
    uint8_t *data;
    int64_t offset;
    int size;
} Page;
struct FrameMediaFile {
    AVIOContext *handle;
    int64_t size;
    Page pages[MEDIA_PAGE_COUNT];
};
typedef struct {
    FrameMediaFile *file;
    int64_t position;
} Cursor;

int frame_media_file_open(FrameMediaFile **result, const char *path) {
    FrameMediaFile *file = av_mallocz(sizeof(*file));
    if (!file)
        return AVERROR(ENOMEM);
    *result = file;
    int rc = avio_open(&file->handle, path, AVIO_FLAG_READ);
    if (rc < 0)
        return rc;
    file->size = avio_size(file->handle);
    return file->size < 0 ? (int)file->size : 0;
}

static int read_page(void *opaque, uint8_t *dest, int requested) {
    Cursor *cursor = opaque;
    FrameMediaFile *file = cursor->file;
    if (cursor->position >= file->size)
        return AVERROR_EOF;
    const int64_t number = cursor->position / MEDIA_PAGE_BYTES;
    Page *page = &file->pages[number % MEDIA_PAGE_COUNT];
    const int64_t start = number * MEDIA_PAGE_BYTES;
    if (!page->data || page->offset != start) {
        if (!page->data && !(page->data = av_malloc(MEDIA_PAGE_BYTES)))
            return AVERROR(ENOMEM);
        int64_t rc = avio_seek(file->handle, start, SEEK_SET);
        if (rc < 0)
            return (int)rc;
        page->size = avio_read(file->handle, page->data, MEDIA_PAGE_BYTES);
        if (page->size <= 0)
            return page->size < 0 ? page->size : AVERROR_EOF;
        page->offset = start;
    }
    int offset = (int)(cursor->position - start);
    int count = FFMIN(requested, page->size - offset);
    if (count <= 0)
        return AVERROR_EOF;
    memcpy(dest, page->data + offset, count);
    cursor->position += count;
    return count;
}

static int64_t seek_cursor(void *opaque, int64_t offset, int whence) {
    Cursor *cursor = opaque;
    int64_t base;
    switch (whence & ~AVSEEK_FORCE) {
    case AVSEEK_SIZE:
        return cursor->file->size;
    case SEEK_SET:
        base = 0;
        break;
    case SEEK_CUR:
        base = cursor->position;
        break;
    case SEEK_END:
        base = cursor->file->size;
        break;
    default:
        return AVERROR(EINVAL);
    }
    if (offset < -base || offset > INT64_MAX - base)
        return AVERROR(EINVAL);
    cursor->position = base + offset;
    return cursor->position;
}

int frame_media_cursor_open(FrameMediaFile *file, AVIOContext **result) {
    Cursor *cursor = av_mallocz(sizeof(*cursor));
    uint8_t *buffer = av_malloc(32768);
    if (cursor && buffer) {
        cursor->file = file;
        *result = avio_alloc_context(buffer, 32768, 0, cursor, read_page, NULL, seek_cursor);
        if (*result)
            return 0;
    }
    av_free(buffer);
    av_free(cursor);
    return AVERROR(ENOMEM);
}

void frame_media_cursor_close(AVIOContext **ctx) {
    if (!*ctx)
        return;
    av_freep(&(*ctx)->opaque);
    av_freep(&(*ctx)->buffer);
    avio_context_free(ctx);
}

void frame_media_file_close(FrameMediaFile **file) {
    if (!*file)
        return;
    avio_closep(&(*file)->handle);
    for (int i = 0; i < MEDIA_PAGE_COUNT; ++i)
        av_free((*file)->pages[i].data);
    av_freep(file);
}
