/* C ABI smoke test: aphrody-av/c_smoke INPUT.mp4 WORKDIR */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "aphrody_av.h"

static int failures = 0;

#define CHECK(condition, ...)                                                   \
    do {                                                                        \
        if (!(condition)) {                                                     \
            fprintf(stderr, "FAIL %s:%d: ", __FILE__, __LINE__);                \
            fprintf(stderr, __VA_ARGS__);                                       \
            fputc('\n', stderr);                                                \
            failures++;                                                         \
        }                                                                       \
    } while (0)

static unsigned char *read_file(const char *path, size_t *length) {
    FILE *file = fopen(path, "rb");
    if (!file) return NULL;
    fseek(file, 0, SEEK_END);
    long size = ftell(file);
    fseek(file, 0, SEEK_SET);
    unsigned char *buffer = malloc((size_t)size);
    if (buffer && fread(buffer, 1, (size_t)size, file) != (size_t)size) {
        free(buffer);
        buffer = NULL;
    }
    fclose(file);
    *length = (size_t)size;
    return buffer;
}

int main(int argc, char **argv) {
    if (argc < 3) {
        fprintf(stderr, "usage: %s INPUT WORKDIR\n", argv[0]);
        return 2;
    }
    size_t length = 0;
    unsigned char *data = read_file(argv[1], &length);
    CHECK(data != NULL, "cannot read %s", argv[1]);
    if (!data) return 1;

    printf("aphrody-av %s\n", av_version());

    AvResult *probe = av_probe(data, length, "in.mp4");
    CHECK(av_result_status(probe) == 0, "probe status %d: %s", av_result_status(probe), av_result_error(probe) ? av_result_error(probe) : "");
    const char *json = av_result_json(probe);
    CHECK(json && strstr(json, "\"format\":\"mp4\"") && strstr(json, "\"codec\":\"h264\""), "probe json: %s", json ? json : "(null)");
    printf("probe: %s\n", json ? json : "(null)");
    av_result_free(probe);

    AvResult *by_path = av_probe_path(argv[1]);
    CHECK(av_result_status(by_path) == 0 && av_result_json(by_path) && strstr(av_result_json(by_path), "aac"), "probe by path");
    av_result_free(by_path);

    AvResult *transcoded = av_transcode(data, length, "in.mp4", "webm", "{\"video_codec\":\"vp9\",\"audio_codec\":\"opus\",\"scale\":\"32:-1\"}");
    CHECK(av_result_status(transcoded) == 0, "transcode status %d: %s", av_result_status(transcoded), av_result_error(transcoded) ? av_result_error(transcoded) : "");
    size_t out_length = 0;
    const unsigned char *out = av_result_data(transcoded, &out_length);
    CHECK(out && out_length > 100 && out[0] == 0x1A && out[1] == 0x45 && out[2] == 0xDF && out[3] == 0xA3, "webm output has no EBML header (%zu bytes)", out_length);
    AvResult *reprobe = av_probe(out, out_length, NULL);
    CHECK(av_result_status(reprobe) == 0 && strstr(av_result_json(reprobe), "\"format\":\"webm\"") && strstr(av_result_json(reprobe), "\"width\":32"), "re-probe of the transcoded bytes: %s", av_result_json(reprobe) ? av_result_json(reprobe) : "(null)");
    av_result_free(reprobe);
    printf("transcode: %zu bytes of webm, report %s\n", out_length, av_result_json(transcoded));
    av_result_free(transcoded);

    char target[1024];
    snprintf(target, sizeof target, "%s/c_out.mkv", argv[2]);
    AvResult *to_file = av_transcode_path(argv[1], target, "{\"video_codec\":\"copy\",\"audio_codec\":\"copy\"}", 1);
    CHECK(av_result_status(to_file) == 0, "transcode_path: %s", av_result_error(to_file) ? av_result_error(to_file) : "");
    av_result_free(to_file);
    size_t mkv_length = 0;
    unsigned char *mkv = read_file(target, &mkv_length);
    CHECK(mkv && mkv_length > 100, "mkv file written");
    free(mkv);

    AvResult *converted = av_convert(data, length, "in.mp4", "{\"formats\":[\"svg\",\"md\"],\"fps\":5,\"width\":32}");
    CHECK(av_result_status(converted) == 0, "convert: %s", av_result_error(converted) ? av_result_error(converted) : "");
    size_t count = av_result_file_count(converted);
    int saw_animated = 0;
    for (size_t i = 0; i < count; i++) {
        size_t file_length = 0;
        const unsigned char *file = av_result_file_data(converted, i, &file_length);
        const char *path = av_result_file_path(converted, i);
        CHECK(file && file_length > 0 && path, "file %zu", i);
        if (path && strcmp(path, "animated.svg") == 0) saw_animated = 1;
    }
    CHECK(count >= 3 && saw_animated, "convert produced %zu files", count);
    printf("convert: %zu files\n", count);
    CHECK(av_result_file_path(converted, count) == NULL, "out-of-range file path is NULL");
    av_result_free(converted);

    /* errors are values, never crashes */
    AvResult *garbage = av_probe((const unsigned char *)"not a media file at all", 23, NULL);
    CHECK(av_result_status(garbage) == 2 && av_result_error(garbage) && strstr(av_result_error(garbage), "unrecognised container"), "garbage: %d %s", av_result_status(garbage), av_result_error(garbage) ? av_result_error(garbage) : "(null)");
    printf("garbage: status %d, %s\n", av_result_status(garbage), av_result_error(garbage));
    av_result_free(garbage);

    AvResult *unsupported = av_transcode(data, length, "in.mp4", "webm", "{\"video_codec\":\"h264\"}");
    CHECK(av_result_status(unsupported) == 1 && strstr(av_result_error(unsupported), "webm cannot hold h264"), "unsupported: %s", av_result_error(unsupported) ? av_result_error(unsupported) : "(null)");
    av_result_free(unsupported);

    AvResult *bad_json = av_transcode(data, length, NULL, "mp4", "{\"no_such_field\":1}");
    CHECK(av_result_status(bad_json) == 3, "unknown option field is an invalid argument, got %d", av_result_status(bad_json));
    av_result_free(bad_json);

    AvResult *null_input = av_probe(NULL, 0, NULL);
    CHECK(av_result_status(null_input) == 3, "null input");
    av_result_free(null_input);
    CHECK(av_result_status(NULL) != 0, "NULL result is not ok");
    av_result_free(NULL);

    AvResult *caps = av_capabilities();
    CHECK(av_result_status(caps) == 0 && strstr(av_result_json(caps), "\"h264\""), "capabilities");
    av_result_free(caps);

    free(data);
    if (failures) {
        fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    printf("c smoke: ok\n");
    return 0;
}
