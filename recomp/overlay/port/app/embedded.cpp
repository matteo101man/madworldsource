#include "embed_host.h"
#include <cstdint>
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
extern "C" uintptr_t __security_cookie;
extern "C" uintptr_t __security_cookie_complement;
// Establish the explicit test cookie before CRT pointer encoding/constructors.
// Setting it in DllMain is too late: the static CRT has already encoded pointers.
extern "C" BOOL WINAPI _DllMainCRTStartup(HINSTANCE,DWORD,LPVOID);
extern "C" __declspec(safebuffers) BOOL WINAPI mw_native_dll_entry(HINSTANCE instance,DWORD reason,LPVOID reserved){
    if(reason==DLL_PROCESS_ATTACH&&(GetEnvironmentVariableW(L"MW_TEST_SNAPSHOT_WRITE",nullptr,0)||GetEnvironmentVariableW(L"MW_TEST_SNAPSHOT_READ",nullptr,0))){
        __security_cookie=0x9f40a6e3d219ull;
        __security_cookie_complement=~__security_cookie;
    }
    return _DllMainCRTStartup(instance,reason,reserved);
}
int main(int,char**);
extern "C" __declspec(dllexport) int mw_native_run(){
    return native_host_run([](){char name[]="madworld_core";char* args[]={name,nullptr};return main(1,args);});
}
