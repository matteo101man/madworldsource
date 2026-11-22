from save_typed_graph import save_graph
"""Publish a typed index of implemented speaker protocol and PC audio routing."""
import json
from index_ui_layout import ROOT

def main():
    ref=json.loads((ROOT/'research/remote-speaker-reference.json').read_text(encoding='utf-8'))
    rows=[dict(type='node',id='speaker:reference',kind='reference',commit=ref['commit'],files=ref['files'])]
    def node(id,kind,**values):rows.append(dict(type='node',id=id,kind=kind,**values))
    def edge(source,target,kind):rows.append(dict(type='edge',source=source,target=target,kind=kind))
    for report,name in [(20,'enable'),(25,'mute'),(24,'sample-data'),(22,'write-register'),(23,'read-register')]:
        id=f'speaker:report:{report}';node(id,'protocol-report',report=report,name=name);edge(id,'speaker:state','updates')
    node('speaker:state','hardware-state',enabledDefault=False,mutedDefault=False,predictorDefault=0,stepDefault=127,snapshotVersion=5,legacySnapshotDefault='speaker reset; retain original version1..4 byte layout')
    registers=[(0,'sample-data'),(1,'stop/playback-flags'),(2,'format'),(3,'rate-divisor-low'),(4,'rate-divisor-high'),(5,'volume'),(6,'unknown'),(7,'decoder-flags/reset'),(8,'playback-enable'),(9,'input-enable')]
    for offset,name in registers:
        id=f'speaker:register:{offset}';node(id,'hardware-register',offset=offset,name=name);edge('speaker:state',id,'contains')
    node('speaker:decode','codec',formats=['Yamaha ADPCM','s8 PCM','s16le PCM'],unhandledFormats='explicitly unsupported',decodeGate='enabled && input.bit0 && !flags.bit0',playGate='playback.bit0 && !muted && !format.bit0',rate='12000000 / divisor (zero maps65535)',hardwareFifo='not emulated')
    node('speaker:native-usb','native-api',source='native-platform/src/input.rs',method='mw_bt_ioctlv')
    node('speaker:mono-sink','audio-api',source='tools/gecko-reference/crates/gecko/src/audio.rs',method='AudioSink::push_remote_mono')
    node('speaker:pc-mixer','pc-audio-adaptation',source='native-platform/src/speaker.rs',method='SpeakerMixer',clock='AID stereo rate',resampling='bounded linear interpolation',channels='mono centered in left/right',noRemote='unchanged AID stereo block forwarding')
    node('speaker:bevy','audio-output',source='crates/mw-game/src/native_audio.rs',method='AudioBridge -> NativePcm -> Bevy device')
    for a,b in [('speaker:state','speaker:decode'),('speaker:decode','speaker:native-usb'),('speaker:native-usb','speaker:mono-sink'),('speaker:mono-sink','speaker:pc-mixer'),('speaker:pc-mixer','speaker:bevy')]:edge(a,b,'routes-to')
    for path in ['native-speaker-mixer-tests.log','native-speaker-codec-tests.log','native-remote-audio-summary.json']:
        id='speaker:evidence:'+path;node(id,'local-evidence',path='evidence/'+path,scope='See artifact; no full audio/timing equivalence claim');edge('speaker:pc-mixer',id,'tested-by')
    save_graph(ROOT/'research/remote-speaker.graph.jsonl',rows,'Source resource/protocol index; see focused API/INDEX for evidence scope')
    (ROOT/'research/REMOTE-SPEAKER-INDEX.md').write_text('# Remote speaker index\n\nReproduce with tools/index_speaker_protocol.py. remote-speaker.graph.jsonl indexes report IDs, register meanings, gated codec state, the PC mono/stereo route and narrowly scoped evidence. REMOTE-SPEAKER-API.md contains bounded protocol/route pseudocode and limits. remote-speaker-reference.json pins the primary upstream sources. native-remote-audio-summary.json separates baseline and updated replay observations.\n',encoding='utf-8')
    print(json.dumps(dict(records=len(rows),sourceCommit=ref['commit'])))
if __name__=='__main__':main()
