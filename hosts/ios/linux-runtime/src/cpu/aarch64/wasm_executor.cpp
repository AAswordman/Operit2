#include "cpu/aarch64/wasm_executor.h"
#include "cpu/aarch64/semantics.h"
#include "acceleration/fixed_components.h"
#include "memory/address_space.h"
#include <algorithm>
#include <chrono>
#include <deque>
#include <set>
namespace operit::arm64 {
#ifndef OPERIT_WASM_WEBKIT
bool wasm_webkit_available() {return false;}
void wasm_webkit_forget(uint64_t) {}
bool wasm_webkit_run(uint64_t,WasmTransfer&,const std::function<void(WasmTransfer&)> &) {return false;}
#endif
WasmExecutor::~WasmExecutor() {clear();}
void WasmExecutor::clear() {
    if(session_)wasm_webkit_forget(session_);
    cache_.clear();code_.clear();resident_.clear();identity_=epoch_=session_=0;
}
WasmExecutor::Cached WasmExecutor::compile(uint64_t entry,Memory &memory,const FixedComponents *intrinsics) {
    if(auto found=cache_.find(entry);found!=cache_.end())return found->second;
    std::deque<uint64_t> todo{entry};std::set<uint64_t> seen;
    std::vector<std::pair<uint64_t,uint32_t>> words;
    while(!todo.empty() && words.size()<1024) {
        auto pc=todo.front();todo.pop_front();
        if((pc&3) || !seen.insert(pc).second || (pc!=entry && cache_.count(pc)) || (intrinsics && intrinsics->contains(pc)))continue;
        auto page=align_down(pc);auto found=code_.find(page);
        if(found==code_.end()) {
            try {found=code_.emplace(page,memory.code_snapshot(pc).words).first;}
            catch(const MemoryFault &) {continue;}
        }
        auto word=(*found->second)[(pc-page)/4];auto i=decode(word);
        if(!WasmRegion::supports(i))continue;
        words.emplace_back(pc,word);
        if(i.op==Op::Branch || i.op==Op::ConditionalBranch || i.op==Op::CompareBranch || i.op==Op::TestBranch)todo.push_back(pc+i.immediate);
        if((i.op!=Op::Branch && i.op!=Op::BranchRegister && i.op!=Op::Svc) || i.encoding==Encoding::BranchLink || i.encoding==Encoding::BranchLinkRegister)todo.push_back(pc+4);
    }
    Cached result;
    if(!words.empty()) {
        const auto begin=std::chrono::steady_clock::now();
        result.region=std::make_shared<WasmRegion>(words,entry);
        static std::atomic<uint64_t> ids{0};result.id=++ids;
        for(auto [pc,selector]:result.region->entries) { (void)selector;cache_.emplace(pc,result); }
        ++compiled_regions;compiled_instructions+=words.size();
        largest_region=std::max(largest_region,result.region->instruction_count);
        largest_block_count=std::max(largest_block_count,result.region->block_count);
        compile_ns+=std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now()-begin).count();
    } else cache_.emplace(entry,result);
    return result;
}
uint32_t WasmExecutor::run(CpuState &cpu,Memory &memory,uint64_t budget,int &reason,const FixedComponents *intrinsics) {
    if(!budget || !wasm_webkit_available() || !memory.wasm_serial())return 0;
    auto epoch=memory.code_epoch.load(std::memory_order_relaxed);
    bool valid=identity_==memory.identity();
    if(valid)for(const auto &page:code_) {
        uint64_t validated;
        try {if(memory.validate_code(page.first,page.second->data(),page_size,validated))continue;}
        catch(const MemoryFault &) {}
        valid=false;break;
    }
    if(!valid) {
        clear();identity_=memory.identity();epoch_=epoch;
        static std::atomic<uint64_t> sessions{0};session_=++sessions;
    }
    epoch_=epoch;
    WasmTransfer t;t.state.resize(WasmRegion::state_size);
    const auto put=[&](unsigned offset,uint64_t value,unsigned size=8) {std::memcpy(t.state.data()+offset,&value,size);};
    const auto get=[&](unsigned offset,unsigned size=8) {uint64_t value=0;std::memcpy(&value,t.state.data()+offset,size);return value;};
    const auto pack=[&] {
        std::memcpy(t.state.data(),cpu.x.data(),248);put(248,cpu.sp);put(256,cpu.pc);put(264,cpu.nzcv,4);put(304,cpu.tls);
        put(352,cpu.fpcr);put(360,cpu.fpsr);put(400,cpu.exclusive_address);put(408,cpu.exclusive_size,4);
        put(412,cpu.exclusive_valid && cpu.exclusive_epoch==memory.exclusive_epoch(cpu.exclusive_address),4);
        std::memcpy(t.state.data()+512,cpu.v.data(),512);
    };
    const auto unpack=[&] {
        std::memcpy(cpu.x.data(),t.state.data(),248);cpu.sp=get(248);cpu.pc=get(256);cpu.nzcv=get(264,4);cpu.tls=get(304);
        cpu.fpcr=get(352);cpu.fpsr=get(360);cpu.exclusive_address=get(400);cpu.exclusive_size=get(408,4);cpu.exclusive_valid=get(412,4);
        if(cpu.exclusive_valid)cpu.exclusive_epoch=memory.exclusive_epoch(cpu.exclusive_address);
        std::memcpy(cpu.v.data(),t.state.data()+512,512);
    };
    const auto refresh=[&](bool boundary) {
        for(auto it=resident_.begin();it!=resident_.end();) {
            auto &r=it->second;bool current=false;
            try {
                const unsigned access=r.permissions&read_perm ? read_perm:write_perm;
                const auto revision=memory.wasm_revision(r.address,access);
                current=revision==r.revision && !(boundary && r.file);
                if(!current) {
                    std::vector<uint8_t> bytes(page_size);bool file=false;
                    const auto permissions=memory.wasm_page(r.address,access,bytes.data(),file);
                    current=permissions==r.permissions && bytes==r.bytes;
                    if(current) {r.revision=revision;r.file=file;}
                }
            } catch(const MemoryFault &) {}
            if(current)++it;else {t.invalidate.push_back(it->first);it=resident_.erase(it);}
        }
    };
    pack();const auto admitted=uint32_t(std::min<uint64_t>(budget,UINT32_MAX));put(272,admitted,4);refresh(true);
    uint32_t retired=0;std::exception_ptr exception;
    const auto service=[&](WasmTransfer &request) {
        (void)request;++services;

        // Every native observation first receives all dirty pages, including
        // same-value stores, through Memory's COW/reservation-aware write API.
        const auto commit_begin=std::chrono::steady_clock::now();
        for(auto &page:t.pages) {
            auto &resident=resident_.at(page.slot);
            memory.copy_in(resident.address,page.bytes.data(),page_size);
            resident.revision=memory.wasm_revision(resident.address,resident.permissions&read_perm ? read_perm:write_perm);
            resident.bytes=page.bytes;
        }
        commit_ns+=std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now()-commit_begin).count();
        t.pages.clear();t.module.reset();t.module_id=0;
        const auto completed=uint32_t(get(268,4)),remaining=uint32_t(get(272,4));
        if(completed<retired || completed>admitted || remaining!=admitted-completed)throw Failure("invalid Wasm retirement state");
        retired=completed;unpack();
        if(t.stop || t.reason==WasmRegion::Syscall || !remaining) {t.stop=true;return;}
        if(t.reason==WasmRegion::Capability) {
            ++capability_exits;++page_faults;
            const auto address=get(WasmRegion::fault_offset),base=align_down(address);
            const unsigned permissions=get(WasmRegion::perms_offset,4),size=get(WasmRegion::size_offset,4);
            WasmTransfer::Page page;page.address=base;page.slot=WasmRegion::page_slot(base);page.bytes.resize(page_size);
            bool file=false;
            if(size<=page_size-address%page_size && (page.permissions=memory.wasm_page(base,permissions,page.bytes.data(),file))) {
                resident_[page.slot]={base,memory.wasm_revision(base,permissions),page.permissions,file,page.bytes};
                t.pages.push_back(std::move(page));return;
            }
            if(size<=page_size-address%page_size) {t.stop=true;return;}
            // A crossing access uses the exact native semantic, then resumes.
        } else if(t.reason==WasmRegion::Branch) {
            ++code_misses;auto compiled=compile(cpu.pc,memory,intrinsics);
            if(compiled.region) {t.module=compiled.region;t.module_id=compiled.id;return;}
        } else if(t.reason==WasmRegion::Budget) {t.stop=true;return;}
        if(intrinsics && intrinsics->contains(cpu.pc)) {t.stop=true;return;}
        // Native assistance covers a complete basic block, then reconnects to
        // compiled code. There is no opcode interpreter inside WebContent.
        const auto assist_begin=std::chrono::steady_clock::now();
        bool terminal=false;
        do {
            auto i=decode(memory.load<4>(cpu.pc,exec_perm));terminal=i.terminal;++assists[i.raw];
            bool syscall=select_semantic(i)(cpu,memory,i);++retired;++native_instructions;
            if(syscall) {t.reason=WasmRegion::Syscall;t.stop=true;break;}
            if(i.terminal || retired==admitted)break;
        } while(!terminal);
        assist_ns+=std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now()-assist_begin).count();
        put(268,retired,4);put(272,admitted-retired,4);pack();refresh(false);
        if(retired==admitted)t.stop=true;
        if(memory.code_epoch.load(std::memory_order_relaxed)!=epoch_)t.stop=true;
    };
    // Initial compilation runs on the owner worker, before crossing WebKit.
    auto first=compile(cpu.pc,memory,intrinsics);
    while(!first.region) {
        if(intrinsics && intrinsics->contains(cpu.pc))return retired;
        auto i=decode(memory.load<4>(cpu.pc,exec_perm));
        const bool syscall=select_semantic(i)(cpu,memory,i);++retired;++native_instructions;
        if(syscall || retired==admitted) {reason=syscall ? WasmRegion::Syscall:WasmRegion::Budget;return retired;}
        first=compile(cpu.pc,memory,intrinsics);
    }
    pack();put(268,retired,4);put(272,admitted-retired,4);refresh(false);
    // Mirror the already resident working set in one transfer. Missing guest
    // pages still fault on demand, without allocating untouched address ranges.
    for(auto address:memory.wasm_resident_pages()) {
        const auto slot=WasmRegion::page_slot(address);
        if(resident_.count(slot))continue;
        WasmTransfer::Page page;page.address=address;page.slot=slot;page.bytes.resize(page_size);bool file=false;
        try {page.permissions=memory.wasm_page(address,read_perm,page.bytes.data(),file);}catch(const MemoryFault &) {continue;}
        if(!page.permissions)continue;
        resident_[slot]={address,memory.wasm_revision(address,read_perm),page.permissions,file,page.bytes};t.pages.push_back(std::move(page));
    }
    if(!valid) {t.module=first.region;t.module_id=first.id;}
    const auto begin=std::chrono::steady_clock::now();++calls;
    bool ok=wasm_webkit_run(session_,t,[&](WasmTransfer &request) {
        if(exception) {t.stop=true;return;}
        try {service(request);}catch(...) {exception=std::current_exception();t.stop=true;}
    });
    transport_ns+=std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now()-begin).count();
    executed_instructions+=t.wasm_retired;dispatches+=t.dispatches;
    module_uploads+=t.module_uploads;module_bytes+=t.module_bytes;
    if(exception)std::rethrow_exception(exception);
    if(!ok) {++failed_calls;clear();}
    reason=t.reason;return retired;
}
}

