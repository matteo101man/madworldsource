"""Read-only bounded diagnostics for this task's own native-core process.
No writes, injection, UI control, or other-process support. Snapshot is not frame-atomic.
"""
import argparse,ctypes as C,hashlib,json,re,struct
from ctypes import wintypes as W
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
class MBI(C.Structure):
    _fields_=[('base',C.c_void_p),('allocation',C.c_void_p),('allocation_protect',W.DWORD),('partition',W.WORD),('padding',W.WORD),('size',C.c_size_t),('state',W.DWORD),('protect',W.DWORD),('kind',W.DWORD)]
def main():
    ap=argparse.ArgumentParser();ap.add_argument('pid',type=int);ap.add_argument('--label',default='native-live-state');ap.add_argument('--trace-log',type=Path);ap.add_argument('--pc-ui-memory',action='store_true',help='Hash-check staged PC atlas pixels in bounded native RAM');ap.add_argument('--bevy-host',action='store_true',help='Allow only this task compiled Bevy native frontend/test binary');ap.add_argument('--ui-widgets',action='store_true',help='Read bounded active original UI widget records and transforms');a=ap.parse_args()
    assert a.label.replace('-','').isalnum()
    k=C.WinDLL('kernel32',use_last_error=True)
    k.OpenProcess.argtypes=[W.DWORD,W.BOOL,W.DWORD];k.OpenProcess.restype=W.HANDLE
    k.QueryFullProcessImageNameW.argtypes=[W.HANDLE,W.DWORD,W.LPWSTR,C.POINTER(W.DWORD)]
    k.VirtualQueryEx.argtypes=[W.HANDLE,C.c_void_p,C.POINTER(MBI),C.c_size_t];k.VirtualQueryEx.restype=C.c_size_t
    k.ReadProcessMemory.argtypes=[W.HANDLE,C.c_void_p,C.c_void_p,C.c_size_t,C.POINTER(C.c_size_t)]
    k.CloseHandle.argtypes=[W.HANDLE]
    handle=k.OpenProcess(0x410,False,a.pid)
    if not handle:raise OSError(C.get_last_error(),'Cannot open owned native process read-only')
    try:
        image=C.create_unicode_buffer(32768);length=W.DWORD(len(image))
        if not k.QueryFullProcessImageNameW(handle,0,image,C.byref(length)):raise OSError(C.get_last_error(),'Process path')
        expected=ROOT/'tools/upstream-native-core/port/build/madworld.exe'
        actual=Path(image.value).resolve()
        allowed=actual==expected.resolve()
        if a.bevy_host:
            debug=Path('D:/GDecomp/build-cache/bevy-0.20/debug').resolve()
            allowed=allowed or actual==debug/'madworld-native.exe' or (actual.parent==debug/'deps' and re.fullmatch(r'madworld_native-[0-9a-f]{16}\.exe',actual.name) is not None)
        if not allowed:raise ValueError('Refusing a process outside this task native-core executable/explicit Bevy host')
        regions={};at=0
        while at<0x7fffffffffff:
            m=MBI()
            if not k.VirtualQueryEx(handle,at,C.byref(m),C.sizeof(m)):break
            if not m.size:break
            regions[m.base or 0]=(m.size,m.state,m.kind)
            at=(m.base or 0)+m.size
        def read_host(address,n):
            out=C.create_string_buffer(n);done=C.c_size_t()
            if not k.ReadProcessMemory(handle,address,out,n,C.byref(done)) or done.value!=n:raise OSError(C.get_last_error(),'Owned native read failed')
            return out.raw
        candidates=[base-0x80000000 for base,(size,state,kind) in regions.items() if size==0x1800000 and state==0x1000 and kind==0x40000 and regions.get(base+0x40000000)==(size,state,kind)]
        # Verify the native mapped code is the owned original DOL, not a heap.
        import sys
        sys.path.insert(0,str(ROOT/'tools/upstream-native-core/port/recomp'))
        from dol import Dol
        dol=Dol(Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/sys/main.dol'))
        expected_word=dol.read32(0x800064d8).to_bytes(4,'big')
        candidates=[base for base in candidates if read_host(base+0x800064d8,4)==expected_word]
        if len(candidates)!=1:raise ValueError('Native RAM mapping is ambiguous')
        base=candidates[0]
        def u32(address):return int.from_bytes(read_host(base+address,4),'big')
        def u16(address):return int.from_bytes(read_host(base+address,2),'big')
        def mapped(address,n):return 0x80000000<=address<=0x81800000-n or 0x90000000<=address<=0x94000000-n
        obj=0x80695434;data=u32(obj+8);generators=u32(obj+0x34)
        result=dict(pid=a.pid,executable=str(actual),scope='Read-only bounded fields of this task native core; concurrent diagnostic, not frame-atomic',demoCounter=C.c_int32(u32(0x80604020)).value,enemySet={hex(offset):hex(u32(obj+offset)) for offset in (4,8,0x14,0x18,0x1c,0x24,0x28,0x2c,0x30,0x34)})
        if mapped(data,32):result['enemyHeader']=dict(type=u16(data+4),count=u16(data+6),field18=hex(u32(data+0x18)))
        if mapped(data,32) and mapped(generators,min(u16(data+6),3)*20):
            result['generators']=[]
            for i in range(min(u16(data+6),3)):
                fields={hex(n):hex(u32(generators+i*20+n)) for n in range(0,20,4)};record=u32(generators+i*20)
                if mapped(record,80):fields['sourceRecordId']=hex(u32(record));fields['sourceRecordWord04']=hex(u32(record+4))
                result['generators'].append(fields)
        rso_base=u32(0x8060bcac)
        result['enemyRsoBase']=hex(rso_base)
        result['enemyFactoryObjectFields']={hex(slot):hex(u32(0x80694aa0+slot)) for slot in range(0,32,4)}
        result['enemyFactoryVtable']={}
        vtable=u32(0x80694aa0)
        import bisect
        symbols=[(int(row.split('\t')[0],16),int(row.split('\t')[1],16),row.split('\t')[2]) for row in (ROOT/'tools/ghidra-functions.tsv').read_text(encoding='utf-8').splitlines()]
        starts=[row[0] for row in symbols]
        for slot in range(0,32,4):
            value=u32(vtable+slot)
            canonical=0x81000000+value-rso_base if rso_base+248<=value<rso_base+248+1188540 else value
            index=bisect.bisect_right(starts,canonical)-1
            name=symbols[index][2] if index>=0 and canonical<symbols[index][0]+symbols[index][1] else 'unknown'
            result['enemyFactoryVtable'][hex(slot)]={'value':hex(value),'canonical':hex(canonical),'name':name}
        if a.trace_log is not None:
            if a.trace_log.resolve().parent!=(ROOT/'evidence').resolve():raise ValueError('Trace must be in this task evidence directory')
            trace=a.trace_log.read_text(encoding='utf-8')
            idle_threads=sorted({int(x,16) for x in re.findall(r'\[idle-thread\] address=([0-9a-f]+)',trace)})[:32]
            result['threads']=[]
            for thread in idle_threads:
                if not mapped(thread,0x318):continue
                queue=u32(thread+0x2dc)
                row={'address':hex(thread),'state':u16(thread+0x2c8),'priority':u32(thread+0x2d0),'waitQueue':hex(queue),'waitingMutex':hex(u32(thread+0x2f0)),'ownedMutexHead':hex(u32(thread+0x2f4)),'ownedMutexTail':hex(u32(thread+0x2f8))}
                if mapped(queue,16):
                    row['queueWords']=[hex(u32(queue+i*4)) for i in range(4)]
                    owner=u32(queue+8)
                    row['queueOwnerMatchesObservedThread']=owner in idle_threads
                result['threads'].append(row)
            patterns={'rin':r'\[rin-native\] onInit returned=00000001 object=([0-9a-f]+) objid=00020440','player':r'kind=actor request=00010000 variant=00010030 result=([0-9a-f]+)'}
            result['actors']={}
            for role,pattern in patterns.items():
                hits=re.findall(pattern,trace)
                if not hits:hits=re.findall(r'\[snapshot-actors\].*\b'+role+r'=([0-9a-f]+)',trace)
                if not hits:continue
                actor=int(hits[-1],16)
                if not mapped(actor,0x11e):continue
                identity=u32(actor+0xf4)
                if (role=='rin' and identity!=0x20440) or (role=='player' and identity not in (0x10000,0x10030)):continue
                position=[struct.unpack('>f',read_host(base+actor+0x64+i*4,4))[0] for i in range(3)]
                result['actors'][role]={'object':hex(actor),'id':hex(identity),'hp':C.c_int16(u16(actor+0xec)).value,'maxHp':C.c_int16(u16(actor+0xee)).value,'position':position,'stateBytes11a11d':[read_host(base+actor+offset,1)[0] for offset in range(0x11a,0x11e)],'healthLayoutAuthority':'Native cEnemyBase::isDeadHp and cObj::getHpPercentage load signed16 atEC/EE'}
        result['nativeGestureSlots']={hex(address):struct.unpack('>f',read_host(base+address,4))[0] for address in range(0x80698b48,0x80698b60,4)}
        index=ROOT/'research/native-motion-live-index.json'
        if not index.exists():index=ROOT/'research/player-motion-live-index.json'
        motion_index=json.loads(index.read_text(encoding='utf-8'))['headers'] if index.exists() else {}
        def native_motion(actor):
            if not mapped(actor,0x214):return None
            motion=actor+0x1ac;pointer=u32(motion+0x54)
            state={'pointer':hex(pointer),'frame':struct.unpack('>f',read_host(base+motion+0x58,4))[0],'previousFrame':struct.unpack('>f',read_host(base+motion+0x5c,4))[0],'scalar60':struct.unpack('>f',read_host(base+motion+0x60,4))[0],'scalar64':struct.unpack('>f',read_host(base+motion+0x64,4))[0],'step68':struct.unpack('>f',read_host(base+motion+0x68,4))[0],'authority':'cByMotion set/clear/setFrame layout; player frameUpdate and cEnemyBase setMotion/updateMotion use actor+1AC'}
            if mapped(pointer,64):
                header=hashlib.sha256(read_host(base+pointer,64)).hexdigest()
                state['headerSha256']=header;state['resourceMatches']=[]
                for resource in motion_index.get(header,[]):
                    size=resource['size']
                    if size<=4*1024*1024 and mapped(pointer,size) and hashlib.sha256(read_host(base+pointer,size)).hexdigest()==resource['sha256']:
                        state['resourceMatches'].append(resource)
                state['resourceAuthority']='Full live resource content SHA256 equals owned DAT catalog member; selection observation, not an action-name or temporal-equivalence claim'
            return state
        if 'player' in result.get('actors',{}):
            player=int(result['actors']['player']['object'],16)
            if mapped(player,0x10a0):result['playerMotionMetrics']={hex(offset):struct.unpack('>f',read_host(base+player+offset,4))[0] for offset in (0xf60,0xf64,0xf68,0xf6c,0x1088,0x108c,0x1090,0x1094,0x1098,0x109c)}
            state=native_motion(player)
            if state is not None:
                if mapped(player,0x1258):state.update(lockOnState=read_host(base+player+0x1250,1)[0],lockOnTarget=hex(u32(player+0x1254)))
                result['playerNativeMotion']=state
            if mapped(player,0x1a95):result['playerAttackFlagBytes']={hex(offset):read_host(base+player+offset,1)[0] for offset in (0x1280,0x1281,0x1a94)}
            result['playerAttackFlagsAuthority']='Original isNowAttackFrm loads these three bytes; raw concurrent observation, not hit confirmation'
            if mapped(player,0x11cc):
                cockpit=u32(player+0x11c8)
                if mapped(cockpit,0x2c):
                    result['nativeSwingNavigation']={}
                    for role,offset in (('remote',0x24),('nunchuk',0x28)):
                        child=u32(cockpit+offset)
                        if mapped(child,0x1c):result['nativeSwingNavigation'][role]={'object':hex(child),'stateByte0':read_host(base+child,1)[0],'kindByte18':read_host(base+child+0x18,1)[0],'styleByte19':read_host(base+child+0x19,1)[0],'variantByte1b':read_host(base+child+0x1b,1)[0]}
            if mapped(player,0x27e8):
                peer=u32(player+0x1268)
                result['playerCompetition']={'stateByte11b':read_host(base+player+0x11b,1)[0],'phaseByte11c':read_host(base+player+0x11c,1)[0],'rawWord1230':u32(player+0x1230),'rawWord1234':u32(player+0x1234),'rawWord1238':u32(player+0x1238),'rawWord123c':u32(player+0x123c),'flags1124':hex(u32(player+0x1124)),'peerObject':hex(peer),'activeByte12d8':read_host(base+player+0x12d8,1)[0],'float1240':struct.unpack('>f',read_host(base+player+0x1240,4))[0],'float1248':struct.unpack('>f',read_host(base+player+0x1248,4))[0],'float124c':struct.unpack('>f',read_host(base+player+0x124c,4))[0],'float27dc':struct.unpack('>f',read_host(base+player+0x27dc,4))[0],'modeWord27e4':u16(player+0x27e4),'word27e6':u16(player+0x27e6),'peerMatchesRin':bool(result['actors'].get('rin')) and peer==int(result['actors']['rin']['object'],16),'authority':'Original startKeenCompetition/isKeenCompetition field loads/stores; raw fields, not a completed competition or gesture-equivalence claim'}
        if 'rin' in result.get('actors',{}):
            rin=int(result['actors']['rin']['object'],16)
            state=native_motion(rin)
            if state is not None:result['rinNativeMotion']=state
            if mapped(rin,0x868):result['rinCompetitionEligibility']={'rawWord864':hex(u32(rin+0x864)),'enabledBit0':bool(u32(rin+0x864)&1),'authority':'Original chkEnableKeenCompetition calls cFlag::ck at Rin+864 with index0; ck returns (word & (1<<index)) !=0'}
            if mapped(rin,0xa2c):
                source=u32(rin+0xa28)
                result['rinHitReservation']={'pendingByte':read_host(base+rin+0x9cd,1)[0],'typeWord':hex(u32(rin+0x9dc)),'word9d0':hex(u32(rin+0x9d0)),'word9e0':hex(u32(rin+0x9e0)),'ownerWord':hex(u32(rin+0xa00)),'sourceObject':hex(source),'sourceMatchesPlayer':bool(result['actors'].get('player')) and source==int(result['actors']['player']['object'],16),'authority':'Original cEnemyBase::reserveAtkHit stores pending byte9CD, type9DC, sourceA28 and words9D0/9E0/A00; concurrent observations do not prove contact or health changes'}
        if a.pc_ui_memory:
            manifest=json.loads((ROOT/'staged/pc-ui/manifest.json').read_text(encoding='utf-8'))
            ui=Path(manifest['output']).read_bytes()
            if hashlib.sha256(ui).hexdigest()!=manifest['outputSha256']:raise ValueError('Staged PC UI digest mismatch')
            pages=[]
            for page in manifest['pages']:
                pixels=ui[page['offset']:page['offset']+page['size']]
                sprite=next(s for s in manifest['sprites'] if s['page']==page['id'])
                x,y,_,_=sprite['rect'];signature_offset=((y//8)*128+x//8)*32
                pages.append(dict(page=page['id'],pixels=pixels,offset=signature_offset,signature=pixels[signature_offset:signature_offset+256],sha256=page['sha256'],candidates=[],matches=[]))
            for region,length in ((0x80000000,0x1800000),(0x90000000,0x4000000)):
                memory=read_host(base+region,length)
                for page in pages:
                    cursor=0
                    while True:
                        found=memory.find(page['signature'],cursor)
                        if found<0:break
                        cursor=found+1;image=found-page['offset'];size=len(page['pixels'])
                        if image<0 or image+size>length:continue
                        page['candidates'].append(hex(region+image))
                        if len(page['candidates'])>64:raise ValueError('Ambiguous UI pixel signature')
                        if hashlib.sha256(memory[image:image+size]).hexdigest()==page['sha256']:page['matches'].append(hex(region+image))
                del memory
            result['pcUiAtlasMemory']={'resourceSha256':manifest['outputSha256'],'pages':[dict(page=p['page'],expectedPixelSha256=p['sha256'],candidateAddresses=p['candidates'],fullImageMatches=p['matches'],pixelBytes=len(p['pixels'])) for p in pages],'scope':'Read-only full CI4 pixel-array matches in original guest RAM; no texture binding, screenshot, legibility or animation correctness claim'}
        if a.ui_widgets:
            save_records=[]
            for slot in range(4):
                address=0x80622ce0+slot*0xc450
                if not mapped(address,0xc450):break
                record=read_host(base+address,0xc450)
                save_records.append(dict(slot=slot,address=hex(address),sha256=hashlib.sha256(record).hexdigest(),
                    score=struct.unpack_from('>I',record,0)[0],stage=struct.unpack_from('>H',record,0xc)[0],
                    playTime=struct.unpack_from('>f',record,0x10)[0],flags=record[0x44e],
                    badge=record[0x44c],modeFlags=record[0x44d]))
            result['uiSaveRecords']=dict(records=save_records,options=read_host(base+0x80653e20,32).hex(),
                scope='Read-only four native record summaries/hashes and separate options; not a save-file write or raw user-data export')
            controller_manager=0x8069d11c
            current=u32(controller_manager+0x14);end=u32(controller_manager+0x18)
            controllers=[];controller_seen=set()
            while current!=end and current and len(controller_seen)<256:
                if current in controller_seen or not mapped(current,12):break
                controller_seen.add(current);obj=u32(current);following=u32(current+8)
                if not mapped(obj,0x28):break
                vtable=u32(obj+0xc)
                if not mapped(vtable,0x40):break
                methods=[u32(vtable+i*4) for i in range(16)]
                # CodeWarrior's vtable[0] points to its own RTTI record.
                # The adjacent name before the vtable belongs to another type.
                rtti=u32(vtable);type_name=None
                if mapped(rtti,8):
                    name=u32(rtti)
                    if mapped(name,64):
                        raw=read_host(base+name,64).split(b'\0',1)[0]
                        if raw and all(32<=b<127 for b in raw):type_name=raw.decode('ascii')
                entry=dict(pointer=hex(obj),vtable=hex(vtable),typeName=type_name,active=read_host(base+obj,1)[0],
                    cluster=hex(u32(obj+4)),state=read_host(base+obj+0x24,1)[0],methods=[hex(v)for v in methods])
                if type_name in ('cIdHpGage','cIdGageCluster','cIdBossHp') and mapped(obj,0x3c):
                    entry['hud']=dict(words={hex(off):hex(u32(obj+off))for off in range(0x10,0x3c,4)},
                        floats={hex(off):struct.unpack('>f',read_host(base+obj+off,4))[0]for off in {'cIdHpGage':(0x1c,0x20),'cIdGageCluster':(0x14,0x18,0x1c),'cIdBossHp':(0x18,0x1c,0x20,0x24,0x28)}[type_name]},
                        bytes=read_host(base+obj+0x24,0x18).hex())
                if 0x8023c290 in methods and mapped(obj,0x68):
                    entry['loadGame']=dict(actionCluster=hex(u32(obj+0x38)),action=read_host(base+obj+0x3c,1)[0],
                        slot=read_host(base+obj+0x3d,1)[0],mode=read_host(base+obj+0x25,1)[0],
                        record=hex(u32(obj+0x48)),child=hex(u32(obj+0x50)),dialog=hex(u32(obj+0x54)),
                        callbacks=[hex(u32(obj+offset)) for offset in [0x5c,0x60,0x64]])
                controllers.append(entry);current=following
            result['uiControllers']=dict(manager=hex(controller_manager),controllers=controllers,
                reachedEnd=current==end,scope='Bounded read-only native controller list and source-identified save-controller fields')
            manager=0x8069d49c
            current=u32(manager+0x14);end=u32(manager+0x18);seen=set();widgets=[]
            def floats(address,count):
                return list(struct.unpack('>'+str(count)+'f',read_host(base+address,count*4)))
            while current!=end and current and len(seen)<768:
                if current in seen or not mapped(current,12):break
                seen.add(current);widget=u32(current);following=u32(current+8)
                if not mapped(widget,0x314):break
                resource=u32(widget+0x1a0)
                if not mapped(resource,0xe4):break
                raw=u32(resource+0x18)
                if not mapped(raw,0xb0):break
                widgets.append(dict(pointer=hex(widget),resource=hex(resource),sourceRecord=hex(raw),
                    vtable=hex(u32(widget)),vertexCallback=hex(u32(u32(widget)+0x28)) if mapped(u32(widget),0x38) else None,
                    uvCallback=hex(u32(u32(widget)+0x30)) if mapped(u32(widget),0x38) else None,
                    gaugeValue=floats(widget+0x318,3) if u32(raw+0x3c)==8 and mapped(widget,0x324) else None,
                    preparedVertices=floats(widget+0x234,12),
                    id=u32(raw+0x2c),kind=u32(raw+0x3c),sourceFlags=hex(u32(raw+0x34)),
                    sourceHeaderSha256=hashlib.sha256(read_host(base+raw,0xb0)).hexdigest(),
                    sourceHeaderHex=read_host(base+raw,0xb0).hex(),
                    cluster=u16(widget+0x3a),runtimeFlags=hex(u32(widget+0x14)),
                    hidden=read_host(base+widget+0x269,1)[0],
                    parent=hex(u32(widget+0x18)),firstChild=hex(u32(widget+0x1c)),next=hex(u32(widget+0x20)),
                    rgba=floats(widget+0x84,4),size=floats(widget+0x26c,3),translation=floats(widget+0x278,3),
                    matrix1c8=floats(widget+0x1c8,12),matrix1f8=floats(widget+0x1f8,12),
                    textureOverride=hex(u32(widget+0x54)),paletteOverride=hex(u32(widget+0x58)),
                    textureOverrideHex=read_host(base+u32(widget+0x54),32).hex() if mapped(u32(widget+0x54),32) else None,
                    sourceUvGroup=u16(resource+0x48),uvFrame=u16(widget+0x190),
                    uvCorners=floats(widget+0x5c,8),
                    channelFrames=dict(position=u16(widget+0xc8),scale=u16(widget+0x108),rgb=u16(widget+0x142),alpha=u16(widget+0x154)),
                    colorBinding=hex(u32(widget+0x1b8)),alphaBinding=hex(u32(widget+0x1bc)),
                    animationWaitFlags={hex(offset):hex(u32(resource+offset)) for offset in [0x40,0x54,0x64,0x74,0x90,0x94,0x98,0x9c,0xbc,0xc0]}))
                if u32(raw+0x3c)==8 and mapped(widget,0x324):
                    widgets[-1]['gauge']=dict(value=floats(widget+0x318,1)[0],current=floats(widget+0x31c,1)[0],speed=floats(widget+0x320,1)[0])
                current=following
            # 80184490 allocates message glyphs from a separate fixed 256-slot
            # pool; they are not registered in the ordinary IDT manager list.
            glyphs=[]
            for slot in range(256):
                widget=0x80653f58+slot*0x318;resource=0x80685758+slot*0xe4
                if not mapped(widget,0x2a0) or not mapped(resource,0xe4):break
                if not u32(widget+0x7c)&1 or read_host(base+resource,1)[0]!=1:continue
                glyphs.append(dict(slot=slot,pointer=hex(widget),resource=hex(resource),
                    hidden=read_host(base+widget+0x269,1)[0],rgba=floats(widget+0x84,4),
                    size=floats(widget+0x26c,3),matrix1f8=floats(widget+0x1f8,12),
                    uvCorners=floats(widget+0x5c,8),preparedVertices=floats(widget+0x234,12)))
            result['messageGlyphs']=dict(glyphs=glyphs,scope='Bounded original message pool80184490; active256 slots maximum, concurrent read-only snapshot')
            result['uiWidgets']=dict(manager=hex(manager),widgets=widgets,reachedEnd=current==end,
                scope='Concurrent read-only UI list; 8024CD44/CD3C list bounds, CB3C next; transforms from C7204/C80C0. Not frame-atomic.')
        scene=0x8069bca8
        result['sceneSpeedFields']={'object':hex(scene),'floats':{hex(offset):struct.unpack('>f',read_host(base+scene+offset,4))[0] for offset in (0x18,0x1c,0x24,0x28)},'authority':'Original getSpeedRate reads18; setSpeedRate writes18/1C and priority24, compares28. Raw concurrent fields; flag18 override not evaluated here.'}
        pointer=u32(0x80697e64)
        result['gameFlags']=dict(pointer=hex(pointer),word0=hex(u32(pointer)) if mapped(pointer,4) else None)
        (ROOT/'evidence'/f'{a.label}.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
        print(json.dumps(result,indent=2))
    finally:k.CloseHandle(handle)
if __name__=='__main__':main()

