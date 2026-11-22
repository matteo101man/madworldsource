#pragma once
#include <bit>
#include <cmath>
#include <cstdint>
#include <limits>

// GQR scales are signed six-bit integers; inverse loads also need exponent32.
// All these powers are normal, exact binary64 values under every rounding mode.
// Retain CRT semantics for callers outside the architectural scale domain.
static inline double ppc_quant_factor(int exponent) {
    static_assert(sizeof(double)==8 && std::numeric_limits<double>::is_iec559);
    if (exponent>=-32 && exponent<=32)
        return std::bit_cast<double>(uint64_t(exponent+1023)<<52);
    return std::ldexp(1.0,exponent);
}
