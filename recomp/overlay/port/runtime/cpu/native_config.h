#pragma once
#include <cstdlib>
#include <map>
#include <mutex>
#include <string>
// Configure before run; frozen options are immutable during all guest execution.
struct NativeConfigState {
    std::map<std::string,std::string,std::less<>> options;
    std::mutex mutex;bool frozen=false;
};
inline NativeConfigState& native_config_state(){static NativeConfigState state;return state;}
inline const char* native_config_get(const char* name){
    auto& state=native_config_state();auto item=state.options.find(name);
    return item==state.options.end()?std::getenv(name):item->second.c_str();
}
// For hot paths: options are frozen before guest code runs, so each call site looks
// its option up once instead of per call (getenv fallback is not free on Windows).
#define NATIVE_OPTION(name) ([]{ static const char* const native_option_value = native_config_get(name); return native_option_value; }())
inline void native_config_freeze(){auto& state=native_config_state();std::lock_guard<std::mutex> lock(state.mutex);state.frozen=true;}
