#include "convert.h"

static int has_name(const char *key, const char *name) {
    size_t len = strlen(name);
    return strncmp(key, name, len) == 0 && (key[len] == 0 || key[len] == ':');
}
static int any_name(const char *key, const char *const *names) {
    for (; *names; names++)
        if (has_name(key, *names))
            return 1;
    return 0;
}

const char *frame_plan_opt(const ConvertPlan *plan, const char *name) {
    for (int i = plan->options_len - 1; i >= 0; --i)
        if (!strcmp(plan->options[i].name, name))
            return plan->options[i].value;
    return NULL;
}

int frame_option_matches(const char *option, const char *name, const ConvertTrack *track) {
    if (!has_name(option, name))
        return 0;
    const char *suffix = option + strlen(name);
    if (!*suffix)
        return 1;
    if (suffix[1] != track->kind)
        return 0;
    if (!suffix[2])
        return 1;
    if (suffix[2] != ':' || !suffix[3])
        return 0;
    char *end;
    long index = strtol(suffix + 3, &end, 10);
    return !*end && index == track->ordinal;
}

const char *frame_track_opt(const ConvertPlan *plan, const ConvertTrack *track, const char *name) {
    for (int i = plan->options_len - 1; i >= 0; --i)
        if (frame_option_matches(plan->options[i].name, name, track))
            return plan->options[i].value;
    return NULL;
}

int frame_convert_parse(ConvertPlan *plan, int argc, const char *const *argv) {
    static const char *const flags[] = {"n",  "y",  "xerror", "nostdin", "hide_banner", "nostats",
                                        "an", "vn", "sn",     "dn",      "copyinkf",    NULL};
    static const char *const values[] = {"i",
                                         "ss",
                                         "t",
                                         "to",
                                         "f",
                                         "map",
                                         "progress",
                                         "loglevel",
                                         "map_metadata",
                                         "metadata",
                                         "disposition",
                                         "c",
                                         "bsf",
                                         "frame_size",
                                         "tag",
                                         "pix_fmt",
                                         "vf",
                                         "af",
                                         "filter_complex",
                                         "r",
                                         "frames",
                                         "fps_mode",
                                         "loop",
                                         "update",
                                         "hash",
                                         "format_version",
                                         "allow_raw_vfw",
                                         "mpegts_m2ts_mode",
                                         "hwaccel",
                                         "ac",
                                         "ar",
                                         "b",
                                         "q",
                                         "g",
                                         "bf",
                                         "crf",
                                         "rc",
                                         "cq",
                                         "deadline",
                                         "cpu-used",
                                         "preset",
                                         "spatial-aq",
                                         "temporal-aq",
                                         "allow_sw",
                                         "huffman",
                                         "lossless",
                                         "quality",
                                         "compression_level",
                                         "compression_algo",
                                         "pred",
                                         "x265-params",
                                         "vbr",
                                         NULL};
    if (argc < 3)
        return AVERROR(EINVAL);
    plan->sources = av_calloc(argc, sizeof(*plan->sources));
    plan->options = av_calloc(argc, sizeof(*plan->options));
    if (!plan->sources || !plan->options)
        return AVERROR(ENOMEM);
    plan->output = argv[argc - 1];
    plan->duration = -1;
    int64_t seek = 0;
    for (int i = 0; i < argc - 1; ++i) {
        const char *key = argv[i];
        if (*key++ != '-')
            return AVERROR(EINVAL);
        const char *value = "1";
        if (!any_name(key, flags)) {
            if (!any_name(key, values) || i + 1 >= argc - 1) {
                fprintf(stderr, "Unknown or incomplete export option: -%s\n", key);
                return AVERROR(EINVAL);
            }
            value = argv[++i];
        }
        if (!strcmp(key, "ss")) {
            if (av_parse_time(&seek, value, 1) < 0)
                return AVERROR(EINVAL);
        } else if (!strcmp(key, "i")) {
            plan->sources[plan->sources_len++] = (Source){value, seek};
            seek = 0;
        } else if (!strcmp(key, "t") || !strcmp(key, "to")) {
            if (av_parse_time(&plan->duration, value, 1) < 0 || plan->duration < 0)
                return AVERROR(EINVAL);
        } else {
            plan->options[plan->options_len++] = (Option){key, value};
        }
    }
    return plan->sources_len ? 0 : AVERROR(EINVAL);
}
