#pragma once
#include <cstdint>
#include <cstddef>
void native_platform_gx(const uint8_t* bytes,size_t size);
void native_platform_gx_list(const uint8_t call[9]);
void native_platform_gx_scene(uint32_t scene);
void native_platform_ai_control(uint32_t value);
void* native_platform_context();
void* native_platform_symbol(const char* name);
void native_platform_vi_write(uint32_t address,uint32_t value,int size);

class StateArchive;
void native_platform_blob(StateArchive&,void*,const char*,const char*);
