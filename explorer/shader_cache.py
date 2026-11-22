"""Read observed Dolphin 2609a pixel UID records into typed GX stage graphs.

Layout references: Dolphin LinearDiskCache.h and PixelShaderGen.h at the cache's
own build revision. No compiled shader or game data is embedded in this adapter.
"""
import hashlib
import json
from pathlib import Path
import struct
from gk.graph import Graph

COLOR=('prev.rgb','prev.a','reg0.rgb','reg0.a','reg1.rgb','reg1.a','reg2.rgb','reg2.a','texture.rgb','texture.a','raster.rgb','raster.a','one','half','konst.rgb','zero')
ALPHA=('prev.a','reg0.a','reg1.a','reg2.a','texture.a','raster.a','konst.a','zero')


def combiner(value,alpha=False):
    names=ALPHA if alpha else COLOR
    shifts=(13,10,7,4) if alpha else (12,8,4,0);mask=7 if alpha else 15
    return dict(inputs={k:names[(value>>s)&mask] for k,s in zip('ABCD',shifts)},bias=(value>>16)&3,subtract=bool(value&(1<<18)),clamp=bool(value&(1<<19)),scale=(value>>20)&3,destination=('prev','reg0','reg1','reg2')[(value>>22)&3],raw=value)


def cache_records(raw):
    if len(raw)<48 or raw[:4]!=b'DCAC':raise ValueError('disk cache magic/header')
    key_size,value_size=struct.unpack_from('<HH',raw,4)
    if (key_size,value_size)!=(348,1):raise ValueError('unsupported pixel UID build layout')
    version=raw[8:48].decode('ascii');cursor=48;out=[];ordinal=1
    while cursor<len(raw):
        if cursor+4>len(raw):raise ValueError('truncated disk cache record')
        size=struct.unpack_from('<I',raw,cursor)[0];start=cursor+4;end=start+key_size+size
        if end+4>len(raw):raise ValueError('truncated compiled shader payload')
        if struct.unpack_from('<I',raw,end)[0]!=ordinal:raise ValueError('disk cache ordinal')
        uid=raw[start:start+key_size];num_values,mode=struct.unpack_from('<II',uid);count=((mode>>16)&15)+1
        stages=[]
        for i in range(count):
            cc,ac,order,swap,pad=struct.unpack_from('<5I',uid,20+i*20)
            stages.append(dict(index=i,color=combiner(cc&0xffffff),alpha=combiner(ac&0xffffff,True),textureMap=(ac>>24)&7,textureCoord=(ac>>27)&7,textureEnabled=bool(ac&(1<<30)),rasterChannel=order&7,indirect=(order>>10)&0x1fffff,rasterSwap=[(swap>>(j*2))&3 for j in range(4)],textureSwap=[(swap>>(10+j*2))&3 for j in range(4)],konstColor=(swap>>18)&31,konstAlpha=(swap>>23)&31))
        out.append(dict(id=hashlib.sha256(uid).hexdigest(),ordinal=ordinal,offset=cursor,compiledBytes=size,uidBytesUsed=num_values,texgens=(mode>>12)&15,stages=stages,version=version))
        cursor=end+4;ordinal+=1
    return out


def build(root,cache):
    root=Path(root);cache=Path(cache);raw=cache.read_bytes();rows=cache_records(raw);g=Graph(dict(game='MadWorld',authority='observed Dolphin 2609a shader UIDs; material and draw bindings not yet identified',cache=str(cache),sha256=hashlib.sha256(raw).hexdigest()))
    for row in rows:
        g.add_node(row['id'],'gx-program',str(row['ordinal']),**{k:v for k,v in row.items() if k!='stages'})
        for stage in row['stages']:
            nid=row['id']+':'+str(stage['index']);g.add_node(nid,'gx-tev-stage',str(stage['index']),**stage);g.add_edge(row['id'],nid,'evaluates')
    g.save(root/'evidence/shaders.graph.jsonl');(root/'evidence/shader-programs.json').write_text(json.dumps(rows,indent=2),encoding='utf-8')
    (root/'evidence/SHADER-API.md').write_text('# Observed GX shader vocabulary\n\n`gx-program evaluates gx-tev-stage`: exact compiled pixel UID from the recorded Dolphin build. Combiner inputs A/B/C/D, bias, subtraction, clamp, scale, register destination, texture map/coordinate, swaps and konst selectors. These records do not capture uniform values, texture bindings, draw visibility or prove which program belongs to a character.\n',encoding='utf-8')
    print(dict(programs=len(rows),stageCounts=[len(r['stages']) for r in rows],build=rows[0]['version'] if rows else None))


if __name__=='__main__':
    import argparse
    p=argparse.ArgumentParser();p.add_argument('root');p.add_argument('cache');a=p.parse_args();build(a.root,a.cache)
