// Low-memory boot information that the Wii's loader (apploader + IOS) normally
// leaves for the game before jumping to __start. Values follow what a retail
// Wii provides; Dolphin's Boot_BS2Emu.cpp is the reference.
#include <cstdio>
#include <cstring>
#include <vector>

#include "hle.h"
#include "ppc_memory.h"

namespace {
std::vector<uint8_t> read_all(const std::string& path) {
    std::vector<uint8_t> d;
    if (FILE* f = std::fopen(path.c_str(), "rb")) {
        std::fseek(f, 0, SEEK_END);
        d.resize((size_t)std::ftell(f));
        std::fseek(f, 0, SEEK_SET);
        if (std::fread(d.data(), 1, d.size(), f) != d.size()) d.clear();
        std::fclose(f);
    }
    return d;
}

void w32(uint8_t* m, uint32_t a, uint32_t v) { st32(m, a, v); }
}  // namespace

std::string g_sys_dir;  // .../orig/RZZE8P/sys

void wii_setup_lowmem(uint8_t* m) {
    std::vector<uint8_t> boot = read_all(g_sys_dir + "/boot.bin");
    std::vector<uint8_t> bi2 = read_all(g_sys_dir + "/bi2.bin");
    std::vector<uint8_t> fst = read_all(g_sys_dir + "/fst.bin");
    if (boot.size() < 0x20 || bi2.empty() || fst.empty())
        ppc_fatal("missing boot.bin / bi2.bin / fst.bin in %s", g_sys_dir.c_str());

    // 0x80000000: disc ID (game code, maker, disc number, version, streaming)
    std::memcpy(m + 0x80000000u, boot.data(), 0x20);native_memory_written(0x80000000u,uint32_t(0x20));
    w32(m, 0x80000020u, 0x0D15EA5Eu);   // boot magic: booted from a disc
    w32(m, 0x80000024u, 0x00000001u);   // version
    w32(m, 0x80000028u, MEM1_SIZE);      // physical MEM1 size
    w32(m, 0x8000002Cu, 0x00000023u);   // console type: retail Wii
    w32(m, 0x800000CCu, 0);              // video mode: NTSC
    w32(m, 0x800000F0u, MEM1_SIZE);      // simulated MEM1 size
    w32(m, 0x800000F8u, 243000000u);     // bus clock
    w32(m, 0x800000FCu, 729000000u);     // CPU clock

    // The apploader copies the FST (disc file table) and bi2 to the top of MEM1
    // and lowers ArenaHi below them.
    uint32_t top = 0x817FE000u;
    uint32_t fst_addr = (top - (uint32_t)fst.size()) & ~31u;
    std::memcpy(m + fst_addr, fst.data(), fst.size());native_memory_written(fst_addr,uint32_t(fst.size()));
    uint32_t bi2_addr = (fst_addr - (uint32_t)bi2.size()) & ~31u;
    std::memcpy(m + bi2_addr, bi2.data(), bi2.size());native_memory_written(bi2_addr,uint32_t(bi2.size()));
    w32(m, 0x80000030u, 0);              // ArenaLo: 0 = let OSInit use the linker value
    w32(m, 0x80000034u, bi2_addr);       // ArenaHi
    w32(m, 0x80000038u, fst_addr);       // FST location
    w32(m, 0x8000003Cu, (uint32_t)fst.size());  // FST max size
    w32(m, 0x800000F4u, bi2_addr);       // bi2 location

    // Wii-specific block (written by IOS / the system menu)
    w32(m, 0x80003100u, MEM1_SIZE);      // MEM1 physical size
    w32(m, 0x80003104u, MEM1_SIZE);      // MEM1 simulated size
    w32(m, 0x8000310Cu, 0);              // MEM1 arena start: 0 = OSInit uses the linker's __ArenaLo
    w32(m, 0x80003110u, bi2_addr);       // MEM1 arena end
    w32(m, 0x80003118u, MEM2_SIZE);      // MEM2 physical size
    w32(m, 0x8000311Cu, MEM2_SIZE);      // MEM2 simulated size
    w32(m, 0x80003120u, 0x93400000u);    // end of MEM2 addressable by the game
    w32(m, 0x80003124u, 0x90000800u);    // usable MEM2 start
    w32(m, 0x80003128u, 0x933E0000u);    // usable MEM2 end
    w32(m, 0x80003130u, 0x933E0000u);    // IOS IPC buffer start
    w32(m, 0x80003134u, 0x93400000u);    // IOS IPC buffer end
    w32(m, 0x80003138u, 0x00000011u);    // Hollywood revision
    w32(m, 0x80003140u, 0x00240000u | 0x1FFF);  // IOS36, revision 0x1FFF
    w32(m, 0x80003144u, 0x00062507u);    // IOS build date
    w32(m, 0x80003188u, 0x00240000u | 0x1FFF);  // IOS version the title asked for
    std::memcpy(m + 0x80003180u, boot.data(), 4);native_memory_written(0x80003180u,uint32_t(4));  // game code
}
