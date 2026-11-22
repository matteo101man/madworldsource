// Synthetic extracted partition; exercise the real /dev/di asynchronous path.
#include "../runtime/ios/ios.h"
#include "ppc_memory.h"
#include "os.h"
#include <filesystem>
#include <fstream>
#include <functional>
#include <vector>
#include <utility>
#include <algorithm>

uint8_t* g_mem = nullptr;
uint64_t clock_now = 0;
IOSDeviceFactory factory = nullptr;
std::vector<std::pair<uint64_t, OSEventData>> events;
OSEventHandler handler=nullptr;
std::vector<std::pair<uint32_t, int32_t>> replies;
uint64_t ppc_timebase() { return clock_now; }
void os_register_event(OSEventKind,OSEventHandler f){handler=f;}
void os_schedule(uint64_t when, OSEventData data) { events.emplace_back(when,data); }
void ios_register(const std::string&, IOSDeviceFactory value) { factory = value; }
void ios_reply(uint32_t request, int32_t value) { replies.emplace_back(request, value); }
int32_t IOSDevice::ioctl(const IOSRequest&, uint32_t, uint32_t, uint32_t, uint32_t, uint32_t) { return IPC_EINVAL; }
int32_t IOSDevice::ioctlv(const IOSRequest&, uint32_t, const IOSVector*, uint32_t, const IOSVector*, uint32_t) { return IPC_EINVAL; }
uint32_t mmio_read32(uint32_t) { std::abort(); }
void mmio_write32(uint32_t, uint32_t) { std::abort(); }
void mmio_write8(uint32_t, uint8_t) { std::abort(); }
#include "../runtime/ios/di.cpp"

void pump() {
    for (;;) {
        auto found = std::find_if(events.begin(), events.end(), [](auto& event) { return event.first <= clock_now; });
        if (found == events.end()) break;
        auto action = std::move(found->second); events.erase(found); handler(action);
    }
}
void be(std::vector<uint8_t>& bytes, size_t offset, uint32_t value) {
    for (int i = 0; i < 4; i++) bytes[offset + i] = uint8_t(value >> (24 - 8 * i));
}
void write_fixture(const std::filesystem::path& path, const std::vector<uint8_t>& bytes) {
    std::ofstream stream(path, std::ios::binary); stream.write((const char*)bytes.data(), bytes.size());
}
#define CHECK(x) do { if (!(x)) { std::fprintf(stderr,"FAIL %s\n",#x); return 1; } } while(0)
int main() {
    auto root = std::filesystem::absolute("port/build/di-async-fixture");
    std::filesystem::create_directories(root / "sys"); std::filesystem::create_directories(root / "files");
    std::vector<uint8_t> boot(0x440), fst(36);
    be(boot, 0x420, 0x6000 / 4); be(boot, 0x424, 0x8000 / 4);
    be(fst, 0, 0x01000000); be(fst, 8, 2); be(fst, 16, 0x10000 / 4); be(fst, 20, 4);
    const char filename[] = "fixture.bin"; std::copy(filename, filename + sizeof filename, fst.begin() + 24);
    write_fixture(root / "sys/boot.bin", boot); write_fixture(root / "sys/fst.bin", fst);
    write_fixture(root / "files/fixture.bin", {0x10,0x20,0x30,0x40});
    g_disc_dir = root.string(); g_mem = ppc_memory_init(); CHECK(g_mem && factory);
    auto device = factory("/dev/di"); CHECK(device->open("/dev/di", 0) == 0);
    IOSRequest request{}; request.addr = 0x80010040;
    uint32_t input = 0x80010000, output = 0x80020000;
    st32(g_mem, input + 4, 4); st32(g_mem, input + 8, 0x10000 / 4);
    st32(g_mem, output, 0xaaaaaaaa);
    CHECK(device->ioctl(request, 0x71, input, 12, output, 4) == IPC_PENDING);
    CHECK(replies.empty() && ld32(g_mem, output) == 0xaaaaaaaa);
    uint64_t first = events[0].first; CHECK(first > 36450);
    request.addr = 0x80010080;
    CHECK(device->ioctl(request, 0x71, input, 12, output + 32, 4) == IPC_PENDING);
    uint64_t second = events[1].first; CHECK(second == first * 2);
    // Requests latch their arguments; caller mutation must not change pending reads.
    st32(g_mem, input + 8, 0);
    clock_now = first - 1; pump(); CHECK(replies.empty() && ld32(g_mem, output) == 0xaaaaaaaa);
    clock_now = first; pump(); CHECK(replies.size() == 1 && replies[0].first == 0x80010040 && replies[0].second == 1);
    CHECK(ld32(g_mem, output) == 0x10203040);
    clock_now = second; pump(); CHECK(replies.size() == 2 && ld32(g_mem, output + 32) == 0x10203040);
    st32(g_mem, input + 8, 0x10000 / 4);
    CHECK(device->ioctl(request, 0x71, input, 12, output, 3) == 2 && events.empty());
    // Delete only the exact synthetic file this test created.
    std::filesystem::remove(root / "files/fixture.bin");
    CHECK(device->ioctl(request, 0x71, input, 12, output, 4) == IPC_PENDING);
    clock_now = events[0].first; pump(); CHECK(replies.back().second == 2);
    ppc_memory_shutdown();
    std::puts("DI: real device defers DMA/reply, latches requests, serializes drive transfers and reports size/missing-file errors");
}
