#ifndef FRAME_SHARED_INPUT_H
#define FRAME_SHARED_INPUT_H
#include <libavformat/avformat.h>

typedef struct FrameMediaFile FrameMediaFile;
/* Single-threaded renderers can seek independently inside one local file.
 * The owning Source outlives all cursors and frees the shared buffer last. */
int frame_media_file_open(FrameMediaFile **file, const char *path);
void frame_media_file_close(FrameMediaFile **file);
int frame_media_cursor_open(FrameMediaFile *file, AVIOContext **cursor);
void frame_media_cursor_close(AVIOContext **cursor);
#endif
