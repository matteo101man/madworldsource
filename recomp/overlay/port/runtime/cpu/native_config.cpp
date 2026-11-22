#include "native_config.h"
extern "C" __declspec(dllexport) int mw_native_set_option(const char* name,const char* value){
    if(!name||!value||std::string(name).rfind("MW_",0)!=0)return -2;
    auto& state=native_config_state();std::lock_guard<std::mutex> lock(state.mutex);
    if(state.frozen)return -1;
    state.options[name]=value;return 0;
}
