#pragma once
#include <cstddef>
#include <cstdint>

// Shared with the Rust hardware adapter; all accesses occur on the guest CPU
// thread. These are metadata, never guest-visible memory or instruction state.
inline uint64_t* g_native_dirty_counter=nullptr;
inline uint64_t* g_native_dirty_pages=nullptr;
constexpr size_t NATIVE_DIRTY_PAGES=(24u+64u)*1024u/4u;
// General case (page-crossing and bulk ranges). Generations: one per write, as below.
inline void native_memory_written_range(uint32_t address,uint32_t length) {
    if(!g_native_dirty_pages||!length)return;
    uint32_t first=address&0x3fffffffu;
    uint64_t end=uint64_t(first)+length-1;
    size_t begin_page,end_page;
    if(first<0x01800000u&&end<0x01800000u) {
        begin_page=first>>12;end_page=size_t(end)>>12;
    } else if(first>=0x10000000u&&end<0x14000000u) {
        begin_page=6144+((first-0x10000000u)>>12);
        end_page=6144+((size_t(end)-0x10000000u)>>12);
    } else return;
    uint64_t generation=++*g_native_dirty_counter;
    if(!generation)generation=++*g_native_dirty_counter;
    for(size_t page=begin_page;page<=end_page;page++)g_native_dirty_pages[page]=generation;
}
#if defined(_MSC_VER)
#define NATIVE_DIRTY_INLINE __forceinline
#else
#define NATIVE_DIRTY_INLINE inline __attribute__((always_inline))
#endif
// Every guest store calls this. MSVC did not inline the general version (one call per
// store); the common single-page case is inlined here with identical generations.
NATIVE_DIRTY_INLINE void native_memory_written(uint32_t address,uint32_t length) {
    uint64_t* pages=g_native_dirty_pages;
    if(!pages)return;
    uint32_t first=address&0x3fffffffu;
    if(length-1u<4096u&&((first^(first+length-1u))>>12)==0) {
        size_t page;
        if(first<0x01800000u)page=first>>12;
        else if(first-0x10000000u<0x04000000u)page=6144+((first-0x10000000u)>>12);
        else return;
        uint64_t generation=++*g_native_dirty_counter;
        if(!generation)generation=++*g_native_dirty_counter;
        pages[page]=generation;
        return;
    }
    native_memory_written_range(address,length);
}
