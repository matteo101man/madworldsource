#include "native_config.h"
#include "device.h"
#include "mmio.h"
#include "os.h"
#include "platform.h"
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <vector>

namespace {
mwgx::Device gx;
std::vector<uint8_t>* fifo_capture=nullptr;
std::span<uint8_t> memory(uint32_t address,size_t length) {
    address&=0x1fffffff;
    bool mem1=address<0x01800000&&length<=0x01800000-address;
    bool mem2=address>=0x10000000&&address<0x14000000&&length<=0x14000000-address;
    if(!mem1&&!mem2)throw std::runtime_error("GX physical RAM out of range");
    return {g_mem+(address|0x80000000u),length};
}
uint32_t read_regs(uint32_t a,int size,bool pixel) {
    unsigned off=a&0xff;
    auto get=[pixel](unsigned o){return pixel?gx.pe_read(o):gx.cp_read(o);};
    if(size==4)return uint32_t(get(off))<<16|get(off+2);
    uint16_t value=get(off&~1u);
    return size==2?value:(value>>((off&1)?0:8))&255;
}
void write_regs(uint32_t a,uint32_t value,int size,bool pixel) {
    unsigned off=a&0xff;
    auto put=[pixel](unsigned o,uint16_t v){if(pixel)gx.pe_write(o,v);else gx.cp_write(o,v);};
    if(size==4){put(off,uint16_t(value>>16));put(off+2,uint16_t(value));}
    else if(size==2)put(off,uint16_t(value));
    else throw std::runtime_error("GX byte register writes unsupported");
}
}
MMIO_DEVICE(gx) {
    gx.ram=memory;gx.irq=[](uint32_t bit,bool on){pi_set_cause(bit,on);};
    gx.observe=[](uint8_t op,mwgx::Bytes p){
        // Forward a display list's commands to the renderer in one call when the list ends,
        // so consecutive draws reach it together and batch (research/GPU-RENDERER.md). Lists
        // hold no guest-visible effects between their commands, so order and results are
        // unchanged. MW_GX_DL_BATCH=0 forwards each command separately.
        static const bool batch=[]{const char* v=native_config_get("MW_GX_DL_BATCH");return !v||std::strcmp(v,"0");}();
        static std::vector<uint8_t> list;
        if(batch&&gx.fifo.in_list)list.insert(list.end(),p.begin(),p.end());
        else if(op==0x40&&gx.fifo.list_passthrough){
            const uint32_t a=gx.fifo.list_address,s=gx.fifo.list_size;
            const uint8_t call[9]={0x40,uint8_t(a>>24),uint8_t(a>>16),uint8_t(a>>8),uint8_t(a),uint8_t(s>>24),uint8_t(s>>16),uint8_t(s>>8),uint8_t(s)};
            native_platform_gx_list(call);
        }
        else if(batch&&op==0x40){if(!list.empty()){native_platform_gx(list.data(),list.size());list.clear();}}
        else native_platform_gx(p.data(),p.size());
        static bool profile=native_config_get("MW_TRACE_GX_PERF")!=nullptr;
        static uint64_t zero_stride=0,empty=0,nonempty=0,vertices=0;
        if(profile&&op>=0x80&&op<=0xbf){
            uint32_t count=(uint32_t(p[1])<<8)|p[2];
            if(!count)++empty;
            else if(!gx.fifo.vertex_size(op&7))++zero_stride;
            else {++nonempty;vertices+=count;}
        }
        static uint64_t last=0;uint64_t now=ppc_timebase();
        if(op==0x61&&p[1]==0x45&&(gx.finishes<=8||(native_config_get("MW_TRACE_WORLD")&&now-last>=60750000)))
            {last=now;std::fprintf(stderr,"[GX] finish=%llu bursts=%llu commands=%llu draws=%llu\n",gx.finishes,gx.bursts,gx.commands,gx.draws);
             if(profile)std::fprintf(stderr,"[GX-perf] zero-stride=%llu empty=%llu nonempty=%llu vertices=%llu\n",zero_stride,empty,nonempty,vertices);}
    };
    mmio_register(0xcc000000,0x100,[](uint32_t a,int n){return read_regs(a,n,false);},[](uint32_t a,uint32_t v,int n){write_regs(a,v,n,false);});
    mmio_register(0xcc001000,0x100,[](uint32_t a,int n){return read_regs(a,n,true);},[](uint32_t a,uint32_t v,int n){write_regs(a,v,n,true);});
    mmio_register(0xcc00300c,16,[](uint32_t a,int n){
        if(n!=4)throw std::runtime_error("GX PI read width");
        switch(a&255){case 12:return gx.cpu_base;case 16:return gx.cpu_end;case 20:return gx.cpu_write;default:return 0u;}
    },[](uint32_t a,uint32_t v,int n){
        if(n!=4)throw std::runtime_error("GX PI write width");
        switch(a&255){case 12:gx.cpu_base=v&0x1fffffe0;break;case 16:gx.cpu_end=v&0x1fffffe0;break;case 20:gx.cpu_write=v&0x1fffffe0;break;case 24:if(v&1)gx.reset();break;}
    });
    mmio_register(0xcc008000,0x1000,[](uint32_t,int)->uint32_t{throw std::runtime_error("GX FIFO read");},[](uint32_t,uint32_t v,int n){
        if(fifo_capture)for(int i=n-1;i>=0;--i)fifo_capture->push_back(uint8_t(v>>(8*i)));
        gx.port(v,n);
    });
}

void native_gx_state(StateArchive& ar){gx.state(ar);}
// GX HLE compare mode (gx_hle.cpp): bytes the guest writes to the FIFO pipe, and the
// BP register image the FIFO has processed so far.
void native_gx_capture(std::vector<uint8_t>* out){fifo_capture=out;}
uint32_t native_gx_bp(unsigned reg){return gx.fifo.bp[reg&255];}
