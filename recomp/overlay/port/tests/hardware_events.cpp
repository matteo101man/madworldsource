#include "os.h"
#include "snapshot.h"
#include <vector>
#include <cstdio>
#include <cstdlib>
#include <cstdarg>
uint64_t now=0;
std::vector<uint64_t> observed;
bool ppc_bench_unthrottled(){return false;}
void ppc_skip_timebase(uint64_t){}
uint64_t ppc_timebase(){return now;}
void os_irq_push(uint32_t){}void os_irq_pop(){}
void ppc_call_indirect(PPCContext&,uint8_t*,uint32_t){std::abort();}
[[noreturn]] void ppc_fatal(const char* format,...){va_list a;va_start(a,format);std::vfprintf(stderr,format,a);va_end(a);std::abort();}
void handle(const OSEventData& event){
    observed.push_back(event.value);
    if(event.a==1)os_schedule(now,{OSEventKind::DiRead,0,0,0,0x300000003ull});
}
int main(){
    os_register_event(OSEventKind::DiRead,handle);
    os_schedule(100,{OSEventKind::DiRead,1,2,3,0x100000001ull});
    os_schedule(100,{OSEventKind::DiRead,0,5,6,0x200000002ull});
    os_schedule(50,{OSEventKind::DiRead,0,8,9,0x400000004ull});
    StateArchive save;os_events_state(save);
    now=100;os_poll_hardware_events();auto expected=observed;
    StateArchive load(save.bytes);os_events_state(load);load.end();
    observed.clear();now=49;os_poll_hardware_events();if(!observed.empty())return 1;
    now=50;os_poll_hardware_events();if(observed!=std::vector<uint64_t>{0x400000004ull})return 1;
    now=100;os_poll_hardware_events();
    if(observed!=expected||observed!=std::vector<uint64_t>{0x400000004ull,0x100000001ull,0x200000002ull,0x300000003ull})return 1;
    std::puts("Hardware events: snapshot restores deadlines, stable equal-time order, 64-bit latched payload and callback rearming");
}
