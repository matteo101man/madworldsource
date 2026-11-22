// Minimal virtual Bluetooth controller for original SDK initialization.
// HCI wire packets are little endian; no remote, pairing keys or ACL path yet.
#pragma once
#include <array>
#include <cstdint>
#include <map>
#include <span>
#include <stdexcept>
#include <vector>
namespace mwinput {
class Hci {
public:
    std::map<uint16_t,std::vector<uint8_t>> settings;
    std::vector<uint8_t> execute(std::span<const uint8_t> p) {
        if(p.size()<3||p.size()!=size_t(p[2])+3)throw std::runtime_error("HCI command length");
        uint16_t op=p[0]|uint16_t(p[1])<<8;
        std::vector<uint8_t> result{0};
        unsigned expected=0;bool setting=false,known=true;
        switch(op) {
        case 0x0c03:settings.clear();break;
        case 0x1001:result={0,3,1,0,3,0xff,0xff,1,0};break;
        case 0x1003:result.resize(9,0);break;
        case 0x1005:result={0,0,1,0,4,0,0,0};break;
        case 0x1009:result={0,1,0,0,0,0,2};break; // local administered virtual address
        case 0x0c0d:expected=7;result={0,0,0,0,0};break; // zero stored pairing records
        case 0x0c01:expected=8;setting=true;break;
        case 0x0c05:expected=p[2];setting=true;break;
        case 0x0c0a:case 0x0c1a:case 0x0c43:case 0x0c45:case 0x0c47:
        case 0x0c20:case 0x0c56:expected=1;setting=true;break;
        case 0x0c18:case 0x0c16:expected=2;setting=true;break;
        case 0x0c24:expected=3;setting=true;break;
        case 0x0c1c:case 0x0c1e:expected=4;setting=true;break;
        case 0x0c33:expected=7;setting=true;break;
        case 0x0c13:expected=248;setting=true;break;
        default:known=false;result={1};break; // Unknown HCI Command
        }
        if(known&&p[2]!=expected)result={0x12}; // Invalid HCI Command Parameters
        else if(setting)settings[op]={p.begin()+3,p.end()};
        std::vector<uint8_t> event{0x0e,uint8_t(3+result.size()),1,uint8_t(op),uint8_t(op>>8)};
        event.insert(event.end(),result.begin(),result.end());return event;
    }
};
}
