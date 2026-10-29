// Stateful fixture for the actual embedded JavaScript/WASM runtime.
void _initialize(void) {}
static int counter;
int next(void) { return ++counter; }
long long echo_i64(long long value) { return value; }
float sum(const float* values, int count) {
    float result = 0;
    for (int i = 0; i < count; ++i) result += values[i];
    return result;
}
int grow(int pages) { return __builtin_wasm_memory_grow(0, pages); }
