#pragma once
#include "cpu/aarch64/block_graph.h"
namespace operit::arm64 {
void optimize_graph(BlockGraph &graph);
void optimize_global_values(BlockGraph &graph);
bool schedulable_integer(const InstructionNode &node);
}
