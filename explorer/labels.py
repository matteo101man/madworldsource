"""Concrete rendered descriptions. These are not recovered original move names."""
import argparse
import json
from pathlib import Path

DESCRIPTIONS={
 ('em/em0615.dat','em0610_0000.mot'):('Pose with one hand raised beside the face','evidence/mathilda-native-strips-0.jpg, row 1'),
 ('em/em0615.dat','em0610_0001.mot'):('Walk with one hand raised','evidence/mathilda-native-strips-0.jpg, row 2'),
 ('em/em0615.dat','em0610_0010.mot'):('Turning dance with a low crouch','evidence/mathilda-native-strips-0.jpg, row 3'),
 ('em/em0440.dat','em0440_0000.mot'):('Rest a closed fan on the shoulder','evidence/rinrin-native-strips-0.jpg, row 1'),
 ('em/em0440.dat','em0440_0001.mot'):('Open both fans and flourish them overhead','evidence/rinrin-native-strips-0.jpg, row 2'),
 ('em/em0440.dat','em0440_0002.mot'):('Lean back and raise the shoulder fan','evidence/rinrin-native-strips-0.jpg, row 3'),
 ('em/em0440.dat','em0440_0003.mot'):('Full-body fan shield — 40-frame hold','evidence/rinrin-native-strips-0.jpg, row 4'),
 ('em/em0440.dat','em0440_0004.mot'):('Full-body fan shield — 25-frame hold','evidence/rinrin-native-strips-1.jpg, row 1'),
 ('em/em0440.dat','em0440_0010.mot'):('Walk with both fans lowered','evidence/rinrin-native-strips-1.jpg, row 2'),
 ('em/em0440.dat','em0440_0020.mot'):('Crouch and spring upward with open fans','evidence/rinrin-native-strips-1.jpg, row 3'),
 ('em/em0440.dat','em0440_0021.mot'):('High vertical leap','evidence/rinrin-native-strips-1.jpg, row 4'),
 ('em/em0440.dat','em0440_0022.mot'):('Descend with fans spread and stand','evidence/rinrin-native-strips-2.jpg, row 1'),
 ('em/em0440.dat','em0440_0023.mot'):('Quick crouch and rise','evidence/rinrin-native-strips-2.jpg, row 2'),
 ('em/em0440.dat','em0440_0024.mot'):('Raised fans into a low crouch','evidence/rinrin-native-strips-2.jpg, row 3'),
 ('em/em0440.dat','em0440_0025.mot'):('Backward aerial somersault','evidence/rinrin-native-strips-2.jpg, row 4'),
 ('pl/pl0000.dat','pl0000_0101.mot'):('Fists raised — guard stance','evidence/jack-guard.jpg'),
 ('em/em0630.dat','em0630_0000.mot'):('Hover with wings spread','evidence/elise-strips-0.jpg, row 1'),
 ('em/em0630.dat','em0630_0001.mot'):('Stand with wings lowered','evidence/elise-strips-0.jpg, row 2'),
 ('em/em0630.dat','em0630_0002.mot'):('Drop into a crouch and rise','evidence/elise-strips-0.jpg, row 3'),
 ('em/em0630.dat','em0630_0003.mot'):('Fold wings, crouch and hop upward','evidence/elise-strips-0.jpg, row 4'),
 ('em/em0630.dat','em0630_0010.mot'):('Walk with wings lowered','evidence/elise-strips-1.jpg, row 1'),
 ('em/em0630.dat','em0630_000a.mot'):('Hold both arms raised','evidence/elise-strips-1.jpg, row 2'),
 ('em/em0630.dat','em0630_000b.mot'):('Hold one arm overhead','evidence/elise-strips-1.jpg, row 3'),
 ('em/em0630.dat','em0630_000c.mot'):('Turn side-on with one wing raised','evidence/elise-strips-1.jpg, row 4'),
 ('em/em0630.dat','em0630_000e.mot'):('Turn away with one arm raised','evidence/elise-strips-2.jpg, row 1'),
 ('em/em0630.dat','em0630_0011.mot'):('Step with arms open','evidence/elise-strips-2.jpg, row 2'),
 ('em/em0630.dat','em0630_0012.mot'):('Reach an arm forward','evidence/elise-strips-2.jpg, row 3'),
 ('em/em0630.dat','em0630_0013.mot'):('Step sideways with wings lowered','evidence/elise-strips-2.jpg, row 4'),
}


