#include "hle.h"
#include "native_config.h"
#include "snapshot.h"
#include <cstdio>
#include <cstring>
#include <set>
#include <vector>
// Frame-atomic fight recorder and pad replay (default off; research/FRAME-COMPARE.md).
// MW_FRAME_RECORD=<path.jsonl>: one JSON row per game frame, i.e. per window between two
// calls of the frame wait 8026F324 (frame_wait.cpp), once the c201 intro demo nesting
// (0x80604020) has unwound and both actors exist. Inside a window the player's
// frameUpdate (802727B0, a virtual: wrapped through ppc_lookup) and Rin's onMove
// (810CAC70, wrapped in rso.cpp) are read after they finish, and every KPADReadEx
// sample (input_hle.cpp) is kept, so all fields of a row belong to one game frame.
// Read-only unless the explicit MW_FRAME_REPLAY_RESEED fixture is enabled.
// MW_INPUT_REPLAY=<recording.jsonl>: KPADReadEx answers with the recorded samples of the
// same frame and read order instead of the PC pad. The recorded frame is found by the cRand word at arming
// (replayBase in the header), so a replay from a snapshot taken after the recording armed lines up.
namespace {
struct Pad {uint32_t hold;float acc[3],fs[3],stick[2];};
struct Actor {bool fresh=false;uint8_t state[4]{};uint32_t motion=0;float frame=0,step=0,pos[3]{},facing=0;int16_t hp=0;};
struct Input {uint32_t hold=0,trig=0;float acc_x=0,acc_y=0,stick[2]{},stick_yaw=0,f64=0,m1088=0,m108c=0,m1090=0,m109c=0;};
struct Recorder {
    FILE* out=nullptr;bool armed=false;uint32_t frame=0;
    Actor player,rin;Input input;std::vector<Pad> pending;std::set<uint32_t> headers;
} g;
struct Replay {bool loaded=false;std::vector<std::vector<Pad>> frames;std::vector<uint32_t> seeds;uint32_t read=0,live=0,base=0,start_seed=0;} r;
bool mapped(uint32_t p,uint32_t n){return (p>=0x80000000u&&p<=0x81800000u-n)||(p>=0x90000000u&&p<=0x94000000u-n);}
const char* record_path(){return NATIVE_OPTION("MW_FRAME_RECORD");}
const char* replay_path(){return NATIVE_OPTION("MW_INPUT_REPLAY");}
uint32_t rin_object=0;
void read_actor(uint8_t* m,uint32_t a,Actor& out){
    out.fresh=true;
    for(int i=0;i<4;i++)out.state[i]=ld8(m,a+0x11A+i);
    out.motion=ld32(m,a+0x1AC+0x54);out.frame=ldf32(m,a+0x1AC+0x58);out.step=ldf32(m,a+0x1AC+0x68);
    for(int i=0;i<3;i++)out.pos[i]=ldf32(m,a+0x64+4*i);
    out.facing=ldf32(m,a+0x74);out.hp=int16_t(ld16(m,a+0xEC));
}
// First sight of a motion resource: its 64-byte header, matched offline against
// research/native-motion-live-index.json (as read_native_core.py does).
void header(uint8_t* m,uint32_t pointer){
    if(!pointer||!mapped(pointer,64)||!g.headers.insert(pointer).second)return;
    std::fprintf(g.out,"{\"motionHeader\":\"%08x\",\"bytes\":\"",pointer);
    for(uint32_t i=0;i<64;i++)std::fprintf(g.out,"%02x",ld8(m,pointer+i));
    std::fprintf(g.out,"\"}\n");
}
void actor_json(const char* name,const Actor& a){
    std::fprintf(g.out,",\"%s\":{\"fresh\":%d,\"state\":[%u,%u,%u,%u],\"motion\":\"%08x\",\"frame\":%.9g,\"step\":%.9g,\"hp\":%d,\"pos\":[%.9g,%.9g,%.9g],\"facing\":%.9g}",
        name,a.fresh?1:0,a.state[0],a.state[1],a.state[2],a.state[3],a.motion,a.frame,a.step,a.hp,a.pos[0],a.pos[1],a.pos[2],a.facing);
}
// Optional, bounded post-update proxy evidence; no guest writes or extra guest calls.
void proxy_json(uint8_t* m){
    if(!NATIVE_OPTION("MW_FRAME_PROXY"))return;
    uint32_t player=native_snapshot_player();
    if(!mapped(player,0x394))return;
    uint32_t wrapper=ld32(m,player+0x390);
    if(!mapped(wrapper,0x20))return;
    uint32_t proxy=ld32(m,wrapper+4);
    if(!mapped(proxy,0xc0))return;
    std::fprintf(g.out,",\"proxy\":{\"height\":%.9g,\"radius\":%.9g,\"velocity\":[%.9g,%.9g,%.9g],\"keep\":%.9g,\"manifold\":[",
        ldf32(m,wrapper+0x14),ldf32(m,wrapper+0x18),ldf32(m,proxy+0x20),ldf32(m,proxy+0x24),ldf32(m,proxy+0x28),ldf32(m,proxy+0x58));
    uint32_t points=ld32(m,proxy+0x74),count=ld32(m,proxy+0x78);
    if(count<=8&&mapped(points,count*0x30))for(uint32_t j=0;j<count;j++){
        uint32_t p=points+j*0x30;
        std::fprintf(g.out,"%s{\"position\":[%.9g,%.9g,%.9g],\"normal\":[%.9g,%.9g,%.9g],\"distance\":%.9g,\"roots\":[\"%08x\",\"%08x\"]}",j?",":"",
            ldf32(m,p),ldf32(m,p+4),ldf32(m,p+8),ldf32(m,p+0x10),ldf32(m,p+0x14),ldf32(m,p+0x18),ldf32(m,p+0x1c),ld32(m,p+0x20),ld32(m,p+0x28));
    }
    std::fprintf(g.out,"],\"flags\":[\"%08x\",\"%08x\"],\"field398\":%u,\"knockTimer\":%.9g,\"physicsStep\":%.9g,\"inversePhysicsStep\":%.9g,\"sceneSpeed\":%.9g}",ld32(m,player+0x1124),ld32(m,player+0x1128),ld32(m,player+0x398),ldf32(m,player+0x12bc),ldf32(m,0x80609a04),ldf32(m,0x80609a08),ldf32(m,0x8069bcc0));
}
void contact_json(uint8_t* m){
    if(!NATIVE_OPTION("MW_FRAME_CONTACTS"))return;
    uint32_t p=native_snapshot_player();
    if(!mapped(p,0x1562)||!mapped(rin_object,0x10a0))return;
    std::fprintf(g.out,",\"contacts\":{\"invuln\":%.9g,\"damageGuard\":%.9g,\"hitMode\":%u,\"hitRate\":%.9g,\"hitFrames\":%u,\"stun\":%u,\"parts\":[",
        ldf32(m,p+0x1158),ldf32(m,p+0x115c),ld32(m,p+0x1218),ldf32(m,p+0x121c),ld32(m,p+0x1220),ld16(m,p+0x1560));
    uint32_t part=ld32(m,p+0xac);bool comma=false;
    for(unsigned j=0;j<ld32(m,p+0xd8)&&j<256&&mapped(part,0xd0);j++,part=ld32(m,part+0xac)){
        if(j!=0x18&&j!=2&&j!=0xf&&j!=8&&j!=0xd&&j!=0x11&&j!=0x15)continue;
        std::fprintf(g.out,"%s{\"index\":%u,\"bone\":%u,\"world\":[%.9g,%.9g,%.9g]}",comma?",":"",j,ld8(m,part+0xcc),ldf32(m,part+0x10),ldf32(m,part+0x20),ldf32(m,part+0x30));comma=true;
    }
    std::fprintf(g.out,"],\"fans\":[");
    for(unsigned j=0;j<2;j++){
        uint32_t f=rin_object+0x1028+j*0x3c,part=ld32(m,f+4);
        if(!mapped(part,0xd0))break;
        std::fprintf(g.out,"%s{\"phase\":%u,\"t\":%.9g,\"dt\":%.9g,\"world\":[%.9g,%.9g,%.9g],\"local\":[%.9g,%.9g,%.9g],\"target\":[%.9g,%.9g,%.9g]",j?",":"",ld8(m,f+0x39),ldf32(m,f+0x30),ldf32(m,f+0x34),ldf32(m,part+0x10),ldf32(m,part+0x20),ldf32(m,part+0x30),ldf32(m,part+0x64),ldf32(m,part+0x68),ldf32(m,part+0x6c),ldf32(m,f+0xc),ldf32(m,f+0x10),ldf32(m,f+0x14));
        uint32_t hand=ld32(m,f+8);
        if(mapped(hand,0x34)){
            std::fprintf(g.out,",\"handMatrix\":[");
            for(unsigned k=0;k<12;k++)std::fprintf(g.out,"%s%.9g",k?",":"",ldf32(m,hand+4+4*k));
            std::fprintf(g.out,"]");
        }
        std::fprintf(g.out,"}");
    }
    std::fprintf(g.out,"],\"playerWorld\":[%.9g,%.9g,%.9g]}",ldf32(m,p+0x10),ldf32(m,p+0x20),ldf32(m,p+0x30));
}
void emit(uint8_t* m){
    header(m,g.player.motion);header(m,g.rin.motion);
    const Input& i=g.input;
    std::fprintf(g.out,"{\"frame\":%u,\"seed\":\"%08x\",\"demo\":%d",g.frame,ld32(m,0x80609B90),int32_t(ld32(m,0x80604020)));
    actor_json("player",g.player);actor_json("rin",g.rin);
    proxy_json(m);
    contact_json(m);
    std::fprintf(g.out,",\"input\":{\"hold\":\"%08x\",\"trig\":\"%08x\",\"accX\":%.9g,\"accY\":%.9g,\"stick\":[%.9g,%.9g],\"stickYaw\":%.9g,\"f64\":%.9g,\"m1088\":%.9g,\"m108c\":%.9g,\"m1090\":%.9g,\"m109c\":%.9g}",
        i.hold,i.trig,i.acc_x,i.acc_y,i.stick[0],i.stick[1],i.stick_yaw,i.f64,i.m1088,i.m108c,i.m1090,i.m109c);
    std::fprintf(g.out,",\"kpad\":[");
    for(size_t k=0;k<g.pending.size();k++){const Pad& p=g.pending[k];
        std::fprintf(g.out,"%s[\"%08x\",%.9g,%.9g,%.9g,%.9g,%.9g,%.9g,%.9g,%.9g]",k?",":"",p.hold,p.acc[0],p.acc[1],p.acc[2],p.fs[0],p.fs[1],p.fs[2],p.stick[0],p.stick[1]);}
    std::fprintf(g.out,"]}\n");std::fflush(g.out);
}
void load_replay(){
    r.loaded=true;
    FILE* f=std::fopen(replay_path(),"r");
    if(!f){std::fprintf(stderr,"[frame-replay] cannot open %s; live pad\n",replay_path());return;}
    static char line[1<<16];
    while(std::fgets(line,sizeof line,f)){
        if(const char* seed=std::strstr(line,"\"seedAtStart\":\""))std::sscanf(seed+15,"%x",&r.start_seed);
        const char* k=std::strstr(line,"\"kpad\":[");if(!k||std::strncmp(line,"{\"frame\":",9))continue;
        std::vector<Pad> reads;const char* p=k+8;
        while(*p=='['||*p==','){
            if(*p==',')p++;
            Pad pad{};int used=0;
            if(std::sscanf(p,"[\"%x\",%g,%g,%g,%g,%g,%g,%g,%g]%n",&pad.hold,&pad.acc[0],&pad.acc[1],&pad.acc[2],&pad.fs[0],&pad.fs[1],&pad.fs[2],&pad.stick[0],&pad.stick[1],&used)!=9)break;
            reads.push_back(pad);p+=used;
        }
        r.frames.push_back(reads);
        uint32_t seed=0;if(const char* s=std::strstr(line,"\"seed\":\""))std::sscanf(s+8,"%x",&seed);
        r.seeds.push_back(seed);
    }
    std::fclose(f);
    std::fprintf(stderr,"[frame-replay] %zu frames from %s\n",r.frames.size(),replay_path());
}
PPCFunc frame_update_original=nullptr;
}
void native_elise_clash_player_row(uint8_t* m,uint32_t player);// cpu/rso.cpp, MW_TEST_ELISE_CLASH only
void native_rin_clash_player_row(uint8_t* m,uint32_t player);// cpu/rso.cpp, MW_RIN_CLASH_ROWS only
namespace {
PPC_FUNC(record_frame_update){
    uint32_t player=c.r[3];
    frame_update_original(c,m);
    native_elise_clash_player_row(m,player);
    native_rin_clash_player_row(m,player);
    if(!g.armed||player!=native_snapshot_player()||!mapped(player,0x27E8))return;
    read_actor(m,player,g.player);
    Input& i=g.input;
    i.hold=ld32(m,player+0xEE0);i.trig=ld32(m,player+0xEE4);i.acc_x=ldf32(m,player+0xEF0);i.acc_y=ldf32(m,player+0xEF4);
    i.stick[0]=ldf32(m,player+0xF1C);i.stick[1]=ldf32(m,player+0xF20);i.stick_yaw=ldf32(m,player+0x113C);i.f64=ldf32(m,player+0xF64);
    i.m1088=ldf32(m,player+0x1088);i.m108c=ldf32(m,player+0x108C);i.m1090=ldf32(m,player+0x1090);i.m109c=ldf32(m,player+0x109C);
}
}
// The frame wait 8026F324 closes one game frame and opens the next.
void native_frame_record_tick(uint8_t* m){
    if(!record_path()&&!replay_path())return;
    if(!g.armed){
        uint32_t player=native_snapshot_player();
        if(int32_t(ld32(m,0x80604020))!=0||!mapped(player,0x27E8)||!mapped(rin_object,0x214)||ld32(m,rin_object+0xF4)!=0x20440)return;
        g.armed=true;g.pending.clear();
        // A replay may start later than its recording (a snapshot taken after the recording armed): the recorded
        // frame whose closing cRand word equals the current one is the frame just before this one.
        if(replay_path()){
            if(!r.loaded)load_replay();
            for(size_t k=0;k<r.seeds.size();k++)if(r.seeds[k]==ld32(m,0x80609B90)){r.base=uint32_t(k+1);break;}
            std::fprintf(stderr,"[frame-replay] base frame %u (cRand %08x)\n",r.base,ld32(m,0x80609B90));
            // Explicit diagnostic fixture: align random state as well as pad input.
            // This writes only cRand; it does not claim a natural, snapshot-exact replay.
            if(NATIVE_OPTION("MW_FRAME_REPLAY_RESEED"))st32(m,0x80609B90,r.base?r.seeds[r.base-1]:r.start_seed);
        }
        if(const char* path=record_path()){
            g.out=std::fopen(path,"w");
            if(!g.out)std::fprintf(stderr,"[frame-record] cannot open %s\n",path);
            else std::fprintf(g.out,"{\"recording\":\"mw-frame-record/1\",\"player\":\"%08x\",\"rin\":\"%08x\",\"seedAtStart\":\"%08x\",\"replay\":%s,\"replayBase\":%u,\"restored\":%d,\"randomStateFixture\":%s}\n",
                player,rin_object,ld32(m,0x80609B90),replay_path()?"true":"false",r.base,native_snapshot_restored()?1:0,replay_path()&&NATIVE_OPTION("MW_FRAME_REPLAY_RESEED")?"true":"false");
        }
        std::fprintf(stderr,"[frame-record] armed player=%08x rin=%08x seed=%08x record=%s replay=%s\n",player,rin_object,ld32(m,0x80609B90),
            record_path()?record_path():"-",replay_path()?replay_path():"-");
        return;
    }
    // A window without a player update (pause) adds no row; its pad reads stay pending.
    if(g.player.fresh){
        if(g.out)emit(m);
        g.frame++;r.read=0;g.pending.clear();g.player.fresh=g.rin.fresh=false;
        if(replay_path()&&NATIVE_OPTION("MW_FRAME_REPLAY_RESEED")&&g.frame+r.base<=r.seeds.size())st32(m,0x80609B90,r.seeds[g.frame+r.base-1]);
    }
}
PPCFunc native_frame_record_observe(uint32_t address,PPCFunc fn){
    if(address!=0x802727B0u||!(record_path()||replay_path()||NATIVE_OPTION("MW_TEST_ELISE_CLASH")||NATIVE_OPTION("MW_RIN_CLASH_ROWS")))return fn;
    frame_update_original=fn;return record_frame_update;
}
uint32_t native_frame_record_rin_object(){return rin_object;}
void native_frame_record_rin(uint8_t* m,uint32_t rin){
    if(!record_path()&&!replay_path())return;
    rin_object=rin;
    if(g.armed&&mapped(rin,0x214))read_actor(m,rin,g.rin);
}
void native_frame_record_pad(uint32_t hold,const float* acc,const float* fs,const float* stick){
    if(!g.armed||!record_path()||g.pending.size()>=16)return;
    g.pending.push_back(Pad{hold,{acc[0],acc[1],acc[2]},{fs[0],fs[1],fs[2]},{stick[0],stick[1]}});
}
// The recorded sample for this read, or false (live pad) before arming / past the end.
bool native_frame_replay_pad(uint32_t& hold,float* acc,float* fs,float* stick){
    if(!replay_path())return false;
    if(!r.loaded)load_replay();
    uint32_t at=g.frame+r.base;
    if(!g.armed||at>=r.frames.size()||r.frames[at].empty()){if(g.armed&&r.live++==0)std::fprintf(stderr,"[frame-replay] live pad from frame %u\n",at);return false;}
    const auto& reads=r.frames[at];
    const Pad& p=reads[r.read<reads.size()?r.read:reads.size()-1];r.read++;
    hold=p.hold;for(int i=0;i<3;i++){acc[i]=p.acc[i];fs[i]=p.fs[i];}stick[0]=p.stick[0];stick[1]=p.stick[1];
    return true;
}
