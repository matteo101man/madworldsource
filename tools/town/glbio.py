import json,struct
import numpy as np
CT={5126:np.float32,5123:np.uint16,5125:np.uint32,5121:np.uint8}
NC={'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4,'MAT4':16}
class Glb:
    def __init__(s,path):
        b=open(path,'rb').read(); l=struct.unpack_from('<I',b,12)[0]
        s.j=json.loads(b[20:20+l]); off=20+l; bl=struct.unpack_from('<I',b,off)[0]; s.bin=bytearray(b[off+8:off+8+bl])
    def acc(s,i):
        a=s.j['accessors'][i]; bv=s.j['bufferViews'][a['bufferView']]
        n=NC[a['type']]; dt=CT[a['componentType']]; st=bv.get('byteStride')
        o=bv.get('byteOffset',0)+a.get('byteOffset',0)
        if st and st!=n*np.dtype(dt).itemsize:
            raw=np.frombuffer(bytes(s.bin[o:o+st*a['count']]),dtype=np.uint8).reshape(a['count'],st)[:,:n*np.dtype(dt).itemsize]
            return np.ascontiguousarray(raw).view(dt).reshape(a['count'],n)
        return np.frombuffer(bytes(s.bin[o:o+a['count']*n*np.dtype(dt).itemsize]),dtype=dt).reshape(a['count'],n)
    def add_view(s,data,target=None):
        while len(s.bin)%4: s.bin.append(0)
        o=len(s.bin); s.bin+=data; bv={'buffer':0,'byteOffset':o,'byteLength':len(data)}
        if target: bv['target']=target
        s.j['bufferViews'].append(bv); return len(s.j['bufferViews'])-1
    def set_uv(s,acc_i,uv):
        uv=np.ascontiguousarray(uv,dtype=np.float32); v=s.add_view(uv.tobytes(),34962)
        a=s.j['accessors'][acc_i]; a['bufferView']=v; a['byteOffset']=0; a['componentType']=5126; a.pop('normalized',None)
        a['min']=uv.min(0).tolist(); a['max']=uv.max(0).tolist()
    def save(s,path):
        while len(s.bin)%4: s.bin.append(0)
        s.j['buffers']=[{'byteLength':len(s.bin)}]
        js=json.dumps(s.j,separators=(',',':')).encode()
        while len(js)%4: js+=b' '
        out=struct.pack('<III',0x46546C67,2,28+len(js)+len(s.bin))+struct.pack('<II',len(js),0x4E4F534A)+js+struct.pack('<II',len(s.bin),0x004E4942)+bytes(s.bin)
        open(path,'wb').write(out)
    def mesh_prims(s):
        # node name -> (mesh index) for role_0_mesh_N
        out={}
        for n in s.j['nodes']:
            if 'mesh' in n and n.get('name','').startswith('role_0_mesh_'): out[int(n['name'][12:])]=n['mesh']
        return out
