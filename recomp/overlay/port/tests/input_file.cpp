#include "../runtime/ios/input_file.h"
#include <atomic>
#include <filesystem>
#include <fstream>
#include <thread>
#include <cstdio>

void write_packet(const std::filesystem::path& path, uint8_t value, size_t count=8) {
    std::ofstream file(path, std::ios::binary); for(size_t i=0;i<count;i++) file.put(char(value));
}
int main() {
    auto root=std::filesystem::absolute("port/build/input-file-fixture");
    std::filesystem::create_directories(root);
    auto path=root/"packet.bin", temp=root/"packet.tmp";
    write_packet(path,1);
    std::atomic<bool> done=false, failed=false;
    std::atomic<uint32_t> reads=0, misses=0, mixed=0;uint32_t move_errors=0, retries=0;
    std::thread reader([&]{
        while(!done) {
            std::array<uint8_t,8> packet{};
            if(!read_native_input(path.string().c_str(),packet)) { misses++; continue; }
            for(auto value:packet) if(value!=packet[0]) {failed=true;mixed++;}
            reads++;
        }
    });
    for(int i=0;i<100;i++) {
        write_packet(temp,uint8_t(i));
#ifdef _WIN32
        bool replaced=false;
        for(int attempt=0;attempt<20;attempt++) {
            if(MoveFileExW(temp.c_str(),path.c_str(),MOVEFILE_REPLACE_EXISTING)) {replaced=true;break;}
            retries++;Sleep(1);
        }
        if(!replaced) {failed=true;move_errors++;}
#else
        std::filesystem::rename(temp,path);
#endif
    }
    done=true; reader.join();
    std::array<uint8_t,8> packet{}; packet.fill(0x55);
    write_packet(path,2,9);
    bool malformed=read_native_input(path.string().c_str(),packet);
    for(auto value:packet) if(value!=0x55) failed=true;
    std::printf("Input fixture diagnostics: reads=%u transientMisses=%u moveErrors=%u retries=%u mixedPackets=%u malformedAccepted=%d\n",unsigned(reads),unsigned(misses),move_errors,retries,unsigned(mixed),malformed);
    if(failed || malformed || !reads) return 1;
    std::printf("Input file:100 atomic replacements during %u concurrent reads; exact8 bytes and no partial-packet acceptance\n",unsigned(reads));
}
