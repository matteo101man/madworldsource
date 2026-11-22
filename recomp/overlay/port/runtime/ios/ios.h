// IOS (the Wii's ARM-side kernel) emulated at the IPC request level.
//
// The game talks to IOS by placing a request in memory and poking the IPC
// registers (ipc.cpp). Each request goes to a device by path ("/dev/fs",
// "/dev/stm/immediate", NAND file paths, ...). Devices answer immediately or
// leave the request pending and reply later with ios_reply().
#pragma once

#include <cstdint>
#include <memory>
#include <string>

#include "hle.h"
#include "snapshot.h"

// IOS error codes
constexpr int32_t IPC_SUCCESS = 0;
constexpr int32_t IPC_EACCES = -1;
constexpr int32_t IPC_EEXIST = -2;
constexpr int32_t IPC_EINVAL = -4;
constexpr int32_t IPC_ENOENT = -6;
constexpr int32_t IPC_PENDING = 0x7FFFFFFF;  // device will reply later

// Guest pointers inside IPC requests are physical addresses.
inline uint32_t ios_virt(uint32_t phys) { return phys ? (phys | 0x80000000u) : 0; }

struct IOSRequest {
    uint32_t addr;   // virtual address of the request block
    uint32_t cmd;
    uint32_t fd;
    uint32_t arg[5];
};

struct IOSVector {
    uint32_t addr;   // virtual
    uint32_t len;
};

class IOSDevice {
public:
    virtual ~IOSDevice() = default;
    // Called once per IOS_Open; return the fd to hand out (or an error).
    virtual int32_t open(const std::string& path, uint32_t mode) { (void)path; (void)mode; return IPC_SUCCESS; }
    virtual int32_t close() { return IPC_SUCCESS; }
    virtual int32_t read(uint32_t buf, uint32_t len) { (void)buf; (void)len; return IPC_EINVAL; }
    virtual int32_t write(uint32_t buf, uint32_t len) { (void)buf; (void)len; return IPC_EINVAL; }
    virtual int32_t seek(int32_t off, uint32_t whence) { (void)off; (void)whence; return IPC_EINVAL; }
    virtual int32_t ioctl(const IOSRequest& r, uint32_t cmd, uint32_t in, uint32_t in_len, uint32_t out, uint32_t out_len);
    virtual int32_t ioctlv(const IOSRequest& r, uint32_t cmd, const IOSVector* in, uint32_t n_in,
                           const IOSVector* io, uint32_t n_io);
    virtual void snapshot_state(StateArchive&) {}
    std::string name;
};

using IOSDeviceFactory = std::unique_ptr<IOSDevice> (*)(const std::string& path);

// Registers a device for an exact path, or every path with the prefix if it ends in '*'.
void ios_register(const std::string& path, IOSDeviceFactory factory);

// Handles the request at the given physical address. Returns false if it's pending.
void ios_execute(uint32_t req_phys);
// Completes a request: writes the result and queues the reply to the game.
void ios_reply(uint32_t req_addr, int32_t result);

// Called by ipc.cpp to queue a reply for delivery.
void ipc_queue_reply(uint32_t req_phys);
