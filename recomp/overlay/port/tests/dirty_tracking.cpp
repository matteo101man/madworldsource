#include "ppc_context.h"
#include "ppc_memory.h"
#include <vector>
#include <cstdio>
#include <cstdlib>
unsigned mmio_writes=0;
void mmio_write8(uint32_t,uint8_t){mmio_writes++;}
void mmio_write16(uint32_t,uint16_t){mmio_writes++;}
void mmio_write32(uint32_t,uint32_t){mmio_writes++;}
uint32_t mmio_read32(uint32_t){std::abort();}
#define CHECK(x) do{if(!(x)){std::fprintf(stderr,"FAIL %s\n",#x);return 1;}}while(0)
int main(){
    auto m=ppc_memory_init();CHECK(m);
    std::vector<uint64_t> pages(NATIVE_DIRTY_PAGES);uint64_t generation=0;
    st32(m,0x80001000,0x12345678);CHECK(generation==0);
    g_native_dirty_counter=&generation;g_native_dirty_pages=pages.data();
    st8(m,0x80001000,1);CHECK(generation==1&&pages[1]==1&&pages[2]==0);
    st16(m,0xc0001fff,0x1234);CHECK(generation==2&&pages[1]==2&&pages[2]==2);
    CHECK(m[0x80001fff]==0x12&&m[0x80002000]==0x34);
    st32(m,0x80001000,0xaabbccdd);CHECK(pages[1]==3&&ld32(m,0xc0001000)==0xaabbccdd);
    st64(m,0xd0003000,0x123456789abcdef0ull);CHECK(pages[6144+3]==4);
    CHECK(pages[6144+2]==0&&pages[6144+4]==0);
    ppc_dcbz(m,0x80002ff3);CHECK(pages[2]==5&&pages[3]==0);
    for(uint32_t a=0x80002fe0;a<0x80003000;a++)CHECK(m[a]==0);
    stf32(m,0x90004000,1.f);CHECK(pages[6144+4]==6);
    native_memory_written(0x93fff000,4096);CHECK(pages.back()==7);
    st32(m,0xcc005000,9);CHECK(mmio_writes==1&&generation==7);
    native_memory_written(0xe0000000,32);CHECK(generation==7&&pages[0]==0);
    g_native_dirty_pages=nullptr;g_native_dirty_counter=nullptr;
    ppc_memory_shutdown();
    std::puts("Native dirty tracking: real byte/half/word/double/float stores, cached aliases, page crossing, dcbz, MEM2 limit and MMIO exclusion passed");
}
