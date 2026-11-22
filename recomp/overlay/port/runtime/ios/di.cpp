#include "native_config.h"
// /dev/di: the disc drive, backed by the extracted data partition (orig/RZZE8P/disc).
//
// Partition reads use offsets inside the decrypted data partition. They're served
// by mapping each offset to the extracted file that covers it, using the FST: the
// system area (boot.bin, bi2.bin, apploader, main.dol, fst.bin) and every file in
// files/. Bytes no file covers read as zero.
#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <vector>

#include "ios.h"
#include "mmio.h"
#include "os.h"
#include "di_transfer.h"
#include "pc_ui_overlay.h"

std::string g_disc_dir;  // .../orig/RZZE8P/disc

namespace {
// DI result codes
constexpr int32_t DI_SUCCESS = 1;
constexpr int32_t DI_DRIVE_ERROR = 2;
DITransferQueue transfer_queue;

uint8_t* mem() { return g_mem; }

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
uint32_t be32(const uint8_t* p) { return (uint32_t)p[0] << 24 | p[1] << 16 | p[2] << 8 | p[3]; }

struct Extent {
    uint64_t offset, size;
    std::string path;
};

class Partition {
public:
    bool load() {
        if (!extents_.empty()) return true;
        std::string sys = g_disc_dir + "/sys/";
        std::vector<uint8_t> boot = read_all(sys + "boot.bin");
        std::vector<uint8_t> fst = read_all(sys + "fst.bin");
        if (boot.size() < 0x440 || fst.size() < 12) {
            std::fprintf(stderr, "[di] extracted disc not found in %s\n", g_disc_dir.c_str());
            return false;
        }
        disc_id_.assign(boot.begin(), boot.begin() + 0x20);
        uint64_t dol_off = (uint64_t)be32(&boot[0x420]) << 2;
        uint64_t fst_off = (uint64_t)be32(&boot[0x424]) << 2;
        auto add = [&](uint64_t off, const std::string& path) {
            std::FILE* f = std::fopen(path.c_str(), "rb");
            if (!f) return;
            std::fseek(f, 0, SEEK_END);
            uint64_t size = (uint64_t)std::ftell(f);
            std::fclose(f);
            extents_.push_back({off, size, path});
        };
        add(0x0, sys + "boot.bin");
        add(0x440, sys + "bi2.bin");
        add(0x2440, sys + "apploader.img");
        add(dol_off, sys + "main.dol");
        add(fst_off, sys + "fst.bin");

        // FST: 12-byte entries {type:8 nameoff:24, offset (>>2 for files) or parent, size or next}
        uint32_t count = be32(&fst[8]);
        const char* names = reinterpret_cast<const char*>(fst.data()) + count * 12;
        std::vector<std::pair<uint32_t, std::string>> dirs = {{count, ""}};  // (end index, path)
        for (uint32_t i = 1; i < count; i++) {
            while (dirs.size() > 1 && i >= dirs.back().first) dirs.pop_back();
            const uint8_t* e = &fst[i * 12];
            std::string name = names + (be32(e) & 0xFFFFFF);
            std::string path = dirs.back().second + "/" + name;
            if (e[0]) dirs.push_back({be32(e + 8), path});
            else {
                std::string source = g_disc_dir + "/files" + path;
                if (path == "/etc/core_us.idd") {
                    const char* overlay = native_config_get("MW_PC_UI_FILE");
                    if (!select_pc_ui_archive(overlay, be32(e + 8), source)) {
                        std::fprintf(stderr, "[di] PC prompt resource missing or wrong size\n");
                        extents_.clear();
                        return false;
                    }
                    if (overlay && *overlay) std::fprintf(stderr, "[di] PC prompts: %s (%u bytes)\n", source.c_str(), be32(e + 8));
                }
                extents_.push_back({(uint64_t)be32(e + 4) << 2, be32(e + 8), source});
            }
        }
        std::sort(extents_.begin(), extents_.end(), [](const Extent& a, const Extent& b) { return a.offset < b.offset; });
        std::fprintf(stderr, "[di] data partition: %zu extents\n", extents_.size());
        return true;
    }

    // Copies [offset, offset+len) of the partition to guest memory.
    bool read(uint64_t offset, uint32_t len, uint32_t dst) {
        bool success=true;
        uint8_t* m = mem();
        for (uint32_t i = 0; i < len; i++) st8(m, dst + i, 0);
        auto it = std::upper_bound(extents_.begin(), extents_.end(), offset,
                                   [](uint64_t o, const Extent& e) { return o < e.offset; });
        if (it != extents_.begin()) --it;
        std::vector<uint8_t> buf;
        for (; it != extents_.end() && it->offset < offset + len; ++it) {
            uint64_t lo = std::max(offset, it->offset), hi = std::min(offset + len, it->offset + it->size);
            if (lo >= hi) continue;
            std::FILE* f = std::fopen(it->path.c_str(), "rb");
            if (!f) {std::fprintf(stderr,"[di] missing extracted file %s\n",it->path.c_str());success=false;continue;}
            buf.resize(hi - lo);
            _fseeki64(f, (int64_t)(lo - it->offset), SEEK_SET);
            size_t n = std::fread(buf.data(), 1, buf.size(), f);
            std::fclose(f);
            if(n!=buf.size()){std::fprintf(stderr,"[di] short read %s: %zu/%zu\n",it->path.c_str(),n,buf.size());success=false;}
            if(NATIVE_OPTION("MW_TRACE_DI"))std::fprintf(stderr,"[di] read %s +%llx len=%zu dst=%08x\n",it->path.c_str(),lo-it->offset,n,dst+(uint32_t)(lo-offset));
            for (size_t k = 0; k < n; k++) st8(m, dst + (uint32_t)(lo - offset + k), buf[k]);
        }
        return success;
    }

