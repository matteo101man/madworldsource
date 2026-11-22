#include "native_config.h"
#include "mmio.h"
#include "snapshot.h"
#include "rso.h"
#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <cmath>
#include <cstring>
#include <string>

extern const PPCFuncEntry g_rso_funcs[];
extern const size_t g_rso_func_count;
extern const PPCFuncName g_rso_func_names[];
extern const size_t g_rso_func_name_count;
static uint8_t* memory = nullptr;
void native_frame_record_rin(uint8_t* m,uint32_t rin);
namespace {
PPCFunc rin_original[4]{};
PPCFunc elise_original=nullptr;
uint32_t elise_seen=0;
uint32_t rin_seen=0;
void rin_trace(const char* stage,PPCContext& c,uint8_t* m,PPCFunc original,bool allocation=false){
    uint32_t object=c.r[3],caller=c.lr;
    bool enabled=NATIVE_OPTION("MW_TRACE_OBJECTS")!=nullptr;
    if(enabled)std::fprintf(stderr,"[rin-native] %s begin this=%08x caller=%08x\n",stage,object,caller);
    original(c,m);
    if(enabled){
        uint32_t result=c.r[3];if(allocation)object=result;
        bool mapped=(object>=0x80000000&&object<0x81800000-0x100)||(object>=0x90000000&&object<0x94000000-0x100);
        std::fprintf(stderr,"[rin-native] %s returned=%08x object=%08x objid=%08x\n",stage,result,object,mapped?ld32(m,object+0xf4):0);
    }
}
PPC_FUNC(trace_rin_new){rin_trace("newWork",c,m,rin_original[0],true);}
PPC_FUNC(trace_rin_init){rin_trace("onInit",c,m,rin_original[1]);}
PPC_FUNC(trace_rin_obj){rin_trace("objCreate",c,m,rin_original[2]);}
// Explicit test-only contact fixture. Normal runs retain original AI/contact
// selection. Setup selects the original slalom once from nearby idle; contact
// calls the original endpoints only during the resulting eligible
// slalom window; it never writes actor health, positions, meter or flags.
void native_clash_contact_fixture(PPCContext& c,uint8_t* m,uint32_t rin){
    static bool used=false,attack_selected=false;
    const char* option=NATIVE_OPTION("MW_TEST_RIN_CLASH_CONTACT");
    if(used||!option||std::strcmp(option,"1")!=0)return;
    auto mapped=[](uint32_t p,uint32_t n){return (p>=0x80000000&&p<=0x81800000-n)||(p>=0x90000000&&p<=0x94000000-n);};
    if(!mapped(rin,0x10a4)||ld32(m,rin+0xf4)!=0x20440)return;
    if(NATIVE_OPTION("MW_TEST_RIN_CLASH_NEAR")){static unsigned beat=0;if(++beat%600==1){uint32_t fl=ld32(m,0x80697e64);
        std::fprintf(stderr,"CLASH_CONTACT_FIXTURE heartbeat nesting=%d flags@%08x=%08x rinState=%u\n",int32_t(ld32(m,0x80604020)),fl,mapped(fl,4)?ld32(m,fl):0xffffffffu,ld8(m,rin+0x11b));}}
    if(int32_t(ld32(m,0x80604020))!=0)return;
    uint32_t flags=ld32(m,0x80697e64);if(!mapped(flags,4)||ld32(m,flags)!=0)return;
    auto lookup=[&](uint32_t canonical){return mw_rso_lookup(mw_rso_runtime_pc(m,canonical));};
    PPCFunc get_player=lookup(0x81064e54),start_attack=lookup(0x810cb34c),start_catch=lookup(0x810cb3cc),change_state=lookup(0x810cb1b4);
    if(!get_player||!start_attack||!start_catch||!change_state)return;
    PPCContext call=c;call.r[3]=rin;call.r[4]=0;get_player(call,m);
    uint32_t player=call.r[3];
    if(!mapped(player,0x27e8)||ld32(m,player+0xf4)!=0x10000||ld8(m,player+0x12d8)!=0)return;
    if(int16_t(ld16(m,player+0xec))<=0||int16_t(ld16(m,rin+0xec))<=0)return;
    // MW_TEST_RIN_CLASH_NEAR=1 (test-only): with no pad driving him, he is put 1.2 m in front of her once, while she idles
    static unsigned idle_frames=0,perch_frames=0;static bool placed=false,under=false;
    // (she opens up on a scaffold and waits for him: he is put on the ground 3 m off its foot once, so she comes down)
    if(NATIVE_OPTION("MW_TEST_RIN_CLASH_NEAR")&&!under&&ld8(m,rin+0x11b)!=0&&++perch_frames>=300){
        under=true;
        stf32(m,player+0x64,ldf32(m,rin+0x64));stf32(m,player+0x6c,ldf32(m,rin+0x6c)+3.f);
        std::fprintf(stderr,"CLASH_CONTACT_FIXTURE player put under her perch (state %u, test-only)\n",ld8(m,rin+0x11b));
    }
    if(NATIVE_OPTION("MW_TEST_RIN_CLASH_NEAR")&&!placed&&ld8(m,rin+0x11b)==0&&++idle_frames>=60){
        placed=true;float f=ldf32(m,rin+0x74);
        stf32(m,player+0x64,ldf32(m,rin+0x64)+std::sin(f)*1.2f);stf32(m,player+0x68,ldf32(m,rin+0x68));stf32(m,player+0x6c,ldf32(m,rin+0x6c)+std::cos(f)*1.2f);
        std::fprintf(stderr,"CLASH_CONTACT_FIXTURE player placed 1.2 m in front of her (test-only)\n");
    }
    float dx=ldf32(m,player+0x64)-ldf32(m,rin+0x64),dy=ldf32(m,player+0x68)-ldf32(m,rin+0x68),dz=ldf32(m,player+0x6c)-ldf32(m,rin+0x6c);
    float distance=std::hypot(dx,dz);
    if(!std::isfinite(distance)||distance>1.6f||!std::isfinite(dy)||std::abs(dy)>0.5f)return;
    const uint8_t player_state=ld8(m,player+0x11b),rin_state=ld8(m,rin+0x11b);
    if(!attack_selected&&rin_state==0&&(player_state==0||player_state==4||player_state==39)){
        attack_selected=true;call=c;call.r[3]=rin;call.r[4]=1;call.r[5]=28;call.r[6]=0;call.r[7]=0;change_state(call,m);
        std::fprintf(stderr,"CLASH_CONTACT_FIXTURE attack-selection rin=%08x player=%08x distance=%.6g rinState=%u authority=synthetic-original-state-API\n",rin,player,distance,ld8(m,rin+0x11b));
        return;
    }
    if(rin_state!=28||!(ld32(m,rin+0x864)&1)||player_state!=39)return;
    used=true;
    std::fprintf(stderr,"CLASH_CONTACT_FIXTURE begin rin=%08x player=%08x distance=%.6g rinState=%u playerState=%u eligibility=%08x authority=synthetic-original-contact-API\n",rin,player,distance,ld8(m,rin+0x11b),ld8(m,player+0x11b),ld32(m,rin+0x864));
    call=c;call.r[3]=rin;call.r[4]=0x2a;call.r[5]=player;start_attack(call,m);
    bool accepted=ld32(m,player+0x1268)==rin&&ld8(m,player+0x12d8)!=0&&(ld32(m,player+0x1124)&0x01000000)!=0;
    if(accepted){call=c;call.r[3]=rin;call.r[4]=0x27;call.r[5]=player;start_catch(call,m);}
    std::fprintf(stderr,"CLASH_CONTACT_FIXTURE result accepted=%u rinState=%u pending=%u peer=%08x mode=%u authority=synthetic-original-contact-API\n",accepted,ld8(m,rin+0x11b),ld8(m,player+0x12d8),ld32(m,player+0x1268),ld16(m,player+0x27e4));
}
PPC_FUNC(trace_rin_move){
    static unsigned count=0;
    uint32_t object=c.r[3];
    native_snapshot_actor(0x20440,object);
    rin_seen=object;
    native_clash_contact_fixture(c,m,object);
    // Explicit test fixture only: once the intro demo nesting (0x80604020) has
    // unwound, set Rin's HP (cEnemyBase signed16 at +0xEC) to 1 so the next
    // original player hit runs the original boss-death path. Default off.
    static bool rin_hp_done=false;
    if(!rin_hp_done&&NATIVE_OPTION("MW_TEST_RIN_HP")&&int32_t(ld32(m,0x80604020))==0&&int32_t(lds16(m,object+0xec))>1){
        rin_hp_done=true;
        std::string want=NATIVE_OPTION("MW_TEST_RIN_HP");
        if(want!="1"&&want!="0"){std::fprintf(stderr,"[fixture] MW_TEST_RIN_HP supports only explicit 1 or 0; ignored\n");}
        else{
            std::fprintf(stderr,"[fixture] MW_TEST_RIN_HP: rin=%08x hp %d -> %s (test-only; original enemy update decides death)\n",object,int32_t(lds16(m,object+0xec)),want.c_str());
            st16(m,object+0xec,want=="1"?1:0);
        }
    }
    // Explicit test fixture only: put Rin into her original finish_wait state
    // (27, what her keen-competition win branch selects via changeState) once
    // the intro nesting has unwound, so the original finish/boss-death path can
    // be exercised without completing the full competition. Default off.
    static unsigned finish_wait_countdown=0;static bool finish_wait_done=false;
    if(!finish_wait_done&&NATIVE_OPTION("MW_TEST_RIN_FINISH_WAIT")&&int32_t(ld32(m,0x80604020))==0&&++finish_wait_countdown==300){
        finish_wait_done=true;
        if(std::string(NATIVE_OPTION("MW_TEST_RIN_FINISH_WAIT"))!="1"){std::fprintf(stderr,"[fixture] MW_TEST_RIN_FINISH_WAIT supports only explicit1; ignored\n");}
        else if(PPCFunc change_state=mw_rso_lookup(mw_rso_runtime_pc(m,0x810cb1b4))){
            PPCContext call=c;call.r[3]=object;call.r[4]=1;call.r[5]=0x1b;call.r[6]=0;call.r[7]=0;change_state(call,m);
            std::fprintf(stderr,"[fixture] MW_TEST_RIN_FINISH_WAIT: rin=%08x original changeState(1,27 finish_wait) issued; state now %u (test-only)\n",object,ld8(m,object+0x11b));
        }
    }
    rin_original[3](c,m);
    native_frame_record_rin(m,object);// game/frame_record.cpp, default off, read-only
    if(NATIVE_OPTION("MW_TRACE_OBJECTS")&&(++count<=4||count%60==0))std::fprintf(stderr,"[rin-native] onMove count=%u object=%08x objid=%08x position=(%.5g,%.5g,%.5g)\n",count,object,ld32(m,object+0xf4),ldf32(m,object+0x64),ldf32(m,object+0x68),ldf32(m,object+0x6c));
}
// Explicit test-only Elise clash fixture (MW_TEST_ELISE_CLASH=1, default off): once the intro has unwound and she
// has been in play for a while, her hide-damage accumulator (+0x1364) is set to the first threshold (csv[0] = 250) so
// her own idle / guard / damage branches send her into a crypt; when she sits in it (state 12, sub 5) the original
// startAtk (81102B14) is called on her with the player as the attacker, as his swing on the coffin does. From there
// everything is original. Each frame of the clash (her state 16) and 90 frames after, one row is printed:
// ELISE_CLASH_ROW {frame, her state/sub/pos/rot/motion frame, his state/pos/facing/motion frame} (read-only).
void native_elise_clash_fixture(PPCContext& c,uint8_t* m,uint32_t elise,bool after){
    static unsigned frames=0;static bool hid=false,struck=false;
    const char* option=NATIVE_OPTION("MW_TEST_ELISE_CLASH");
    if(!option||std::strcmp(option,"1")!=0)return;
    auto mapped=[](uint32_t p,uint32_t n){return (p>=0x80000000&&p<=0x81800000-n)||(p>=0x90000000&&p<=0x94000000-n);};
    static unsigned calls=0;
    if(!after&&(++calls%300)==1)std::fprintf(stderr,"ELISE_CLASH_FIXTURE onMove call=%u this=%08x objid=%08x nesting=%d event=%u/%u state=%u\n",calls,elise,mapped(elise,0x100)?ld32(m,elise+0xf4):0,int32_t(ld32(m,0x80604020)),ld32(m,0x80606cf8+4),ld32(m,0x80606cf8+0x114),mapped(elise,0x200)?ld8(m,elise+0x11b):0);
    if(!mapped(elise,0x1400)||ld32(m,elise+0xf4)!=0x20630)return;
    // (booted straight into c303 the room's entry demo never closes: the nesting stays at -1; her AI runs regardless)
    if(int32_t(ld32(m,0x80604020))<-1)return;
    auto lookup=[&](uint32_t canonical){return mw_rso_lookup(mw_rso_runtime_pc(m,canonical));};
    PPCFunc get_player=lookup(0x81064e54),start_attack=lookup(0x81102b14);
    if(!get_player||!start_attack)return;
    PPCContext call=c;call.r[3]=elise;call.r[4]=0;get_player(call,m);
    uint32_t player=call.r[3];
    if(!mapped(player,0x27e8))return;
    const uint8_t st=ld8(m,elise+0x11b),sub=ld8(m,elise+0x11c);
    if(after){elise_seen=elise;return;}
    ++frames;
    if(frames%300==1)std::fprintf(stderr,"ELISE_CLASH_FIXTURE heartbeat frame=%u state=%u sub=%u elise=(%.4g,%.4g,%.4g) player=(%.4g,%.4g,%.4g) playerState=%u\n",frames,st,sub,ldf32(m,elise+0x64),ldf32(m,elise+0x68),ldf32(m,elise+0x6c),ldf32(m,player+0x64),ldf32(m,player+0x68),ldf32(m,player+0x6c),ld8(m,player+0x11b));
    // (her start state waits for him within 12.5 m and nobody drives the pad here: he is put 8 m in front of her once)
    static bool placed=false;
    if(!placed&&frames==200&&st==0){placed=true;stf32(m,player+0x64,ldf32(m,elise+0x64));stf32(m,player+0x6c,ldf32(m,elise+0x6c)-8.f);std::fprintf(stderr,"ELISE_CLASH_FIXTURE player placed 8 m from her (test-only)\n");}
    if(!hid&&frames>=600&&(st==1||st==8||st==9)){
        hid=true;st32(m,elise+0x1364,250);
        std::fprintf(stderr,"ELISE_CLASH_FIXTURE hide-accumulator elise=%08x state=%u -> +0x1364=250 (test-only)\n",elise,st);
    }
    if(hid&&!struck&&st==12&&sub==5){
        struck=true;
        std::fprintf(stderr,"ELISE_CLASH_FIXTURE coffin elise=(%.6g,%.6g,%.6g) rotY=%.6g player=(%.6g,%.6g,%.6g)\n",ldf32(m,elise+0x64),ldf32(m,elise+0x68),ldf32(m,elise+0x6c),ldf32(m,elise+0x74),ldf32(m,player+0x64),ldf32(m,player+0x68),ldf32(m,player+0x6c));
        // MW_TEST_ELISE_CLASH_AT=x,y,z,yaw: her spot (a crypt's, from getCrypticPoint's table) for the strike (test-only)
        if(const char* at=NATIVE_OPTION("MW_TEST_ELISE_CLASH_AT")){
            float v[4]{};if(std::sscanf(at,"%f,%f,%f,%f",&v[0],&v[1],&v[2],&v[3])==4){
                stf32(m,elise+0x64,v[0]);stf32(m,elise+0x68,v[1]);stf32(m,elise+0x6c,v[2]);stf32(m,elise+0x74,v[3]);
                std::fprintf(stderr,"ELISE_CLASH_FIXTURE placed at (%g,%g,%g) yaw %g (test-only)\n",v[0],v[1],v[2],v[3]);
            }
        }
        call=c;call.r[3]=elise;call.r[4]=0x2a;call.r[5]=player;start_attack(call,m);
        std::fprintf(stderr,"ELISE_CLASH_FIXTURE startAtk issued state=%u peer=%08x mode=%u\n",ld8(m,elise+0x11b),ld32(m,player+0x1268),ld16(m,player+0x27e4));
    }
}
PPC_FUNC(trace_elise_move){
    uint32_t object=c.r[3];
    native_elise_clash_fixture(c,m,object,false);
    elise_original(c,m);
    native_elise_clash_fixture(c,m,object,true);
}
PPCFunc observe_rin(uint32_t address,PPCFunc original){
    if(address==0x81102560&&NATIVE_OPTION("MW_TEST_ELISE_CLASH")){
        if(!elise_original)std::fprintf(stderr,"ELISE_CLASH_FIXTURE onMove hooked\n");
        elise_original=original;return trace_elise_move;}
    const uint32_t addresses[]={0x81120be4,0x810c9cf0,0x810cdbd4,0x810cac70};
    const PPCFunc wrappers[]={trace_rin_new,trace_rin_init,trace_rin_obj,trace_rin_move};
    for(unsigned i=0;i<4;++i)if(address==addresses[i]){rin_original[i]=original;return wrappers[i];}
    return original;
}
}

