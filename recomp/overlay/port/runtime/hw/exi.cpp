// EXI (expansion interface): three serial channels at 0xCD006800.
//
// Per channel (stride 0x14): CSR +0, MAR +4 (DMA address), LEN +8, CR +0xC, DATA +0x10.
// Transfers complete instantly: writing CR with TSTART runs the transfer, clears
// TSTART and raises TCINT. The only device modelled is channel 0 / chip select 1:
// the RTC + SRAM chip. Everything else reads as absent (all zero).
#include <chrono>
#include <cstring>

#include "mmio.h"

namespace {
constexpr uint32_t EXI_BASE = 0xCD006800u;

// CSR bits
constexpr uint32_t CSR_TCINT = 1u << 3;
constexpr uint32_t CSR_CS_SHIFT = 7;     // bits 7-9: chip select (one-hot)
// CR bits
constexpr uint32_t CR_TSTART = 1u << 0;
constexpr uint32_t CR_DMA = 1u << 1;

struct Channel {
    uint32_t csr = 0, mar = 0, len = 0, cr = 0, data = 0;
};
Channel g_ch[3];

// --- RTC / SRAM (channel 0, device 1) ----------------------------------
struct RtcSram {
    uint8_t sram[64];
    bool have_cmd = false;
    bool writing = false;
    uint32_t addr = 0;   // byte offset into the selected region
    uint32_t region = 0; // 0x20000000 RTC, 0x20000100 SRAM, other: ignored

    RtcSram() {
        std::memset(sram, 0, sizeof sram);
        sram[0x13] = 0x2C;  // flags: progressive off, sound stereo (what a fresh console holds)
        update_checksum();
    }
    void update_checksum() {
        // Sum of the big-endian u16 words at 0x0C..0x13, stored at 0x00 / 0x02.
        uint16_t sum = 0, inv = 0;
        for (int i = 0x0C; i < 0x14; i += 2) {
            uint16_t w = (uint16_t)(sram[i] << 8 | sram[i + 1]);
            sum += w;
            inv += (uint16_t)~w;
        }
        sram[0] = sum >> 8;
        sram[1] = sum & 0xFF;
        sram[2] = inv >> 8;
        sram[3] = inv & 0xFF;
    }
    static uint32_t rtc_now() {
        // Seconds since 2000-01-01 00:00:00 (local time is close enough for now).
        using namespace std::chrono;
        int64_t unix_s = duration_cast<seconds>(system_clock::now().time_since_epoch()).count();
        return (uint32_t)(unix_s - 946684800);
    }
    void select() { have_cmd = false; }
    uint8_t read_byte() {
        uint8_t v = 0;
        if (region == 0x20000000u) {
            uint32_t t = rtc_now();
            v = (uint8_t)(t >> (24 - 8 * (addr & 3)));
        } else if (region == 0x20000100u && addr < sizeof sram) {
            v = sram[addr];
        }
        addr++;
        return v;
    }
    void write_byte(uint8_t v) {
        if (region == 0x20000100u && addr < sizeof sram) sram[addr] = v;
        addr++;
    }
    // Immediate transfer of n bytes; data is MSB-first in the DATA register.
    uint32_t imm(uint32_t data, int n, bool write) {
        if (!have_cmd) {
            // First 4-byte write after select is the command: bit 31 = write, address in bits 6-29.
            have_cmd = true;
            writing = (data & 0x80000000u) != 0;
            uint32_t a = data & 0x7FFFFFC0u;
            region = a >= 0x20000100u && a < 0x20000140u ? 0x20000100u : a == 0x20000000u ? 0x20000000u : a;
            addr = region == 0x20000100u ? (a - 0x20000100u) >> 6 : 0;
            return 0;
        }
        uint32_t out = 0;
        for (int i = 0; i < n; i++) {
            if (write) write_byte((uint8_t)(data >> (24 - 8 * i)));
            else out |= (uint32_t)read_byte() << (24 - 8 * i);
        }
        return out;
    }
} g_rtc;

int selected_device(const Channel& ch) {
    uint32_t cs = (ch.csr >> CSR_CS_SHIFT) & 7;
    return cs & 1 ? 0 : cs & 2 ? 1 : cs & 4 ? 2 : -1;
}

void run_transfer(int chn) {
    Channel& ch = g_ch[chn];
    int dev = selected_device(ch);
    bool rtc = chn == 0 && dev == 1;
    uint32_t rw = (ch.cr >> 2) & 3;  // 0 read, 1 write, 2 read/write
    if (ch.cr & CR_DMA) {
        for (uint32_t i = 0; i < ch.len; i++) {
            if (rw == 1) {
                if (rtc) g_rtc.write_byte(g_mem[0x80000000u + ((ch.mar + i) & 0x01FFFFFFu)]);
            } else {
                g_mem[0x80000000u + ((ch.mar + i) & 0x01FFFFFFu)] = rtc ? g_rtc.read_byte() : 0;
            }
        }
    } else {
        int n = (int)((ch.cr >> 4) & 3) + 1;
        uint32_t r = rtc ? g_rtc.imm(ch.data, n, rw == 1) : 0;
        if (rw != 1) ch.data = r;
    }
    ch.cr &= ~CR_TSTART;
    ch.csr |= CSR_TCINT;
}

uint32_t exi_read(uint32_t a, int) {
    uint32_t off = a & 0x3F;
    int chn = (int)(off / 0x14);
    if (chn > 2) return 0;
    const Channel& ch = g_ch[chn];
    switch (off % 0x14) {
        case 0x0: return ch.csr;
        case 0x4: return ch.mar;
        case 0x8: return ch.len;
        case 0xC: return ch.cr;
        case 0x10: return ch.data;
    }
    return 0;
}

void exi_write(uint32_t a, uint32_t v, int) {
    uint32_t off = a & 0x3F;
    int chn = (int)(off / 0x14);
    if (chn > 2) return;
    Channel& ch = g_ch[chn];
    switch (off % 0x14) {
        case 0x0: {
            uint32_t old_cs = (ch.csr >> CSR_CS_SHIFT) & 7;
            // Interrupt status bits (EXIINT, TCINT, EXTINT) are write-1-to-clear;
            // everything else takes the written value.
            const uint32_t ints = (1u << 1) | CSR_TCINT | (1u << 11);
            ch.csr = (v & ~ints) | (ch.csr & ints & ~v);
            uint32_t new_cs = (ch.csr >> CSR_CS_SHIFT) & 7;
            if (chn == 0 && (new_cs & 2) && !(old_cs & 2)) g_rtc.select();
            break;
        }
        case 0x4: ch.mar = v & 0x03FFFFE0u; break;
        case 0x8: ch.len = v & 0x03FFFFE0u; break;
        case 0xC:
            ch.cr = v;
            if (v & CR_TSTART) run_transfer(chn);
            break;
        case 0x10: ch.data = v; break;
    }
}
}  // namespace

MMIO_DEVICE(exi) { mmio_register(EXI_BASE, 0x40, exi_read, exi_write); }

#include "snapshot.h"
void native_exi_state(StateArchive& ar){ar.pod(g_ch);ar.pod(g_rtc);}
