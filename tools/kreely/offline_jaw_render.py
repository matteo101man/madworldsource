"""Offline render of the jaw (bone 24) at 0, 0.4, 1.0 rad, front + side. python -I tools/kreely/offline_jaw_render.py <out.png>"""
import sys, json, numpy as np
sys.path.insert(0, r'D:\Games\gamesfordecomp\ports\madworld-rinrin\tools\kreely'); import kreely_pose as kp
from PIL import Image, ImageDraw
B=r'D:\Games\gamesfordecomp\ports\madworld-rinrin\assets\characters\kreely\build'
model=json.load(open(B+r'\kreely.model.json')); mesh=model['meshes'][0]
I=np.array(mesh['indices']).reshape(-1,3); uv=np.array(mesh['uvs'])
tex=np.asarray(Image.open(B+r'\kreely_color.png').convert('RGB')); h,w,_=tex.shape
col=tex[(np.clip(uv[:,1],0,1)*(h-1)).astype(int),(np.clip(uv[:,0],0,1)*(w-1)).astype(int)].astype(float)
idx={b['id']:i for i,b in enumerate(model['bones'])}
out=Image.new('RGB',(1500,800))
for k,(ang,view) in enumerate([(0,'front'),(0.4,'front'),(1.0,'front'),(0,'side'),(0.4,'side'),(1.0,'side')]):
  clip=dict(frames=1,tracks=[dict(bone=24,channel=3,values=[ang])])
  W=kp.worlds(model,clip,0); V=kp.skin(model,mesh,W)
  c=W[idx[4]][:3,3]+[0,0.0,0.05]; span=0.25
  A,d=(np.array([[1,0,0],[0,1,0]]),np.array([0,0,1])) if view=='front' else (np.array([[0,0,1],[0,1,0]]),np.array([1,0,0]))
  img=Image.new('RGB',(500,400),(40,48,52)); dr=ImageDraw.Draw(img)
  T=V[I]; n=np.cross(T[:,1]-T[:,0],T[:,2]-T[:,0]); n/=np.linalg.norm(n,axis=1,keepdims=True)+1e-12
  q=(T-c)@A.T; keep=(np.abs(q)<span/2).all(2).any(1); sh=0.35+0.65*np.abs(n@d)
  for t in np.argsort((T@d).mean(1)):
    if keep[t]: dr.polygon([((q[t,m,0]/span+0.5)*500,(0.5-q[t,m,1]/span)*500-50) for m in range(3)],fill=tuple(int(x) for x in col[I[t]].mean(0)*sh[t]))
  dr.text((5,5),f'jaw {ang} rad {view}',fill=(255,255,0)); out.paste(img,((k%3)*500,(k//3)*400))
out.save(sys.argv[1])