// One row per player frame update while Elise is in her clash (state 16) and 90 frames after (game/frame_record.cpp
// calls it after cPl0000_frameUpdate): her pair is driven from the player's update, not her onMove.
void native_elise_clash_player_row(uint8_t* m,uint32_t player){
    static unsigned tail=0,row=0;
    const char* option=NATIVE_OPTION("MW_TEST_ELISE_CLASH");
    if(!option||std::strcmp(option,"1")!=0||!elise_seen)return;
    uint32_t elise=elise_seen;
    if(ld32(m,elise+0xf4)!=0x20630||(ld32(m,player+0xf4)!=0x10000&&ld32(m,player+0xf4)!=0x10030))return;
    const uint8_t st=ld8(m,elise+0x11b),sub=ld8(m,elise+0x11c);
    if(st==16)tail=90;
    if(st==16||tail){ if(st!=16)--tail;
        std::fprintf(stderr,"ELISE_CLASH_ROW {\"row\":%u,\"elise\":{\"state\":%u,\"sub\":%u,\"pos\":[%.6g,%.6g,%.6g],\"rot\":[%.6g,%.6g,%.6g],\"motion\":\"%08x\",\"frame\":%.6g},\"player\":{\"state\":%u,\"phase\":%u,\"pos\":[%.6g,%.6g,%.6g],\"facing\":%.6g,\"motion\":\"%08x\",\"frame\":%.6g,\"hp\":%d},\"cam\":{\"pos\":[%.6g,%.6g,%.6g],\"dest\":[%.6g,%.6g,%.6g]}}\n",
            row++,st,sub,ldf32(m,elise+0x64),ldf32(m,elise+0x68),ldf32(m,elise+0x6c),ldf32(m,elise+0x70),ldf32(m,elise+0x74),ldf32(m,elise+0x78),
            ld32(m,elise+0x1AC+0x54),ldf32(m,elise+0x1AC+0x58),
            ld8(m,player+0x11b),ld8(m,player+0x11c),ldf32(m,player+0x64),ldf32(m,player+0x68),ldf32(m,player+0x6c),ldf32(m,player+0x74),
            ld32(m,player+0x1AC+0x54),ldf32(m,player+0x1AC+0x58),int32_t(lds16(m,player+0xec)),
            ldf32(m,0x8069BD80u+0x324u*ld32(m,player+0x10F0)+0x1B0),ldf32(m,0x8069BD80u+0x324u*ld32(m,player+0x10F0)+0x1B4),ldf32(m,0x8069BD80u+0x324u*ld32(m,player+0x10F0)+0x1B8),
            ldf32(m,0x8069BD80u+0x324u*ld32(m,player+0x10F0)+0x1BC),ldf32(m,0x8069BD80u+0x324u*ld32(m,player+0x10F0)+0x1C0),ldf32(m,0x8069BD80u+0x324u*ld32(m,player+0x10F0)+0x1C4));
    }
}
// MW_RIN_CLASH_ROWS=1 (test-only, read-only): one row per player frame update while RinRin is in her clash (state 17)
// and 60 after: both actors and the shown camera (g_Camera 8069BD80 + 0x324 * player+0x10F0: pos +0x1B0, dest +0x1BC).
void native_rin_clash_player_row(uint8_t* m,uint32_t player){
    static unsigned tail=0,row=0;
    if(!NATIVE_OPTION("MW_RIN_CLASH_ROWS")||!rin_seen)return;
    uint32_t rin=rin_seen;
    if(ld32(m,rin+0xf4)!=0x20440||(ld32(m,player+0xf4)!=0x10000&&ld32(m,player+0xf4)!=0x10030))return;
    const uint8_t st=ld8(m,rin+0x11b),sub=ld8(m,rin+0x11c);
    if(st==17)tail=60;
    if(st!=17&&!tail)return;
    if(st!=17)--tail;
    uint32_t cam=0x8069BD80u+0x324u*ld32(m,player+0x10F0);
    std::fprintf(stderr,"RIN_CLASH_ROW {\"row\":%u,\"rin\":{\"state\":%u,\"sub\":%u,\"pos\":[%.6g,%.6g,%.6g],\"facing\":%.6g,\"motion\":\"%08x\",\"frame\":%.6g},\"player\":{\"state\":%u,\"pos\":[%.6g,%.6g,%.6g],\"facing\":%.6g,\"motion\":\"%08x\",\"frame\":%.6g},\"cam\":{\"pos\":[%.6g,%.6g,%.6g],\"dest\":[%.6g,%.6g,%.6g]}}\n",
        row++,st,sub,ldf32(m,rin+0x64),ldf32(m,rin+0x68),ldf32(m,rin+0x6c),ldf32(m,rin+0x74),ld32(m,rin+0x1AC+0x54),ldf32(m,rin+0x1AC+0x58),
        ld8(m,player+0x11b),ldf32(m,player+0x64),ldf32(m,player+0x68),ldf32(m,player+0x6c),ldf32(m,player+0x74),ld32(m,player+0x1AC+0x54),ldf32(m,player+0x1AC+0x58),
        ldf32(m,cam+0x1B0),ldf32(m,cam+0x1B4),ldf32(m,cam+0x1B8),ldf32(m,cam+0x1BC),ldf32(m,cam+0x1C0),ldf32(m,cam+0x1C4));
}
void mw_rso_attach(uint8_t* m) { memory = m; }
// Guest memory is g_mem; attach normally happens on the poll path, so fall back to it.
static uint8_t* attached() { return memory ? memory : g_mem; }
uint32_t mw_rso_current_base() { uint8_t* m = attached(); return m ? mw_rso_base(m) : 0; }
PPCFunc mw_rso_lookup(uint32_t actual) {
    if (!memory) memory = g_mem;
    if (!memory) return nullptr;
    uint32_t base = mw_rso_base(memory);
    if (!base || actual - base - MW_RSO_TEXT_OFF >= MW_RSO_TEXT_SIZE) return nullptr;
    uint32_t address = MW_RSO_BASE + actual - base;
    auto end = g_rso_funcs + g_rso_func_count;
    auto it = std::lower_bound(g_rso_funcs, end, address,
        [](const PPCFuncEntry& entry, uint32_t value) { return entry.addr < value; });
    return it != end && it->addr == address ? observe_rin(address,it->fn) : nullptr;
}
const char* mw_rso_func_name(uint32_t address, uint32_t* start) {
    if (address - MW_RSO_BASE - MW_RSO_TEXT_OFF >= MW_RSO_TEXT_SIZE) {
        if (!memory) return nullptr;
        address = mw_rso_canonical_pc(memory, address);
    }
    auto it = std::upper_bound(g_rso_func_names, g_rso_func_names + g_rso_func_name_count, address,
        [](uint32_t value, const PPCFuncName& entry) { return value < entry.addr; });
    if (it == g_rso_func_names) return nullptr;
    --it;
    if (address >= it->end) return nullptr;
    if (start) *start = it->addr;
    return it->name;
}