    const std::vector<uint8_t>& disc_id() const { return disc_id_; }

private:
    std::vector<Extent> extents_;
    std::vector<uint8_t> disc_id_;
};

Partition& partition() {
    static Partition p;
    return p;
}

void complete_read(const OSEventData& event) {
    int32_t result = partition().read(event.value,event.b,event.c) ? DI_SUCCESS : DI_DRIVE_ERROR;
    ios_reply(event.a,result);
}

class DevDi : public IOSDevice {
public:
    int32_t open(const std::string&, uint32_t) override { return partition().load() ? IPC_SUCCESS : IPC_ENOENT; }

    int32_t ioctl(const IOSRequest& request, uint32_t cmd, uint32_t in, uint32_t, uint32_t out, uint32_t out_len) override {
        uint8_t* m = mem();
        auto zero_out = [&] {
            for (uint32_t i = 0; i < out_len; i++) st8(m, out + i, 0);
        };
        switch (cmd) {
            case 0x12:  // Inquiry: drive revision / date
                zero_out();
                if (out_len >= 8) {
                    st32(m, out, 0x00000002u);
                    st32(m, out + 4, 0x20060526u);
                }
                return DI_SUCCESS;
            case 0x70:  // ReadDiskID
                for (uint32_t i = 0; i < out_len && i < 0x20; i++) st8(m, out + i, partition().disc_id()[i]);
                return DI_SUCCESS;
            case 0x71: {  // Read(length, offset >> 2) from the open partition
                uint32_t len = ld32(m, in + 4);
                uint64_t off = (uint64_t)ld32(m, in + 8) << 2;
                if (len > out_len) return DI_DRIVE_ERROR;
                uint32_t reply = request.addr;
                uint64_t deadline = transfer_queue.submit(ppc_timebase(), len);
                os_schedule(deadline, {OSEventKind::DiRead,reply,len,out,off});
                return IPC_PENDING;
            }
            case 0x8D: {  // UnencryptedRead: raw disc area (header, partition table, region)
                uint32_t len = ld32(m, in + 4);
                uint64_t off = (uint64_t)ld32(m, in + 8) << 2;
                zero_out();
                if (off == 0) {
                    for (uint32_t i = 0; i < len && i < 0x20; i++) st8(m, out + i, partition().disc_id()[i]);
                } else if (off == 0x40000 && len >= 0x28) {
                    st32(m, out, 1);                        // one partition in table 0
                    st32(m, out + 4, 0x40020u >> 2);        // table location
                    st32(m, out + 0x20, 0x50000u >> 2);     // data partition offset
                    st32(m, out + 0x24, 0);                 // type: data
                }
                return DI_SUCCESS;
            }
            case 0x7A:  // GetCoverRegister: cover closed
            case 0xE0:  // RequestError: none
                zero_out();
                return DI_SUCCESS;
            case 0x88:  // GetCoverStatus: disc inserted
                zero_out();
                if (out_len >= 4) st32(m, out, 2);
                return DI_SUCCESS;
            case 0x79: case 0x7E: case 0x86: case 0x8A: case 0x8C: case 0x8E: case 0xAB:
            case 0xD2: case 0xD9: case 0xDB: case 0xDC: case 0xDD: case 0xDF: case 0xE3: case 0xE4:
                zero_out();
                return DI_SUCCESS;
        }
        std::fprintf(stderr, "[di] unhandled ioctl 0x%X\n", cmd);
        return DI_SUCCESS;
    }

    int32_t ioctlv(const IOSRequest&, uint32_t cmd, const IOSVector*, uint32_t, const IOSVector* io,
                   uint32_t n_io) override {
        uint8_t* m = mem();
        if (cmd == 0x8B) {  // OpenPartition -> TMD, ES error
            std::vector<uint8_t> tmd = read_all(g_disc_dir + "/tmd.bin");
            if (n_io >= 1)
                for (uint32_t i = 0; i < io[0].len; i++) st8(m, io[0].addr + i, i < tmd.size() ? tmd[i] : 0);
            if (n_io >= 2 && io[1].len >= 4) st32(m, io[1].addr, 0);
            return DI_SUCCESS;
        }
        std::fprintf(stderr, "[di] unhandled ioctlv 0x%X\n", cmd);
        return DI_SUCCESS;
    }
};

struct RegisterDi {
    RegisterDi() {
        os_register_event(OSEventKind::DiRead, complete_read);
        ios_register("/dev/di", [](const std::string&) -> std::unique_ptr<IOSDevice> {
            return std::make_unique<DevDi>();
        });
    }
} g_register_di;
}  // namespace

#include "snapshot.h"
void native_di_state(StateArchive& ar){ar.pod(transfer_queue);if(ar.reading&&!partition().load())throw std::runtime_error("Snapshot partition unavailable");}
