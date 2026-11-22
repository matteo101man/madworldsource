#include "native_config.h"
#include "snapshot.h"
#include "hle.h"
#include "hle_funcs.h"
#include "platform.h"
#include <cstdlib>
#include <cstdio>
#include <string>
uint32_t native_frame_record_rin_object();
namespace {
// Default-off occupied-save presentation fixture. The fight shortcut bypasses
// the ordinary E00 load, so import explicit native-format files into this test
// process only. No SDK/file writes; preserve the original menu consumers.
void save_fixture_import(uint8_t* m,uint32_t scene){
    static bool done=false;const char* records=NATIVE_OPTION("MW_TEST_SAVE_RECORDS");const char* options=NATIVE_OPTION("MW_TEST_SAVE_OPTIONS");
    if(done||scene!=0xe00||!records||!options)return;
    auto import=[&](const char* path,uint32_t address,size_t size){
        FILE* file=std::fopen(path,"rb");if(!file)ppc_fatal("Save UI fixture input missing");
        std::vector<uint8_t> bytes(size);bool valid=std::fread(bytes.data(),1,size,file)==size&&std::fgetc(file)==EOF;std::fclose(file);
        if(!valid)ppc_fatal("Save UI fixture input size");for(size_t i=0;i<size;++i)st8(m,address+uint32_t(i),bytes[i]);
    };
    import(records,0x80622ce0,0x31140);import(options,0x80653e20,32);
    st8(m,0x8069bc95,0);done=true;
    std::fprintf(stderr,"[save-ui-fixture] imported explicit records/options into test RAM; fresh-record reset disabled; no disk writes\n");
}
bool enabled(){static bool on=NATIVE_OPTION("MW_TRACE_WORLD")!=nullptr;return on;}
uint32_t effect_table(uint8_t* m){return ld32(m,0x80695560);}
// Default-off original message layout fixture. Original MSD/MTD bytes and
// registration/show/draw APIs; no actor state writes. The profile subtitle
// bit is enabled only around show(), then restored without saving settings.
unsigned message_fixture_age=0;bool message_fixture_ready=false,message_fixture_submitted=false;uint32_t message_fixture_msd=0,message_fixture_mtd=0;
void message_fixture_submit(PPCContext& c,uint8_t* m){
    message_fixture_submitted=false;const char* output=NATIVE_OPTION("MW_TEST_MESSAGE_RECORD");const char* input=NATIVE_OPTION("MW_TEST_MESSAGE_MESS");if(!output||!input||message_fixture_age>=52)return;
    static bool saw_intro=false;if(ld32(m,0x80606cf8+4)==5)saw_intro=true;
    uint32_t flags=ld32(m,0x80697e64);
    if(!message_fixture_ready&&(!saw_intro||ld32(m,0x80606cf8+4)!=0||ld32(m,0x8069bcdc)!=0x201||!ld32(m,0x806a4198)||int32_t(ld32(m,0x80604020))<0||flags<0x80000000u||ld32(m,flags)!=0))return;
    PPCContext saved=c;
    auto call=[&](uint32_t address){auto fn=ppc_lookup(address);if(!fn)ppc_fatal("Missing message fixture API");fn(c,m);};
    if(!message_fixture_ready){
        FILE* file=std::fopen(input,"rb");if(!file)ppc_fatal("Cannot open original message fixture source");std::fseek(file,0,SEEK_END);long size=std::ftell(file);std::rewind(file);if(size<32||size>1024*1024)ppc_fatal("Message fixture source bounds");
        c.r[3]=uint32_t(size)+31;c.r[4]=0x80606cf8+0x358;call(0x800baea0);uint32_t buffer=(c.r[3]+31)&~31u;if(!((buffer>=0x80000000u&&buffer<=0x81800000u-uint32_t(size))||(buffer>=0x90000000u&&buffer<=0x94000000u-uint32_t(size))))ppc_fatal("Message fixture allocation bounds");
        if(std::fread(m+buffer,1,size,file)!=size_t(size))ppc_fatal("Message fixture source read");std::fclose(file);
        if(ld32(m,buffer)!=0x4d455353||ld32(m,buffer+4)+8!=uint32_t(size))ppc_fatal("Message fixture framing");
        uint32_t msd=0,mtd=0;
        for(uint32_t at=buffer+8;at<buffer+size;){uint32_t bytes=ld32(m,at+4);if(bytes>uint32_t(size)||at+8+bytes>buffer+size)ppc_fatal("Message fixture child bounds");if(ld32(m,at)==0x4d534420)msd=(at+39)&~31u;if(ld32(m,at)==0x4d544420)mtd=(at+39)&~31u;at+=8+bytes;}
        if(!msd||!mtd)ppc_fatal("Missing message fixture banks");
        uint32_t metrics=mtd+ld32(m,mtd);
        if(ld32(m,msd)!=4||ld32(m,mtd+8)!=35||ld32(m,metrics+4)!=24||ld32(m,metrics+8)!=20)ppc_fatal("Message fixture expects original EV00 banks");
        message_fixture_msd=msd;message_fixture_mtd=mtd;
        c=saved;c.r[3]=0x80606be4;c.r[4]=msd;c.r[5]=mtd;c.r[6]=0;call(0x800980fc);
        // Publish MSD context and initialize the message before show() uses it.
        c=saved;c.r[3]=0x80606be4;call(0x800982a0);message_fixture_ready=true;
    }
    if(message_fixture_age<48){
        if(message_fixture_age%12==0){
            c=saved;c.r[3]=0x80606be4;c.r[4]=message_fixture_msd;c.r[5]=message_fixture_mtd;c.r[6]=0;call(0x800980fc);c=saved;c.r[3]=0x80606be4;call(0x800982a0);
            uint32_t profile=ld32(m,0x805a8938);if(profile<0x80000000u||profile>=0x81800000u)ppc_fatal("Message fixture profile bounds");uint32_t option=0x80653e20+ld8(m,profile)*8;unsigned old=ld8(m,option);st8(m,option,old|4);c=saved;c.r[3]=0x80606be4;c.r[4]=message_fixture_age/12;call(0x80098178);st8(m,option,old);
            std::fprintf(stderr,"[message-fixture] row=%u msd=%08x font=%08x selected=%08x count=%u state=%u dims=%.9g/%.9g\n",message_fixture_age,ld32(m,0x806a41c8),ld32(m,0x806a41a8),ld32(m,0x80606be8+0x20),ld32(m,0x80606be8+0x2c),ld32(m,0x80606be8+0x24),ldf32(m,0x80606be8+0x50),ldf32(m,0x80606be8+0x54));
        }
        c=saved;c.r[3]=0x80606be4;call(0x800982a0);
    }else if(message_fixture_age==48){c=saved;c.r[3]=0x80606be4;call(0x80098200);}
    c=saved;message_fixture_submitted=true;
}
void message_fixture_record(uint8_t* m){
    const char* path=NATIVE_OPTION("MW_TEST_MESSAGE_RECORD");if(!path||!message_fixture_ready||!message_fixture_submitted||message_fixture_age>=52)return;
    static FILE* out=nullptr;if(!out){out=std::fopen(path,"w");if(!out)ppc_fatal("Message fixture output");std::fprintf(out,"{\"schema\":\"mw-event-message-fixture/1\",\"scope\":\"Original EV00 MSD/MTD through original registration/show/draw APIs; temporary subtitle option restored immediately, no actor writes. Controlled UI layout component, not native EV00 event playback\"}\n");}
    std::fprintf(out,"{\"row\":%u,\"message\":%d,\"state\":%u,\"position\":[%.9g,%.9g],\"dimensions\":[%.9g,%.9g,%.9g],\"glyphs\":[",message_fixture_age,message_fixture_age<48?int(message_fixture_age/12):-1,ld32(m,0x80606be8+0x24),ldf32(m,0x80606be8+0x7c),ldf32(m,0x80606be8+0x80),ldf32(m,0x80606be8+0x84)*24.f,ldf32(m,0x80606be8+0x88)*24.f,ldf32(m,0x80606be8+0x8c));
    bool comma=false;for(unsigned slot=0;slot<256;++slot){uint32_t w=0x80653f58+slot*0x318,r=0x80685758+slot*0xe4;if(!(ld32(m,w+0x7c)&1)||ld8(m,r)!=1)continue;std::fprintf(out,"%s{\"slot\":%u,\"hidden\":%u,\"rgba\":[",comma?",":"",slot,ld8(m,w+0x269));comma=true;for(unsigned i=0;i<4;i++)std::fprintf(out,"%s%.9g",i?",":"",ldf32(m,w+0x84+i*4));std::fprintf(out,"],\"size\":[%.9g,%.9g],\"matrix\":[",ldf32(m,w+0x26c),ldf32(m,w+0x270));for(unsigned i=0;i<12;i++)std::fprintf(out,"%s%.9g",i?",":"",ldf32(m,w+0x1f8+i*4));std::fprintf(out,"],\"uv\":[");for(unsigned i=0;i<8;i++)std::fprintf(out,"%s%.9g",i?",":"",ldf32(m,w+0x5c+i*4));std::fprintf(out,"]}");}
    std::fprintf(out,"]}\n");std::fflush(out);++message_fixture_age;
}
// Explicit dedicated HUD fixture. Only original UI factory/submission APIs
// mutate state. No actor, gameplay HP, flag or position writes.
uint32_t hud_low_owner=0;unsigned hud_low_age=0;
float hud_low_ratio(unsigned age){return age<20?1.f:age<70?.6f:age<100?.25f:age<150?.2f:age<200?1.f:age<240?.29f:age<260?.3f:age<300?.299f:1.f;}
void hud_low_submit(PPCContext& c,uint8_t* m){
    if(!NATIVE_OPTION("MW_TEST_HUD_LOW_RECORD"))return;
    static bool saw_intro=false;if(ld32(m,0x80606cf8+4)==5)saw_intro=true;
    if(hud_low_age>=340)return;
    if(!hud_low_owner){
        uint32_t flags=ld32(m,0x80697e64);
        if(!saw_intro||ld32(m,0x80606cf8+4)!=0||ld32(m,0x8069bcdc)!=0x201||int32_t(ld32(m,0x80604020))<0||flags<0x80000000u||ld32(m,flags)!=0)return;
        PPCContext saved=c;auto create=ppc_lookup(0x802233a8);if(!create)ppc_fatal("HUD fixture factory missing");
        c.r[3]=0x8069d11c;c.r[4]=0x17;create(c,m);hud_low_owner=c.r[3];c=saved;
        if(hud_low_owner<0x80000000u)ppc_fatal("HUD fixture allocation failed");
    }
    PPCContext saved=c;auto submit=ppc_lookup(0x80224290);if(!submit)ppc_fatal("HUD fixture setter missing");
    c.r[3]=hud_low_owner;c.f[1].ps0=c.f[1].ps1=100.;c.f[2].ps0=c.f[2].ps1=double(hud_low_ratio(hud_low_age)*100.f);submit(c,m);c=saved;
}
void hud_low_record(uint8_t* m){
    const char* path=NATIVE_OPTION("MW_TEST_HUD_LOW_RECORD");if(!path||!hud_low_owner||hud_low_age>=340)return;
    static FILE* out=nullptr;if(!out){out=std::fopen(path,"w");if(!out)ppc_fatal("Cannot open HUD low fixture output");std::fprintf(out,"{\"schema\":\"mw-hud-low-fixture/1\",\"scope\":\"Dedicated original UI controller and controlled UI health submissions; no actor HP/position/flag writes. Read-only end-of-world widget observations; synthetic UI component evidence\"}\n");}
    auto mapped=[](uint32_t p,uint32_t n){return(p>=0x80000000u&&p<=0x81800000u-n)||(p>=0x90000000u&&p<=0x94000000u-n);};
    uint32_t hp=ld32(m,hud_low_owner+0x10);if(!mapped(hp,0x30))return;
    uint32_t main=ld32(m,hp+4),effect=ld32(m,hp+0x10),wmain=ld32(m,hp+0x14),idt=0;
    // The original controller can be registered while UI updates are gated.
    // Keep submitting initial full health until its own widgets are allocated.
    if(main==0xffffffffu||effect==0xffffffffu)return;
    if(mapped(wmain,0x1a4)){uint32_t r=ld32(m,wmain+0x1a0);if(mapped(r,0x1c)){uint32_t raw=ld32(m,r+0x18);if(mapped(raw,0x220))idt=raw-589204;}}
    std::fprintf(out,"{\"row\":%u,\"input\":%.9g,\"target\":%.9g,\"committed\":%.9g,\"delay\":%u,\"low\":%u,\"pendingLow\":%u,\"damage\":%u,\"submitted\":%u,\"widgets\":[",hud_low_age,hud_low_ratio(hud_low_age),ldf32(m,hp+0x1c),ldf32(m,hp+0x20),ld8(m,hp+0x24),ld8(m,hp+0x2e),ld8(m,hp+0x2d),ld8(m,hp+0x2f),ld8(m,hud_low_owner+0x20));
    bool comma=false;uint32_t first=ld32(m,0x8069d49c+0x14),end=ld32(m,0x8069d49c+0x18);
    for(uint32_t node=first,i=0;idt&&node&&node!=end&&i<768&&mapped(node,12);i++){
        uint32_t w=ld32(m,node);node=ld32(m,node+8);if(!mapped(w,0x2a0))continue;uint32_t cluster=ld16(m,w+0x3a);if(cluster!=main&&cluster!=effect)continue;
        uint32_t r=ld32(m,w+0x1a0);if(!mapped(r,0xe4))continue;uint32_t raw=ld32(m,r+0x18);if(!mapped(raw,0x220))continue;
        std::fprintf(out,"%s{\"program\":%u,\"offset\":%u,\"id\":%u,\"hidden\":%u,\"flags\":%u,\"clocks\":[%u,%u,%u,%u],\"rgba\":[",comma?",":"",cluster==main?39:58,raw-idt,ld32(m,raw+0x2c),ld8(m,w+0x269),ld32(m,w+0x14),ld16(m,w+0xc8),ld16(m,w+0x108),ld16(m,w+0x142),ld16(m,w+0x154));comma=true;
        for(unsigned k=0;k<4;k++)std::fprintf(out,"%s%.9g",k?",":"",ldf32(m,w+0x84+k*4));
        std::fprintf(out,"],\"size\":[%.9g,%.9g],\"matrix\":[",ldf32(m,w+0x26c),ldf32(m,w+0x270));for(unsigned k=0;k<12;k++)std::fprintf(out,"%s%.9g",k?",":"",ldf32(m,w+0x1f8+k*4));
        std::fprintf(out,"],\"vertices\":[");for(unsigned k=0;k<12;k++)std::fprintf(out,"%s%.9g",k?",":"",ldf32(m,w+0x234+k*4));
        std::fprintf(out,"],\"bindings\":[");for(unsigned k=0;k<3;k++){uint32_t bound=ld32(m,w+(k==0?0x1a4:k==1?0x1b8:0x1bc));std::fprintf(out,"%s%u",k?",":"",mapped(bound,0x1c)?ld32(m,bound+0x18)-idt:0);}
        std::fprintf(out,"]}");
    }
    std::fprintf(out,"]}\n");std::fflush(out);hud_low_age++;
}
// Explicit isolated arithmetic fixture: original800C5508 on private stack
// storage, restored before returning. Never touches actor/HUD object state.
void hud_gauge_fixture(PPCContext& c,uint8_t* m){
    const char* path=NATIVE_OPTION("MW_TEST_HUD_GAUGE");static bool done=false;if(!path||done)return;done=true;
    FILE* out=std::fopen(path,"w");if(!out)ppc_fatal("Cannot open HUD gauge fixture output");
    auto fn=ppc_lookup(0x800c5508);if(!fn)ppc_fatal("Missing original gauge helper");
    uint32_t gauge=(c.r[1]-0x400)&~15u;if(gauge<0x80001000||gauge>0x817ffc00)ppc_fatal("HUD gauge fixture stack bounds");
    uint8_t previous[12];std::memcpy(previous,m+gauge+0x318,12);PPCContext saved=c;
    std::fprintf(out,"{\"schema\":\"mw-hud-gauge-fixture/1\",\"scope\":\"Synthetic private scratch fields and original800C5508 calls; restored storage/context; no actor or actual HUD writes; arithmetic only\"}\n");
    const float starts[]={1.f,.2f,1.f,.5f};const float targets[]={.5f,1.f,1.f,.49f};const float speeds[]={float(100.f/30.f)/100.f,.05f,.05f,float(100.f/30.f)/320.f};
    for(unsigned test=0;test<4;test++){
        stf32(m,gauge+0x318,starts[test]);stf32(m,gauge+0x31c,starts[test]);stf32(m,gauge+0x320,speeds[test]);
        for(unsigned step=0;step<100;step++){
            float before=ldf32(m,gauge+0x31c);c=saved;c.r[3]=gauge;c.f[1].ps0=c.f[1].ps1=targets[test];fn(c,m);
            std::fprintf(out,"{\"case\":%u,\"step\":%u,\"before\":%.9g,\"target\":%.9g,\"speed\":%.9g,\"after\":%.9g,\"done\":%s}\n",test,step,before,targets[test],speeds[test],ldf32(m,gauge+0x31c),c.r[3]?"true":"false");
        }
    }
    c=saved;std::memcpy(m+gauge+0x318,previous,12);std::fclose(out);
}
void challenge_record(uint8_t* m){
    const char* path=NATIVE_OPTION("MW_CHALLENGE_RECORD");if(!path)return;
    auto mapped=[](uint32_t p,uint32_t n){return(p>=0x80000000u&&p<=0x81800000u-n)||(p>=0x90000000u&&p<=0x94000000u-n);};
    uint32_t owner=ld32(m,0x8069e2c0);if(!mapped(owner,0x40))return;
    uint32_t object=ld32(m,owner+0x3c);static FILE* out=nullptr;static unsigned rows=0;static bool seen=false,done=false;
    if(done||rows>=1024)return;
    if(!mapped(object,0x30)){if(seen&&out){std::fprintf(out,"{\"row\":%u,\"closed\":true}\n",rows++);std::fflush(out);done=true;}return;}
    if(!out){out=std::fopen(path,"w");if(!out)return;std::fprintf(out,"{\"schema\":\"mw-challenge-record/1\",\"scope\":\"Bounded read-only world-update controller/widget/message observations; no guest calls or writes\"}\n");}
    seen=true;
    std::fprintf(out,"{\"row\":%u,\"active\":%u,\"phase\":%u,\"result\":%u,\"yes\":%u,\"ranking\":%u,\"boss\":%u,\"program\":%u,\"vtable\":\"%08x\",\"down\":%u,\"widgets\":[",rows++,ld8(m,object),ld32(m,object+0x24),ld8(m,object+0x29),ld8(m,object+0x2a),ld32(m,object+0x1c),ld32(m,object+0x2c),ld32(m,object+0x20),ld32(m,object+0xc),ld32(m,0x80698b3c));
    uint32_t cluster=ld32(m,object+4),first=ld32(m,0x8069d49c+0x14),end=ld32(m,0x8069d49c+0x18),idt=0;
    for(uint32_t node=first,i=0;node&&node!=end&&i<768&&mapped(node,12);i++){
        uint32_t w=ld32(m,node);node=ld32(m,node+8);if(!mapped(w,0x2a0)||ld16(m,w+0x3a)!=cluster)continue;
        uint32_t r=ld32(m,w+0x1a0);if(!mapped(r,0xe4))continue;uint32_t raw=ld32(m,r+0x18);
        if(mapped(raw,0x100)&&ld32(m,raw+0x2c)==2){idt=raw-606160;break;}
    }
    bool comma=false;
    for(uint32_t node=first,i=0;idt&&node&&node!=end&&i<768&&mapped(node,12);i++){
        uint32_t w=ld32(m,node);node=ld32(m,node+8);if(!mapped(w,0x2a0)||ld16(m,w+0x3a)!=cluster)continue;
        uint32_t r=ld32(m,w+0x1a0);if(!mapped(r,0xe4))continue;uint32_t raw=ld32(m,r+0x18);if(!mapped(raw,0x100))continue;
        std::fprintf(out,"%s{\"offset\":%u,\"id\":%u,\"hidden\":%u,\"flags\":%u,\"uv\":[%u,%u],\"clocks\":[%u,%u,%u,%u],\"rgba\":[",comma?",":"",raw-idt,ld32(m,raw+0x2c),ld8(m,w+0x269),ld32(m,w+0x14),ld16(m,r+0x48),ld16(m,w+0x190),ld16(m,w+0xc8),ld16(m,w+0x108),ld16(m,w+0x142),ld16(m,w+0x154));comma=true;
        for(unsigned k=0;k<4;k++)std::fprintf(out,"%s%.9g",k?",":"",ldf32(m,w+0x84+k*4));
        std::fprintf(out,"],\"size\":[%.9g,%.9g],\"matrix\":[",ldf32(m,w+0x26c),ldf32(m,w+0x270));
        for(unsigned k=0;k<12;k++)std::fprintf(out,"%s%.9g",k?",":"",ldf32(m,w+0x1f8+k*4));
        std::fprintf(out,"],\"bindings\":[");
        for(unsigned k=0;k<3;k++){uint32_t b=ld32(m,w+(k==0?0x1ac:k==1?0x1b0:0x1b8));std::fprintf(out,"%s%u",k?",":"",mapped(b,0x1c)?ld32(m,b+0x18)-idt:0);}
        std::fprintf(out,"]}");
    }
    std::fprintf(out,"],\"glyphs\":[");comma=false;
    for(unsigned slot=0;slot<256;slot++){
        uint32_t w=0x80653f58+slot*0x318,r=0x80685758+slot*0xe4;
        if(!(ld32(m,w+0x7c)&1)||ld8(m,r)!=1)continue;
        std::fprintf(out,"%s{\"slot\":%u,\"hidden\":%u,\"size\":[%.9g,%.9g],\"matrix\":[",comma?",":"",slot,ld8(m,w+0x269),ldf32(m,w+0x26c),ldf32(m,w+0x270));comma=true;
        for(unsigned k=0;k<12;k++)std::fprintf(out,"%s%.9g",k?",":"",ldf32(m,w+0x1f8+k*4));
        std::fprintf(out,"],\"uv\":[");for(unsigned k=0;k<8;k++)std::fprintf(out,"%s%.9g",k?",":"",ldf32(m,w+0x5c+k*4));std::fprintf(out,"]}");
    }
    std::fprintf(out,"],\"windows\":[");comma=false;
    uint32_t manager=0x80653f04;
    if(mapped(manager,0x20))for(uint32_t node=ld32(m,manager+0x18),end=ld32(m,manager+0x1c),i=0;node&&node!=end&&i<32&&mapped(node,12);i++){
        uint32_t w=ld32(m,node);node=ld32(m,node+8);if(!mapped(w,0xc8)||!(ld32(m,w+0x9c)&1))continue;
        std::fprintf(out,"%s{\"message\":%u,\"type\":%u,\"flags\":%u,\"singleRow\":%u,\"scale\":%.9g,\"cursor\":[%.9g,%.9g],\"anchor\":[%.9g,%.9g],\"factors\":[%.9g,%.9g],\"lineSpacing\":%.9g}",comma?",":"",ld32(m,w+0x20),ld32(m,w+0x18),ld32(m,w+0x9c),ld8(m,w+0xc0),ldf32(m,w+0x48),ldf32(m,w+0x68),ldf32(m,w+0x6c),ldf32(m,w+0x7c),ldf32(m,w+0x80),ldf32(m,w+0x84),ldf32(m,w+0x88),ldf32(m,w+0x8c));comma=true;
    }
    std::fprintf(out,"]}\n");std::fflush(out);
}
void event_lifecycle(uint8_t* m){
    const char* path=NATIVE_OPTION("MW_EVENT_LIFECYCLE");if(!path||ld32(m,0x8069bcdc)!=0x201)return;
    static FILE* out=nullptr;static bool opened=false;static unsigned rows=0;
    if(rows>=8192)return;
    if(!opened){opened=true;out=std::fopen(path,"w");if(out)std::fprintf(out,"{\"schema\":\"mw-event-lifecycle/1\",\"scope\":\"Bounded read-only world-update edges; combat Rin pointer becomes known after its first onMove. No guest calls/writes\"}\n");}
    if(!out)return;
    const uint32_t manager=0x80606cf8,fade=0x80607078;
    std::fprintf(out,"{\"row\":%u,\"state\":%u,\"skipEnabled\":%u,\"word130\":%u,\"frame\":%u,\"cut\":%u,\"local\":%u,\"demo\":%d,\"point\":%u,\"padDown\":%u,\"fade\":{\"state\":%u,\"rgba\":%u,\"from\":%u,\"to\":%u,\"duration\":%.9g,\"elapsed\":%.9g}",rows++,ld32(m,manager+4),ld8(m,manager+1),ld32(m,manager+0x130),ld32(m,manager+0x114),ld32(m,manager+0x110),ld32(m,manager+0x118),int32_t(ld32(m,0x80604020)),ld32(m,0x8069bce0),ld32(m,0x80698b3c),ld32(m,fade+0x28),ld32(m,fade+0x1c),ld32(m,fade+0x14),ld32(m,fade+0x18),ldf32(m,fade+0x20),ldf32(m,fade+0x24));
    auto mapped=[](uint32_t p,uint32_t n){return(p>=0x80000000u&&p<=0x81800000u-n)||(p>=0x90000000u&&p<=0x94000000u-n);};
    auto actor=[&](const char* name,uint32_t p){
        std::fprintf(out,",\"%s\":",name);
        if(!mapped(p,0x214)){std::fprintf(out,"null");return;}
        std::fprintf(out,"{\"pointer\":\"%08x\",\"state\":[%u,%u,%u,%u],\"pos\":[%.9g,%.9g,%.9g],\"hp\":%d,\"facing\":%.9g,\"motionFrame\":%.9g}",p,ld8(m,p+0x11a),ld8(m,p+0x11b),ld8(m,p+0x11c),ld8(m,p+0x11d),ldf32(m,p+0x64),ldf32(m,p+0x68),ldf32(m,p+0x6c),int16_t(ld16(m,p+0xec)),ldf32(m,p+0x74),ldf32(m,p+0x204));
    };
    actor("player",native_snapshot_player());actor("rinMoved",native_frame_record_rin_object());
    const uint32_t sound=0x806a1d6c;
    std::fprintf(out,",\"sound\":{\"currentMusic\":%u,\"eventMode\":%u,\"slots\":[",ld32(m,sound+0x196c),ld8(m,sound+0xc0));
    for(unsigned i=0;i<3;i++){
        uint32_t at=sound+i*0x48;
        std::fprintf(out,"%s{\"state\":%u,\"volume\":%u,\"name\":\"",i?",":"",ld8(m,at+0x1516),ld32(m,at+0x14f4));
        for(unsigned j=0;j<48;j++){unsigned ch=ld8(m,sound+i*0x30+j);if(!ch)break;if(ch=='\"'||ch=='\\')std::fputc('\\',out);if(ch<32)std::fprintf(out,"\\u%04x",ch);else std::fputc(ch,out);}
        std::fprintf(out,"\"}");
    }
    std::fprintf(out,"]}");
    std::fprintf(out,"}\n");std::fflush(out);
}
void bitmap(uint8_t* m){
    uint32_t p=ld32(m,0x80697e50),count=ld32(m,0x80697e54);
    std::fprintf(stderr,"[effect] flags ptr=%08x words=%u",p,count);
    if((p>=0x80000000&&p<0x81800000-8)||(p>=0x90000000&&p<0x94000000-8))std::fprintf(stderr," bits=%08x/%08x",ld32(m,p),ld32(m,p+4));
    std::fprintf(stderr,"\n");
}
}
PPC_FUNC(f_802140E4){
    static bool first=true;
    uint32_t scene=c.r[4];
    if(native_snapshot_restored())first=false;
    if(first) {
        first=false;
        if(const char* requested=native_config_get("MW_BOOT_SCENE")) {
            char* end=nullptr;unsigned long selected=std::strtoul(requested,&end,0);
            if(!end||*end||(selected!=0x201&&selected!=0x303)||scene!=0xe00)ppc_fatal("MW_BOOT_SCENE supports only explicit first E00->201 / E00->303 (Elise) diagnostic setup");
            std::fprintf(stderr,"[setup] diagnostic first-world argument override %03x -> %03lx; other scene/progression state is original\n",scene,selected);
            c.r[4]=scene=uint32_t(selected);
        }
        if(const char* point=native_config_get("MW_BOOT_POINT")) {
            char* end=nullptr;unsigned long selected=std::strtoul(point,&end,0);
            if(!end||*end||!((selected==5&&scene==0x201)||(selected==1&&scene==0x303)))ppc_fatal("MW_BOOT_POINT supports only c201 PLP index5 / c303 PlayerTbl index1 (Boss)");
            uint32_t before=c.r[5];c.r[5]=(before&~255u)|uint32_t(selected);
            std::fprintf(stderr,"[setup] diagnostic native player-point argument %08x -> %08x (PLP boss index5); progression flags unchanged\n",before,c.r[5]);
        }
    }
    if(enabled())std::fprintf(stderr,"[world] begin scene=%x flags=%x effect-table=%08x\n",scene,c.r[5],effect_table(m));
    f_802140E4_recomp(c,m);
    save_fixture_import(m,scene);
    native_platform_gx_scene(scene);
    if(enabled())std::fprintf(stderr,"[world] end scene=%x effect-table=%08x\n",scene,effect_table(m));
}
PPC_FUNC(f_801FC51C){
    if(enabled())std::fprintf(stderr,"[effect] init this=%08x args=%x/%x\n",c.r[3],c.r[4],c.r[5]);
    if(enabled())bitmap(m);
    f_801FC51C_recomp(c,m);
    if(enabled())std::fprintf(stderr,"[effect] init returned table=%08x\n",effect_table(m));
}
PPC_FUNC(f_801FC87C){
    if(enabled())std::fprintf(stderr,"[effect] cleanup table=%08x\n",effect_table(m));
    f_801FC87C_recomp(c,m);
    if(enabled())std::fprintf(stderr,"[effect] cleanup returned table=%08x\n",effect_table(m));
}
PPC_FUNC(f_80215160){
    static unsigned logged=0;static uint64_t last=0;
    static unsigned boss_updates=0;static bool boss_requested=false;
    native_snapshot_checkpoint(boss_requested);
    native_snapshot_report_world();
    uint64_t now=ppc_timebase();
    if(enabled()&&(logged++<8||now-last>=60750000)){last=now;std::fprintf(stderr,"[world] update scene=%x effect-table=%08x pad-hold=%08x pad-down=%08x\n",ld32(m,c.r[3]+0x34),effect_table(m),ld32(m,0x80698b38),ld32(m,0x80698b3c));}
    hud_low_submit(c,m);
    message_fixture_submit(c,m);
    f_80215160_recomp(c,m);
    message_fixture_record(m);
    hud_low_record(m);
    hud_gauge_fixture(c,m);
    challenge_record(m);
    event_lifecycle(m);
    if(native_snapshot_restored())boss_requested=true;
    if(!boss_requested&&NATIVE_OPTION("MW_BOOT_BOSS")&&(ld32(m,0x8069bcdc)==0x201||ld32(m,0x8069bcdc)==0x303)&&++boss_updates==10){
        if(ld32(m,0x8069bce0)!=(ld32(m,0x8069bcdc)==0x303?1u:5u))ppc_fatal("MW_BOOT_BOSS requires the room PLP boss point (c201 5, c303 1)");
        if(std::string(NATIVE_OPTION("MW_BOOT_BOSS"))!="1")ppc_fatal("MW_BOOT_BOSS supports only explicit1 diagnostic request");
        boss_requested=true;
        std::fprintf(stderr,"[setup] diagnostic original BossSw/invoke callback -> execRoomEvent(-97)\n");
        PPCContext saved=c;
        auto request=ppc_lookup(0x801327a4);
        if(!request)ppc_fatal("missing original boss switch callback");
        request(c,m);
        c=saved;
        std::fprintf(stderr,"[setup] original boss switch callback returned\n");
    }
    // Explicit test fixture only: send the original "onBossDead" message to the
    // original "boss" target (what the boss death path sends) ~5s after the
    // intro demo nesting unwinds, so the result/roomClear transition can be
    // exercised without a full fight. Default off; strings are the DOL's own.
    static unsigned boss_dead_countdown=0;static bool boss_dead_done=false;
    if(!boss_dead_done&&boss_requested&&NATIVE_OPTION("MW_TEST_BOSS_DEAD")&&int32_t(ld32(m,0x80604020))==0){
        if(std::string(NATIVE_OPTION("MW_TEST_BOSS_DEAD"))!="1")ppc_fatal("MW_TEST_BOSS_DEAD supports only explicit1 fixture request");
        if(++boss_dead_countdown==300){
            boss_dead_done=true;
            PPCContext saved=c;uint32_t target=(c.r[1]-0x40)&~15u;
            auto make_target=ppc_lookup(0x801133dc);auto send=ppc_lookup(0x8005b6a4);
            if(!make_target||!send)ppc_fatal("missing original message target/send");
            c.r[3]=target;c.r[4]=0x805a4dfb;make_target(c,m);
            c=saved;c.r[3]=target;c.r[4]=0x805a4de9;send(c,m);
            c=saved;
            std::fprintf(stderr,"[fixture] MW_TEST_BOSS_DEAD: original send(boss,onBossDead) issued (test-only)\n");
        }
    }
    // Explicit test fixture only: once the boss intro demo nesting has unwound,
    // set the player's HP to 1 so the next original hit runs the lethal path.
    // Never natural; never a death write. Default off.
    static bool hp_fixture_done=false;
    if(!hp_fixture_done&&boss_requested&&NATIVE_OPTION("MW_TEST_PLAYER_HP1")&&int32_t(ld32(m,0x80604020))==0){
        if(std::string(NATIVE_OPTION("MW_TEST_PLAYER_HP1"))!="1")ppc_fatal("MW_TEST_PLAYER_HP1 supports only explicit1 fixture request");
        PPCContext saved=c;c.r[3]=0x8069bca8;c.r[4]=0;
        auto get_player=ppc_lookup(0x8021639c);if(!get_player)ppc_fatal("missing original player list getter");
        get_player(c,m);uint32_t player=c.r[3];c=saved;
        if(player&&int32_t(lds16(m,player+0xec))>1){
            hp_fixture_done=true;
            // Optional companion fixture: force the save-data continue count
            // (cSaveData 0x80616880 +4, read by getContinue) so the original
            // game-over branch of the continue sequence can be exercised.
            if(const char* continues=NATIVE_OPTION("MW_TEST_PLAYER_CONTINUES")){
                if(std::string(continues)!="0")ppc_fatal("MW_TEST_PLAYER_CONTINUES supports only explicit0 fixture request");
                std::fprintf(stderr,"[fixture] MW_TEST_PLAYER_CONTINUES: save continues %u -> 0 (test-only)\n",ld32(m,0x80616884));
                st32(m,0x80616884,0);
            }
            std::fprintf(stderr,"[fixture] MW_TEST_PLAYER_HP1: player=%08x hp %d -> 1 (test-only, original hit path decides death)\n",player,int32_t(lds16(m,player+0xec)));
            st16(m,player+0xec,1);
        }
    }
}
PPC_FUNC(f_800DA39C){
    uint32_t saved[32];std::memcpy(saved,c.r,sizeof saved);
    if(enabled())std::fprintf(stderr,"[common] initializer entry sp=%08x r31=%08x\n",c.r[1],c.r[31]);
    f_800DA39C_recomp(c,m);
    if(enabled()){std::fprintf(stderr,"[common] initializer returned sp=%08x r31=%08x\n",c.r[1],c.r[31]);for(int i=14;i<32;i++)if(c.r[i]!=saved[i])std::fprintf(stderr,"[ABI] init r%d %08x -> %08x\n",i,saved[i],c.r[i]);bitmap(m);}
}
PPC_FUNC(f_800DA420){
    uint32_t saved[32];std::memcpy(saved,c.r,sizeof saved);
    if(enabled())std::fprintf(stderr,"[common] loader entry\n");
    f_800DA420_recomp(c,m);
    if(enabled()){std::fprintf(stderr,"[common] loader returned=%x\n",c.r[3]);for(int i=14;i<32;i++)if(c.r[i]!=saved[i])std::fprintf(stderr,"[ABI] loader r%d %08x -> %08x\n",i,saved[i],c.r[i]);}
}
PPC_FUNC(f_8024C8E4){
    if(enabled())std::fprintf(stderr,"[common] CORE pack entry this=%08x name=%08x\n",c.r[3],c.r[4]);
    f_8024C8E4_recomp(c,m);
    if(enabled())std::fprintf(stderr,"[common] CORE pack returned=%x\n",c.r[3]);
}

