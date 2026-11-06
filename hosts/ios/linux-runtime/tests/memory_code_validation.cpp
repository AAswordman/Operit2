#include "memory/address_space.h"
#include <array>
#include <cstdio>
#include <cstring>
#include <memory>
#include <stdexcept>
#include <sys/mman.h>
#include <unistd.h>

using namespace operit::arm64;

namespace {
void require(bool condition,const char *message) {
    if(!condition)throw std::runtime_error(message);
}

bool equal_lines(const void *actual,const void *expected,size_t bytes) {
    return memory_detail::code_bytes_equal(actual,expected,bytes,memory_detail::CodeComparison::Line);
}

void every_mismatching_byte() {
    std::array<uint8_t,512> actual{},expected{};
    for(size_t bytes:{64u,128u,192u,256u}) {
        for(size_t left:{0u,1u,7u,15u}) for(size_t right:{0u,3u,11u,31u}) {
            actual.fill(0x5a);expected.fill(0xa5);
            for(size_t index=0;index<bytes;++index)
                actual[left+index]=expected[right+index]=uint8_t(index*29+(index/7)*19);
            require(equal_lines(actual.data()+left,expected.data()+right,bytes),
                    "equal line spans rejected or bytes outside span compared");
            for(size_t index=0;index<bytes;++index) {
                expected[right+index]^=uint8_t(1u<<(index&7));
                require(!equal_lines(actual.data()+left,expected.data()+right,bytes),
                        "line comparison missed a changed byte");
                expected[right+index]^=uint8_t(1u<<(index&7));
            }
        }
    }
}

class GuardedBytes {
    size_t host_page_=size_t(::sysconf(_SC_PAGESIZE));
    void *mapping_=::mmap(nullptr,3*host_page_,PROT_NONE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);
  public:
    GuardedBytes() {
        require(mapping_!=MAP_FAILED,"guard-page mapping");
        if(::mprotect(data(),host_page_,PROT_READ|PROT_WRITE)) {
            ::munmap(mapping_,3*host_page_);
            throw std::runtime_error("guard-page protection");
        }
    }
    ~GuardedBytes() { ::munmap(mapping_,3*host_page_); }
    uint8_t *data() const { return static_cast<uint8_t *>(mapping_)+host_page_; }
    uint8_t *end() const { return data()+host_page_; }
    size_t size() const { return host_page_; }
    GuardedBytes(const GuardedBytes &)=delete;
    GuardedBytes &operator=(const GuardedBytes &)=delete;
};

void bounded_reads() {
    GuardedBytes actual,expected;
    for(size_t bytes:{64u,128u,192u,256u,320u,4096u}) {
        require(bytes<=actual.size() && bytes<=expected.size(),"host page smaller than guest page");
        // Place each input against both protected boundaries independently.
        for(bool left_tail:{false,true}) for(bool right_tail:{false,true}) {
            auto *left=left_tail ? actual.end()-bytes:actual.data();
            auto *right=right_tail ? expected.end()-bytes:expected.data();
            std::memset(left,0x6d,bytes);std::memset(right,0x6d,bytes);
            require(equal_lines(left,right,bytes),"bounded equal comparison failed");
            right[bytes-1]^=1;
            require(!equal_lines(left,right,bytes),"bounded comparison missed last byte");
            right[bytes-1]^=1;right[0]^=1;
            require(!equal_lines(left,right,bytes),"bounded comparison missed first byte");
        }
    }
}

void file_proof_semantics() {
    constexpr uint64_t base=0x120000;
    constexpr uint32_t nop=0xd503201f;
    std::array<uint32_t,page_size/4> initial;
    initial.fill(nop);
    std::unique_ptr<FILE,decltype(&std::fclose)> file(std::tmpfile(),&std::fclose);
    require(bool(file),"code file fixture");
    const int fd=fileno(file.get());
    require(::pwrite(fd,initial.data(),page_size,0)==ssize_t(page_size),"code file initialization");
    const auto *bytes=reinterpret_cast<const uint8_t *>(initial.data());
    const auto write_byte=[&](size_t offset,uint8_t value) {
        require(::pwrite(fd,&value,1,off_t(offset))==1,"external code write");
    };
    Memory memory(page_size);memory.set_parallel_access(false);
    Memory::Backing backing;backing.shared=true;
    memory.map_file(base,page_size,read_perm|exec_perm,fd,backing);
    Memory::CodeSnapshot snapshot;
    {
        Memory::ExecutionScope scope(memory);
        snapshot=memory.code_snapshot(base);
    }
    for(unsigned lines:{1u,2u,3u,4u,5u,64u}) for(unsigned first:{0u,64u-lines}) {
        const uint64_t mask=lines==64 ? UINT64_MAX:((uint64_t{1}<<lines)-1)<<first;
        Memory::CodeProof proof(memory.identity());
        uint64_t epoch=0;
        {
            Memory::ExecutionScope scope(memory);
            require(memory.validate_code_lines(base,snapshot.words,mask,epoch,&proof),
                    "contiguous line proof rejected unchanged bytes");
            require(proof.verified==mask,"line proof covered unrequested bytes");
            require(epoch==memory.code_epoch.load(),"line proof lost code epoch");
        }
        const size_t changed=(first+lines)*64-1;
        write_byte(changed,bytes[changed]^1);
        {
            Memory::ExecutionScope scope(memory);
            require(!memory.validate_code_lines(base,snapshot.words,mask,epoch,&proof),
                    "new scope retained externally modified line proof");
            require((proof.verified&mask)!=mask,"mismatching run published proof");
        }
        write_byte(changed,bytes[changed]);
    }
    // Two separated runs exercise one-line and two-line comparisons without
    // widening the proof across the gap or beyond the final guest page byte.
    {
        Memory::ExecutionScope scope(memory);
        Memory::CodeProof proof(memory.identity());uint64_t epoch=0;
        const uint64_t mask=1|(uint64_t{3}<<62);
        write_byte(64,bytes[64]^1);
        require(memory.validate_code_lines(base,snapshot.words,mask,epoch,&proof),
                "sparse proof compared a gap");
        require(proof.verified==mask,"sparse proof covered a gap");
        write_byte(64,bytes[64]);
    }
    // A changed uncovered byte may decline a whole-line proof, but exact
    // instruction validation must still succeed without certifying that line.
    write_byte(0,bytes[0]^1);
    {
        Memory::ExecutionScope scope(memory);
        Memory::CodeProof proof(memory.identity());uint64_t epoch=0;
        require(!memory.validate_code_lines(base,snapshot.words,1,epoch,&proof),
                "uncovered change unexpectedly proved complete line");
        require(memory.validate_code_snapshot_cached(base+4,snapshot.words,1,1,epoch,&proof),
                "line mismatch invalidated unchanged exact instruction");
        require(!(proof.verified&1),"exact match certified mismatching line");
        write_byte(4,bytes[4]^1);
        require(!memory.validate_code_snapshot_cached(base+4,snapshot.words,1,1,epoch,&proof),
                "exact fallback missed changed instruction in same scope");
    }
    write_byte(0,bytes[0]);write_byte(4,bytes[4]);
    Memory::CodeProof proof(memory.identity());uint64_t epoch=0;
    {
        Memory::ExecutionScope scope(memory);
        require(memory.validate_code_lines(base,snapshot.words,1,epoch,&proof),"initial permission proof");
        memory.protect(base,page_size,read_perm);
        bool faulted=false;
        try { memory.validate_code_lines(base,snapshot.words,1,epoch,&proof); }
        catch(const MemoryFault &fault) {
            faulted=fault.kind==MemoryFault::Kind::Protection && fault.address==base &&
                    fault.permissions==exec_perm;
        }
        require(faulted,"execute permission removal lost precise fault");
    }
    memory.protect(base,page_size,read_perm|exec_perm);
    {
        Memory::ExecutionScope scope(memory);
        require(memory.validate_code_lines(base,snapshot.words,1,epoch,&proof),"restored execute permission");
    }
    require(!::ftruncate(fd,0),"code file truncation");
    {
        Memory::ExecutionScope scope(memory);
        bool faulted=false;
        try { memory.validate_code_lines(base,snapshot.words,1,epoch,&proof); }
        catch(const MemoryFault &fault) {
            faulted=fault.kind==MemoryFault::Kind::Bus && fault.address==base &&
                    fault.permissions==exec_perm;
        }
        require(faulted,"truncated code file reached byte comparison");
    }
}
} // namespace

int main() {
    try {
        every_mismatching_byte();bounded_reads();file_proof_semantics();
        std::puts("memory code validation passed");return 0;
    } catch(const std::exception &error) {
        std::fprintf(stderr,"%s\n",error.what());return 1;
    } catch(const GuestError &error) {
        std::fprintf(stderr,"guest error %d\n",error.number);return 1;
    }
}
