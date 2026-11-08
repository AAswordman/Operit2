#pragma once
#include <cstddef>
namespace operit::arm64 {
struct BasicBlock;
struct BlockGraph;
// Installs only build-time generated code after matching the live graph.
std::size_t install_guest_aot(BasicBlock &block);
#if OPERIT_ARM64_GUEST_AOT_TRACES
std::size_t install_guest_traces(BlockGraph &graph);
bool is_guest_trace_entry(const void *entry);
std::size_t guest_trace_block_count(const void *entry);
#endif
bool is_guest_aot_entry(const void *entry);
std::size_t guest_aot_entry_length(const void *entry);
}
