// Hardware registers (0xCC000000 legacy GameCube block, 0xCD000000 Hollywood).
//
// Devices register handlers for their address ranges (mmio_register). Anything
// unclaimed falls back to a plain register file: writes are remembered, reads
// return the last value written (0 initially), and the first access to each
// address is logged so missing devices are easy to spot.
#include "mmio.h"
#include "os.h"

#include <cstdio>
#include <unordered_map>
#include <unordered_set>
#include <vector>

namespace {
struct Device {
    uint32_t base, size;
    MMIORead read;
    MMIOWrite write;
};
std::vector<Device>& devices() {
    static std::vector<Device> d;
    return d;
}

std::unordered_map<uint32_t, uint32_t> g_regs;   // fallback, by 32-bit aligned address
std::unordered_set<uint32_t> g_seen;

void note(const char* what, uint32_t a, uint32_t v) {
    if (g_seen.size() < 1000 && g_seen.insert(a).second)
        std::fprintf(stderr, "[mmio] unhandled %s 0x%08X (0x%08X)\n", what, a, v);
}

// Canonical address for device lookup:
//  - DI/SI/EXI/AI (offset 0x6000-0x6FFF) appear at both 0xCC006xxx and 0xCD006xxx;
//  - Hollywood registers at 0xCD8xxxxx mirror 0xCD0xxxxx.
uint32_t canonical(uint32_t a) {
    if ((a & 0xFFF00000u) == 0xCD800000u) a &= ~0x00800000u;
    if ((a & 0x00FFF000u) == 0x00006000u) a &= ~0x01000000u;
    return a;
}

const Device* find(uint32_t a) {
    a = canonical(a);
    for (const Device& d : devices())
        if (a - d.base < d.size) return &d;
    return nullptr;
}
}  // namespace

void mmio_register(uint32_t base, uint32_t size, MMIORead read, MMIOWrite write) {
    devices().push_back({canonical(base), size, std::move(read), std::move(write)});
}

uint32_t mmio_reg_get(uint32_t a) {
    auto it = g_regs.find(a & ~3u);
    return it == g_regs.end() ? 0 : it->second;
}
void mmio_reg_set(uint32_t a, uint32_t v) { g_regs[a & ~3u] = v; }

static uint32_t do_read(uint32_t a, int size) {
    os_poll_hardware_events();
    if (const Device* d = find(a)) return d->read(a, size);
    uint32_t w = mmio_reg_get(a);
    uint32_t v = size == 4 ? w : size == 2 ? (w >> ((a & 2) ? 0 : 16)) & 0xFFFF : (w >> (24 - 8 * (a & 3))) & 0xFF;
    note(size == 4 ? "read32 " : size == 2 ? "read16 " : "read8  ", a, v);
    return v;
}

static void do_write(uint32_t a, uint32_t v, int size) {
    if (const Device* d = find(a)) {
        d->write(a, v, size);
        return;
    }
    note(size == 4 ? "write32" : size == 2 ? "write16" : "write8 ", a, v);
    if (size == 4) {
        mmio_reg_set(a, v);
    } else {
        uint32_t sh = size == 2 ? ((a & 2) ? 0 : 16) : 24 - 8 * (a & 3);
        uint32_t mask = (size == 2 ? 0xFFFFu : 0xFFu) << sh;
        mmio_reg_set(a, (mmio_reg_get(a) & ~mask) | ((v << sh) & mask));
    }
}

uint32_t mmio_read32(uint32_t a) { return do_read(a, 4); }
uint16_t mmio_read16(uint32_t a) { return (uint16_t)do_read(a, 2); }
uint8_t mmio_read8(uint32_t a) { return (uint8_t)do_read(a, 1); }
void mmio_write32(uint32_t a, uint32_t v) { do_write(a, v, 4); }
void mmio_write16(uint32_t a, uint16_t v) { do_write(a, v, 2); }
void mmio_write8(uint32_t a, uint8_t v) { do_write(a, v, 1); }

uint8_t* g_mem;

namespace {
std::vector<MMIODeviceInit::Fn>& device_inits() {
    static std::vector<MMIODeviceInit::Fn> v;
    return v;
}
}  // namespace

MMIODeviceInit::MMIODeviceInit(Fn fn) { device_inits().push_back(fn); }

void mmio_init_devices() {
    for (MMIODeviceInit::Fn fn : device_inits()) fn();
}

#include "snapshot.h"
void native_mmio_state(StateArchive& ar){
    struct Register{uint32_t address,value;};std::vector<Register> values;
    if(!ar.reading)for(const auto& [a,v]:g_regs)values.push_back({a,v});
    ar.vector(values,65536);
    if(ar.reading){g_regs.clear();g_seen.clear();for(const auto& r:values){if(g_regs.contains(r.address))throw std::runtime_error("Duplicate fallback MMIO register");g_regs[r.address]=r.value;}}
}
