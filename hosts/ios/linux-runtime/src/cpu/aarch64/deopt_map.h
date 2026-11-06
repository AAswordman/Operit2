#pragma once
#include "cpu/aarch64/execution_frame.h"
#include "cpu/aarch64/block_graph.h"
namespace operit::arm64 {
// Resident values remain in the same slots across every native graph edge.
// Shadow homes are synchronized only at helper, budget and fault boundaries.
inline void ExecutionFrame::publish(uint32_t gp_mask,uint32_t vector_mask) {
    if(!graph) return;
    for(unsigned n=0;n<gp.size();++n) {
        const auto r=resident_gp[n];
        if(gp_mask&(uint32_t{1}<<r)) shadow.set(r,gp[n],64,true);
    }
    for(unsigned n=0;n<vectors.size();++n) {
        const auto r=resident_vector[n];
        if(vector_mask&(uint32_t{1}<<r)) shadow.v[r]=vectors[n];
    }
    shadow.nzcv=uint8_t(flags>>28);
}
inline void ExecutionFrame::reload(uint32_t gp_mask,uint32_t vector_mask) {
    if(!graph) return;
    for(unsigned n=0;n<gp.size();++n) {
        const auto r=resident_gp[n];
        if(gp_mask&(uint32_t{1}<<r)) gp[n]=shadow.get(r,64,true);
    }
    for(unsigned n=0;n<vectors.size();++n) {
        const auto r=resident_vector[n];
        if(vector_mask&(uint32_t{1}<<r)) vectors[n]=shadow.v[r];
    }
    flags=uint64_t(shadow.nzcv)<<28;
}
}
