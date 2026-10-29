// Small WASM ABI adapter around USearch's public C API. Index ownership stays inside the module.
#include <cstdint>
#include <cstdlib>
#include <algorithm>
#include <vector>
#include "usearch.h"
static usearch_error_t error = nullptr;
extern "C" {
const char* index_error() { return error; }
usearch_index_t index_create(uint32_t dimensions, uint32_t capacity) {
    error = nullptr;
    usearch_init_options_t options = {};
    options.dimensions = dimensions;
    options.metric_kind = usearch_metric_l2sq_k;
    options.quantization = usearch_scalar_f32_k;
    options.connectivity = 16;
    options.expansion_add = 128;
    options.expansion_search = 512;
    usearch_index_t index = usearch_init(&options, &error);
    if (error) return nullptr;
    usearch_reserve(index, capacity, &error);
    if (error) { usearch_error_t ignored = nullptr; usearch_free(index, &ignored); return nullptr; }
    return index;
}
uint32_t index_add_batch(usearch_index_t index, const uint64_t* keys, const float* vectors, uint32_t count, uint32_t dimensions) {
    error = nullptr;
    size_t required = usearch_size(index, &error) + count;
    if (error) return 0;
    if (required > usearch_capacity(index, &error)) usearch_reserve(index, required * 2, &error);
    if (error) return 0;
    for (uint32_t i = 0; i < count; ++i) {
        usearch_add(index, keys[i], vectors + i * dimensions, usearch_scalar_f32_k, &error);
        if (error) return i;
    }
    return count;
}
uint32_t index_search(usearch_index_t index, const float* vector, uint32_t count, uint64_t* keys, float* distances) {
    error = nullptr;
    return usearch_search(index, vector, usearch_scalar_f32_k, count, keys, distances, &error);
}
struct KeyFilter { const uint64_t* allowed; uint32_t allowed_count; const uint64_t* excluded; uint32_t excluded_count; bool restrict; };
int accepts_key(usearch_key_t key, void* state) {
    auto& f = *static_cast<KeyFilter*>(state);
    return (!f.restrict || std::binary_search(f.allowed, f.allowed + f.allowed_count, key)) &&
        (!f.excluded_count || !std::binary_search(f.excluded, f.excluded + f.excluded_count, key));
}
uint32_t index_search_filtered(usearch_index_t index, const float* vector, uint32_t count, uint64_t* keys, float* distances,
    const uint64_t* allowed, uint32_t allowed_count, const uint64_t* excluded, uint32_t excluded_count, uint32_t restrict_keys) {
    error = nullptr;
    KeyFilter filter = {allowed, allowed_count, excluded, excluded_count, restrict_keys != 0};
    return usearch_filtered_search(index, vector, usearch_scalar_f32_k, count, accepts_key, &filter, keys, distances, &error);
}
// Exact scores are only requested for lexical/graph candidates, never for a corpus scan.
uint32_t index_score_keys(usearch_index_t index, const float* query, const uint64_t* keys, uint32_t count, uint32_t dimensions, double* scores) {
    error = nullptr;
    std::vector<float> vector(dimensions);
    for (uint32_t i = 0; i < count; ++i) {
        if (usearch_get(index, keys[i], 1, vector.data(), usearch_scalar_f32_k, &error) != 1 || error) return i;
        double score = 0;
        for (uint32_t d = 0; d < dimensions; ++d) score += double(query[d]) * vector[d];
        scores[i] = score;
    }
    return count;
}
uint32_t index_size(usearch_index_t index) { error = nullptr; return usearch_size(index, &error); }
uint32_t index_remove(usearch_index_t index, uint64_t key) { error = nullptr; return usearch_remove(index, key, &error); }
uint32_t index_save_size(usearch_index_t index) { error = nullptr; return usearch_serialized_length(index, &error); }
uint32_t index_save(usearch_index_t index, void* buffer, uint32_t length) {
    error = nullptr; usearch_save_buffer(index, buffer, length, &error); return error ? 0 : 1;
}
uint32_t index_load(usearch_index_t index, const void* buffer, uint32_t length) {
    error = nullptr; usearch_load_buffer(index, buffer, length, &error); return error ? 0 : 1;
}
void index_destroy(usearch_index_t index) { error = nullptr; usearch_free(index, &error); }
}
