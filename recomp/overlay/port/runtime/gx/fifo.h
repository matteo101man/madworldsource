// Bounded GX command framing/state for the native CPU core. No rasterizer yet.
// Protocol reference: Dolphin OpcodeDecoding/CPMemory/PixelEngine, commit409881c.
// Independent implementation; retained state is intended for the Bevy bridge.
#pragma once
#include <algorithm>
#include <array>
#include <bit>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <unordered_map>
#include <cstdint>
#include <functional>
#include <span>
#include <stdexcept>
#include <vector>
#include "snapshot.h"
#include "native_dirty.h"

namespace mwgx {
using Bytes=std::span<const uint8_t>;
// MW_GX_DL_STATS=1: once per second, how GX work splits between display lists and
// top-level (inline) commands, and how often a display list is called again with the
// same content. Measurement for the display-list cache (research/GX-HLE.md stage 3).
struct DlStats {
    bool on=std::getenv("MW_GX_DL_STATS")!=nullptr;
    uint64_t calls=0,bytes=0,same=0,rewritten=0,fresh=0,dl_draws=0,dl_vertices=0,top_draws=0,top_vertices=0,top_state=0,dl_state=0;
    std::unordered_map<uint64_t,uint64_t> seen;  // (address<<32|size) -> content hash
    std::chrono::steady_clock::time_point last=std::chrono::steady_clock::now();
    static uint64_t hash(Bytes b){uint64_t h=1469598103934665603ull;for(uint8_t x:b){h^=x;h*=1099511628211ull;}return h;}
    void list(uint32_t address,uint32_t size,Bytes content){
        ++calls;bytes+=size;
        uint64_t key=uint64_t(address)<<32|size,h=hash(content);
        auto it=seen.find(key);
        if(it==seen.end()){++fresh;seen.emplace(key,h);}
        else if(it->second==h)++same;
        else{++rewritten;it->second=h;}
    }
    void command(uint8_t op,Bytes packet,unsigned depth){
        bool draw=op>=0x80&&op<=0xbf;
        uint32_t count=draw?(uint32_t(packet[1])<<8|packet[2]):0;
        if(depth){if(draw){++dl_draws;dl_vertices+=count;}else if(op!=0)++dl_state;}
        else{if(draw){++top_draws;top_vertices+=count;}else if(op!=0&&op!=0x40)++top_state;}
        auto now=std::chrono::steady_clock::now();
        if(now-last<std::chrono::seconds(1))return;
        double dt=std::chrono::duration<double>(now-last).count();last=now;
        std::fprintf(stderr,"[gx-dl] calls/s=%.0f MB/s=%.2f same=%llu rewritten=%llu fresh=%llu unique=%zu | dl draws/s=%.0f verts/s=%.0f state/s=%.0f | top draws/s=%.0f verts/s=%.0f state/s=%.0f\n",
            calls/dt,bytes/dt/1e6,(unsigned long long)same,(unsigned long long)rewritten,(unsigned long long)fresh,seen.size(),
            dl_draws/dt,dl_vertices/dt,dl_state/dt,top_draws/dt,top_vertices/dt,top_state/dt);
        calls=bytes=same=rewritten=fresh=dl_draws=dl_vertices=top_draws=top_vertices=top_state=dl_state=0;
    }
};
inline uint32_t be32(const uint8_t* p) { return uint32_t(p[0])<<24|uint32_t(p[1])<<16|uint32_t(p[2])<<8|p[3]; }
class Fifo {
public:
    std::array<uint32_t,256> cp{},bp{};
    std::array<uint32_t,0x1100> xf{};
    std::function<Bytes(uint32_t,size_t)> memory;
    std::function<void(uint8_t,Bytes)> command;
    DlStats dl_stats;
    // True while the commands of a display list are being decoded.
    bool in_list=false;
    // MW_GX_DL_PASSTHROUGH=1: a display list already seen to hold only draws, whose pages
    // were not written since, is not re-parsed here. The call itself (masked address and
    // size) goes to the renderer, which reads the list from RAM at the same moment.
    bool passthrough_enabled=[]{const char* v=std::getenv("MW_GX_DL_PASSTHROUGH");return v&&v[0]=='1';}();
    bool list_passthrough=false;
    uint32_t list_address=0,list_size=0;
    Fifo() { bp[0xfe]=0xffffff; }
    void state(StateArchive& ar){ar.pod(cp);ar.pod(bp);ar.pod(xf);ar.vector(queue);ar.pod(cursor);if(cursor>queue.size())throw std::runtime_error("Snapshot GX FIFO cursor");}
    size_t pending() const { return queue.size()-cursor; }
    void clear_pending() { queue.clear();cursor=0; }
    void push(uint32_t value,int size) {
        if(size!=1&&size!=2&&size!=4)throw std::runtime_error("invalid GX port write width");
        std::array<uint8_t,4> bytes{};
        for(int i=0;i<size;++i)bytes[i]=uint8_t(value>>(8*(size-1-i)));
        push_bytes(Bytes(bytes.data(),size));
    }
    // A CP hardware burst has no intervening guest execution. Append it once,
    // then decode complete packets in exactly the same order as byte writes.
    void push_bytes(Bytes bytes) {
        if(bytes.size()>16*1024*1024-pending())throw std::runtime_error("GX pending command exceeds bound");
        queue.insert(queue.end(),bytes.begin(),bytes.end());
        while(cursor<queue.size()) {
            size_t n=parse(Bytes(queue).subspan(cursor),0);
            if(!n)break;
            cursor+=n;
        }
        if(cursor==queue.size()){queue.clear();cursor=0;}
        else if(cursor>65536){queue.erase(queue.begin(),queue.begin()+cursor);cursor=0;}
    }
    uint32_t vertex_size(unsigned vat) const {
        if(vat>=8)throw std::runtime_error("invalid GX VAT");
        uint32_t lo=cp[0x50],hi=cp[0x60],a=cp[0x70+vat],b=cp[0x80+vat],c=cp[0x90+vat];
        uint32_t n=std::popcount(lo&0x1ff);
        n+=component((lo>>9)&3,2+(a&1),(a>>1)&7);
        bool nbt=(a>>9)&1,index3=(a>>31)&1;
        n+=component((lo>>11)&3,nbt?9:3,(a>>10)&7,nbt&&index3?3:1);
        n+=color_component((lo>>13)&3,(a>>14)&7);
        n+=color_component((lo>>15)&3,(a>>18)&7);
        std::array<uint32_t,8> counts{1+((a>>21)&1),1+(b&1),1+((b>>9)&1),1+((b>>18)&1),1+((b>>27)&1),1+((c>>5)&1),1+((c>>14)&1),1+((c>>23)&1)};
        std::array<uint32_t,8> formats{(a>>22)&7,(b>>1)&7,(b>>10)&7,(b>>19)&7,(b>>28)&7,(c>>6)&7,(c>>15)&7,(c>>24)&7};
        for(unsigned i=0;i<8;i++)n+=component((hi>>(i*2))&3,counts[i],formats[i]);
        return n;
    }
private:
    struct ListInfo {uint64_t generation;bool draws_only;};
    std::unordered_map<uint64_t,ListInfo> lists;
    // Newest dirty-page generation over a guest range; UINT64_MAX when untracked.
    static uint64_t range_generation(uint32_t address,uint32_t size) {
        if(!g_native_dirty_pages||!size)return UINT64_MAX;
        uint32_t first=address&0x3fffffffu;uint64_t end=uint64_t(first)+size-1;
        size_t begin_page,end_page;
        if(first<0x01800000u&&end<0x01800000u){begin_page=first>>12;end_page=size_t(end)>>12;}
        else if(first>=0x10000000u&&end<0x14000000u){begin_page=6144+((first-0x10000000u)>>12);end_page=6144+((size_t(end)-0x10000000u)>>12);}
        else return UINT64_MAX;
        uint64_t g=0;
        for(size_t page=begin_page;page<=end_page;page++)g=std::max(g,g_native_dirty_pages[page]);
        return g;
    }
    std::vector<uint8_t> queue;
    size_t cursor=0;
    static uint32_t scalar(unsigned format) {
        constexpr uint32_t sizes[]{1,1,2,2,4};
        if(format>=5)throw std::runtime_error("unsupported GX scalar format");
        return sizes[format];
    }
    static uint32_t color(unsigned format) {
        constexpr uint32_t sizes[]{2,3,4,2,3,4};
        if(format>=6)throw std::runtime_error("unsupported GX color format");
        return sizes[format];
    }
    static uint32_t component(unsigned type,uint32_t count,unsigned format,uint32_t indices=1) {
        return type==0?0:type==1?count*scalar(format):(type-1)*indices;
    }
    static uint32_t color_component(unsigned type,unsigned format) {
        return type==0?0:type==1?color(format):type-1;
    }
    Bytes fetch(uint32_t address,size_t size) {
        if(!memory)throw std::runtime_error("GX memory resolver missing");
        Bytes bytes=memory(address,size);
        if(bytes.size()!=size)throw std::runtime_error("truncated GX memory range");
        return bytes;
    }
    size_t parse(Bytes bytes,unsigned depth) {
        uint8_t op=bytes[0];size_t n=1;
        if(op==8)n=6;
        else if(op==0x61||op==0x20||op==0x28||op==0x30||op==0x38)n=5;
        else if(op==0x40)n=9;
        else if(op==0x10){
            if(bytes.size()<5)return 0;
            uint32_t count=be32(bytes.data()+1)>>16;
            if(count>=16)throw std::runtime_error("GX XF transfer exceeds16 registers");
            n=5+(count+1)*4;
        }else if(op>=0x80&&op<=0xbf){
            if(bytes.size()<3)return 0;
            n=3+size_t((bytes[1]<<8)|bytes[2])*vertex_size(op&7);
        }else if(op!=0&&op!=0x48)throw std::runtime_error("unknown GX command");
        if(bytes.size()<n)return 0;
        Bytes packet=bytes.first(n);
        if(op==8)cp[bytes[1]]=be32(bytes.data()+2);
        else if(op==0x61){
            uint32_t word=be32(bytes.data()+1),reg=word>>24,value=word&0xffffff;
            if(reg==0xfe)bp[reg]=value;
            else {bp[reg]=(bp[reg]&~bp[0xfe])|(value&bp[0xfe]);bp[0xfe]=0xffffff;}
        }else if(op==0x10){
            uint32_t word=be32(bytes.data()+1),address=word&0xffff,count=(word>>16)+1;
            if(address+count>xf.size())throw std::runtime_error("GX XF register bounds");
            for(unsigned i=0;i<count;i++)xf[address+i]=be32(bytes.data()+5+i*4);
        }else if(op==0x20||op==0x28||op==0x30||op==0x38){
            uint32_t word=be32(bytes.data()+1),array=op/8+8,count=((word>>12)&15)+1,dest=word&0xfff;
            if(dest+count>0x1000)throw std::runtime_error("GX indexed XF bounds");
            uint32_t address=(cp[0xa0+array]&0x1fffffff)+(word>>16)*(cp[0xb0+array]&0xff);
            Bytes source=fetch(address,count*4);
            for(unsigned i=0;i<count;i++)xf[dest+i]=be32(source.data()+i*4);
        }else if(op==0x40){
            if(depth)throw std::runtime_error("nested GX display list unsupported");
            uint32_t address=be32(bytes.data()+1)&~31u,size=be32(bytes.data()+5)&~31u;
            if(size>16*1024*1024)throw std::runtime_error("GX display list too large");
            list_address=address;list_size=size;list_passthrough=false;
            uint64_t generation=passthrough_enabled?range_generation(address,size):UINT64_MAX;
            uint64_t key=uint64_t(address)<<32|size;
            if(generation!=UINT64_MAX){
                auto known=lists.find(key);
                if(known!=lists.end()&&known->second.draws_only&&generation<=known->second.generation){
                    list_passthrough=true;
                    if(command)command(op,packet);
                    return n;
                }
            }
            bool draws_only=true;
            Bytes list=fetch(address,size);size_t p=0;
            if(dl_stats.on)dl_stats.list(address,size,list);
            in_list=true;
            while(p<list.size()){
                uint8_t inner=list[p];
                draws_only&=inner==0||(inner>=0x80&&inner<=0xbf);
                size_t consumed=parse(list.subspan(p),depth+1);
                if(!consumed){in_list=false;throw std::runtime_error("truncated GX display list command");}
                p+=consumed;
            }
            in_list=false;
            if(generation!=UINT64_MAX)lists[key]=ListInfo{generation,draws_only};
        }
        if(dl_stats.on)dl_stats.command(op,packet,depth);
        if(command)command(op,packet);
        return n;
    }
};
}
