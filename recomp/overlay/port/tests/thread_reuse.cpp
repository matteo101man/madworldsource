// Exercise the real scheduler/fibers with synthetic SDK thread records.
#include "ppc_memory.h"
#include "../runtime/os/threads.cpp"
#include <cstdarg>
PPCTrace trace{};PPCTrace* g_ppc_trace=&trace;
uint8_t* test_memory=nullptr;
unsigned runs[3]{};uint16_t exit_state=0;bool failed=false;
constexpr uint32_t main_thread=0x80010000,worker_thread=0x80011000;
uint16_t mmio_read16(uint32_t){std::abort();}
void mmio_write16(uint32_t,uint16_t){std::abort();}
uint64_t ppc_timebase(){return 1;}
uint32_t mmio_read32(uint32_t){std::abort();}
void mmio_write32(uint32_t,uint32_t){std::abort();}
void native_host_poll(){}
void native_host_fail(const char*){std::abort();}
void os_deliver_interrupts(PPCContext&,uint8_t*){std::abort();}
void os_wait_for_event(){std::abort();}
void os_trace_hardware_state(){}
[[noreturn]] void ppc_fatal(const char* format,...){
    va_list args;va_start(args,format);std::vfprintf(stderr,format,args);va_end(args);
    failed=true;g_cur=g_boot;g_ppc_trace=g_boot->trace;host_fiber_switch(*g_boot->fiber);std::abort();
}
void ppc_call_indirect(PPCContext& c,uint8_t* m,uint32_t address){
    if(address==0x802fab38){st32(m,CUR_CONTEXT,c.r[3]);return;}
    if(address==0x802fad60)return;
    if(address>=0x80001000&&address<0x8000100c){runs[(address-0x80001000)/4]++;return;}
    if(address==0x80002000){st16(m,worker_thread+T_STATE,exit_state);c.r[3]=0;f_80301900(c,m);ppc_fatal("Exited worker resumed its old host fiber\n");}
    ppc_fatal("Unexpected synthetic SDK call %08x\n",address);
}
void run_test(void*){
    uint8_t* m=test_memory;
    PPCContext cpu{};cpu.msr=MSR_EE;cpu.r[1]=0x80030000;
    st32(m,CUR_THREAD,main_thread);st32(m,CUR_CONTEXT,main_thread);
    st16(m,main_thread+T_STATE,STATE_RUNNING);st32(m,main_thread+T_PRIORITY,16);
    os_threads_init(m,&cpu);
    for(unsigned generation=0;generation<3&&!failed;generation++){
        // Reuse the exact guest OSThread address with a fresh entry/context.
        std::memset(m+worker_thread,0,0x318);
        st32(m,worker_thread+4,0x80040000);st32(m,worker_thread+0x84,0x80002000);
        st32(m,worker_thread+0x198,0x80001000+generation*4);st32(m,worker_thread+0x19c,MSR_EE);
        st16(m,worker_thread+T_STATE,STATE_READY);st32(m,worker_thread+T_PRIORITY,8);
        enqueue_ready(m,worker_thread);exit_state=generation==1?STATE_MORIBUND:0;
        cpu.r[3]=0;f_80301900(cpu,m);
    }
    }
int main(){
    uint8_t* m=ppc_memory_init();if(!m)return 2;
    test_memory=m;
    host_fiber_call(run_test,nullptr);
    bool good=!failed&&runs[0]==1&&runs[1]==1&&runs[2]==1;
    os_threads_shutdown(nullptr);ppc_memory_shutdown();
    if(!good){std::fprintf(stderr,"thread reuse failed: entries=%u/%u/%u\n",runs[0],runs[1],runs[2]);return 1;}
    std::puts("Three reused SDK thread generations ran exactly once across detached/joinable exits");return 0;
}