PPC_FUNC(f_802E4AE8){
    if(enabled())std::fprintf(stderr,"[throw] enter sp=%08x lr=%08x r3=%08x r4=%08x r5=%08x\n",c.r[1],c.lr,c.r[3],c.r[4],c.r[5]);
    f_802E4AE8_recomp(c,m);
    if(enabled())std::fprintf(stderr,"[throw] original returned sp=%08x lr=%08x ctr=%08x r31=%08x\n",c.r[1],c.lr,c.ctr,c.r[31]);
}

PPC_FUNC(f_80145934){
    uint32_t object=c.r[3];
    if(enabled())std::fprintf(stderr,"[stage201-logic] setup this=%08x caller=%08x\n",object,c.lr);
    f_80145934_recomp(c,m);
    if(enabled()){
        std::fprintf(stderr,"[stage201-logic] setup returned\n");
        // These19 native event records are initialized by801447EC. Record
        // raw known field offsets; callback/condition semantics are separate.
        for(unsigned index=0;index<19;++index){
            uint32_t record=object+0xe8+index*0x28;
            std::fprintf(stderr,"[room-binding] record=%08x key=%d flags=%08x fields08to18=%08x/%08x/%08x/%08x/%08x\n",record,int32_t(ld32(m,record)),ld32(m,record+4),ld32(m,record+8),ld32(m,record+12),ld32(m,record+16),ld32(m,record+20),ld32(m,record+24));
        }
    }
}
PPC_FUNC(f_801E3AAC){
    uint32_t object=c.r[3],index=c.r[4],caller=c.lr;
    f_801E3AAC_recomp(c,m);
    static unsigned count=0;
    if(enabled()&&index==24&&count++<32)std::fprintf(stderr,"[boss-threshold] this=%08x enum=%u value=%u caller=%08x\n",object,index,c.r[3],caller);
}

