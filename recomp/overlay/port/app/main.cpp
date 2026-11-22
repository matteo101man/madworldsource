#include "host_fiber.h"
#include "snapshot.h"
#include "native_config.h"
// MadWorld native port: boots the recompiled game.
//
// Usage: madworld [path/to/orig/RZZE8P]
#include <cstdio>
#include <string>
#include <vector>

#define WIN32_LEAN_AND_MEAN
#include <windows.h>

#include <cstdlib>
#include <thread>

#include "hle.h"
#include "embed_host.h"
#include "mmio.h"
#include "os.h"
#include "ppc_memory.h"

extern std::string g_sys_dir;
extern std::string g_disc_dir;
extern PPCContext* g_ppc_context_for_debug;

namespace {
uint8_t* g_m;

LONG WINAPI on_crash(EXCEPTION_POINTERS* ep) {
    const EXCEPTION_RECORD* er = ep->ExceptionRecord;
    std::fprintf(stderr, "\n[host] crash 0x%08lX at host %p", er->ExceptionCode, er->ExceptionAddress);
    if (er->ExceptionCode == EXCEPTION_ACCESS_VIOLATION && er->NumberParameters >= 2) {
        uintptr_t a = (uintptr_t)er->ExceptionInformation[1];
        std::fprintf(stderr, ", %s host %p", er->ExceptionInformation[0] ? "write to" : "read from", (void*)a);
        if (g_m && a >= (uintptr_t)g_m && a < (uintptr_t)g_m + 0x100000000ull)
            std::fprintf(stderr, " = guest 0x%08X", (uint32_t)(a - (uintptr_t)g_m));
    }
    std::fprintf(stderr, "\n");
    ppc_print_guest_stack(stderr);
    if(auto c=g_ppc_context_for_debug){
        for(unsigned i=0;i<32;++i)std::fprintf(stderr,"r%u=%08X%s",i,c->r[i],i%4==3?"\n":" ");
        std::fprintf(stderr,"lr=%08X ctr=%08X cr=%08X msr=%08X\n",c->lr,c->ctr,c->cr,c->msr);
    }
    std::fflush(stderr);
    return EXCEPTION_EXECUTE_HANDLER;
}

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
}  // namespace

int main(int argc, char** argv) {
    std::string root = argc > 1 ? argv[1] : "orig/RZZE8P";
    g_sys_dir = root + "/sys";
    g_disc_dir = root + "/disc";
    // Local adapter: use the owner's existing extracted DATA partition directly.
    if (const char* partition = native_config_get("MW_PARTITION_DIR")) {
        g_sys_dir = std::string(partition) + "/sys";
        g_disc_dir = partition;
    }
    if(!native_host_embedded())SetUnhandledExceptionFilter(on_crash);
    setvbuf(stdout, nullptr, _IONBF, 0);

    std::vector<uint8_t> dol = read_all(g_sys_dir + "/main.dol");
    if (dol.empty()) {
        std::fprintf(stderr, "can't read %s/main.dol\n", g_sys_dir.c_str());
        return 1;
    }
    uint8_t* m = g_m = ppc_memory_init();
    if (!m) {
        std::fprintf(stderr, "can't reserve guest memory\n");
        return 1;
    }
    uint32_t entry = ppc_load_dol(m, dol.data(), dol.size());
    wii_setup_lowmem(m);
    g_mem = m;
    mmio_init_devices();

    // MW_TIMEOUT=<seconds>: dump the guest call stack and exit if still running (debugging hangs).
    if (const char* t = native_host_embedded()?nullptr:native_config_get("MW_TIMEOUT")) {
        int secs = std::atoi(t);
        std::thread([secs] {
            std::this_thread::sleep_for(std::chrono::seconds(secs));
            std::fprintf(stderr, "\n[host] still running after %d s\n", secs);
            ppc_print_guest_stack(stderr);
            std::fflush(stderr);
            std::_Exit(3);
        }).detach();
    }

    PPCFunc start = ppc_lookup(entry);
    if (!start) ppc_fatal("no recompiled function at entry point 0x%08X", entry);
    std::printf("[host] booting MadWorld, entry 0x%08X\n", entry);

    static PPCContext c;
    // The boot frame survives in a checkpoint; it must not own DOL heap storage.
    std::vector<uint8_t>().swap(dol);
    c.msr = 0x00002032;  // FP available, instruction/data translation on
    struct Boot { uint8_t* m; PPCContext* c; PPCFunc start; } boot{m, &c, start};
    auto run = [](void* arg) {
        auto& boot = *static_cast<Boot*>(arg);
        os_threads_init(boot.m, boot.c);
        ppc_enable_interrupt_polling();
        native_snapshot_load_request();
        boot.start(*boot.c, boot.m);
    };
    if (native_host_embedded()) run(&boot);
    else { host_fiber_call(run, &boot); os_threads_shutdown(nullptr); }
    std::printf("[host] __start returned\n");
    if(!native_host_embedded())ppc_memory_shutdown();
    return 0;
}
