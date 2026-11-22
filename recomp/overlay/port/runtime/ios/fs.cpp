#include "native_config.h"
// NAND file system: /dev/fs (ISFS ioctls) and NAND file paths ("/shared2/...", "/title/...").
//
// The NAND lives in a host folder (port/user/nand by default, MW_USER_DIR overrides
// the parent). On first run the system files the SDK needs (SYSCONF, setting.txt)
// are copied from Dolphin's NAND if there is one, so the console settings match.
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <system_error>
#include <vector>

#include "ios.h"
#include "mmio.h"

namespace fs = std::filesystem;

namespace {
// /dev/fs error codes
constexpr int32_t FS_EINVAL = -101;
constexpr int32_t FS_EEXIST = -105;
constexpr int32_t FS_ENOENT = -106;
constexpr int32_t FS_EIO = -114;

uint8_t* mem() { return g_mem; }

fs::path nand_root() {
    static fs::path root = [] {
        const char* user = native_config_get("MW_USER_DIR");
        fs::path r = fs::path(user ? user : "port/user") / "nand";
        std::error_code ec;
        fs::create_directories(r, ec);
        // Seed from Dolphin's NAND on first run.
        if (const char* home = native_config_get("USERPROFILE")) {
            fs::path dolphin = fs::path(home) / "Documents" / "Dolphin Emulator" / "Wii";
            for (const char* rel : {"shared2/sys/SYSCONF", "title/00000001/00000002/data/setting.txt"}) {
                fs::path src = dolphin / rel, dst = r / rel;
                if (!fs::exists(dst) && fs::exists(src)) {
                    fs::create_directories(dst.parent_path(), ec);
                    fs::copy_file(src, dst, ec);
                    std::fprintf(stderr, "[fs] seeded %s from Dolphin's NAND\n", rel);
                }
            }
        }
        return r;
    }();
    return root;
}

// NAND path ("/shared2/sys/SYSCONF") -> host path. Rejects escapes out of the root.
bool host_path(const std::string& nand, fs::path* out) {
    if (nand.empty() || nand[0] != '/' || nand.find("..") != std::string::npos) return false;
    // Rust's canonical user directory can carry the Windows extended-path
    // prefix. Those paths do not accept Wii '/' separators in fopen; normalize
    // the complete host path after appending the NAND-relative components.
    *out = (nand_root() / nand.substr(1)).make_preferred();
    return true;
}

std::string read_path(uint32_t addr) { return guest_string(mem(), addr, 64); }

// ---- an open NAND file -------------------------------------------------
class NandFile : public IOSDevice {
public:
    int32_t open(const std::string& path, uint32_t mode) override {
        fs::path p;
        if (!host_path(path, &p)) return FS_EINVAL;
        if (!fs::is_regular_file(p)) return FS_ENOENT;
        mode_ = mode;
        f_ = std::fopen(p.string().c_str(), mode & 2 ? "r+b" : "rb");
        return f_ ? IPC_SUCCESS : FS_EIO;
    }
    int32_t close() override {
        if (f_) std::fclose(f_);
        f_ = nullptr;
        return IPC_SUCCESS;
    }
    ~NandFile() override { close(); }
    void snapshot_state(StateArchive& ar) override {
        int64_t position=ar.reading?0:_ftelli64(f_);ar.pod(mode_);ar.pod(position);
        if(position<0)throw std::runtime_error("Snapshot NAND file position");
        if(ar.reading){if(open(name,mode_)<0||_fseeki64(f_,position,SEEK_SET))throw std::runtime_error("Cannot reopen snapshot NAND file");}
    }
    int32_t read(uint32_t buf, uint32_t len) override {
        if (!(mode_ & 1)) return -102;
        std::vector<uint8_t> tmp(len);
        size_t n = std::fread(tmp.data(), 1, len, f_);
        for (size_t i = 0; i < n; i++) st8(mem(), buf + (uint32_t)i, tmp[i]);
        return (int32_t)n;
    }
    int32_t write(uint32_t buf, uint32_t len) override {
        if (!(mode_ & 2)) return -102;
        std::vector<uint8_t> tmp(len);
        for (uint32_t i = 0; i < len; i++) tmp[i] = ld8(mem(), buf + i);
        size_t n = std::fwrite(tmp.data(), 1, len, f_);
        std::fflush(f_);
        return (int32_t)n;
    }
    int32_t seek(int32_t off, uint32_t whence) override {
        static const int map[] = {SEEK_SET, SEEK_CUR, SEEK_END};
        if (whence > 2 || std::fseek(f_, off, map[whence])) return FS_EINVAL;
        return (int32_t)std::ftell(f_);
    }
    int32_t ioctl(const IOSRequest& r, uint32_t cmd, uint32_t in, uint32_t in_len, uint32_t out,
                  uint32_t out_len) override {
        if (cmd == 0x0B && out_len >= 8) {  // GetFileStats: size, position
            long pos = std::ftell(f_);
            std::fseek(f_, 0, SEEK_END);
            long size = std::ftell(f_);
            std::fseek(f_, pos, SEEK_SET);
            st32(mem(), out, (uint32_t)size);
            st32(mem(), out + 4, (uint32_t)pos);
            return IPC_SUCCESS;
        }
        return IOSDevice::ioctl(r, cmd, in, in_len, out, out_len);
    }

private:
    FILE* f_ = nullptr;
    uint32_t mode_ = 0;
};

// ---- /dev/fs -------------------------------------------------------------
// ISFS attribute block: owner u32, group u16, path[64], owner/group/other perms, attributes.
constexpr uint32_t ATTR_PATH = 6, ATTR_SIZE = 0x4C;

class DevFs : public IOSDevice {
public:
    int32_t ioctl(const IOSRequest& r, uint32_t cmd, uint32_t in, uint32_t in_len, uint32_t out,
                  uint32_t out_len) override {
        uint8_t* m = mem();
        std::error_code ec;
        fs::path p;
        // Bounded diagnostics for original NAND request layouts. The SDK's
        // creation buffer must be observed before assuming GetAttr's layout.
        static unsigned create_traces = 0;
        const bool trace_request = (cmd == 3 || cmd == 9 || cmd == 7 || cmd == 8) && NATIVE_OPTION("MW_TRACE_NAND") && create_traces++ < 64;
        if (trace_request) {
            std::fprintf(stderr, "[fs-create] cmd=%u in=%08x bytes=%u payload=", cmd, in, in_len);
            for (uint32_t i = 0; i < in_len && i < 96; ++i) std::fprintf(stderr, "%02x", ld8(m, in+i));
            std::fprintf(stderr, "\n");
        }
        switch (cmd) {
            case 0x02:  // GetStats
                for (uint32_t i = 0; i + 4 <= out_len && i < 0x1C; i += 4) st32(m, out + i, 0);
                if (out_len >= 8) {
                    st32(m, out, 0x4000);        // cluster size
                    st32(m, out + 4, 0x5000);    // free clusters
                }
                return IPC_SUCCESS;
            case 0x03:  // CreateDir(attr)
                if (!host_path(read_path(in + ATTR_PATH), &p)) return FS_EINVAL;
                if (fs::exists(p)) return FS_EEXIST;
                return fs::create_directories(p, ec) ? IPC_SUCCESS : FS_EIO;
            case 0x05:  // SetAttr
                return IPC_SUCCESS;
            case 0x06: {  // GetAttr(path) -> attr
                std::string path = read_path(in);
                if (!host_path(path, &p)) return FS_EINVAL;
                if (!fs::exists(p)) return FS_ENOENT;
                for (uint32_t i = 0; i < ATTR_SIZE && i < out_len; i++) st8(m, out + i, 0);
                for (size_t i = 0; i < path.size() && i < 63; i++) st8(m, out + ATTR_PATH + (uint32_t)i, path[i]);
                st8(m, out + 0x46, 3);  // owner perm: read/write
                st8(m, out + 0x47, 3);  // group perm
                st8(m, out + 0x48, 3);  // other perm
                return IPC_SUCCESS;
            }
            case 0x07:  // Delete(path)
                if (!host_path(read_path(in), &p)) return FS_EINVAL;
                if (trace_request) std::fprintf(stderr,"[fs-delete] path=%s\n",p.string().c_str());
                if (!fs::exists(p)) return FS_ENOENT;
                fs::remove_all(p, ec);
                return ec ? FS_EIO : IPC_SUCCESS;
            case 0x08: {  // Rename(old[64], new[64])
                fs::path to;
                if (!host_path(read_path(in), &p) || !host_path(read_path(in + 64), &to)) return FS_EINVAL;
                if (!fs::exists(p)) return FS_ENOENT;
                fs::remove_all(to, ec);
                fs::create_directories(to.parent_path(), ec);
                fs::rename(p, to, ec);
                return ec ? FS_EIO : IPC_SUCCESS;
            }
            case 0x09: {  // CreateFile(attr)
                if (!host_path(read_path(in + ATTR_PATH), &p)) return FS_EINVAL;
                if (trace_request) std::fprintf(stderr,"[fs-create-file] path=%s exists=%d\n",p.string().c_str(),int(fs::exists(p)));
                if (fs::exists(p)) return FS_EEXIST;
                fs::create_directories(p.parent_path(), ec);
                FILE* f = std::fopen(p.string().c_str(), "wb");
                if (trace_request) std::fprintf(stderr,"[fs-create-file] opened=%d error=%s\n",int(f!=nullptr),ec.message().c_str());
                if (!f) return FS_EIO;
                std::fclose(f);
                return IPC_SUCCESS;
            }
            case 0x0A:  // SetFileVerCtrl
            case 0x0D:  // Shutdown
            case 0x01:  // Format
                return IPC_SUCCESS;
        }
        return IOSDevice::ioctl(r, cmd, in, in_len, out, out_len);
    }

