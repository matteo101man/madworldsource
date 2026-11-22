#include "../runtime/hw/ai_clock.h"
#include <iostream>
#include <stdexcept>
#define CHECK(x) do{if(!(x))throw std::runtime_error(#x);}while(0)
int main(){
    mwaudio::Clock c;bool irq=false;c.irq=[&](bool on){irq=on;};
    c.tick(100);CHECK(c.count==0);c.target=48000;c.set_control(7,100);
    c.tick(100+60750000);CHECK(c.count==48000&&irq);
    c.set_control(15,100+60750000);CHECK(!irq&&c.control==7);
    c.set_control(6,100+60750000);c.tick(100+2*60750000);CHECK(c.count==48000);
    c.set_control(5|32,100+2*60750000);CHECK(c.count==0&&!(c.control&32));
    c.tick(100+3*60750000);CHECK(c.count==32000);
    c.set_count(0xfffffff0,100+3*60750000);c.target=4;
    c.tick(100+3*60750000+60750);CHECK(c.count==16&&irq);
    mwaudio::Clock parts;parts.set_control(3,0);
    for(uint64_t n=1;n<=10000;++n)parts.tick(n*6075);
    CHECK(parts.count==48000);
    std::cout<<"AI clock: stopped/32K/48K rates, fractional ticks, reset, wrap and interrupt ACK passed\n";
}
