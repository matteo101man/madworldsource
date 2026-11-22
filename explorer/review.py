"""Apply explicit rendered-review annotations without filename roster inference."""
import argparse
import json
from pathlib import Path
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[2]))
from gk.graph import Graph

# Each entry follows the saved native contact sheets, not a sorted string list.
REVIEW={
 ('pl/pl0000.dat','pl0000.mdb'):('Jack Cayman','Main gameplay body','evidence/contact-3.jpg: tile 2; native player archive and matching rendered face/clothing'),
 ('pl/pl0000.dat','pl0000_LOD0.mdb'):('Jack Cayman','Native LOD0 body','evidence/contact-3.jpg: tile 3; matching rendered Jack; LOD0 is the literal source designation, not a costume'),
 ('pl/pl0010.dat','pl0010.mdb'):('Jack Cayman','Separate native body pl0010','evidence/contact-3.jpg: tile 4; rendered Jack face, bionic arm and jacket'),
 ('pl/pl0030.dat','pl0030.mdb'):('Jack Cayman','Detailed native body pl0030','evidence/contact-3.jpg: tile 5; rendered Jack face, bionic arm and jacket'),
 ('em/em0440.dat','em0440.mdb'):('RinRin','Native boss body','evidence/contact-1.jpg: tile 8; rendered fan weapons, qipao, hair buns; native cEm0440 martial-arts/throw handlers; external character reference corroborates appearance'),
 ('em/em0610.dat','em0610.mdb'):('Mathilda','Native assistant body','evidence/contact-1.jpg: tile 18; rendered helmet, corset and boots; external Mathilda resource metadata independently names em0610 texture'),
 ('em/em0630.dat','em0630.mdb'):('Elise','Native boss body','evidence/contact-2.jpg: tile 3; rendered horned headpiece, false wings and dress; native cEm0630 bat-mistress/hide/blackdragon/somersault handlers corroborate boss behavior'),
 ('em/em0920.dat','em0920.mdb'):('Naomi Ann Borras','Native story body','evidence/contact-2.jpg: tile 10; rendered long dark hair and striped/fringed dress; appearance identification, native identity string binding unresolved'),
 ('em/em0930.dat','em0930.mdb'):('Amala','Native story body','evidence/contact-2.jpg: tile 12; rendered short dark hair, earrings, open white blouse and trousers; appearance identification, native identity string binding unresolved'),
 ('em/em0960.dat','em0960.mdb'):('Unidentified woman','Hat and patterned dress','evidence/contact-2.jpg: tile 17; rendered woman; proper identity and any Geisha/reporter association remain unresolved'),
 ('em/em0925.dat','em0925.mdb'):('Unidentified female silhouette','Native body em0925','evidence/contact-2.jpg: tile 11; distinct white silhouette geometry and 27-bone rig; relationship to Naomi is unproven'),
 ('em/em0935.dat','em0935.mdb'):('Unidentified female silhouette','Native body em0935','evidence/contact-2.jpg: tile 13; distinct white silhouette geometry and 31-bone rig; relationship to Amala is unproven'),
}

FINGERPRINTS={
 'em0440.mdb':'66807dbec53f7a7baeacdbb883427c63bee07d7f3c3c5778edfe57f316d76dd0',
 'em0610.mdb':'c9332fbf63c19edc2bf621266827028a461d7ba37bbd6b67ed1032953ad85243',
 'em0630.mdb':'4b19f0c4ed327f37989af19a66747ebd5ee9ddb314bf295417e667e1efc48aa8',
 'em0920.mdb':'b2b79ece94c24f9f661362ca1a8732859bed753532209d9ab2692a95f63b1527',
 'em0925.mdb':'a08fb602ce12c6eba671b7aadcf211d2a6853599d94ec99fd2bf36af0a250358',
 'em0930.mdb':'3eac584ec078313913d041d5d4fc0553473b5215c08e3bdfe1b107a289f04db0',
 'em0935.mdb':'39ea54c3666596777a02b0d12dbb5112c434efe4732d0d426d328128039cb905',
 'em0960.mdb':'93024b2a1d8c7dd18cb9b29dc6b08bebc1b2d0ef69dbf6ead425c40767f51f13',
 'pl0000.mdb':'0bb67f0aab1ced0ce4b911829e2167cd2dd4e4c2c7e118a36df5d31ac81bc7ff',
 'pl0000_LOD0.mdb':'8d936074bee8d9f078218f5a2556bfbc96878a88f5e13c4b9e641099958f28a7',
 'pl0010.mdb':'ca085976982568166308ad01773db67d96fe2c99621cb724c19617411dd739ff',
 'pl0030.mdb':'3578b095945dc4d4f0028d9490c034bcd8e031646b576395d7031d3089c90561',
}


