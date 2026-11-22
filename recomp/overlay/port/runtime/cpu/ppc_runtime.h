// Runtime services shared by the host-side code (not needed by generated code).
#pragma once

#include <cstdio>

#include "ppc_context.h"

[[noreturn]] void ppc_fatal(const char* fmt, ...);
PPCFunc ppc_lookup(uint32_t addr);
const char* ppc_func_name(uint32_t addr, uint32_t* start = nullptr);
void ppc_print_guest_stack(FILE* out);
uint64_t ppc_timebase();  // 60.75 MHz ticks since start
unsigned ppc_test_time_scale();  // MW_TEST_TIME_SCALE (1..16), test-only guest clock acceleration; 1 = original
