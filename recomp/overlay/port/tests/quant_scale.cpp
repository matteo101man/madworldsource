// Compare actual quantized-memory helpers with the prior CRT implementation.
#include "ppc_context.h"
#include <array>
#include <bit>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cfenv>
#include <cerrno>
#include <limits>
uint8_t mmio_read8(uint32_t){std::abort();}
uint16_t mmio_read16(uint32_t){std::abort();}
uint32_t mmio_read32(uint32_t){std::abort();}
void mmio_write8(uint32_t,uint8_t){std::abort();}
void mmio_write16(uint32_t,uint16_t){std::abort();}
void mmio_write32(uint32_t,uint32_t){std::abort();}
double reference_load(uint8_t* m,uint32_t a,uint32_t type,int scale){
 double f=std::ldexp(1.0,-scale);
 switch(type){case 4:return ld8(m,a)*f;case 5:return ld16(m,a)*f;case 6:return (int8_t)ld8(m,a)*f;case 7:return (int16_t)ld16(m,a)*f;default:return (double)ldf32(m,a);}
}
void reference_store(uint8_t* m,uint32_t a,uint32_t type,int scale,double v){
 double s=v*std::ldexp(1.0,scale);auto clamp=[](double x,double lo,double hi){return x<lo?lo:x>hi?hi:x;};
 switch(type){case 4:st8(m,a,(uint32_t)clamp(s,0,255));break;case 5:st16(m,a,(uint32_t)clamp(s,0,65535));break;case 6:st8(m,a,(uint32_t)(int32_t)clamp(s,-128,127));break;case 7:st16(m,a,(uint32_t)(int32_t)clamp(s,-32768,32767));break;default:stf32(m,a,(float)v);break;}
}
#define CHECK(x) do{if(!(x)){std::fprintf(stderr,"FAIL line %d: %s\n",__LINE__,#x);return 1;}}while(0)
__declspec(noinline) double benchmark(uint8_t* m,bool reference){
 double sum=0;uint32_t state=71;
 for(unsigned i=0;i<4000000;i++){state=state*1664525u+1013904223u;int scale=int((state>>16)&63)-32;uint32_t type=4+(state&3);sum+=reference?reference_load(m,0x100,type,scale):ppc_dequant(m,0x100,type,scale);}
 return sum;
}
int main(){
 std::array<uint8_t,4096> m{};uint64_t loads=0,stores=0;
 for(int rounding:{FE_TONEAREST,FE_DOWNWARD,FE_UPWARD,FE_TOWARDZERO}){
  CHECK(std::fesetround(rounding)==0);
  for(int exponent=-32;exponent<=32;exponent++){
   volatile int input=exponent;
   std::feclearexcept(FE_ALL_EXCEPT);std::feraiseexcept(FE_DIVBYZERO);errno=71;
   double expected=std::ldexp(1.0,input);int expectedFlags=std::fetestexcept(FE_ALL_EXCEPT),expectedErrno=errno;
   std::feclearexcept(FE_ALL_EXCEPT);std::feraiseexcept(FE_DIVBYZERO);errno=71;
   double actual=ppc_quant_factor(input);
   CHECK(std::bit_cast<uint64_t>(actual)==std::bit_cast<uint64_t>(expected));CHECK(std::fetestexcept(FE_ALL_EXCEPT)==expectedFlags);CHECK(errno==expectedErrno);
  }
  for(int exponent:{-2048,-1075,-1074,-1023,-33,33,1023,1024,2048}){
   volatile int input=exponent;std::feclearexcept(FE_ALL_EXCEPT);errno=0;
   double expected=std::ldexp(1.0,input);int expectedFlags=std::fetestexcept(FE_ALL_EXCEPT),expectedErrno=errno;
   std::feclearexcept(FE_ALL_EXCEPT);errno=0;double actual=ppc_quant_factor(input);
   CHECK(std::bit_cast<uint64_t>(actual)==std::bit_cast<uint64_t>(expected));CHECK(std::fetestexcept(FE_ALL_EXCEPT)==expectedFlags);CHECK(errno==expectedErrno);
  }
  std::feclearexcept(FE_ALL_EXCEPT);

  for(int scale=-32;scale<=31;scale++)for(uint32_t type=4;type<=7;type++){
   unsigned limit=(type==4||type==6)?256:65536;
   for(unsigned raw=0;raw<limit;raw++){
    if(limit==256)st8(m.data(),0x100,raw);else st16(m.data(),0x100,raw);
    CHECK(std::bit_cast<uint64_t>(reference_load(m.data(),0x100,type,scale))==std::bit_cast<uint64_t>(ppc_dequant(m.data(),0x100,type,scale)));++loads;
   }
   for(double value:{-1e300,-65536.,-32768.5,-32768.,-128.5,-128.,-1.5,-1.,-0.,0.,0.5,1.,127.,127.9,255.,255.9,32767.,65535.,65536.,1e300}){
    reference_store(m.data(),0x200,type,scale,value);ppc_quant(m.data(),0x300,type,scale,value);
    CHECK(ld8(m.data(),0x200)==ld8(m.data(),0x300));if(limit==65536)CHECK(ld8(m.data(),0x201)==ld8(m.data(),0x301));++stores;
   }
  }
  for(int scale=-32;scale<=31;scale++)for(uint32_t raw:{0u,0x80000000u,0x3f800000u,1u,0x7f7fffffu,0x7f800000u,0xff800000u,0x7fc00000u}){
   st32(m.data(),0x100,raw);double actual=ppc_dequant(m.data(),0x100,0,scale),expected=reference_load(m.data(),0x100,0,scale);
   CHECK(std::bit_cast<uint64_t>(actual)==std::bit_cast<uint64_t>(expected));++loads;
  }
 }
 CHECK(std::fesetround(FE_TONEAREST)==0);st32(m.data(),0x100,0x52ad7183);
 using Clock=std::chrono::steady_clock;
 for(int repeat=0;repeat<3;repeat++)for(bool reference:{true,false}){
  auto start=Clock::now();double sum=benchmark(m.data(),reference);auto ns=std::chrono::duration_cast<std::chrono::nanoseconds>(Clock::now()-start).count();
  std::printf("QUANT_BENCH reference=%d ns=%lld checksum=%016llx\n",reference,(long long)ns,(unsigned long long)std::bit_cast<uint64_t>(sum));
 }
 std::printf("QUANT_SCALE_PASS loads=%llu stores=%llu roundingModes=4\n",(unsigned long long)loads,(unsigned long long)stores);
}
