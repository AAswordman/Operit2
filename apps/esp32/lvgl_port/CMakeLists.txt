idf_component_register(
    SRCS "operit_lvgl.c" "operit_font_zh_14.c" "layout_store.c"
    INCLUDE_DIRS "."
    REQUIRES lvgl__lvgl esp_timer esp_partition spi_flash
)
