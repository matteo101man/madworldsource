#include "native_config.h"
// IOS request dispatch, file descriptors and the device registry.
#include "ios.h"

#include <cstdio>
#include <cstdlib>
#include <map>
#include <vector>

#include "mmio.h"

namespace {
enum : uint32_t { IOS_OPEN = 1, IOS_CLOSE, IOS_READ, IOS_WRITE, IOS_SEEK, IOS_IOCTL, IOS_IOCTLV, IOS_REPLY };

struct Registration {
    std::string path;
    bool prefix;
    IOSDeviceFactory factory;
};
std::vector<Registration>& registry() {
    static std::vector<Registration> r;
    return r;
}

std::map<uint32_t, std::unique_ptr<IOSDevice>> g_fds;
uint32_t g_next_fd = 3;
bool g_trace = native_config_get("MW_TRACE_IOS") != nullptr;

uint8_t* mem() { return g_mem; }

const char* cmd_name(uint32_t cmd) {
    static const char* names[] = {"?", "open", "close", "read", "write", "seek", "ioctl", "ioctlv"};
    return cmd < 8 ? names[cmd] : "?";
}

std::unique_ptr<IOSDevice> make_device(const std::string& path) {
    const Registration* best = nullptr;
    for (const Registration& r : registry()) {
        bool match = r.prefix ? path.compare(0, r.path.size(), r.path) == 0 : path == r.path;
        if (match && (!best || r.path.size() > best->path.size())) best = &r;
    }
    if (!best) return nullptr;
    std::unique_ptr<IOSDevice> d = best->factory(path);
    if (d) d->name = path;
    return d;
}
}  // namespace

void ios_register(const std::string& path, IOSDeviceFactory factory) {
    bool prefix = !path.empty() && path.back() == '*';
    registry().push_back({prefix ? path.substr(0, path.size() - 1) : path, prefix, factory});
}

int32_t IOSDevice::ioctl(const IOSRequest&, uint32_t cmd, uint32_t, uint32_t in_len, uint32_t, uint32_t out_len) {
    std::fprintf(stderr, "[ios] %s: unhandled ioctl 0x%X (in %u, out %u bytes)\n", name.c_str(), cmd, in_len, out_len);
    return IPC_EINVAL;
}

int32_t IOSDevice::ioctlv(const IOSRequest&, uint32_t cmd, const IOSVector*, uint32_t n_in, const IOSVector*,
                          uint32_t n_io) {
    std::fprintf(stderr, "[ios] %s: unhandled ioctlv 0x%X (%u in, %u io)\n", name.c_str(), cmd, n_in, n_io);
    return IPC_EINVAL;
}

void ios_reply(uint32_t req_addr, int32_t result) {
    uint8_t* m = mem();
    uint32_t cmd = ld32(m, req_addr);
    st32(m, req_addr + 4, (uint32_t)result);
    st32(m, req_addr + 8, cmd);          // the reply carries the original command here
    st32(m, req_addr, IOS_REPLY);
    ipc_queue_reply(req_addr & 0x3FFFFFFFu);
}