namespace operit::arm64 {
void WasmExecutor::record(const CpuState &cpu,uint64_t total,uint64_t completed,bool syscall) const {
    const auto *path=std::getenv("OPERIT_EXECUTION_DIAGNOSTICS_OUTPUT");
    if(!path || !*path)return;
    FILE *out=(!std::strcmp(path,"stderr") || !std::strcmp(path,"-")) ? stderr:std::fopen(path,"a");
    if(!out)return;
    std::fprintf(out,"{\"kind\":\"wasm_resident\",\"total\":%llu,\"completed\":%llu,\"pc\":%llu,\"syscall\":%lld,\"arg0\":%llu,\"wasm\":%llu,\"calls\":%llu,\"dispatches\":%llu,\"services\":%llu,\"pages\":%llu,\"code\":%llu,\"native_assist\":%llu,\"failed\":%llu,\"transport_ns\":%llu,\"compile_ns\":%llu,\"assist_ns\":%llu,\"commit_ns\":%llu}\n",
        (unsigned long long)total,(unsigned long long)completed,(unsigned long long)cpu.pc,syscall ? (long long)cpu.x[8]:-1ll,(unsigned long long)cpu.x[0],
        (unsigned long long)executed_instructions,(unsigned long long)calls,(unsigned long long)dispatches,(unsigned long long)services,
        (unsigned long long)page_faults,(unsigned long long)code_misses,(unsigned long long)native_instructions,(unsigned long long)failed_calls,
        (unsigned long long)transport_ns,(unsigned long long)compile_ns,(unsigned long long)assist_ns,(unsigned long long)commit_ns);
    if(syscall) {
        std::fprintf(out,"{\"kind\":\"wasm_assists\",\"total\":%llu,\"opcodes\":[",(unsigned long long)total);
        bool comma=false;for(auto [raw,count]:assists) {std::fprintf(out,"%s[%u,%llu]",comma ? ",":"",raw,(unsigned long long)count);comma=true;}
        std::fprintf(out,"]}\n");
    }
    if(out!=stderr)std::fclose(out);
}
}
