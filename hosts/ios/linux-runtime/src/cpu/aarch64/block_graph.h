#pragma once
#include "cpu/aarch64/semantics.h"
#include "cpu/aarch64/executor_abi.h"
#include "cpu/aarch64/memory_effects.h"
#include "cpu/aarch64/cfg_diagnostics.h"
#include <array>
#include <memory>
#include <unordered_map>
#include <vector>
namespace operit::arm64 {
class FixedComponents;
// The executable vocabulary is bounded. Registers in these names are value
// roles, never guest register numbers or dynamically encoded machine registers.
enum class StaticCode : uint8_t {
    Block, Bridge, IndirectDispatch, Next, Helper, Nop, Move, Keep, Add, Sub, Adc, Sbc, And, Or, Xor,
    Lsl, Lsr, Asr, Ror, Mul, Udiv, Sdiv, Select,
    Operand, Prepare, AddPrepared, SubPrepared, AndPrepared, OrPrepared, XorPrepared,
    Madd, Msub, SmulHigh, UmulHigh, Bitfield, Extract, Unary,
    Load, Store, SignedLoad, VectorLoad, VectorStore,
    PairLoad, PairStore, VectorPairLoad, VectorPairStore,
    LoadPrepared, StorePrepared, SignedLoadPrepared, VectorLoadPrepared, VectorStorePrepared,
    Fadd, Fsub, Fmul, Fdiv, Fsqrt, Fma, Fcmp, Fselect,
    GpToVector, VectorToGp, VectorImmediate, UnsignedToDouble,
    Jump, Call, Indirect, IndirectCall, Branch, CompareBranch, TestBranch,
    CompareImmediateBranch, ReadTls, WriteTls, ReadFlags, WriteFlags, ConditionalCompare,
    LoadWriteback, StoreWriteback, SignedLoadWriteback, VectorLoadWriteback, VectorStoreWriteback, FUnary, BlockBody, Count
};
struct InstructionNode {
    uint64_t pc=0,constant=0;
    Semantic semantic=nullptr;
    Instruction instruction;
    Effects effects;
    bool constant_result=false,dead_result=false,discard_flags=false;
};
// One fixed-size native record. index locates cold metadata; it is never a
// guest retirement count. Native control edges use b for their retired prefix.
struct alignas(32) StaticOperation {
    const void *entry=nullptr;
    uint64_t immediate=0;
    uint16_t a=0,b=0,d=0,index=0;
    union {const void *target=nullptr;uint64_t extra;};
};
static_assert(sizeof(StaticOperation)==OP_SIZE && alignof(StaticOperation)==32);
static_assert(offsetof(StaticOperation,immediate)==OP_IMMEDIATE && offsetof(StaticOperation,a)==OP_A);
static_assert(offsetof(StaticOperation,b)==OP_B && offsetof(StaticOperation,d)==OP_D);
static_assert(offsetof(StaticOperation,index)==OP_INDEX && offsetof(StaticOperation,target)==OP_TARGET);
// origin is an InstructionNode pointer for block-owned records, or a full guest
// PC for detached Bridge/PIC records. Current native metadata uses low 40 bits:
// payload is bits 0..31 and mode is bits 32..39. Preserve retirement separately.
struct OperationMetadata {
    uint64_t origin=0;
    uint32_t payload=0;
    uint16_t prefix=0;
    uint8_t mode=0;
    StaticCode code=StaticCode::Block;
};
static_assert(sizeof(OperationMetadata)==OM_SIZE);
static_assert(offsetof(OperationMetadata,origin)==OM_ORIGIN && offsetof(OperationMetadata,payload)==OM_PAYLOAD);
static_assert(offsetof(OperationMetadata,prefix)==OM_PREFIX && offsetof(OperationMetadata,mode)==OM_MODE);
static_assert(offsetof(OperationMetadata,code)==OM_KIND && offsetof(InstructionNode,pc)==0);
static_assert(unsigned(StaticCode::Bridge)==OP_KIND_BRIDGE && unsigned(StaticCode::IndirectDispatch)==OP_KIND_INDIRECT_DISPATCH);
static_assert(unsigned(StaticCode::Next)==OP_KIND_NEXT);
inline OperationMetadata make_operation_metadata(const InstructionNode &node,StaticCode code,uint16_t prefix,uint64_t value=0) {
    // Do not silently truncate a future lowering's metadata extension.
    if(value&0xffffff0000000000ull)throw Failure("static metadata exceeds 40 bits");
    return {reinterpret_cast<uintptr_t>(&node),uint32_t(value),prefix,uint8_t(value>>32),code};
}
inline OperationMetadata detached_operation_metadata(uint64_t pc,StaticCode code) noexcept {
    return {pc,0,0,0,code};
}
inline bool operation_control_edge(StaticCode code) noexcept {
    switch(code) {
    case StaticCode::Next:case StaticCode::Jump:case StaticCode::Call:
    case StaticCode::Indirect:case StaticCode::IndirectCall:case StaticCode::Branch:
    case StaticCode::CompareBranch:case StaticCode::TestBranch:case StaticCode::CompareImmediateBranch:return true;
    default:return false;
    }
}
struct StaticProof {
    const Memory::CodeProof *proof=nullptr;
    uint64_t mask=0;
};
static_assert(sizeof(StaticProof)==16);
struct BlockGraph;
struct BridgePlan {
    StaticOperation operation;
    OperationMetadata metadata;
    const uint64_t *generation=nullptr;
    uint64_t expected_generation=0;
    const uint64_t *target_epoch=nullptr,*target_scope=nullptr;
    BlockGraph *graph=nullptr;
    const StaticOperation *source=nullptr;
    const StaticProof *proofs=nullptr;
    uint64_t proof_count=0;
    // Survives target eviction without retaining its graph or executable records.
    std::shared_ptr<uint64_t> target_lifetime;
};
static_assert(offsetof(BridgePlan,metadata)==OP_SIZE);
static_assert(offsetof(BridgePlan,source)==88 && offsetof(BridgePlan,proofs)==96);
static_assert(offsetof(BridgePlan,proof_count)==104 && sizeof(BridgePlan)==128);
static_assert(offsetof(Memory::CodeProof,epoch)==8 && offsetof(Memory::CodeProof,generation)==16 && offsetof(Memory::CodeProof,verified)==24);
static_assert(offsetof(BridgePlan,generation)==48 && offsetof(BridgePlan,target_epoch)==64);
static_assert(offsetof(BridgePlan,graph)==80);
// A bounded four-way associative cache per indirect edge. All cached target
// pointers refer to source-owned records. Each bridge separately guards the
// target lifetime before dereferencing any target graph state.
struct IndirectTarget { uint64_t pc=0; const StaticOperation *operation=nullptr; };
struct IndirectCache {
    StaticOperation operation;
    OperationMetadata metadata;
    const uint64_t *generation=nullptr;
    uint64_t expected_generation=0;
    const StaticOperation *source=nullptr;
    std::array<IndirectTarget,128> targets{};
    std::array<std::unique_ptr<BridgePlan>,128> bridges{};
    std::array<uint8_t,32> replacement{};
};
static_assert(offsetof(IndirectCache,metadata)==OP_SIZE);
static_assert(offsetof(IndirectCache,generation)==48 && offsetof(IndirectCache,source)==64 && offsetof(IndirectCache,targets)==72);
static_assert(sizeof(IndirectTarget)==16);
struct BasicBlock {
    uint64_t pc=0;
    std::vector<InstructionNode> instructions;
    std::vector<uint64_t> successors;
    std::vector<StaticOperation> code;
    std::vector<OperationMetadata> metadata;
};
// Detached records keep their metadata inline in their source-owned plan.
// Their metadata remains readable even after the guarded destination is evicted.
inline const OperationMetadata &operation_metadata(const StaticOperation &op) noexcept {
    if(op.index==OP_DETACHED_INDEX)
        return *reinterpret_cast<const OperationMetadata*>(reinterpret_cast<uintptr_t>(&op)+OP_SIZE);
    const auto *header=&op-op.index;
    return static_cast<const OperationMetadata*>(header->target)[op.index];
}
struct OperationInfo {
    const InstructionNode *node;
    uint64_t pc,auxiliary;
    uint16_t prefix;
    StaticCode code;
};
// All C++ consumers use this single interpretation. Snapshots must copy these
// resolved values while the graph is alive, rather than retaining origin.
inline OperationInfo operation_info(const StaticOperation &op) noexcept {
    const auto &metadata=operation_metadata(op);
    const bool detached=metadata.code==StaticCode::Bridge || metadata.code==StaticCode::IndirectDispatch;
    const auto *node=detached ? nullptr:reinterpret_cast<const InstructionNode*>(metadata.origin);
    const auto pc=detached ? metadata.origin:node->pc+(metadata.code==StaticCode::Next ? 4:0);
    const auto auxiliary=uint64_t(metadata.payload)|(uint64_t(metadata.mode)<<32)|(uint64_t(metadata.code)<<56);
    return {node,pc,auxiliary,metadata.prefix,metadata.code};
}
struct CodeDependency {
    uint64_t address;
    std::shared_ptr<const Memory::CodeWords> words;
    std::shared_ptr<Memory::CodeProof> proof;
    uint32_t offset,count;
};
struct BlockGraph {
    uint64_t referenced=1;
    std::shared_ptr<uint64_t> lifetime=std::make_shared<uint64_t>(1);
    uint64_t address=0, validated_epoch=0, validated_scope=0;
    std::vector<std::unique_ptr<BasicBlock>> blocks;
    std::unordered_map<uint64_t,BasicBlock*> entries;
    std::vector<CodeDependency> dependencies;
    std::vector<StaticProof> proofs;
    std::unordered_map<uint64_t,const InstructionNode*> source;
    uint64_t eliminated=0,fused=0;
    std::unordered_map<const StaticOperation*,std::unique_ptr<BridgePlan>> bridges;
    std::unordered_map<const StaticOperation*,std::unique_ptr<IndirectCache>> indirect_caches;
    OPERIT_EXEC_DIAGNOSTIC(execution_diagnostics::State *diagnostics=nullptr;)
    bool validate(Memory &memory);
    bool current(Memory &memory,uint64_t scope) const;
};
static_assert(offsetof(BlockGraph,referenced)==0);
// Shared decoded CFG and optimization pipeline, before backend-specific lowering.
std::unique_ptr<BlockGraph> build_instruction_graph(const std::vector<std::pair<uint64_t,uint32_t>> &words,uint64_t pc);
std::unique_ptr<BlockGraph> build_graph(uint64_t pc,Memory &memory,const FixedComponents *intrinsics=nullptr);
void lower_graph(BlockGraph &graph);
// Table lookup happens during lowering, never to select arithmetic at runtime.
const void *static_entry(StaticCode code,unsigned bits=64,unsigned d=7,unsigned a=7,unsigned b=7,unsigned flags=0);
}
