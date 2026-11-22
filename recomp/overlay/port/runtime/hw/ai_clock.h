// Wii AI counter/control only. No DSP, PCM DMA, disc stream or mixer yet.
#pragma once
#include <cstdint>
#include <functional>
#include "snapshot.h"
namespace mwaudio {
class Clock {
public:
    static constexpr uint64_t ticks_per_second=60750000;
    uint32_t control=0x42,volume=0,count=0,target=0;
    std::function<void(bool)> irq;
    void tick(uint64_t now) {
        uint64_t elapsed=now-last;last=now;
        if(!(control&1))return;
        uint64_t scaled=elapsed*((control&2)?48000:32000)+fraction;
        uint64_t samples=scaled/ticks_per_second;fraction=scaled%ticks_per_second;
        uint32_t distance=target-count;
        if((distance&&samples>=distance)||samples>=0x100000000ull)control|=8;
        count+=uint32_t(samples);line();
    }
    void set_control(uint32_t value,uint64_t now) {
        tick(now);
        uint32_t status=control&8;if(value&8)status=0;
        if((value^control)&3)fraction=0;
        control=(value&0x57)|status;
        if(value&32){count=0;fraction=0;}
        line();
    }
    void set_count(uint32_t value,uint64_t now){tick(now);count=value;fraction=0;}
    void state(StateArchive& ar){ar.pod(control);ar.pod(volume);ar.pod(count);ar.pod(target);ar.pod(last);ar.pod(fraction);}
    void line(){if(irq)irq((control&12)==12);}
private:
    uint64_t last=0,fraction=0;
};
}
