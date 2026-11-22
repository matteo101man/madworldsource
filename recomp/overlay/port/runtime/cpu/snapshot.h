#pragma once
#include <cstdint>
#include <cstring>
#include <string>
#include <vector>
#include <stdexcept>
#include <type_traits>

// Versioned, bounded host-state stream. Only explicitly listed values enter it.
class StateArchive {
    size_t cursor=0;
public:
    bool reading=false;
    std::vector<uint8_t> bytes;
    StateArchive()=default;
    explicit StateArchive(std::vector<uint8_t> input):reading(true),bytes(std::move(input)){}
    void raw(void* data,size_t size){
        if(size>512ull*1024*1024)throw std::runtime_error("Snapshot field exceeds bound");
        if(reading){
            if(cursor>bytes.size()||size>bytes.size()-cursor)throw std::runtime_error("Truncated snapshot");
            if(size)std::memcpy(data,bytes.data()+cursor,size);cursor+=size;
        }else{
            if(bytes.size()>512ull*1024*1024-size)throw std::runtime_error("Snapshot exceeds bound");
            if(size){auto* p=static_cast<uint8_t*>(data);bytes.insert(bytes.end(),p,p+size);}
        }
    }
    template<class T> void pod(T& value){static_assert(std::is_trivially_copyable_v<T>);raw(&value,sizeof value);}
    template<class T> void vector(std::vector<T>& values,size_t maximum=16*1024*1024){
        static_assert(std::is_trivially_copyable_v<T>);
        uint64_t count=values.size();pod(count);
        if(count>maximum)throw std::runtime_error("Snapshot vector exceeds bound");
        if(reading)values.resize(size_t(count));raw(values.data(),size_t(count)*sizeof(T));
    }
    void string(std::string& value){
        uint64_t count=value.size();pod(count);if(count>4096)throw std::runtime_error("Snapshot string exceeds bound");
        if(reading)value.resize(size_t(count));raw(value.data(),size_t(count));
    }
    void tag(uint32_t expected){uint32_t actual=expected;pod(actual);if(actual!=expected)throw std::runtime_error("Snapshot schema mismatch");}
    void end(){if(reading&&cursor!=bytes.size())throw std::runtime_error("Snapshot has trailing data");}
};
void os_events_state(StateArchive&);
void os_threads_state(StateArchive&);
void native_ai_state(StateArchive&);
void native_mmio_state(StateArchive&);
void native_cpu_state(StateArchive&);
void native_vi_state(StateArchive&);
void native_exi_state(StateArchive&);
void native_gx_state(StateArchive&);
void native_ipc_state(StateArchive&);
void native_di_state(StateArchive&);
void native_ios_state(StateArchive&);
void native_bluetooth_state(StateArchive&);
void native_input_state(StateArchive&);
uint32_t native_snapshot_player();
void native_platform_state(StateArchive&);
void native_snapshot_checkpoint(bool eligible);
void native_snapshot_start_clock();
void native_snapshot_report_world();
void native_snapshot_actor(uint32_t id,uint32_t object);
void native_snapshot_load_request();
bool native_snapshot_restored();
void native_snapshot_process(bool load);
void native_host_snapshot_park(bool load);
void ppc_set_timebase(uint64_t);