def review(root):
    root=Path(root);path=root/'catalog.json';catalog=json.loads(path.read_text(encoding='utf-8'));annotations=[]
    for item in catalog['models']:
        row=REVIEW.get((item['bank'],item['name']))
        if row and item['sha256']==FINGERPRINTS[item['name']]:
            identity,variant,evidence=row
            item.update(displayName=identity+' · '+variant,identity=identity,variant=variant,review='reviewed',identityEvidence=evidence,identityAuthority='rendered identification; native class/identity linkage separately stated')
            annotations.append({k:v for k,v in item.items() if k not in ('texture',)})
    path.write_text(json.dumps(catalog,separators=(',',':')),encoding='utf-8')
    (root/'evidence/roster-review.json').write_text(json.dumps(dict(scope='70 unique resources with >15 bones visually reviewed. All 714 MDB resources indexed. Unrigged/small-rig actor forms, static representations, and pre-rendered-only characters remain a coverage gap.',annotations=annotations,properIdentityUnresolved=['em0960','em0925','em0935'],references=['https://www.platinumgames.com/works/madworld','https://models.spriters-resource.com/wii/madworld/asset/291789/','https://madworld.fandom.com/wiki/Amala','https://madworld.fandom.com/wiki/RinRin','https://madworld.fandom.com/wiki/Elise']),indent=2),encoding='utf-8')
    graph=Graph(dict(game='MadWorld',authority='inspection candidate; native dispatch and contact unverified'))
    graph.add_node('pair:elise-0203','motion-pair','Elise / Jack simultaneous inspection',confidence='inferred-shared-archive-equal-duration',nativeDispatch='unknown',worldAlignment='zero added world offset, decoded source roots only',fullSequence='unknown')
    for name,role in [('pl0030_0463.mot','performer'),('em0630_0203.mot','receiver')]:
        c=next(c for c in catalog['clips'] if c['bank']=='em/em0630.dat' and c['name']==name)
        graph.add_node('clip:'+c['id'],'motion',name,**c)
        graph.add_edge('pair:elise-0203','clip:'+c['id'],'role',role=role,authority='inspection role; actual native dispatch unresolved')
    graph.save(root/'evidence/paired.graph.jsonl')
    (root/'evidence/PAIRED-INDEX.md').write_text('# Paired inspection\n\nOne two-role candidate: Jack pl0030_0463 and Elise em0630_0203 in em0630.dat, each 178 frames. Same-archive containment and temporal length motivate inspection only. Actual native dispatch, alignment, facing, transitions and get-up are unknown.\n',encoding='utf-8')
    (root/'evidence/PAIRED-API.md').write_text('# Pair vocabulary\n\n`role`: manually assigned inspection performer/receiver, not a recovered native role binding.\n`motion-pair`: candidate for simultaneous review, never a gameplay trigger.\n',encoding='utf-8')
    (root/'evidence/scripts/paired-inspection.md').write_text('# Paired inspection pseudocode\n\nAdapter policy, not recovered game code:\n\n1. Load both native rigs.\n2. Sample each named source clip on one adjustable clock.\n3. Apply decoded local root once; add zero world displacement.\n4. Hold terminal poses.\n5. Do not synthesize continuation or claim native contact.\n',encoding='utf-8')
    print(f'{len(annotations)} annotated model bindings')


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root');a=p.parse_args();review(a.root)
