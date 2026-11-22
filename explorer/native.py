"""Bounded MadWorld readers. Format reference: Kerilk/noesis_bayonetta_pc (MIT).

No game bytes or third-party implementation are embedded here.
"""
import struct
import math


class Reader:
    def __init__(self, data):
        self.data = data

    def get(self, fmt, offset):
        size = struct.calcsize('>' + fmt)
        if offset < 0 or offset + size > len(self.data):
            raise ValueError(f'out of bounds {offset:#x}+{size:#x}/{len(self.data):#x}')
        return struct.unpack_from('>' + fmt, self.data, offset)

    def u32(self, offset):
        return self.get('I', offset)[0]


def camera(data):
    """CAM samples; native event getCutFrames0x800a4c1c reads u16 at+6."""
    if len(data)<32 or data[:4]!=b'cam\0':raise ValueError('camera magic')
    flags,count,offset=struct.unpack_from('>HHI',data,4)
    if offset!=32 or len(data)!=offset+count*32:raise ValueError('camera sample bounds')
    samples=[list(struct.unpack_from('>8f',data,offset+i*32)) for i in range(count)]
    if not all(math.isfinite(v) for row in samples for v in row):raise ValueError('nonfinite camera sample')
    return dict(frames=count,headerField04=flags,headerUnknownHex=data[12:32].hex(),samples=samples,
                interpretation='position, target, roll, vertical FOV radians: inspection interpretation; runtime comparison pending')


