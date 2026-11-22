// Synchronous, headless GX transport. No rasterization or GPU timing claim.
#pragma once
#include "fifo.h"
#include "native_dirty.h"
#include <algorithm>
#include <cstring>

namespace mwgx {
class Device {
public:
    Fifo fifo;
    std::function<std::span<uint8_t>(uint32_t,size_t)> ram;
    std::function<void(uint32_t,bool)> irq;
    std::function<void(uint8_t,Bytes)> observe;
    uint32_t cpu_base=0,cpu_end=0,cpu_write=0;
    uint64_t bursts=0,commands=0,draws=0,finishes=0;
    Device() {
        fifo.memory=[this](uint32_t a,size_t n)->Bytes {return ram(a,n);};
        fifo.command=[this](uint8_t op,Bytes p) {
            ++commands;
            if(op>=0x80&&op<=0xbf)++draws;
            if(op==0x61) {
                unsigned reg=p[1];uint32_t value=fifo.bp[reg];
                if(reg==0x45&&(value&255)==2){finish=true;++finishes;}
                if(reg==0x47||reg==0x48){pe[7]=uint16_t(value);if(reg==0x48)token=true;}
                lines();
            }
            if(observe)observe(op,p);
        };
    }
    uint32_t cp_read(unsigned off) {
        if(off==0)return status();
        return cp.at(off/2);
    }
    void cp_write(unsigned off,uint16_t value) {
        cp.at(off/2)=value;
        if(off>=0x20&&off<=0x3e) {
            if((off&2)==0)cp[off/2]&=0xffe0;
            else cp[off/2]&=0x1fff;
        }
        pump();lines();
    }
    uint16_t pe_read(unsigned off) const {return pe.at(off/2);}
    void pe_write(unsigned off,uint16_t value) {
        if(off==0xa){if(value&4)token=false;if(value&8)finish=false;value&=3;}
        pe.at(off/2)=value;lines();
    }
    void port(uint32_t value,int size) {
        if(size!=1&&size!=2&&size!=4)throw std::runtime_error("GX gather width");
        for(int i=size-1;i>=0;--i) {
            gather[used++]=uint8_t(value>>(8*i));
            if(used==32) {
                auto target=ram(cpu_write,32);
                if(target.size()!=32)throw std::runtime_error("GX gather destination");
                std::memcpy(target.data(),gather.data(),32);native_memory_written(cpu_write,32);
                cpu_write=cpu_write==cpu_end?cpu_base:cpu_write+32;
                used=0;++bursts;
                if(cp[1]&16) {
                    set_pair(0x34,next(pair(0x34)));
                    set_pair(0x30,pair(0x30)+32);
                    if(cp[1]&1){cpu_base=pair(0x20);cpu_end=pair(0x24);cpu_write=pair(0x34);}
                }
                pump();
            }
        }
    }
    void state(StateArchive& ar){if(pumping)throw std::runtime_error("Cannot snapshot an executing GX pump");ar.pod(cpu_base);ar.pod(cpu_end);ar.pod(cpu_write);ar.pod(bursts);ar.pod(commands);ar.pod(draws);ar.pod(finishes);ar.pod(cp);ar.pod(pe);ar.pod(gather);ar.pod(used);ar.pod(token);ar.pod(finish);if(used>=32)throw std::runtime_error("Snapshot GX gather bounds");fifo.state(ar);}
    void reset() {used=0;fifo.clear_pending();cp[1]=16;set_pair(0x2c,0);set_pair(0x28,0x1fffffe0);lines();}
    size_t gather_pending() const {return used;}
private:
    std::array<uint16_t,0x80> cp{},pe{};
    std::array<uint8_t,32> gather{};
    size_t used=0;
    bool token=false,finish=false,pumping=false;
    uint32_t pair(unsigned off) const {return cp[off/2]|uint32_t(cp[off/2+1])<<16;}
    void set_pair(unsigned off,uint32_t value){cp[off/2]=uint16_t(value);cp[off/2+1]=uint16_t(value>>16);}
    uint32_t next(uint32_t p) const {return p==pair(0x24)?pair(0x20):p+32;}
    bool at_break() const {return (cp[1]&2)&&pair(0x38)==pair(0x3c);}
    uint16_t status() const {
        uint32_t distance=pair(0x30);
        return (distance>pair(0x28)?1:0)|(distance<pair(0x2c)?2:0)|
            (!distance||pair(0x38)==pair(0x34)?4:0)|(!distance||at_break()||!(cp[1]&1)?8:0)|(at_break()?16:0);
    }
    void lines() {
        if(!irq)return;
        irq(0x200,token&&(pe[5]&1));irq(0x400,finish&&(pe[5]&2));
        uint16_t s=status(),c=cp[1];
        irq(0x800,(c&1)&&(((s&1)&&(c&4))||((s&2)&&(c&8))||((s&16)&&(c&32))));
    }
    void pump() {
        if(pumping)return;
        pumping=true;
        try {
            size_t budget=16*1024*1024;
            while((cp[1]&1)&&pair(0x30)>=32&&!at_break()) {
                if(budget<32)throw std::runtime_error("GX FIFO distance exceeds bound");
                budget-=32;
                auto data=ram(pair(0x38),32);
                if(data.size()!=32)throw std::runtime_error("GX CP source");
                // Consume the complete burst before any interrupt can enter guest code.
                set_pair(0x38,next(pair(0x38)));set_pair(0x30,pair(0x30)-32);
                fifo.push_bytes(data);
            }
            pumping=false;lines();
        }catch(...){pumping=false;throw;}
    }
};
}
