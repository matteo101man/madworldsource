"""In-process God Hand explorer server (builders/serve_main.Server) for solved clip sampling: server().raw_sample(rig, motion).
Usage: python tools/godhand/godhand_server.py pl00-1 pl00:59   (prints a quick summary)
"""
import sys,time
from pathlib import Path
R=Path(r'D:/Games/gamesfordecomp/exports/godhand')
sys.path.insert(0,str(R/'builders'))
import serve_main
def server():
    s=serve_main.Server(R,0);return s
if __name__=='__main__':
    s=server();t=time.time()
    out=s.raw_sample(sys.argv[1],sys.argv[2])
    print(time.time()-t,s.clips.hits,out.keys(),out['joints'],len(out['frames']),len(out['frames'][0]),out['fps'],out['matrix_space'])
    print(out['ik_chains'])
    s.server_close()