PPC_FUNC(f_80147BBC){
    if(enabled())std::fprintf(stderr,"[boss-entry] original Rin entry begin this=%08x caller=%08x\n",c.r[3],c.lr);
    f_80147BBC_recomp(c,m);
    if(enabled())std::fprintf(stderr,"[boss-entry] original Rin entry returned\n");
}
// Original rpn named-message send: builds a target/name tag, finds a listener in
// the global cMessageHandler<void()> tree and invokes8005B724 only when found.
namespace {
bool rpn_trace(){static bool on=NATIVE_OPTION("MW_TRACE_RPN")!=nullptr;return on;}
unsigned rpn_depth=0,rpn_invoked=0;
std::string rpn_guest_name(uint8_t* m,uint32_t a){
    std::string s;bool mapped=(a>=0x80000000&&a<0x81800000-64)||(a>=0x90000000&&a<0x94000000-64);
    for(unsigned i=0;mapped&&i<63;++i){uint8_t ch=ld8(m,a+i);if(ch<32||ch>126)break;s.push_back(char(ch));}
    return s;
}
}
PPC_FUNC(f_8005B6A4){
    uint32_t target=c.r[3],caller=c.lr;
    if(!rpn_trace()){f_8005B6A4_recomp(c,m);return;}
    std::string name=rpn_guest_name(m,c.r[4]);
    unsigned before=rpn_invoked;++rpn_depth;
    f_8005B6A4_recomp(c,m);
    --rpn_depth;
    static unsigned count=0;
    bool notable=name.find("Dead")!=std::string::npos||name.find("Mission")!=std::string::npos;
    if(notable||count++<64)std::fprintf(stderr,"[rpn-send] name=%s target=%08x tag=%08x caller=%08x listener=%s\n",name.c_str(),target,ld32(m,target),caller,rpn_invoked!=before?"invoked":"none");
}
PPC_FUNC(f_8005B724){
    if(rpn_depth)++rpn_invoked;
    f_8005B724_recomp(c,m);
}
// Original broadcast send(name) (tag -1) used by room scripts for roomClear,
// onContinueRestart and subMission clear. Same MW_TRACE_RPN gate.
PPC_FUNC(f_8005B674){
    uint32_t caller=c.lr;
    if(!rpn_trace()){f_8005B674_recomp(c,m);return;}
    std::string name=rpn_guest_name(m,c.r[3]);
    unsigned before=rpn_invoked;++rpn_depth;
    f_8005B674_recomp(c,m);
    --rpn_depth;
    std::fprintf(stderr,"[rpn-broadcast] name=%s caller=%08x listener=%s\n",name.c_str(),caller,rpn_invoked!=before?"invoked":"none");
}
// Original BeginDemoMode(bool)/EndDemoMode(): nesting counter at 0x80604020,
// IsDemoMode() is its sign bit. Logged so the death/continue gate can be read.
PPC_FUNC(f_80086C9C){
    uint32_t caller=c.lr,arg=c.r[3];int32_t before=int32_t(ld32(m,0x80604020));
    f_80086C9C_recomp(c,m);
    if(rpn_trace())std::fprintf(stderr,"[demo-mode] begin arg=%u counter %d -> %d caller=%08x\n",arg,before,int32_t(ld32(m,0x80604020)),caller);
}
PPC_FUNC(f_80086D58){
    uint32_t caller=c.lr;int32_t before=int32_t(ld32(m,0x80604020));
    f_80086D58_recomp(c,m);
    if(rpn_trace())std::fprintf(stderr,"[demo-mode] end counter %d -> %d caller=%08x\n",before,int32_t(ld32(m,0x80604020)),caller);
}
// Player-side demo guard pair (object at player+0x1568, flag byte +1): begin
// 80280F84 calls BeginDemoMode(1) once, end 802788B0 calls EndDemoMode once.
PPC_FUNC(f_80280F84){
    uint32_t guard=c.r[3],caller=c.lr;uint8_t flag=ld8(m,guard+1);
    f_80280F84_recomp(c,m);
    if(rpn_trace())std::fprintf(stderr,"[demo-guard] begin guard=%08x was=%u caller=%08x state=%u\n",guard,flag,caller,ld8(m,guard-0x1568+0x11b));
}
PPC_FUNC(f_802788B0){
    uint32_t guard=c.r[3],caller=c.lr;uint8_t flag=ld8(m,guard+1);
    f_802788B0_recomp(c,m);
    if(rpn_trace())std::fprintf(stderr,"[demo-guard] end guard=%08x was=%u caller=%08x state=%u\n",guard,flag,caller,ld8(m,guard-0x1568+0x11b));
}
// Continue window open (80244DBC: layout widget id3, mode0 continue / mode1 game over)
// and game-over banner (80245370: widget id0x11). Logged with the save-data
// continue count 0x80616880+4 so the room-event fiber's wait can be read.
PPC_FUNC(f_80244DBC){
    uint32_t ui=c.r[3],mode=c.r[4],caller=c.lr;
    f_80244DBC_recomp(c,m);
    if(rpn_trace())std::fprintf(stderr,"[continue-ui] window mode=%u widget=%08x continues=%u caller=%08x\n",mode,ld32(m,ui+0x1c),ld32(m,0x80616884),caller);
}
PPC_FUNC(f_80245370){
    uint32_t ui=c.r[3],arg=c.r[4],caller=c.lr;
    f_80245370_recomp(c,m);
    if(rpn_trace())std::fprintf(stderr,"[continue-ui] gameover banner arg=%u widget=%08x caller=%08x\n",arg,ld32(m,ui+0x50),caller);
}
// Original cListenerBase::set(this, tree&, const cMessageTarget&, const char* name):
// the only registration path for named-message listeners.
PPC_FUNC(f_80113900){
    uint32_t listener=c.r[3],tree=c.r[4],target=c.r[5],caller=c.lr;
    if(!rpn_trace()){f_80113900_recomp(c,m);return;}
    std::string name=rpn_guest_name(m,c.r[6]);
    uint32_t tag=(target>=0x80000000&&target<0x81800000-4)||(target>=0x90000000&&target<0x94000000-4)?ld32(m,target):0;
    std::string tagname=rpn_guest_name(m,tag);
    f_80113900_recomp(c,m);
    std::fprintf(stderr,"[rpn-set] name=%s tag=%08x tagstr=%s listener=%08x tree=%08x caller=%08x\n",name.c_str(),tag,tagname.c_str(),listener,tree,caller);
}
PPC_FUNC(f_80123820){
    uint32_t event=c.r[3];
    if(enabled())std::fprintf(stderr,"[event] request=%02x caller=%08x\n",event,c.lr);
    f_80123820_recomp(c,m);
    if(enabled())std::fprintf(stderr,"[event] request=%02x returned\n",event);
}
PPC_FUNC(f_800A6F60){
    uint32_t object=c.r[3];
    const uint32_t before=ld32(m,object+0x10c);
    f_800A6F60_recomp(c,m);
    uint32_t frame=ld32(m,object+0x10c);
    if(enabled()&&(frame<4||frame%60==0||frame+1==ld32(m,object+0x104)))std::fprintf(stderr,"[event-clock] this=%08x frame=%u total=%u cut=%u local=%u\n",object,frame,ld32(m,object+0x104),ld32(m,object+0x108),ld32(m,object+0x110));
    // Focused read-only Rin intro oracle: no guest calls or writes. The clock
    // wrapper observes camera state from the preceding camera application, so
    // retain both clock edges and the unit's own source-sample index.
    static FILE* event_out=nullptr;static bool event_opened=false;static unsigned event_rows=0;
    const char* path=NATIVE_OPTION("MW_EVENT_RECORD");
    if(!path||ld32(m,object+0x104)!=1080||event_rows>=4096)return;
    if(!event_opened){event_opened=true;event_out=std::fopen(path,"w");if(event_out)std::fprintf(event_out,"{\"schema\":\"mw-event-record/1\",\"scope\":\"Read-only clock-edge and preceding camera observations; no guest writes\"}\n");}
    if(!event_out)return;
    auto mapped=[](uint32_t p,uint32_t n){return(p>=0x80000000u&&p<=0x81800000u-n)||(p>=0x90000000u&&p<=0x94000000u-n);};
    std::fprintf(event_out,"{\"before\":%u,\"frame\":%u,\"cut\":%u,\"local\":%u,\"timebase\":%llu,\"framesPerUpdate\":%u",before,frame,ld32(m,object+0x108),ld32(m,object+0x110),(unsigned long long)ppc_timebase(),ld32(m,0x805cbe20));
    uint32_t unit=ld32(m,0x80602d54);
    if(mapped(unit,0x214)){
        uint32_t source=ld32(m,unit+0x1ec),samples=ld32(m,unit+0x1f0),index=ld16(m,unit+0x1fc);
        if(mapped(source,32)&&ld32(m,source)==0x63616d00&&index<ld16(m,source+6)&&mapped(samples+index*32,32)){
            std::fprintf(event_out,",\"camera\":{\"unit\":\"%08x\",\"frames\":%u,\"index\":%u,\"flags\":%u,\"sample\":[",unit,ld16(m,source+6),index,ld32(m,unit+0x1e4));
            for(unsigned i=0;i<8;++i)std::fprintf(event_out,"%s%.9g",i?",":"",ldf32(m,samples+index*32+i*4));
            std::fprintf(event_out,"]}");
        }
    }
    std::fprintf(event_out,",\"applied\":{");
    for(unsigned group=0;group<3;++group){uint32_t at=0x8069bd80+0x1b0+group*12;std::fprintf(event_out,"%s\"%s\":[%.9g,%.9g,%.9g]",group?",":"",group==0?"eye":group==1?"target":"up",ldf32(m,at),ldf32(m,at+4),ldf32(m,at+8));}
    std::fprintf(event_out,",\"fovDegrees\":%.9g}",ldf32(m,0x8069bd80+0x288));
    std::fprintf(event_out,",\"fade\":{\"active\":%u,\"rgba\":\"%08x\"}",ld32(m,0x80607078+0x28),ld32(m,0x80607078+0x1c));
    if(NATIVE_OPTION("MW_EVENT_AUDIO")){
        const uint32_t sound=0x806a1d6c;
        auto text=[&](uint32_t at){std::fprintf(event_out,"\"");for(unsigned i=0;i<48;++i){unsigned ch=ld8(m,at+i);if(!ch)break;if(ch=='\"'||ch=='\\')std::fputc('\\',event_out);if(ch<32)std::fprintf(event_out,"\\u%04x",ch);else std::fputc(ch,event_out);}std::fprintf(event_out,"\"");};
        std::fprintf(event_out,",\"sound\":{\"musicLevel\":%u,\"voiceLevel\":%u,\"currentMusic\":%u,\"musicSlots\":[",ld32(m,sound+0x1dbc),ld32(m,sound+0x1dc0),ld32(m,sound+0x196c));
        for(unsigned i=0;i<3;++i){uint32_t at=sound+i*0x48;std::fprintf(event_out,"%s{\"name\":",i?",":"");text(sound+i*0x30);std::fprintf(event_out,",\"state\":%u,\"volume\":%u,\"fade\":%u}",ld8(m,at+0x1516),ld32(m,at+0x14f4),ld16(m,at+0x1514));}
        std::fprintf(event_out,"],\"voice\":{\"name\":");text(sound+0x90);std::fprintf(event_out,",\"state\":%u,\"volume\":%u,\"id\":%u}}",ld8(m,sound+0x15ee),ld32(m,sound+0x15cc),ld32(m,sound+0x1970));
    }
    if(NATIVE_OPTION("MW_EVENT_SE")){
        // Original EV21 SE slots and their live SDK wave/mixer records. Bounded
        // reads only: no extra guest calls or writes, and no output PCM claim.
        const uint32_t sound=0x806a1d6c,sdk=ld32(m,0x807184c8),mix=ld32(m,0x80718518);
        uint32_t voices=mapped(sdk,0xdc0)?ld32(m,sdk+0xdbc):0;
        bool comma=false;std::fprintf(event_out,",\"eventSE\":[");
        for(unsigned slot=0;slot<64;++slot){
            uint32_t id=ld32(m,sound+0xf0+slot*4);if(id!=7522001&&id!=7522006)continue;
            uint32_t handle=ld32(m,sound+0x2f0+slot*0x48);
            std::fprintf(event_out,"%s{\"slot\":%u,\"id\":%u,\"state\":%u,\"handle\":%u,\"voices\":[",comma?",":"",slot,id,ld32(m,sound+0x1f0+slot*4),handle);comma=true;
            bool voice_comma=false;
            for(unsigned j=0;handle&&j<64&&mapped(voices+j*0x1d8,0x1d8);++j){
                uint32_t voice=voices+j*0x1d8;if(ld32(m,voice+0x16c)!=handle||!ld8(m,voice+0x2a))continue;
                uint32_t wave=ld32(m,voice+0x1c),channel=ld8(m,voice+0x1d2),record=mix+channel*0xd0,program=ld32(m,voice);
                std::fprintf(event_out,"%s{\"index\":%u,\"state\":%u,\"waveBlock\":%u,\"waveIndex\":%u,\"opcodeIndex\":%u,\"note\":%u,\"waitTicks\":%u,\"seStep\":%u,\"channel\":%u,\"program\":\"",voice_comma?",":"",j,ld8(m,voice+0x2a),mapped(wave,0x18)?ld32(m,wave+0x14):0,ld32(m,voice+0x19c),ld16(m,voice+0x1c8),ld8(m,voice+0x28),ld32(m,voice+0x17c),mapped(sdk,0x1234)?ld32(m,sdk+0x1230):0,channel);voice_comma=true;
                if(mapped(program,32))for(unsigned k=0;k<32;++k)std::fprintf(event_out,"%02x",ld8(m,program+k));
                std::fprintf(event_out,"\",\"mixWords\":[");
                for(unsigned k=0;k<6;++k)std::fprintf(event_out,"%s%u",k?",":"",ld32(m,voice+0xb0+k*4));
                std::fprintf(event_out,"],\"left\":%u,\"right\":%u}",mapped(record,0x74)?ld16(m,record+0x6c):0,mapped(record,0x74)?ld16(m,record+0x70):0);
            }
            std::fprintf(event_out,"]}");
        }
        std::fprintf(event_out,"]");
    }
    if(NATIVE_OPTION("MW_EVENT_UI")){
        uint32_t manager=object-8,idt=0,idt_end=0;
        for(unsigned i=0;i<ld32(m,manager+0x13c)&&i<64;++i){
            uint32_t task=ld32(m,manager+0x140+i*4);if(!mapped(task,0x410))continue;
            uint32_t vt=ld32(m,task+0x404);if(!mapped(vt,0x28)||ld32(m,vt+0x24)!=0x800da154)continue;
            uint32_t idd=ld32(m,task+0x408);if(!mapped(idd,0x28)||ld32(m,idd)!=0x49444400)continue;
            idt=idd+ld32(m,idd+0x1c);idt_end=idd+ld32(m,idd+0x24);break;
        }
        std::fprintf(event_out,",\"eventUi\":[");bool comma=false;
        uint32_t current=ld32(m,0x8069d49c+0x14),end=ld32(m,0x8069d49c+0x18);
        for(unsigned i=0;idt&&current&&current!=end&&i<768&&mapped(current,12);++i){
            uint32_t widget=ld32(m,current);current=ld32(m,current+8);if(!mapped(widget,0x2a0))continue;
            uint32_t resource=ld32(m,widget+0x1a0);if(!mapped(resource,0xe4))continue;
            uint32_t raw=ld32(m,resource+0x18);if(raw<idt||raw>=idt_end||!mapped(raw,0x100))continue;
            std::fprintf(event_out,"%s{\"offset\":%u,\"pointer\":\"%08x\",\"hidden\":%u,\"flags\":%u,\"uv\":[%u,%u],\"clocks\":[%u,%u,%u,%u],\"rgba\":[",comma?",":"",raw-idt,widget,ld8(m,widget+0x269),ld32(m,widget+0x14),ld16(m,resource+0x48),ld16(m,widget+0x190),ld16(m,widget+0xc8),ld16(m,widget+0x108),ld16(m,widget+0x142),ld16(m,widget+0x154));comma=true;
            for(unsigned k=0;k<4;++k)std::fprintf(event_out,"%s%.9g",k?",":"",ldf32(m,widget+0x84+k*4));
            std::fprintf(event_out,"],\"size\":[%.9g,%.9g],\"matrix\":[",ldf32(m,widget+0x26c),ldf32(m,widget+0x270));
            for(unsigned k=0;k<12;++k)std::fprintf(event_out,"%s%.9g",k?",":"",ldf32(m,widget+0x1f8+k*4));
            std::fprintf(event_out,"]}");
        }
        std::fprintf(event_out,"]");
    }
    if(NATIVE_OPTION("MW_EVENT_WRITING")){
        // Writing-character bank is separate from the event's IDD. Bounded
        // read-only observations; no guest calls, random draws or writes.
        uint32_t writing=ld32(m,0x8069e2cc),idt=0,idt_end=0;
        if(mapped(writing,0x58)){
            uint32_t idd=ld32(m,writing+0x14);
            if(mapped(idd,0x28)&&ld32(m,idd)==0x49444400){idt=idd+ld32(m,idd+0x1c);idt_end=idd+ld32(m,idd+0x24);}
        }
        std::fprintf(event_out,",\"writingUi\":[");bool comma=false;
        uint32_t current=ld32(m,0x8069d49c+0x14),end=ld32(m,0x8069d49c+0x18);
        for(unsigned i=0;idt&&current&&current!=end&&i<768&&mapped(current,12);++i){
            uint32_t widget=ld32(m,current);current=ld32(m,current+8);if(!mapped(widget,0x2a0))continue;
            uint32_t resource=ld32(m,widget+0x1a0);if(!mapped(resource,0xe4))continue;
            uint32_t raw=ld32(m,resource+0x18);if(raw<idt||raw>=idt_end||!mapped(raw,0x100))continue;
            std::fprintf(event_out,"%s{\"offset\":%u,\"pointer\":\"%08x\",\"hidden\":%u,\"flags\":%u,\"uv\":[%u,%u],\"clocks\":[%u,%u,%u,%u],\"rgba\":[",comma?",":"",raw-idt,widget,ld8(m,widget+0x269),ld32(m,widget+0x14),ld16(m,resource+0x48),ld16(m,widget+0x190),ld16(m,widget+0xc8),ld16(m,widget+0x108),ld16(m,widget+0x142),ld16(m,widget+0x154));comma=true;
            for(unsigned k=0;k<4;++k)std::fprintf(event_out,"%s%.9g",k?",":"",ldf32(m,widget+0x84+k*4));
            std::fprintf(event_out,"],\"size\":[%.9g,%.9g],\"matrix\":[",ldf32(m,widget+0x26c),ldf32(m,widget+0x270));
            for(unsigned k=0;k<12;++k)std::fprintf(event_out,"%s%.9g",k?",":"",ldf32(m,widget+0x1f8+k*4));
            std::fprintf(event_out,"],\"position\":[%.9g,%.9g,%.9g],\"scale\":[%.9g,%.9g],\"rotation\":[%.9g,%.9g,%.9g]}",ldf32(m,resource+0x78),ldf32(m,resource+0x7c),ldf32(m,resource+0x80),ldf32(m,resource+0x68),ldf32(m,resource+0x6c),ldf32(m,resource+0xa0),ldf32(m,resource+0xa4),ldf32(m,resource+0xa8));
        }
        std::fprintf(event_out,"]");
    }
    if(NATIVE_OPTION("MW_EVENT_ACTORS")){
        auto matrix=[&](uint32_t at){std::fprintf(event_out,"[");for(unsigned i=0;i<12;++i)std::fprintf(event_out,"%s%.9g",i?",":"",ldf32(m,at+i*4));std::fprintf(event_out,"]");};
        auto name=[&](uint32_t at,unsigned limit){std::fprintf(event_out,"\"");for(unsigned i=0;i<limit;++i){unsigned ch=ld8(m,at+i);if(!ch)break;if(ch=='\"'||ch=='\\')std::fputc('\\',event_out);if(ch<32)std::fprintf(event_out,"\\u%04x",ch);else std::fputc(ch,event_out);}std::fprintf(event_out,"\"");};
        const uint32_t manager=object-8;bool comma=false;std::fprintf(event_out,",\"actors\":[");
        for(unsigned i=0;i<ld32(m,manager+0x13c)&&i<64;++i){
            uint32_t task=ld32(m,manager+0x140+i*4);if(!mapped(task,0x524))continue;
            uint32_t vt=ld32(m,task+0x404);if(!mapped(vt,0x28)||ld32(m,vt+0x24)!=0x8011db08)continue;
            uint32_t actor=ld32(m,task+0x410);if(!mapped(actor,0x208))continue;
            std::fprintf(event_out,"%s{\"name\":",comma?",":"");name(task+0x518,12);comma=true;
            std::fprintf(event_out,",\"pointer\":\"%08x\",\"visible\":%s,\"motionFrame\":%.9g,\"attachment\":\"%08x\",\"world\":",actor,ld32(m,actor+0x1a0)&4?"true":"false",ldf32(m,actor+0x204),ld32(m,actor+0xa4));matrix(actor+4);
            uint32_t motion=ld32(m,actor+0x200);if(mapped(motion,32)){std::fprintf(event_out,",\"motionHeader\":\"");for(unsigned h=0;h<32;++h)std::fprintf(event_out,"%02x",ld8(m,motion+h));std::fprintf(event_out,"\"");}
            uint32_t model=ld32(m,actor+0xfc);std::fprintf(event_out,",\"meshes\":[");
            if(mapped(model,0x98)){uint32_t groups=ld32(m,model+0x94),count=ld32(m,model+0x90);for(unsigned g=0;g<count&&g<64&&mapped(groups+g*8,8);++g){uint32_t mesh=ld32(m,groups+g*8+4);if(!mapped(mesh,0x60))break;std::fprintf(event_out,"%s{\"name\":",g?",":"");name(mesh+0x20,60);std::fprintf(event_out,",\"cancelled\":%s}",ld32(m,groups+g*8)&1?"true":"false");}}
            std::fprintf(event_out,"],\"parts\":[");bool part_comma=false;uint32_t part=ld32(m,actor+0xac);
            for(unsigned p=0;p<ld32(m,actor+0xd8)&&p<256&&mapped(part,0xd0);++p,part=ld32(m,part+0xac)){unsigned id=ld8(m,part+0xcc);if(id!=8&&id!=14&&id!=100&&id!=115)continue;std::fprintf(event_out,"%s{\"bone\":%u,\"pointer\":\"%08x\",\"world\":",part_comma?",":"",id,part);matrix(part+4);std::fprintf(event_out,"}");part_comma=true;}
            std::fprintf(event_out,"]}");
        }
        std::fprintf(event_out,"]");
    }
    std::fprintf(event_out,"}\n");std::fflush(event_out);++event_rows;
}
