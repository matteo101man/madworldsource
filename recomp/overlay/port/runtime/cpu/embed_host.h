#pragma once
#include <cstdint>
bool native_host_embedded();
bool native_host_started();
void native_host_poll();
void native_host_fail(const char* message);
int native_host_run(int(*entry)());
struct HostFiber;
void os_threads_shutdown(HostFiber* retained_fiber);
void native_platform_shutdown();
void ppc_reset_debug_context();
void ios_shutdown_devices();
void os_clear_hardware_events();
