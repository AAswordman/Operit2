#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "SKP_Silk_SDK_API.h"

#define SAMPLE_RATE 24000
#define FRAME_SAMPLES (SAMPLE_RATE / 50)
#define MAX_PACKET_BYTES 1024
#define MAX_PACKET_FRAMES 5
#define MAX_SAMPLES (120 * SAMPLE_RATE)
#define MAX_INPUT_BYTES (2 * 1024 * 1024)

/* Returns the number of PCM samples, or zero for invalid/truncated data. */
int silk_decode(const uint8_t *input, int size, int16_t *pcm, int capacity) {
    if (size <= 9 || size > MAX_INPUT_BYTES || capacity <= 0 || capacity > MAX_SAMPLES)
        return 0;
    int at = input[0] == 2 ? 1 : 0;
    if (memcmp(input + at, "#!SILK_V3", 9) != 0) return 0;
    at += 9;

    SKP_int32 state_size;
    if (SKP_Silk_SDK_Get_Decoder_Size(&state_size)) return 0;
    void *state = malloc(state_size);
    if (!state) return 0;
    SKP_SILK_SDK_DecControlStruct control = { .API_sampleRate = SAMPLE_RATE };
    int count = 0;
    if (SKP_Silk_SDK_InitDecoder(state)) goto invalid;
    while (at < size) {
        if (size - at < 2) goto invalid;
        int length = input[at] | ((int)input[at + 1] << 8);
        at += 2;
        if (length == 0xffff && at == size) break;
        if (length > MAX_PACKET_BYTES || length > size - at) goto invalid;

        for (int frames = 0; ; ++frames) {
            if (frames == MAX_PACKET_FRAMES || capacity - count < FRAME_SAMPLES)
                goto invalid;
            int16_t generated = 0;
            if (SKP_Silk_SDK_Decode(state, &control, length == 0, input + at,
                                    length, pcm + count, &generated) || generated != FRAME_SAMPLES)
                goto invalid;
            count += generated;
            if (!control.moreInternalDecoderFrames) break;
        }
        at += length;
    }
    free(state);
    return count;

invalid:
    free(state);
    return 0;
}