    int32_t ioctlv(const IOSRequest& r, uint32_t cmd, const IOSVector* in, uint32_t n_in, const IOSVector* io,
                   uint32_t n_io) override {
        uint8_t* m = mem();
        fs::path p;
        if (cmd == 0x04) {  // ReadDir(path [, max count]) -> [names (13 bytes each),] count
            if (n_in < 1 || !host_path(read_path(in[0].addr), &p)) return FS_EINVAL;
            if (!fs::is_directory(p)) return FS_ENOENT;
            std::vector<std::string> names;
            for (const auto& e : fs::directory_iterator(p)) names.push_back(e.path().filename().string());
            if (n_in == 1 && n_io == 1) {  // count only
                st32(m, io[0].addr, (uint32_t)names.size());
                return IPC_SUCCESS;
            }
            if (n_in < 2 || n_io < 2) return FS_EINVAL;
            uint32_t max = ld32(m, in[1].addr), count = 0;
            uint32_t dst = io[0].addr;
            for (const std::string& n : names) {
                if (count >= max) break;
                for (size_t i = 0; i < 12; i++) st8(m, dst + (uint32_t)i, i < n.size() ? n[i] : 0);
                st8(m, dst + 12, 0);
                dst += 13;
                count++;
            }
            st32(m, io[1].addr, count);
            return IPC_SUCCESS;
        }
        if (cmd == 0x0C) {  // GetUsage(path) -> blocks, inodes
            if (n_in < 1 || n_io < 2 || !host_path(read_path(in[0].addr), &p)) return FS_EINVAL;
            if (!fs::is_directory(p)) return FS_ENOENT;
            uint64_t bytes = 0;
            uint32_t inodes = 0;
            std::error_code ec;
            for (const auto& e : fs::recursive_directory_iterator(p, ec)) {
                inodes++;
                if (e.is_regular_file(ec)) bytes += e.file_size(ec);
            }
            st32(m, io[0].addr, (uint32_t)((bytes + 0x3FFF) / 0x4000));
            st32(m, io[1].addr, inodes + 1);
            return IPC_SUCCESS;
        }
        return IOSDevice::ioctlv(r, cmd, in, n_in, io, n_io);
    }
};

struct RegisterFs {
    RegisterFs() {
        ios_register("/dev/fs", [](const std::string&) -> std::unique_ptr<IOSDevice> {
            return std::make_unique<DevFs>();
        });
        // Every other absolute path that isn't a device is a NAND file.
        for (const char* prefix : {"/shared1/*", "/shared2/*", "/title/*", "/tmp/*", "/meta/*", "/sys/*",
                                   "/ticket/*", "/import/*"}) {
            ios_register(prefix, [](const std::string&) -> std::unique_ptr<IOSDevice> {
                return std::make_unique<NandFile>();
            });
        }
    }
} g_register_fs;
}  // namespace