def message_tokens(data):
    """MSD tables may have a zero prefix; validate the entire candidate table."""
    r=Reader(data);candidates=[]
    for base in range(0,min(64,len(data)-8)+1,4):
        if any(data[:base]):continue
        count,marker=r.get('II',base)
        if marker!=0xffffffff or count>100000 or base+8+count*12>len(data):continue
        rows=[]
        for i in range(count):
            mid,off,size=r.get('3I',base+8+i*12)
            if size%2 or off<8+count*12 or base+off+size>len(data):break
            rows.append(dict(id=mid,tokens=list(r.get(str(size//2)+'H',base+off))))
        else:candidates.append(rows)
    if len(candidates)!=1:raise ValueError('MSD header/table bounds ambiguous or invalid')
    return candidates[0]


def dat_entries(data):
    r = Reader(data)
    if data[:4] != b'DAT\0':
        raise ValueError('not DAT')
    count, offsets, extensions, names, sizes = r.get('5I', 4)
    if count > 100000:
        raise ValueError('DAT count')
    width = r.u32(names)
    if not 1 <= width <= 1024:
        raise ValueError('DAT name width')
    result = []
    for i in range(count):
        off, size = r.u32(offsets + i*4), r.u32(sizes + i*4)
        name = r.get(f'{width}s', names + 4 + width*i)[0].split(b'\0')[0].decode('utf-8')
        if '/' in name or '\\' in name or name in ('.', '..'):
            raise ValueError('unsafe DAT name')
        r.get(f'{size}s', off)
        result.append(dict(name=name, offset=off, size=size))
    return result


def mdb(data):
    r = Reader(data)
    if data[:2] != b'JK':
        raise ValueError('not MDB')
    flags = r.get('H', 2)[0]
    keys = ('positions', 'normals', 'uvs', 'colors', 'palette')
    h = {key: r.get('II', 4+i*8) for i,key in enumerate(keys)}
    nb, parents, relative, absolute, nm, materials, _, meshes = r.get('8I', 44)
    posdiv, uvdiv = r.get('hh', 76)
    names = r.u32(92)
    strange = r.u32(124)
    bones = []
    for i in range(nb):
        parent = r.get('b', parents+i)[0]
        if parent >= nb or parent == i:
            raise ValueError('bone parent')
        bones.append(dict(id=r.get('B', names+i)[0], parent=parent,
                          position=r.get('3f', absolute+12*i), relative=r.get('3f', relative+12*i)))
    out = []
    visited = set()
    while meshes:
        if meshes in visited:
            raise ValueError('mesh cycle')
        visited.add(meshes)
        nxt, mat, mf, ua, ub = r.get('IBBBB', meshes)
        size, blocks, uc = r.get('III', meshes+20)
        name = r.get('60s' if flags & 0x200 else '32s', meshes+32)[0].split(b'\0')[0].decode('utf-8', 'replace')
        end = meshes+nxt if nxt else len(data)
        p = end-size if size else ((meshes+128+(64 if flags & 0x200 else 32))//32+1)*32
        vertices, normals, uvs, joints, weights, colors, indices = [], [], [], [], [], [], []
        for block in range(blocks or 1):
            kind, count = r.get('BH', p); p += 3
            if kind not in (0x90,0x98,0xa0):
                raise ValueError(f'primitive {kind:#x} at {p-3:#x}')
            base = len(vertices)
            for _ in range(count):
                if strange & 0x100:
                    p += 1
                pi = r.get('H',p)[0]; p += 2
                if pi >= h['positions'][0]:
                    raise ValueError('position index')
                off = h['positions'][1] + pi*(16 if posdiv == -1 else 8)
                xyz = r.get('3f' if posdiv == -1 else '3h', off)
                vertices.append([v/(1 if posdiv == -1 else 2**posdiv) for v in xyz])
                if nb and h['palette'][0]:
                    pal = r.get('H',off+(12 if posdiv == -1 else 6))[0]
                    if pal >= h['palette'][0]:
                        raise ValueError('palette index')
                    pairs = r.get('8B',h['palette'][1]+pal*8)
                    ji, we = [], []
                    for j,w in zip(pairs[::2],pairs[1::2]):
                        ji.append(j if j != 255 else 0); we.append(w/100 if j != 255 else 0)
                    if any(j >= nb for j,w in zip(ji,we) if w):
                        raise ValueError('joint index')
                    joints.append(ji); weights.append(we)
                if h['normals'][0] and not mf & 4:
                    ni = r.get('H',p)[0]; p += 2
                    if ni >= h['normals'][0]: raise ValueError('normal index')
                    normals.append([v/64 for v in r.get('3b',h['normals'][1]+ni*3)])
                if h['colors'][0] and mf & 1:
                    ci = r.get('H',p)[0]; p += 2
                    colors.append(r.get('4B',h['colors'][1]+ci*4))
                if h['uvs'][0] and not mf & 8:
                    ui = r.get('H',p)[0]; p += 2
                    if ui >= h['uvs'][0]: raise ValueError('UV index')
                    uvs.append([v/2**uvdiv for v in r.get('2h',h['uvs'][1]+ui*4)])
            for k in range(count//3 if kind == 0x90 else max(0,count-2)):
                tri = [base+k*3+j for j in range(3)] if kind == 0x90 else ([base,base+k+1,base+k+2] if kind==0xa0 else ([base+k+1,base+k,base+k+2] if k%2 else [base+k,base+k+1,base+k+2]))
                indices.extend(tri)
        texture = r.get('b',materials+mat*16)[0] if mat < nm else -1
        out.append(dict(name=name, material=mat, texture=texture, flags=mf,renderFlags=(ua<<8)|ub,inkThreshold=r.u32(meshes+92)&255 if flags&0x200 else None, positions=vertices,
                        normals=normals, uvs=uvs, joints=joints, weights=weights, colors=colors, indices=indices))
        meshes = meshes+nxt if nxt else 0
    return dict(bones=bones, meshes=out, flags=flags)


def tpl(data, selected=None):
    """Decode Nintendo tiled base mip; preserve RGBA including source alpha."""
    from PIL import Image
    r = Reader(data)
    magic, count, table = r.get('3I', 0)
    if magic != 0x20af30 or count > 4096:
        raise ValueError('TPL header')
    images = []
    def expand5(v):return (v<<3)|(v>>2)
    def expand6(v):return (v<<2)|(v>>4)
    def rgb565(v):
        return (expand5(v>>11), expand6((v>>5)&63), expand5(v&31), 255)
    def rgb5a3(v):
        a=(v>>12)&7
        return (expand5((v>>10)&31), expand5((v>>5)&31), expand5(v&31), 255) if v&32768 else (((v>>8)&15)*17, ((v>>4)&15)*17, (v&15)*17, (a<<5)|(a<<2)|(a>>1))
    for i in range(count):
        if selected is not None and i != selected:
            continue
        header, palette = r.get('II',table+i*8)
        height,width,fmt,p = r.get('HHII',header)
        if not 0 < width <= 8192 or not 0 < height <= 8192:
            raise ValueError('TPL dimensions')
        rgba = bytearray(width*height*4)
        pal = []
        if palette:
            n,_,_,pf,po = r.get('HBBII',palette)
            for j in range(n):
                v=r.get('H',po+2*j)[0]
                pal.append(rgb565(v) if pf==1 else rgb5a3(v) if pf==2 else (v&255,)*3+(v>>8,))
        def put(x,y,c):
            if x < width and y < height:
                rgba[(y*width+x)*4:(y*width+x)*4+4]=bytes(c)
        dims = {0:(8,8),1:(8,4),2:(8,4),3:(4,4),4:(4,4),5:(4,4),6:(4,4),8:(8,8),9:(8,4),10:(4,4),14:(8,8)}
        if fmt not in dims: raise ValueError(f'TPL format {fmt}')
        bw,bh=dims[fmt]
        for by in range(0,height,bh):
            for bx in range(0,width,bw):
                if fmt==14:
                    for sy,sx in ((0,0),(0,4),(4,0),(4,4)):
                        c0,c1,bits=r.get('HHI',p);p+=8
                        a,b=rgb565(c0),rgb565(c1)
                        # GX CMPR uses 5/8 and 3/8 interpolation. Its transparent
                        # endpoint retains the averaged RGB, unlike PC DXT1.
                        average=tuple((a[k]+b[k])//2 for k in range(3))
                        cs=[a,b,tuple((5*a[k]+3*b[k])>>3 for k in range(3))+(255,),tuple((3*a[k]+5*b[k])>>3 for k in range(3))+(255,)] if c0>c1 else [a,b,average+(255,),average+(0,)]
                        for k in range(16):put(bx+sx+k%4,by+sy+k//4,cs[(bits>>(30-2*k))&3])
                elif fmt==6:
                    block=r.get('64s',p)[0];p+=64
                    for k in range(16):put(bx+k%4,by+k//4,(block[k*2+1],block[32+k*2],block[33+k*2],block[k*2]))
                else:
                    step=0.5 if fmt in (0,8) else 1 if fmt in (1,2,9) else 2
                    block=r.get(f'{int(bw*bh*step)}s',p)[0];p+=len(block)
                    for k in range(bw*bh):
                        v=((block[k//2]>>(4 if k%2==0 else 0))&15) if step==0.5 else block[k] if step==1 else struct.unpack_from('>H',block,k*2)[0]
                        if fmt in (8,9,10): c=pal[v&16383]
                        elif fmt==0:c=(v*17,)*4
                        elif fmt==1:c=(v,)*4
                        elif fmt==2:c=((v&15)*17,)*3+((v>>4)*17,)
                        elif fmt==3:c=(v&255,)*3+(v>>8,)
                        else:c=rgb565(v) if fmt==4 else rgb5a3(v)
                        put(bx+k%bw,by+k//bw,c)
        images.append((Image.frombytes('RGBA',(width,height),bytes(rgba)),dict(format=fmt,width=width,height=height,wrap=r.get('II',header+12))))
    return images


def pghalf(v):
    sign=-1 if v&32768 else 1
    e=(v>>9)&63; m=v&511
    if e==63: raise ValueError('nonfinite PG half')
    return sign*(m/512*2**(1-47) if e==0 else (1+m/512)*2**(e-47))


def motion(data):
    r=Reader(data)
    if data[:4]!=b'mot\0': raise ValueError('not MOT')
    flag,frames,off,count=r.get('hhII',4)
    if not 0 < frames <= 32767 or count>10000: raise ValueError('MOT header')
    tracks=[]
    for i in range(count):
        bone,channel,fmt,n,unknown,value=r.get('hBBhhI',off+i*12)
        if bone==32767 or bone>=0xf60 or fmt==255: continue
        if channel>15 or n<0:raise ValueError('MOT entry')
        if fmt==0:
            values=[r.get('f',off+i*12+8)[0]]*frames
        elif fmt in (1,2,3):
            if not n:raise ValueError('MOT empty samples')
            if fmt==1:values=list(r.get(f'{n}f',value))
            elif fmt==2:
                a,b=r.get('2f',value); values=[a+b*v for v in r.get(f'{n}H',value+8)]
            else:
                a,b=map(pghalf,r.get('2H',value)); values=[a+b*v for v in r.get(f'{n}B',value+4)]
            values=(values+[values[-1]]*frames)[:frames]
        elif fmt in (4,6,7,8):
            if not n:raise ValueError('MOT empty spline')
            hdr=list(r.get('6f',value)) if fmt==4 else list(map(pghalf,r.get('6H',value)))
            cursor=value+(24 if fmt==4 else 12); keys=[]; frame=0
            for j in range(n):
                if fmt==4:
                    t,a,b,c=r.get('4H',cursor);cursor+=8
                elif fmt==6:
                    t,a,b,c=r.get('4B',cursor);cursor+=4
                    frame=t if not j else frame+t;t=frame
                elif fmt==7:
                    t,_,a,b,c=r.get('H4B',cursor);cursor+=6
                else:
                    t,a,b,c=r.get('H3B',cursor);cursor+=5
                keys.append((t,hdr[0]+hdr[1]*a,hdr[2]+hdr[3]*b,hdr[4]+hdr[5]*c))
            # Native ev71 contains byte-identical duplicate keys at one timestamp.
            # Removing an identical key preserves both neighbouring Hermite spans.
            keys=[key for j,key in enumerate(keys) if j==0 or key!=keys[j-1]]
            if any(b[0]<=a[0] for a,b in zip(keys,keys[1:])):raise ValueError('MOT nonascending keys')
            values=[];k=0
            for t in range(frames):
                while k+1<len(keys) and t>keys[k+1][0]: k+=1
                a=keys[k]
                if t<=keys[0][0]:v=keys[0][1]
                elif k+1==len(keys):v=a[1]
                else:
                    b=keys[k+1];u=(t-a[0])/(b[0]-a[0])
                    v=(2*u**3-3*u*u+1)*a[1]+(u**3-2*u*u+u)*a[3]+(-2*u**3+3*u*u)*b[1]+(u**3-u*u)*b[2]
                values.append(v)
        else:raise ValueError(f'MOT unsupported encoding {fmt}')
        if not all(math.isfinite(v) for v in values): raise ValueError('MOT nonfinite values')
        tracks.append(dict(bone=bone,channel=channel,encoding=fmt,values=values))
    return dict(frames=frames,tracks=tracks,headerFlag=flag,fps=60,fpsEvidence='upstream reader preview; runtime timing unverified')
