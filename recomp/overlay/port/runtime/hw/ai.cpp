#include "ai_clock.h"
#include "mmio.h"
#include "os.h"
#include "platform.h"
#include <stdexcept>
namespace {
mwaudio::Clock clock;
void update(const OSEventData&){clock.tick(ppc_timebase());os_schedule(ppc_timebase()+60750,{OSEventKind::Ai});}
}
MMIO_DEVICE(ai) {
    os_register_event(OSEventKind::Ai, update);
    clock.irq=[](bool on){pi_set_cause(PI_INT_AI,on);};
    clock.tick(ppc_timebase());os_schedule(ppc_timebase()+60750,{OSEventKind::Ai});
    mmio_register(0xcc006c00,16,[](uint32_t a,int n)->uint32_t{
        if(n!=4)throw std::runtime_error("AI register read width");
        clock.tick(ppc_timebase());
        switch(a&15){case 0:return clock.control;case 4:return clock.volume;case 8:return clock.count;case 12:return clock.target;}
        return 0;
    },[](uint32_t a,uint32_t value,int n){
        if(n!=4)throw std::runtime_error("AI register write width");
        uint64_t now=ppc_timebase();clock.tick(now);
        switch(a&15){case 0:clock.set_control(value,now);native_platform_ai_control(value);break;case 4:clock.volume=value;break;case 8:clock.set_count(value,now);break;case 12:clock.target=value;break;}
    });
}

#include "snapshot.h"
void native_ai_state(StateArchive& ar){clock.state(ar);}
