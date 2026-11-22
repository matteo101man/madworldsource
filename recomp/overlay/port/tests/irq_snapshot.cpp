// Exercise the real interrupt-delivery routine with non-LIFO host fibers.
#include "mmio.h"
#include "os.h"
#include "ppc_memory.h"
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <cstdarg>
#define WIN32_LEAN_AND_MEAN
#include <windows.h>

namespace {
PPCContext a{},b{};uint8_t* memory;void* fiber_a;void* fiber_b;
}
bool ppc_bench_unthrottled(){return false;}
void ppc_skip_timebase(uint64_t){}
uint64_t ppc_timebase(){return 1000;}
void os_irq_push(uint32_t){}void os_irq_pop(){}
[[noreturn]] void ppc_fatal(const char* format,...){va_list v;va_start(v,format);std::vfprintf(stderr,format,v);va_end(v);std::abort();}
void ppc_call_indirect(PPCContext& c,uint8_t*,uint32_t){
    pi_set_cause(PI_INT_CP,false);
    c.r[3]=0xdeadbeef;c.r[7]=0x12345678;c.cr=0xffffffff;c.f[4].ps1=999.;c.gqr[2]=123;
    if(&c==&a){pi_set_cause(PI_INT_CP,true);SwitchToFiber(fiber_b);}
    else SwitchToFiber(fiber_a);
}
void WINAPI run_b(void*){os_deliver_interrupts(b,memory);SwitchToFiber(fiber_a);std::abort();}
int main(){
    memory=ppc_memory_init();if(!memory)return 2;g_mem=memory;
    st32(memory,0x800000d4,0x80003000);
    mmio_init_devices();mmio_write32(0xcc003004,PI_INT_CP);
    a.msr=b.msr=MSR_EE;a.r[3]=111;b.r[3]=222;a.f[4].ps1=1.25;b.f[4].ps1=2.5;
    PPCContext expected_a=a,expected_b=b;
    fiber_a=ConvertThreadToFiberEx(nullptr,FIBER_FLAG_FLOAT_SWITCH);
    fiber_b=CreateFiberEx(0,1<<20,FIBER_FLAG_FLOAT_SWITCH,run_b,nullptr);
    if(!fiber_a||!fiber_b)return 2;
    pi_set_cause(PI_INT_CP,true);os_deliver_interrupts(a,memory);
    bool first=std::memcmp(&a,&expected_a,sizeof a)==0;
    SwitchToFiber(fiber_b);
    bool second=std::memcmp(&b,&expected_b,sizeof b)==0;
    DeleteFiber(fiber_b);ConvertFiberToThread();ppc_memory_shutdown();
    std::printf("IRQ interleaving: A r3=%u expected111, B r3=%u expected222; full contexts %s/%s\n",a.r[3],b.r[3],first?"preserved":"CORRUPTED",second?"preserved":"CORRUPTED");
    return first&&second?0:1;
}
