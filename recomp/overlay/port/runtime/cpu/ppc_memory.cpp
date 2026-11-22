#include "ppc_memory.h"
#include "native_dirty.h"
#include "native_config.h"

#include <cstring>
#include <initializer_list>

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <windows.h>

namespace {
uint8_t* g_base;
HANDLE g_mem1, g_mem2;

bool map_view(HANDLE section, uint8_t* at, uint32_t size) {
    return MapViewOfFileEx(section, FILE_MAP_ALL_ACCESS, 0, 0, size, at) == at;
}

void unmap_all() {
    if (!g_base) return;
    for (uint32_t a : {0x80000000u, 0xC0000000u, 0x90000000u, 0xD0000000u})
        UnmapViewOfFile(g_base + a);
    VirtualFree(g_base + LOCKED_CACHE, 0, MEM_RELEASE);
}
}  // namespace

uint8_t* ppc_memory_init() {
    g_mem1 = CreateFileMappingW(INVALID_HANDLE_VALUE, nullptr, PAGE_READWRITE, 0, MEM1_SIZE, nullptr);
    g_mem2 = CreateFileMappingW(INVALID_HANDLE_VALUE, nullptr, PAGE_READWRITE, 0, MEM2_SIZE, nullptr);
    if (!g_mem1 || !g_mem2) return nullptr;
    // Find a free 4 GB window, release it, then map our views into it.
    // Another thread could grab part of it in between, so retry a few times.
    for (int attempt = 0; attempt < 16; attempt++) {
        bool fixed=native_config_get("MW_TEST_SNAPSHOT_WRITE")||native_config_get("MW_TEST_SNAPSHOT_READ");
        void* p = VirtualAlloc(fixed?reinterpret_cast<void*>(0x20000000000ull):nullptr, 0x100000000ull, MEM_RESERVE, PAGE_NOACCESS);
        if (!p) return nullptr;
        VirtualFree(p, 0, MEM_RELEASE);
        g_base = (uint8_t*)p;
        bool ok = map_view(g_mem1, g_base + 0x80000000u, MEM1_SIZE) &&
                  map_view(g_mem1, g_base + 0xC0000000u, MEM1_SIZE) &&
                  map_view(g_mem2, g_base + 0x90000000u, MEM2_SIZE) &&
                  map_view(g_mem2, g_base + 0xD0000000u, MEM2_SIZE) &&
                  VirtualAlloc(g_base + LOCKED_CACHE, LOCKED_CACHE_SIZE, MEM_RESERVE | MEM_COMMIT,
                               PAGE_READWRITE) == g_base + LOCKED_CACHE;
        if (ok) return g_base;
        unmap_all();
    }
    g_base = nullptr;
    return nullptr;
}

void ppc_memory_shutdown() {
    unmap_all();
    if (g_mem1) CloseHandle(g_mem1);
    if (g_mem2) CloseHandle(g_mem2);
    g_base = nullptr;
}
#else
#error "Only Windows is supported for now"
#endif

uint32_t ppc_load_dol(uint8_t* m, const uint8_t* dol, size_t size) {
    auto be32 = [&](size_t off) {
        return (uint32_t)dol[off] << 24 | (uint32_t)dol[off + 1] << 16 | (uint32_t)dol[off + 2] << 8 | dol[off + 3];
    };
    if (size < 0x100) return 0;
    for (int i = 0; i < 18; i++) {
        uint32_t off = be32(i * 4), addr = be32(0x48 + i * 4), sz = be32(0x90 + i * 4);
        if (!sz) continue;
        if ((size_t)off + sz > size) return 0;
        std::memcpy(m + addr, dol + off, sz);native_memory_written(addr,sz);
    }
    // .bss needs no clearing: freshly mapped guest memory is zero.
    return be32(0xE0);
}
