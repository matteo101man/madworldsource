#include "../runtime/gx/device.h"
#include <iostream>
#include <map>
#define CHECK(x) do{if(!(x))throw std::runtime_error(#x);}while(0)
using namespace mwgx;
void word(Fifo& f,uint8_t op,uint32_t value){f.push(op,1);f.push(value,4);}
int main() {
    // Byte and burst partitions must emit identical packets/state, including
    // long vertex payloads containing apparent command bytes and display lists.
    std::vector<uint8_t> stream;
    auto append=[&](uint8_t op,uint32_t value){stream.push_back(op);for(int i=3;i>=0;--i)stream.push_back(uint8_t(value>>(8*i)));};
    auto cpword=[&](uint8_t reg,uint32_t value){stream.push_back(8);append(reg,value);};
    cpword(0x50,1<<9);cpword(0x70,1|(4<<1));
    stream.insert(stream.end(),{0x90,0,96});
    for(unsigned i=0;i<96*12;++i)stream.push_back(uint8_t(i*61));
    append(0x61,0xfe0000ff);append(0x61,0x420123ab);
    append(0x10,0x00011000);for(uint8_t value:{1,2,3,4,5,6,7,8})stream.push_back(value);
    append(0x40,0x2000);for(uint8_t value:{0,0,0,32})stream.push_back(value);
    std::array<uint8_t,32> display{};display[0]=0x61;display[1]=0x45;display[4]=2;
    std::vector<std::vector<uint8_t>> expected;
    Fifo baseline;baseline.memory=[&](uint32_t a,size_t n)->Bytes{CHECK(a==0x2000&&n==32);return display;};
    baseline.command=[&](uint8_t,Bytes packet){expected.emplace_back(packet.begin(),packet.end());};
    for(uint8_t byte:stream)baseline.push(byte,1);
    CHECK(baseline.pending()==0&&baseline.vertex_size(0)==12);
    for(size_t chunk=1;chunk<=64;++chunk){
        Fifo batched;std::vector<std::vector<uint8_t>> actual;
        batched.memory=baseline.memory;batched.command=[&](uint8_t,Bytes packet){actual.emplace_back(packet.begin(),packet.end());};
        for(size_t at=0;at<stream.size();at+=chunk)batched.push_bytes(Bytes(stream).subspan(at,std::min(chunk,stream.size()-at)));
        CHECK(batched.pending()==0&&actual==expected);
        CHECK(batched.cp==baseline.cp&&batched.bp==baseline.bp&&batched.xf==baseline.xf);
    }
    Fifo f;int completed=0;
    f.command=[&](uint8_t,Bytes){++completed;};
    f.push(8,1);f.push(0x50,1);f.push(0x0200,2);CHECK(completed==0);f.push(0,2);
    CHECK(completed==1&&f.cp[0x50]==0x02000000);
    // Ignored attributes may have invalid formats; they contribute zero bytes.
    f.cp[0x50]=1<<9;f.cp[0x70]=1|(4<<1)|(7<<10)|(7<<14)|(7<<18);
    CHECK(f.vertex_size(0)==12);
    f.push(0x90,1);f.push(1,2);f.push(0x61000000,4);f.push(0x08000000,4);
    CHECK(completed==1);f.push(0x45000002,4);CHECK(completed==2);
    word(f,0x61,0xfe0000ff);word(f,0x61,0x420123ab);
    CHECK(f.bp[0x42]==0xab&&f.bp[0xfe]==0xffffff);
    word(f,0x61,0x420123cd);CHECK(f.bp[0x42]==0x0123cd);
    Device d;std::vector<uint8_t> ram(0x4000);std::map<uint32_t,bool> irq;
    d.ram=[&](uint32_t a,size_t n)->std::span<uint8_t>{if(a>ram.size()||n>ram.size()-a)throw std::runtime_error("RAM bounds");return {ram.data()+a,n};};
    d.irq=[&](uint32_t bit,bool value){irq[bit]=value;};
    auto pair=[&](unsigned off,uint32_t value){d.cp_write(off,uint16_t(value));d.cp_write(off+2,uint16_t(value>>16));};
    d.cpu_base=0x100;d.cpu_end=0x120;d.cpu_write=0x100;
    pair(0x20,0x100);pair(0x24,0x120);pair(0x34,0x100);pair(0x38,0x100);
    d.cp_write(2,17);d.pe_write(0xa,2);
    d.port(0x61,1);d.port(0x45000002,4);
    for(int i=0;i<26;++i)d.port(0,1);
    CHECK(d.bursts==0&&d.finishes==0&&d.gather_pending()==31);
    d.port(0,1);CHECK(d.bursts==1&&d.finishes==1&&irq[0x400]);
    CHECK(d.cpu_write==0x120&&d.cp_read(0x30)==0&&ram[0x100]==0x61);
    d.pe_write(0xa,10);CHECK(!irq[0x400]&&d.pe_read(0xa)==2);
    for(int i=0;i<32;++i)d.port(0,1);CHECK(d.cpu_write==0x100);
    // Detached display-list generation writes RAM without executing commands.
    d.cp_write(2,1);d.cpu_base=0x1000;d.cpu_end=0x1100;d.cpu_write=0x1000;
    d.port(0x61,1);d.port(0x48001234,4);for(int i=0;i<27;++i)d.port(0,1);
    CHECK(d.cpu_write==0x1020&&d.pe_read(0xe)==0&&!irq[0x200]);
    d.cpu_base=0x100;d.cpu_end=0x120;d.cpu_write=0x100;d.cp_write(2,17);
    d.pe_write(0xa,3);d.port(0x40,1);d.port(0x1000,4);d.port(32,4);
    for(int i=0;i<23;++i)d.port(0,1);
    CHECK(d.pe_read(0xe)==0x1234&&irq[0x200]);d.pe_write(0xa,7);CHECK(!irq[0x200]);
    // A complete burst queued while reads are disabled executes only on enable.
    d.cp_write(2,16);d.port(0x61,1);d.port(0x45000002,4);for(int i=0;i<27;++i)d.port(0,1);
    CHECK(d.finishes==1&&d.cp_read(0x30)==32);d.cp_write(2,17);CHECK(d.finishes==2);
    d.fifo.cp[0x50]=0x1234;d.port(0x61,1);d.reset();
    CHECK(d.gather_pending()==0&&d.fifo.cp[0x50]==0x1234);
    pair(0x3c,d.cp_read(0x38)|(d.cp_read(0x3a)<<16));d.cp_write(2,51);
    CHECK((d.cp_read(0)&16)&&irq[0x800]);
    d.port(0x61,1);d.port(0x45000002,4);for(int i=0;i<27;++i)d.port(0,1);
    CHECK(d.finishes==2&&d.cp_read(0x30)==32);
    d.cp_write(2,17);CHECK(d.finishes==3&&!irq[0x800]);
    Fifo indexed;std::array<uint8_t,8> xfbytes{0x12,0x34,0x56,0x78,0x90,0xab,0xcd,0xef};
    indexed.cp[0xac]=0x200;indexed.cp[0xbc]=8;
    indexed.memory=[&](uint32_t a,size_t n)->Bytes{CHECK(a==0x210&&n==8);return xfbytes;};
    word(indexed,0x20,0x00021004);CHECK(indexed.xf[4]==0x12345678&&indexed.xf[5]==0x90abcdef);
    Fifo badlist;std::array<uint8_t,32> truncated{};truncated[0]=0x90;truncated[2]=20;
    badlist.cp[0x50]=1<<9;badlist.cp[0x70]=1|(4<<1);
    badlist.memory=[&](uint32_t,size_t)->Bytes{return truncated;};
    bool bad=false;try{word(badlist,0x40,0x1000);badlist.push(32,4);}catch(const std::runtime_error&){bad=true;}CHECK(bad);
    bool rejected=false;try{f.push(0x18,1);}catch(const std::runtime_error&){rejected=true;}CHECK(rejected);
    std::cout<<"GX transport: fragmentation, vertex payload, masks, gather/wrap, detached DL, queued reads, PE IRQ/ACK, CP breakpoint, indexed XF, malformed DL passed\n";
}
