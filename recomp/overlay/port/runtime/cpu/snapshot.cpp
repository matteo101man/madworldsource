#include "snapshot.h"
#include "native_config.h"
#include "embed_host.h"
#include "ppc_memory.h"
#include "ppc_runtime.h"
#include "platform.h"
#include "mmio.h"
#include "rso.h"
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <bcrypt.h>
#include <array>
#include <cstdio>
#include <chrono>
#include <fstream>
#pragma comment(lib,"bcrypt.lib")
extern std::string g_sys_dir;
extern unsigned ppc_test_time_scale();
extern "C" uintptr_t __security_cookie;
namespace {
bool restored=false,seen_demo=false,captured=false,world_reported=false;
uint32_t player_actor=0,rin_actor=0;
auto native_start=std::chrono::steady_clock::now();
struct ModuleIdentity {uint64_t base=0,size=0;std::array<uint8_t,32> hash{};};
struct Header {
    uint64_t magic=0x3150414e53574dull;
    uint32_t version=1,scale=0;
    ModuleIdentity core,platform;
    std::array<uint8_t,20> dol{};
    std::array<uint8_t,32> prompts{},payload_hash{};
    uint64_t memory=0,cookie=0,payload_size=0;
};
struct Hasher {
    BCRYPT_ALG_HANDLE algorithm=nullptr;BCRYPT_HASH_HANDLE hash=nullptr;
    explicit Hasher(const wchar_t* name){if(BCryptOpenAlgorithmProvider(&algorithm,name,nullptr,0)<0||BCryptCreateHash(algorithm,&hash,nullptr,0,nullptr,0,0)<0)throw std::runtime_error("Cannot create snapshot hash");}
    ~Hasher(){if(hash)BCryptDestroyHash(hash);if(algorithm)BCryptCloseAlgorithmProvider(algorithm,0);}
    void add(const void* data,size_t count){if(count>0xffffffffu||BCryptHashData(hash,(PUCHAR)data,ULONG(count),0)<0)throw std::runtime_error("Snapshot hash update failed");}
    template<size_t N> void finish(std::array<uint8_t,N>& out){if(BCryptFinishHash(hash,out.data(),ULONG(N),0)<0)throw std::runtime_error("Snapshot hash failed");}
};
template<size_t N> void hash_file(const std::wstring& path,std::array<uint8_t,N>& output){
    std::ifstream file(path,std::ios::binary);if(!file)throw std::runtime_error("Snapshot identity file unavailable");
    Hasher hasher(N==20?BCRYPT_SHA1_ALGORITHM:BCRYPT_SHA256_ALGORITHM);std::array<char,65536> buf;
    while(file){file.read(buf.data(),buf.size());auto n=file.gcount();if(n)hasher.add(buf.data(),size_t(n));}
    if(!file.eof())throw std::runtime_error("Snapshot identity read failed");hasher.finish(output);
}
std::wstring wide(const char* path){
    int length=MultiByteToWideChar(CP_UTF8,MB_ERR_INVALID_CHARS,path,-1,nullptr,0);if(length<=1)throw std::runtime_error("Invalid snapshot path");
    std::wstring out(length,L'\0');MultiByteToWideChar(CP_UTF8,MB_ERR_INVALID_CHARS,path,-1,out.data(),length);out.pop_back();return out;
}
std::wstring snapshot_path(bool load){
    const char* value=native_config_get(load?"MW_TEST_SNAPSHOT_READ":"MW_TEST_SNAPSHOT_WRITE");if(!value)throw std::runtime_error("Snapshot path missing");
    auto path=wide(value);wchar_t full[32768];DWORD length=GetFullPathNameW(path.c_str(),32768,full,nullptr);
    if(!length||length>=32768||(full[0]!=L'D'&&full[0]!=L'd')||full[1]!=L':')throw std::runtime_error("Snapshot must stay on D:");
    return full;
}
ModuleIdentity module(void* address){
    HMODULE handle=nullptr;if(!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS|GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,(LPCWSTR)address,&handle))throw std::runtime_error("Snapshot module missing");
    auto* base=reinterpret_cast<uint8_t*>(handle);auto* dos=reinterpret_cast<IMAGE_DOS_HEADER*>(base);auto* nt=reinterpret_cast<IMAGE_NT_HEADERS64*>(base+dos->e_lfanew);
    if(nt->OptionalHeader.DllCharacteristics&IMAGE_DLLCHARACTERISTICS_DYNAMIC_BASE||nt->OptionalHeader.ImageBase!=reinterpret_cast<uint64_t>(base))throw std::runtime_error("Snapshot requires unrelocated fixed modules");
    ModuleIdentity out;out.base=reinterpret_cast<uint64_t>(base);out.size=nt->OptionalHeader.SizeOfImage;
    wchar_t path[32768];if(!GetModuleFileNameW(handle,path,32768))throw std::runtime_error("Snapshot module path unavailable");hash_file(std::wstring(path),out.hash);return out;
}
Header identity(){
    Header h;h.scale=ppc_test_time_scale();h.core=module(reinterpret_cast<void*>(native_snapshot_process));
    void* platform=native_platform_symbol("mw_platform_abi");if(!platform)throw std::runtime_error("Snapshot platform unavailable");h.platform=module(platform);
    hash_file(wide((g_sys_dir+"/main.dol").c_str()),h.dol);
    if(const char* prompts=native_config_get("MW_PC_UI_FILE"))hash_file(wide(prompts),h.prompts);
    h.memory=reinterpret_cast<uint64_t>(g_mem);h.cookie=__security_cookie;return h;
}
bool same(const ModuleIdentity& a,const ModuleIdentity& b){return a.base==b.base&&a.size==b.size&&a.hash==b.hash;}
void machine(StateArchive& ar){
    ar.tag(0x4d574d31);
    uint64_t tb=ar.reading?0:ppc_timebase();ar.pod(tb);
    ar.pod(player_actor);ar.pod(rin_actor);
    // Hardware save flushes deferred RAM writes before any host-stack capture.
    native_platform_state(ar);
    ar.raw(g_mem+LOCKED_CACHE,LOCKED_CACHE_SIZE);
    native_cpu_state(ar);native_mmio_state(ar);
    native_ai_state(ar);native_vi_state(ar);native_exi_state(ar);native_gx_state(ar);
    native_ipc_state(ar);native_di_state(ar);native_ios_state(ar);native_bluetooth_state(ar);native_input_state(ar);
    os_events_state(ar);os_threads_state(ar);ar.end();
    ppc_set_timebase(tb);
}
}
void native_snapshot_start_clock(){native_start=std::chrono::steady_clock::now();}
uint32_t native_snapshot_player(){return player_actor;}
void native_snapshot_actor(uint32_t id,uint32_t object){if(id==0x10000)player_actor=object;if(id==0x20440)rin_actor=object;}
void native_snapshot_report_world(){
    if(restored&&!world_reported){world_reported=true;double ms=std::chrono::duration<double,std::milli>(std::chrono::steady_clock::now()-native_start).count();std::fprintf(stderr,"[snapshot-world] scene=%x demo=%d since-native-start-ms=%.3f\n",ld32(g_mem,0x8069bcdc),int32_t(ld32(g_mem,0x80604020)),ms);}
}
bool native_snapshot_restored(){return restored;}
void native_snapshot_load_request(){
    if(native_config_get("MW_TEST_SNAPSHOT_READ"))native_host_snapshot_park(true);
}
void native_snapshot_checkpoint(bool eligible){
    if(restored||captured||!native_config_get("MW_TEST_SNAPSHOT_WRITE")||!eligible)return;
    if(int32_t(ld32(g_mem,0x80604020))<0){seen_demo=true;return;}
    if(!seen_demo||ld32(g_mem,0x8069bcdc)!=0x201)return;
    uint32_t flags=ld32(g_mem,0x80697e64);
    auto mapped=[](uint32_t p,uint32_t n){return (p>=0x80000000&&p<=0x81800000-n)||(p>=0x90000000&&p<=0x94000000-n);};
    if(!mapped(flags,4)||ld32(g_mem,flags)!=0||!mapped(player_actor,0x11e)||!mapped(rin_actor,0x11e)||ld32(g_mem,player_actor+0xf4)!=0x10000||ld32(g_mem,rin_actor+0xf4)!=0x20440||int16_t(ld16(g_mem,player_actor+0xec))<=0)return;
    captured=true;native_host_snapshot_park(false);
}
void native_snapshot_process(bool load){
    auto began=std::chrono::steady_clock::now();auto path=snapshot_path(load);
    if(native_config_get("MW_TEST_SNAPSHOT_READ")&&native_config_get("MW_TEST_SNAPSHOT_WRITE"))throw std::runtime_error("Choose snapshot read or write, not both");
    Header expected=identity(),header{};
    if(load){
        std::ifstream file(path,std::ios::binary|std::ios::ate);if(!file)throw std::runtime_error("Cannot open snapshot");
        auto length=file.tellg();if(length<std::streamoff(sizeof(Header))||length>std::streamoff((512ull<<20)+sizeof(Header)))throw std::runtime_error("Snapshot file size invalid");
        file.seekg(0);file.read(reinterpret_cast<char*>(&header),sizeof header);
        if(header.magic!=expected.magic||header.version!=expected.version||header.scale!=expected.scale||!same(header.core,expected.core)||!same(header.platform,expected.platform)||header.dol!=expected.dol||header.prompts!=expected.prompts||header.memory!=expected.memory||header.cookie!=expected.cookie)throw std::runtime_error("Snapshot identity mismatch (version/modules/bases/DOL/prompts/RAM/cookie/clock)");
        if(header.payload_size!=uint64_t(length)-sizeof(Header))throw std::runtime_error("Snapshot payload size mismatch");
        std::vector<uint8_t> data(size_t(header.payload_size));file.read(reinterpret_cast<char*>(data.data()),data.size());if(!file)throw std::runtime_error("Truncated snapshot payload");
        Hasher hasher(BCRYPT_SHA256_ALGORITHM);std::array<uint8_t,32> digest;hasher.add(data.data(),data.size());hasher.finish(digest);if(digest!=header.payload_hash)throw std::runtime_error("Snapshot payload checksum mismatch");
        StateArchive ar(std::move(data));machine(ar);restored=true;
        std::fprintf(stderr,"[snapshot-actors] player=%08x rin=%08x\n",player_actor,rin_actor);mw_rso_attach(g_mem);ppc_enable_interrupt_polling();
        for(uint32_t address:{0x81120be4u,0x810c9cf0u,0x810cdbd4u,0x810cac70u})mw_rso_lookup(mw_rso_runtime_pc(g_mem,address));
    }else{
        header=expected;StateArchive ar;machine(ar);header.payload_size=ar.bytes.size();
        Hasher hasher(BCRYPT_SHA256_ALGORITHM);hasher.add(ar.bytes.data(),ar.bytes.size());hasher.finish(header.payload_hash);
        auto temporary=path+L".tmp";std::ofstream file(temporary,std::ios::binary|std::ios::trunc);
        file.write(reinterpret_cast<const char*>(&header),sizeof header);file.write(reinterpret_cast<const char*>(ar.bytes.data()),ar.bytes.size());file.flush();if(!file)throw std::runtime_error("Snapshot write failed");file.close();
        if(!MoveFileExW(temporary.c_str(),path.c_str(),MOVEFILE_REPLACE_EXISTING|MOVEFILE_WRITE_THROUGH))throw std::runtime_error("Snapshot publish failed");
    }
    double ms=std::chrono::duration<double,std::milli>(std::chrono::steady_clock::now()-began).count();
    std::fprintf(stderr,"[snapshot] %s bytes=%llu ms=%.3f core=%llx platform=%llx guest-base=%llx gpu-caches-reset=%u\n",load?"restored":"captured",header.payload_size,ms,header.core.base,header.platform.base,header.memory,load?1:0);
}
