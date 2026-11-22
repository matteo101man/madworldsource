#pragma once
#include <cstdint>
#include <cstdio>
#include <string>
// A single same-length archive override; no generic filesystem remapping.
inline bool select_pc_ui_archive(const char* overlay, uint64_t expected, std::string& path) {
    if (!overlay || !*overlay) return true;
    FILE* file = std::fopen(overlay, "rb");
    if (!file) return false;
    const bool seek_ok = _fseeki64(file, 0, SEEK_END) == 0;
    const auto length = seek_ok ? _ftelli64(file) : -1;
    std::fclose(file);
    if (length < 0 || static_cast<uint64_t>(length) != expected) return false;
    path = overlay;
    return true;
}
