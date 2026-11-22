#pragma once
#include <array>
#include <cstdint>
#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#else
#include <fstream>
#endif

inline bool read_native_input(const char* path, std::array<uint8_t,8>& packet) {
    std::array<uint8_t,9> candidate{};
#ifdef _WIN32
    HANDLE handle = CreateFileA(path, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                                nullptr, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (handle == INVALID_HANDLE_VALUE) return false;
    DWORD count = 0;
    bool valid = ReadFile(handle, candidate.data(), DWORD(candidate.size()), &count, nullptr) && count == 8;
    CloseHandle(handle);
#else
    std::ifstream file(path, std::ios::binary);
    file.read(reinterpret_cast<char*>(candidate.data()), candidate.size());
    bool valid = file.gcount() == 8;
#endif
    if (valid) for (size_t i=0; i<packet.size(); i++) packet[i]=candidate[i];
    return valid;
}
