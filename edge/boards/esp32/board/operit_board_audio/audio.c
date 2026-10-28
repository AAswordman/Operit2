// ESP32-2432S028: INMP441 on I2S1; the onboard amplifier uses DAC GPIO26 / I2S0.
#include "driver/i2s_std.h"
#include "driver/dac_continuous.h"
#include "esp_attr.h"
#include "esp_timer.h"
#include <stdatomic.h>
#include <stdlib.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/queue.h"
#include "freertos/semphr.h"

static atomic_uint microphone_overflows;
static bool IRAM_ATTR microphone_overflow(i2s_chan_handle_t channel, i2s_event_data_t *event, void *context) {
    atomic_fetch_add_explicit(&microphone_overflows, 1, memory_order_relaxed);
    return false;
}
// Match Rust CaptureBlock. The discarded low bits do not enter PCM conversion.
typedef struct {
    uint32_t nonzero, peak, right_peak;
    int16_t samples[256];
} capture_block_t;
_Static_assert(sizeof(capture_block_t) == 524, "CaptureBlock ABI");
typedef struct {
    i2s_chan_handle_t channel;
    QueueHandle_t blocks;
    SemaphoreHandle_t exited;
    TaskHandle_t task;
    atomic_bool stop;
    atomic_int error;
    int32_t input[512];
    capture_block_t output;
} microphone_t;
static uint32_t peak24(int32_t raw) {
    int32_t sample = raw >> 8;
    return sample < 0 ? (uint32_t)-sample : (uint32_t)sample;
}
// Drain hardware at higher priority than Host workflows. The fixed 32-block
// PCM queue stores 512 ms in 16,768 bytes; no full recording is retained.
static void microphone_task(void *context) {
    microphone_t *mic = context;
    int64_t previous = esp_timer_get_time();
    unsigned max_gap = 0;
    while (!atomic_load(&mic->stop)) {
        unsigned gap = (unsigned)((esp_timer_get_time() - previous) / 1000);
        if (gap > max_gap) max_gap = gap;
        size_t count = 0;
        esp_err_t err = i2s_channel_read(mic->channel, mic->input, sizeof(mic->input), &count, 100);
        previous = esp_timer_get_time();
        if (atomic_load(&microphone_overflows)) err = -10000 - (int)max_gap;
        if (err == ESP_OK && count != sizeof(mic->input)) err = ESP_ERR_INVALID_SIZE;
        if (err == ESP_OK) {
            mic->output.nonzero = mic->output.peak = mic->output.right_peak = 0;
            for (unsigned i = 0; i < 256; i++) {
                int32_t left = mic->input[i * 2];
                mic->output.samples[i] = left >> 16;
                mic->output.nonzero += left != 0;
                uint32_t peak = peak24(left), right = peak24(mic->input[i * 2 + 1]);
                if (peak > mic->output.peak) mic->output.peak = peak;
                if (right > mic->output.right_peak) mic->output.right_peak = right;
            }
            if (xQueueSend(mic->blocks, &mic->output, 0) != pdTRUE) err = -20000;
        }
        if (err != ESP_OK) { atomic_store(&mic->error, err); break; }
    }
    xSemaphoreGive(mic->exited);
    vTaskDelete(NULL);
}
void operit_audio_mic_close(void *context) {
    microphone_t *mic = context;
    atomic_store(&mic->stop, true);
    if (mic->task) xSemaphoreTake(mic->exited, portMAX_DELAY);
    if (mic->channel) { i2s_channel_disable(mic->channel); i2s_del_channel(mic->channel); }
    if (mic->blocks) vQueueDelete(mic->blocks);
    if (mic->exited) vSemaphoreDelete(mic->exited);
    free(mic);
}
int operit_audio_mic_open(void **result) {
    *result = NULL;
    microphone_t *mic = calloc(1, sizeof(*mic));
    if (!mic) return ESP_ERR_NO_MEM;
    atomic_init(&mic->stop, false);
    atomic_init(&mic->error, ESP_OK);
    mic->blocks = xQueueCreate(32, sizeof(mic->output));
    mic->exited = xSemaphoreCreateBinary();
    esp_err_t err = ESP_ERR_NO_MEM;
    if (!mic->blocks || !mic->exited) goto fail;
    i2s_chan_config_t config = I2S_CHANNEL_DEFAULT_CONFIG(I2S_NUM_1, I2S_ROLE_MASTER);
    config.dma_desc_num = 4;
    config.dma_frame_num = 256;
    err = i2s_new_channel(&config, NULL, &mic->channel);
    if (err != ESP_OK) goto fail;
    i2s_std_config_t standard = {
        .clk_cfg = I2S_STD_CLK_DEFAULT_CONFIG(16000),
        .slot_cfg = I2S_STD_PHILIPS_SLOT_DEFAULT_CONFIG(I2S_DATA_BIT_WIDTH_32BIT, I2S_SLOT_MODE_STEREO),
        .gpio_cfg = {.mclk = I2S_GPIO_UNUSED, .bclk = GPIO_NUM_27, .ws = GPIO_NUM_22,
                     .dout = I2S_GPIO_UNUSED, .din = GPIO_NUM_35},
    };
    err = i2s_channel_init_std_mode(mic->channel, &standard);
    const i2s_event_callbacks_t callbacks = {.on_recv_q_ovf = microphone_overflow};
    atomic_store(&microphone_overflows, 0);
    if (err == ESP_OK) err = i2s_channel_register_event_callback(mic->channel, &callbacks, NULL);
    if (err == ESP_OK) err = i2s_channel_enable(mic->channel);
    if (err == ESP_OK && xTaskCreate(microphone_task, "operit-mic", 3072, mic, 10, &mic->task) != pdPASS)
        err = ESP_ERR_NO_MEM;
    if (err != ESP_OK) goto fail;
    *result = mic;
    return ESP_OK;
fail:
    operit_audio_mic_close(mic);
    return err;
}
int operit_audio_mic_read(void *context, void *buffer, size_t length, size_t *read) {
    microphone_t *mic = context;
    *read = 0;
    if (length != sizeof(mic->output)) return ESP_ERR_INVALID_SIZE;
    esp_err_t err = atomic_load(&mic->error);
    if (err != ESP_OK) return err;
    if (xQueueReceive(mic->blocks, buffer, pdMS_TO_TICKS(100)) != pdTRUE) {
        err = atomic_load(&mic->error);
        return err == ESP_OK ? ESP_ERR_TIMEOUT : err;
    }
    err = atomic_load(&mic->error);
    if (err == ESP_OK) *read = sizeof(mic->output);
    return err;
}
int operit_audio_speaker_open(unsigned rate, void **result) {
    *result = NULL;
    dac_continuous_handle_t channel = NULL;
    dac_continuous_config_t config = {
        .chan_mask = DAC_CHANNEL_MASK_CH1, // GPIO26, not GPIO25 (touch SPI clock).
        .desc_num = 4, .buf_size = 256, .freq_hz = rate,
        .offset = 0, .clk_src = DAC_DIGI_CLK_SRC_APLL, .chan_mode = DAC_CHANNEL_MODE_SIMUL,
    };
    esp_err_t err = dac_continuous_new_channels(&config, &channel);
    if (err != ESP_OK) return err;
    err = dac_continuous_enable(channel);
    if (err != ESP_OK) { dac_continuous_del_channels(channel); return err; }
    *result = channel;
    return ESP_OK;
}
int operit_audio_speaker_write(void *channel, const void *buffer, size_t length, size_t *written) {
    return dac_continuous_write(channel, (uint8_t *)buffer, length, written, 250);
}
void operit_audio_speaker_close(void *channel) {
    dac_continuous_disable(channel);
    dac_continuous_del_channels(channel);
}