void ios_execute(uint32_t req_phys) {
    uint8_t* m = mem();
    IOSRequest r;
    r.addr = ios_virt(req_phys);
    r.cmd = ld32(m, r.addr);
    r.fd = ld32(m, r.addr + 8);
    for (int i = 0; i < 5; i++) r.arg[i] = ld32(m, r.addr + 0xC + 4 * i);
    if (g_trace)
        std::fprintf(stderr, "[ios] request @0x%08X cmd=%u fd=%d args=%08X %08X %08X %08X %08X\n", r.addr, r.cmd,
                     (int32_t)r.fd, r.arg[0], r.arg[1], r.arg[2], r.arg[3], r.arg[4]);

    int32_t result = IPC_EINVAL;
    if (r.cmd == IOS_OPEN) {
        std::string path = guest_string(m, ios_virt(r.arg[0]), 64);
        std::unique_ptr<IOSDevice> dev = make_device(path);
        if (!dev) {
            std::fprintf(stderr, "[ios] open %s: no such device\n", path.c_str());
            result = IPC_ENOENT;
        } else {
            result = dev->open(path, r.arg[1]);
            if (result >= 0) {
                uint32_t fd = g_next_fd++;
                g_fds[fd] = std::move(dev);
                result = (int32_t)fd;
            }
            std::fprintf(stderr, "[ios] open %s (mode %u) -> %d\n", path.c_str(), r.arg[1], result);
        }
    } else {
        auto it = g_fds.find(r.fd);
        if (it == g_fds.end()) {
            std::fprintf(stderr, "[ios] %s on bad fd %u\n", cmd_name(r.cmd), r.fd);
            result = IPC_EINVAL;
        } else {
            IOSDevice& d = *it->second;
            switch (r.cmd) {
                case IOS_CLOSE:
                    result = d.close();
                    g_fds.erase(it);
                    break;
                case IOS_READ: result = d.read(ios_virt(r.arg[0]), r.arg[1]); break;
                case IOS_WRITE: result = d.write(ios_virt(r.arg[0]), r.arg[1]); break;
                case IOS_SEEK: result = d.seek((int32_t)r.arg[0], r.arg[1]); break;
                case IOS_IOCTL:
                    result = d.ioctl(r, r.arg[0], ios_virt(r.arg[1]), r.arg[2], ios_virt(r.arg[3]), r.arg[4]);
                    break;
                case IOS_IOCTLV: {
                    uint32_t n_in = r.arg[1], n_io = r.arg[2], vec = ios_virt(r.arg[3]);
                    std::vector<IOSVector> v(n_in + n_io);
                    for (uint32_t i = 0; i < n_in + n_io; i++)
                        v[i] = {ios_virt(ld32(m, vec + 8 * i)), ld32(m, vec + 8 * i + 4)};
                    result = d.ioctlv(r, r.arg[0], v.data(), n_in, v.data() + n_in, n_io);
                    break;
                }
                default:
                    std::fprintf(stderr, "[ios] unknown command %u\n", r.cmd);
            }
        }
    }
    if (result != IPC_PENDING) ios_reply(r.addr, result);
}

// ---- simple devices --------------------------------------------------------

namespace {
// /dev/stm/immediate: power/video control. Accept everything.
class StmImmediate : public IOSDevice {
public:
    int32_t ioctl(const IOSRequest&, uint32_t cmd, uint32_t, uint32_t, uint32_t out, uint32_t out_len) override {
        if (out && out_len >= 4) st32(mem(), out, 0);
        std::fprintf(stderr, "[ios] stm/immediate ioctl 0x%X\n", cmd);
        return IPC_SUCCESS;
    }
};

// /dev/stm/eventhook: the event hook only replies on a power/reset button press.
class StmEventHook : public IOSDevice {
public:
    int32_t ioctl(const IOSRequest&, uint32_t, uint32_t, uint32_t, uint32_t, uint32_t) override { return IPC_PENDING; }
};

struct RegisterSimple {
    RegisterSimple() {
        ios_register("/dev/stm/immediate", [](const std::string&) -> std::unique_ptr<IOSDevice> {
            return std::make_unique<StmImmediate>();
        });
        ios_register("/dev/stm/eventhook", [](const std::string&) -> std::unique_ptr<IOSDevice> {
            return std::make_unique<StmEventHook>();
        });
    }
} g_register_simple;
}  // namespace

// Terminal host teardown, while platform code and borrowed guest RAM are live.
void ios_shutdown_devices(){g_fds.clear();}

void native_ios_state(StateArchive& ar){
    ar.tag(0x494f5344);ar.pod(g_next_fd);
    uint64_t count=g_fds.size();ar.pod(count);if(count>256)throw std::runtime_error("Snapshot IOS fd count");
    if(ar.reading){
        g_fds.clear();
        for(uint64_t i=0;i<count;++i){uint32_t fd=0;std::string path;ar.pod(fd);ar.string(path);auto dev=make_device(path);if(!dev||fd<3||fd>=g_next_fd||g_fds.contains(fd))throw std::runtime_error("Snapshot IOS device unavailable");dev->snapshot_state(ar);g_fds.emplace(fd,std::move(dev));}
    }else for(auto& [fd,dev]:g_fds){uint32_t key=fd;ar.pod(key);ar.string(dev->name);dev->snapshot_state(ar);}
}
