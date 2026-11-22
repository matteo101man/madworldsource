#include "ios.h"
#include "hci.h"
#include "mmio.h"
#include <cstdio>
#include <cstring>
#include <deque>

std::unique_ptr<IOSDevice> create_platform_bluetooth();

namespace {
class Bluetooth : public IOSDevice {
    mwinput::Hci controller;
    std::deque<std::vector<uint8_t>> events;
    uint32_t pending=0,buffer=0,capacity=0;
    void deliver() {
        if(!pending||events.empty())return;
        auto& bytes=events.front();
        if(bytes.size()>capacity)throw std::runtime_error("HCI event exceeds submitted buffer");
        std::memcpy(g_mem+buffer,bytes.data(),bytes.size());
        uint32_t req=pending;pending=0;
        int32_t size=int32_t(bytes.size());events.pop_front();ios_reply(req,size);
    }
public:
    int32_t ioctlv(const IOSRequest& r,uint32_t cmd,const IOSVector* in,uint32_t ni,const IOSVector* io,uint32_t no) override {
        if(no!=1||!io[0].addr)return IPC_EINVAL;
        if(cmd==0) {
            if(ni!=6||in[4].len<2)return IPC_EINVAL;
            uint32_t length=uint32_t(g_mem[in[4].addr])|uint32_t(g_mem[in[4].addr+1])<<8;
            if(length>io[0].len||length<3)return IPC_EINVAL;
            std::span<const uint8_t> bytes(g_mem+io[0].addr,length);
            auto reply=controller.execute(bytes);
            std::fprintf(stderr,"[bluetooth] HCI %04X status=%02X\n",bytes[0]|uint16_t(bytes[1])<<8,reply[5]);
            if(events.size()>=128)throw std::runtime_error("HCI event queue bound");
            events.push_back(std::move(reply));deliver();return IPC_SUCCESS;
        }
        if((cmd==1||cmd==2)&&ni==2&&in[0].len>=1&&in[1].len>=2) {
            uint8_t endpoint=g_mem[in[0].addr];
            uint32_t length=ld16(g_mem,in[1].addr);
            if(length>io[0].len)return IPC_EINVAL;
            if(cmd==2&&endpoint==0x81) {
                if(pending)return IPC_EINVAL;
                pending=r.addr;buffer=io[0].addr;capacity=length;deliver();return IPC_PENDING;
            }
            if(cmd==1&&endpoint==0x82)return IPC_PENDING; // no remote ACL packets yet
        }
        std::fprintf(stderr,"[bluetooth] unsupported USB command=%u vectors=%u/%u\n",cmd,ni,no);
        return IPC_EINVAL;
    }
};
struct RegisterBluetooth {
    RegisterBluetooth(){ios_register("/dev/usb/oh1/57e/305",[](const std::string&)->std::unique_ptr<IOSDevice>{auto platform=create_platform_bluetooth();if(platform)return platform;return std::make_unique<Bluetooth>();});}
} registration;
}
