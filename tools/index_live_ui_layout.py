from save_typed_graph import save_graph
"""Catalog the standalone common UI actually read by the native game."""
import hashlib,json,struct
from pathlib import Path
from index_ui_layout import ROOT,parse
SOURCE=Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/files/etc/core_us.idd')
SHA='b5d50f1b0ae656235fa7aee877faabf142b7e8a7e90f37606200016208d5f455'
def load():
    raw=SOURCE.read_bytes()
    if hashlib.sha256(raw).hexdigest()!=SHA:raise ValueError('Unsupported standalone core UI')
    pages=[];tpl=4096
    count,table=struct.unpack_from('>II',raw,tpl+4)
    for i in range(count):
        th,ph=struct.unpack_from('>II',raw,tpl+table+8*i)
        h,w,fmt,data=struct.unpack_from('>HHII',raw,tpl+th)
        page=dict(id=i,width=w,height=h,format=fmt,dataOffset=tpl+data,maxLod=raw[tpl+th+34])
        if fmt in (8,9):
            n,unpacked,pad,pfmt,pdata=struct.unpack_from('>HBBII',raw,tpl+ph)
            page.update(paletteEntries=n,paletteFormat=pfmt,paletteOffset=tpl+pdata)
        pages.append(page)
    layout=parse(raw,[(p['width'],p['height']) for p in pages])
    layout['source']=dict(path=str(SOURCE),size=len(raw),sha256=SHA)
    layout.pop('width');layout.pop('height');layout['pages']=pages
    for sprite in layout['sprites']:sprite['texturePage']=sprite['unknown16']
    return raw,layout

def main():
    raw,layout=load();out=ROOT/'research'
    (out/'ui-live-core-layout.json').write_text(json.dumps(layout,indent=2)+'\n',encoding='utf-8')
    graph=[dict(type='node',id='ui:live-core',kind='ui-resource',source=layout['source'])]
    for page in layout['pages']:
        graph.append(dict(type='node',id=f"ui:live-core:page:{page['id']}",kind='ui-atlas',**{k:v for k,v in page.items() if k!='id'}))
        graph.append(dict(type='edge',source='ui:live-core',target=f"ui:live-core:page:{page['id']}",kind='contains'))
    for g in layout['groups']:
        graph.append(dict(type='node',id=f"ui:live-core:group:{g['id']}",kind='ui-uv-group',first=g['first'],count=g['count']))
        graph.append(dict(type='edge',source='ui:live-core',target=f"ui:live-core:group:{g['id']}",kind='contains'))
    for s in layout['sprites']:
        sid=f"ui:live-core:sprite:{s['id']}";graph.extend([dict(type='node',id=sid,kind='ui-sprite-rect',**{k:v for k,v in s.items() if k!='id'}),dict(type='edge',source=f"ui:live-core:group:{s['group']}",target=sid,kind='contains'),dict(type='edge',source=sid,target=f"ui:live-core:page:{s['texturePage']}",kind='uses-texture')])
    save_graph(out/'ui-live-core-layout.graph.jsonl',graph,'Source resource/protocol index; see focused API/INDEX for evidence scope')
    print(json.dumps(dict(groups=len(layout['groups']),sprites=len(layout['sprites']),pages=len(layout['pages']))))
if __name__=='__main__':main()
