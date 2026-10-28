idf_component_register(
    SRCS "operit_mini_ui.c"
    INCLUDE_DIRS "."
    REQUIRES esp_timer esp_partition spi_flash nvs_flash
)
message(STATUS "Operit UI: allocator-free self-drawn renderer")