FINGERPRINTS={'cff3aa062513a08502226e1de27272a3f4ceba662be140e3539b831818e4c5f6': 'Rest a closed fan on the shoulder', '112d14dc233d1f4d3b05995d428cfba996fca154ccff95e49014976429e5dfb6': 'Open both fans and flourish them overhead', '68a29f4962de65a1d43adda4dbf58a0b82f32e94564f02d535fdb7e4faa17400': 'Lean back and raise the shoulder fan', '33ee4329dbb7a34096c414f08e1de190403c40426453df360d8bce8a5791c4a3': 'Full-body fan shield — 40-frame hold', 'b2600169c3457938ae252c87d339dac892a436c4b517afd1b959c075d1683bf2': 'Full-body fan shield — 25-frame hold', 'ef123c232c1d12496c215b239efa38cc7a8247c268ca25c41d21f3e855ed7841': 'Walk with both fans lowered', '840225f0e8a21ff325eb190ae729a9391bfdba94c8075fdc498b44d91d48e4d5': 'Crouch and spring upward with open fans', 'f5446780fd39ef5e0e6f2e2dfc540b792077f771a1debcfbae1d675c78b6ffca': 'High vertical leap', '1af389f1d173b844aec04ea7c166dce5cec2a79c0820559f1dafe2529338c4fa': 'Descend with fans spread and stand', 'ff777e827c3c7d8d03d4ec433797bfa58c3197ba274eca8731d81ecf78792e48': 'Quick crouch and rise', '1ec2b6e992e819daa17e569f1291db55e05fa603ccaeef8ff013d4dcfa60e188': 'Raised fans into a low crouch', '183c2a0a6d062a2f180225fcb2b89f690b3fdc7c68c44792c5d2ae65fc272389': 'Backward aerial somersault', 'e044e958ef117bf08f10e8661a229a3e0e98af9de5df10815fdc8174f1fb6171': 'Pose with one hand raised beside the face', 'd075e24cc660aa7048eddffa16bdfdbb082f7526b62025d2b460a54d4ff46b7c': 'Walk with one hand raised', '16d3d4736600fcc38411b8f1c1248f48afe3ee0c50aeea94862e7dc167366a3c': 'Turning dance with a low crouch', 'cf84bad2e2b03c64bf7a0d456aeb3ca8326014a147778858cefa95daa7037642': 'Hover with wings spread', '41a92ca14eb6f3033f187f9516d66812c7f02ae53a00def95c5f977bd9483c32': 'Stand with wings lowered', 'bc93aae2644e05bf3f5237c4bb893eff23420cf76984e344cff8b1c2a357f943': 'Drop into a crouch and rise', '3660d63ff755c82e9415460eb6811a81ee7b6a12fa2e606e1c2166f107fd9d7b': 'Fold wings, crouch and hop upward', '49d9d085fb8387ed18e9b7701dfa8aa00e63fed1831ae59acb269bd6262b25fc': 'Walk with wings lowered', '5d28ad899567edbf23f38f5ddf363044cd83188bb9f1c6bf70c8b1c99224e3f0': 'Hold both arms raised', '670473dc816adef31e135e1c92a517d2650aadce6bd43c76fee5b9b8635be219': 'Hold one arm overhead', '94c89d782785166788e7753ef43843f4393f97f2f95572dbd1c8bf21563bf0fa': 'Turn side-on with one wing raised', '500212b9dbcdac57ff78cf4f90a0a4ceb6b7e1798c5a46e8f590d9a39b79c0db': 'Turn away with one arm raised', '20b5c6c3ab62f8870daa8027b5cca383350c835e5335c96a8d78d249ffe61403': 'Step with arms open', '4156bddc5c7d08bffe8f9d9015bd8424a93f4ccfc9d37e2b89eef48c01be177b': 'Reach an arm forward', 'b0b0a9d85f3a8c4cbca950b300f4b30064614e55c7dc589f84439b558e217631': 'Step sideways with wings lowered', 'af1f69b68ddc81910cc4d96b203d07836f03768293bf0ab049b57d97db4d7b3e': 'Fists raised — guard stance'}

def apply(root):
    root=Path(root);path=root/'catalog.json';catalog=json.loads(path.read_text(encoding='utf-8'));annotations=[]
    for c in catalog['clips']:
        row=DESCRIPTIONS.get((c['bank'],c['name']))
        if not row or c['sha256'] not in FINGERPRINTS:continue
        label,evidence=row
        if not (root/evidence.split(',')[0]).exists():continue
        c.update(reviewedLabel=label,labelAuthority='rendered descriptive label; not native action name or gameplay selection',labelEvidence=evidence)
        annotations.append(dict(id=c['id'],sha256=c['sha256'],label=label,evidence=evidence))
    path.write_text(json.dumps(catalog,separators=(',',':')),encoding='utf-8')
    (root/'evidence/reviewed-labels.json').write_text(json.dumps(annotations,indent=2),encoding='utf-8')
    print(f'{len(annotations)} concretely described clips / {len(catalog["clips"])} source motions')


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('root');a=p.parse_args();apply(a.root)
