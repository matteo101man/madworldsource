#include "mmio.h"
#include "os.h"
#include "../runtime/ios/ios.h"
#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <cstdarg>
#include <vector>
namespace {
uint64_t now=0;bool irq=false;std::vector<std::pair<uint64_t,OSEventData>> events;
OSEventHandler handler=nullptr;
std::vector<uint32_t> executed;
}
uint64_t ppc_timebase(){return now;}
void os_register_event(OSEventKind,OSEventHandler f){handler=f;}
void os_schedule(uint64_t t,OSEventData data){events.push_back({t,data});}
void os_poll_hardware_events(){
    for(;;){auto i=std::find_if(events.begin(),events.end(),[](auto& e){return e.first<=now;});if(i==events.end())return;auto fn=std::move(i->second);events.erase(i);handler(fn);}
}
void hollywood_set_irq(uint32_t,bool on){irq=on;}
void ios_execute(uint32_t request){executed.push_back(request);ipc_queue_reply(request);}
[[noreturn]] void ppc_fatal(const char* f,...){va_list a;va_start(a,f);std::vfprintf(stderr,f,a);va_end(a);std::abort();}
#define CHECK(x) do{if(!(x)){std::fprintf(stderr,"FAIL %s\n",#x);return 1;}}while(0)
int main(){
    mmio_init_devices();
    mmio_write32(0xcd000000,0x123400);mmio_write32(0xcd000004,0x31);
    CHECK((mmio_read32(0xcd000004)&7)==1&&!irq&&executed.empty());
    mmio_write32(0xcd000000,0x567800);now=499;
    CHECK((mmio_read32(0xcd000004)&7)==1&&executed.empty());
    now=500;CHECK((mmio_read32(0xcd000004)&7)==2&&irq);
    CHECK(executed.size()==1&&executed[0]==0x123400);
    mmio_write32(0xcd000004,0x32);
    CHECK((mmio_read32(0xcd000004)&7)==4&&mmio_read32(0xcd000008)==0x123400&&irq);
    mmio_write32(0xcd000004,0x34);CHECK(!irq);
    mmio_write32(0xcd000004,0x31);now=1000;
    CHECK((mmio_read32(0xcd000004)&7)==2&&executed.back()==0x567800);
    std::puts("IPC transport: delayed ACK, latched pointer, ACK-before-reply, W1C and IRQ levels passed");
}
