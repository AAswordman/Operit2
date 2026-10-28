#pragma once
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
typedef struct {unsigned size;} esp_partition_t;
typedef unsigned esp_partition_mmap_handle_t;
#define ESP_PARTITION_TYPE_DATA 1
#define ESP_PARTITION_SUBTYPE_DATA_SPIFFS 0x82
#define ESP_PARTITION_MMAP_DATA 0
#define ESP_OK 0
static unsigned char fake_flash[65536];
static esp_partition_t fake_partition={65536};
static const esp_partition_t *esp_partition_find_first(int t,int s,const char *name){return &fake_partition;}
static int esp_partition_mmap(const esp_partition_t *p,unsigned o,unsigned n,int mode,const void **result,esp_partition_mmap_handle_t *handle){*result=fake_flash+o;return 0;}
static int esp_partition_write(const esp_partition_t *p,unsigned o,const void *data,unsigned n){if(o+n>sizeof(fake_flash))abort();const uint8_t *in=data;for(unsigned i=0;i<n;i++){if((fake_flash[o+i]&in[i])!=in[i])abort();fake_flash[o+i]&=in[i];}return 0;}
static int esp_partition_erase_range(const esp_partition_t *p,unsigned o,unsigned n){if(o+n>sizeof(fake_flash))abort();memset(fake_flash+o,255,n);return 0;}
