// Synthetic scalar FPU harness; no original game data or generated game bodies.
#include "../runtime/cpu/ppc_context.h"
#include <cstdio>
#include <limits>
struct Case {uint32_t operation,fpscr;uint64_t a,b,c;};
static_assert(sizeof(Case)==32);
int main(int argc,char**argv){
    if(argc!=3)return 2;
    // Independent Broadway conversion marker checks, including a negative
    // fraction rounded to0 and all four architectural rounding selectors.
    if(ppc_fctiw_impl(-0.0,0)!=0xfff8000100000000ull)return 6;
    if(ppc_fctiw_impl(-0.25,1)!=0xfff8000100000000ull)return 7;
    if(ppc_fctiw_impl(0.0,0)!=0xfff8000000000000ull)return 8;
    const uint64_t expected[4]={0xfff80000fffffffeull,0xfff80000fffffffeull,0xfff80000fffffffeull,0xfff80000fffffffdull};
    for(int mode=0;mode<4;++mode)if(ppc_fctiw_impl(-2.5,mode)!=expected[mode])return 9;
    FILE*input=std::fopen(argv[1],"rb");FILE*output=std::fopen(argv[2],"wb");
    if(!input||!output)return 3;
    if(ppc_fctiw_impl(std::numeric_limits<double>::infinity(),1)!=0xfff800007fffffffull)return 10;
    if(ppc_fctiw_impl(-std::numeric_limits<double>::infinity(),1)!=0xfff8000080000000ull)return 11;
    if(ppc_fctiw_impl(std::numeric_limits<double>::quiet_NaN(),1)!=0xfff8000080000000ull)return 12;
    Case row;unsigned count=0;
    while(std::fread(&row,sizeof row,1,input)==1){
        PPCContext c{};c.fpscr=row.fpscr;c.f[2].u0=row.a;c.f[3].u0=row.b;c.f[4].u0=row.c;
        switch(row.operation){
#include "fpu_fixture.inc"
        default:return 4;
        }
        if(std::fwrite(&c.f[1].u0,8,1,output)!=1)return 5;
        ++count;
    }
    std::fclose(input);std::fclose(output);std::printf("Synthetic FPU cases:%u\n",count);return 0;
}
