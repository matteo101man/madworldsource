// Captured native SDK attribute layout, in an isolated temporary NAND.
#include "../runtime/ios/ios.h"
#include "ppc_memory.h"
#include "native_config.h"
#include <chrono>
#include <filesystem>
#include <vector>
#include <cstdio>

uint8_t* g_mem=nullptr;
void ios_register(const std::string&,IOSDeviceFactory){}
int32_t IOSDevice::ioctl(const IOSRequest&,uint32_t,uint32_t,uint32_t,uint32_t,uint32_t){return IPC_EINVAL;}
int32_t IOSDevice::ioctlv(const IOSRequest&,uint32_t,const IOSVector*,uint32_t,const IOSVector*,uint32_t){return IPC_EINVAL;}
uint32_t mmio_read32(uint32_t){std::abort();}
uint8_t mmio_read8(uint32_t){std::abort();}
void mmio_write32(uint32_t,uint32_t){std::abort();}
void mmio_write8(uint32_t,uint8_t){std::abort();}
std::string guest_string(uint8_t* m,uint32_t at,size_t limit){std::string s;for(size_t i=0;i<limit&&m[at+i];++i)s.push_back(char(m[at+i]));return s;}
#include "../runtime/ios/fs.cpp"
#define CHECK(v) do{if(!(v)){std::fprintf(stderr,"NAND check failed line %d: %s\n",__LINE__,#v);return 1;}}while(0)
int main(){
    auto root=fs::temp_directory_path()/("mw-nand-fs-"+std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()));
    CHECK(fs::create_directory(root));
    // Rust canonicalize() supplies this prefix for the isolated UI test NAND.
    // Extended Windows paths require native separators even after appending a
    // Wii path containing '/' characters.
    native_config_state().options["MW_USER_DIR"]="\\\\?\\"+root.string();
    g_mem=ppc_memory_init();CHECK(g_mem);
    const uint32_t input=0x80010000,output=0x80010200;
    auto text=[&](uint32_t at,const char* value){while(*value)st8(g_mem,at++,uint8_t(*value++));st8(g_mem,at,0);};
    // Exact layout observed at native 8023C290's new-game save request:
    // stale prefix in the first six bytes, path at +6, total 76 bytes.
    for(unsigned i=0;i<76;++i)st8(g_mem,input+i,0);
    text(input,"/title");text(input+6,"/tmp");st8(g_mem,input+70,3);
    DevFs device;IOSRequest request{};
    CHECK(device.ioctl(request,3,input,76,0,0)==IPC_SUCCESS);
    text(input+6,"/tmp/option.dat");
    CHECK(device.ioctl(request,9,input,76,0,0)==IPC_SUCCESS);
    NandFile file;CHECK(file.open("/tmp/option.dat",3)==IPC_SUCCESS);
    st32(g_mem,output,0x12345678);CHECK(file.write(output,4)==4);CHECK(file.seek(0,0)==0);
    st32(g_mem,output,0);CHECK(file.read(output,4)==4);CHECK(ld32(g_mem,output)==0x12345678);CHECK(file.close()==0);
    CHECK(fs::file_size(root/"nand/tmp/option.dat")==4);
    // Remove only the four exact paths created by this synthetic test.
    CHECK(fs::remove(root/"nand/tmp/option.dat"));CHECK(fs::remove(root/"nand/tmp"));CHECK(fs::remove(root/"nand"));CHECK(fs::remove(root));
    ppc_memory_shutdown();std::puts("Native NAND attribute create/open/read/write passed");
}
