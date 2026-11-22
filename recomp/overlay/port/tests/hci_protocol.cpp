#include "../runtime/ios/hci.h"
#include <iostream>
#define CHECK(x) do{if(!(x))throw std::runtime_error(#x);}while(0)
int main() {
    mwinput::Hci h;
    auto command=[&](std::initializer_list<uint8_t> bytes){return h.execute(std::span(bytes.begin(),bytes.size()));};
    auto r=command({3,12,0});CHECK((r==std::vector<uint8_t>{14,4,1,3,12,0}));
    r=command({5,16,0});CHECK(r.size()==13&&r[6]==0&&r[7]==1&&r[9]==4);
    r=command({26,12,1,3});CHECK(r[5]==0&&h.settings[0x0c1a][0]==3);
    r=command({26,12,0});CHECK(r[5]==0x12&&h.settings[0x0c1a][0]==3);
    r=command({0x34,0x12,0});CHECK(r[5]==1&&r[3]==0x34&&r[4]==0x12);
    bool rejected=false;try{command({3,12,1});}catch(const std::runtime_error&){rejected=true;}CHECK(rejected);
    command({3,12,0});CHECK(h.settings.empty());
    std::cout<<"HCI protocol: little endian framing, capabilities, settings, invalid/unknown errors, reset passed\n";
}
