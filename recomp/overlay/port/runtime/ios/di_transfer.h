#pragma once
#include <algorithm>
#include <cstdint>

// Buffered-read lower bound, not a full optical seek/spindle/cache model.
// Hardware measurements documented by Dolphin DVDInterface.cpp at409881c3:
// read-command latency600us; buffered DMA32MiB/s. Independent queue arithmetic.
struct DITransferQueue {
    uint64_t available = 0;
    uint64_t submit(uint64_t now, uint32_t bytes) {
        constexpr uint64_t tb = 60750000, rate = 32u * 1024u * 1024u;
        uint64_t duration = tb * 600 / 1000000 + (uint64_t(bytes) * tb + rate - 1) / rate;
        available = std::max(now, available) + duration;
        return available;
    }
};
