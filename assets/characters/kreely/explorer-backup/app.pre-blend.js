import * as THREE from 'three';
import {OrbitControls} from 'three/addons/controls/OrbitControls.js';
import {exportNative} from './export.js?v=6a13dd9ac571';
import {clone as cloneSkeleton} from 'three/addons/utils/SkeletonUtils.js';
import {colorize,colorProvenance} from './color.js?v=kreely1';
import {inkMaterial} from './ink.js?v=1d0303764c77';
const $=id=>document.getElementById(id), view=$('view');
const renderer=new THREE.WebGLRenderer({antialias:true,preserveDrawingBuffer:true});renderer.setPixelRatio(Math.min(devicePixelRatio,2));view.append(renderer.domElement);
renderer.outputColorSpace=THREE.SRGBColorSpace;
const cameraTools=document.createElement('div');cameraTools.style='position:absolute;right:10px;top:10px;display:flex;gap:5px;z-index:2';view.style.position='relative';view.append(cameraTools);
for(const [label,factor] of [['Zoom in',.8],['Zoom out',1.25]]){const b=document.createElement('button');b.textContent=label==='Zoom in'?'+':'−';b.setAttribute('aria-label',label);b.onclick=()=>{camera.position.sub(orbit.target).multiplyScalar(factor).add(orbit.target);orbit.update()};cameraTools.append(b)}

const scene=new THREE.Scene();scene.background=new THREE.Color('#202729');
const camera=new THREE.PerspectiveCamera(38,1,.01,1000);camera.position.set(3,1.4,4);
const orbit=new OrbitControls(camera,renderer.domElement);orbit.target.set(0,.9,0);orbit.enableDamping=true;
scene.add(new THREE.HemisphereLight(0xffffff,0x52606b,2));const light=new THREE.DirectionalLight(0xffffff,2.5);light.position.set(2,4,3);scene.add(light);
const grid=new THREE.GridHelper(12,24,0x66716f,0x374443);scene.add(grid);
const memo=new Map(),textureMemo=new Map(),modelMemo=new Map();
let eventIndex=null,eventState=null,eventLoading=false,eventBoundaryPending=false;
const PAIRS={'elise-0203':{bank:'em/em0630.dat',a:['pl/pl0030.dat','pl0030.mdb'],b:['em/em0630.dat','em0630.mdb'],ca:'pl0030_0463.mot',cb:'em0630_0203.mot',label:'Jack strikes / Elise falls — paired study (unverified dispatch)',who:['Jack Cayman','Elise']},'rin-aa':{bank:'em/em0440.dat',a:['pl/pl0030.dat','pl0030.mdb'],b:['em/em0440.dat','em0440.mdb'],ca:'pl0030_04aa.mot',cb:'em0440_0110.mot',label:'Rin Power Struggle · AA / 0110 start (paired study)',who:['Jack Cayman','RinRin']},'rin-ab':{bank:'em/em0440.dat',a:['pl/pl0030.dat','pl0030.mdb'],b:['em/em0440.dat','em0440.mdb'],ca:'pl0030_04ab.mot',cb:'em0440_0111.mot',label:'Rin Power Struggle · AB / 0111 struggle (paired study)',who:['Jack Cayman','RinRin']},'rin-ac':{bank:'em/em0440.dat',a:['pl/pl0030.dat','pl0030.mdb'],b:['em/em0440.dat','em0440.mdb'],ca:'pl0030_04ac.mot',cb:'em0440_0112.mot',label:'Rin Power Struggle · AC / 0112 Jack WINS (paired study)',who:['Jack Cayman','RinRin']},'rin-ad':{bank:'em/em0440.dat',a:['pl/pl0030.dat','pl0030.mdb'],b:['em/em0440.dat','em0440.mdb'],ca:'pl0030_04ad.mot',cb:'em0440_0113.mot',label:'Rin Power Struggle · AD / 0113 Jack LOSES (paired study)',who:['Jack Cayman','RinRin']},'rin-sc':{bank:'custom/clips',a:['pl/pl0030.dat','pl0030.mdb'],b:['em/em0440.dat','em0440.mdb'],ca:'pl0030_04adsc.mot',cb:'em0440_0113sc.mot',label:'Rin Power Struggle · ALT loss: fan scissors on neck (added animation)',who:['Jack Cayman','RinRin'],sever:{main:[4]}}};let pairKey='elise-0203';
window.__mwPoseAt=t=>pose(t);window.__mwState=()=>({severed:pairState?.severed??null});window.__mwView=(p,t)=>{camera.position.set(...p);orbit.target.set(...t);orbit.update()};window.__mwBoneWorld=(role,id)=>{const st=role==='pair'?pairState:{data,bones};const i=st.data.bones.findIndex(b=>b.id===id);const v=new THREE.Vector3();st.bones[i].updateWorldMatrix(true,false);st.bones[i].getWorldPosition(v);return v.toArray().map(x=>+x.toFixed(3))};let catalog,selected,data,actor,bones=[],helper,clip=null,clipItem=null,time=0,playing=true,request=0,loadMs=0,pairState=null,pairLoading=false,clipRequest=0,uiLocks=0;
async function get(url){if(!memo.has(url))memo.set(url,fetch(url).then(async r=>{let d=await r.json();if(!r.ok)throw Error(d.error||r.statusText);return d}).catch(e=>{memo.delete(url);throw e}));return memo.get(url)}
function lockUi(delta){uiLocks=Math.max(0,uiLocks+delta);for(const el of document.querySelectorAll('button,select,input'))el.disabled=uiLocks>0||(el.id==='inplace'&&!!eventState);view.setAttribute('aria-busy',String(uiLocks>0))}
function fail(e){$('error').textContent=String(e);console.error(e)}
function resize(){const w=view.clientWidth,h=view.clientHeight;renderer.setSize(w,h);camera.aspect=eventState&&$('event-camera').checked?16/9:w/h;camera.updateProjectionMatrix()};new ResizeObserver(resize).observe(view);
function attribute(values,width,type=Float32Array){return new THREE.BufferAttribute(new type(values.flat()),width)}
async function texture(item,index,bank="primary"){const url=`/api/texture?id=${encodeURIComponent(item.id)}&index=${index}&bank=${bank}`;if(!textureMemo.has(url))textureMemo.set(url,new THREE.TextureLoader().loadAsync(url).then(t=>{t.colorSpace=bank==='ab'?THREE.NoColorSpace:THREE.SRGBColorSpace;t.wrapS=t.wrapT=THREE.RepeatWrapping;t.flipY=false;return t}));return textureMemo.get(url)}
function setMaterials(){if(!actor)return;const colored=$('colorize').checked;for(const root of [actor,...(pairState?[pairState.actor]:[]),...(eventState?eventState.roles.map(r=>r.actor):[])]){root.userData.colorMode=colored?'added inspection palette':'source monochrome';root.traverse(o=>{if(!o.isMesh||o.userData.fxPart)return;const type=$('material').value;o.material=type==='normal'?o.normalMat:type==='ink'?(colored?o.colorInkMat:o.inkMat):type==='lit'?(colored?o.colorLitMat:o.litMat):(colored?o.colorMat:o.sourceMat)})}if(selected)details();}
function auxiliary(name){return false}
function meshVisibility(){if(!actor)return;actor.traverse(o=>{if(o.isMesh)o.visible=o.userData.enabled!==false});}
function bounds(root){const box=new THREE.Box3();root.traverse(o=>{if(o.isMesh&&o.visible){if(o.isSkinnedMesh){o.computeBoundingBox();box.union(o.boundingBox.clone().applyMatrix4(o.matrixWorld))}else{o.geometry.computeBoundingBox();box.union(o.geometry.boundingBox.clone().applyMatrix4(o.matrixWorld))}}});return box}
function frame(){if(!actor)return;const box=eventState?eventBounds():bounds(actor);if(pairState)box.union(bounds(pairState.actor));if(box.isEmpty())box.setFromObject(actor);const size=box.getSize(new THREE.Vector3()),c=box.getCenter(new THREE.Vector3()),radius=Math.max(size.x,size.y,size.z,.1);orbit.target.copy(c);camera.position.copy(c).add(new THREE.Vector3(radius*.85,radius*.25,radius*1.65));camera.near=Math.max(.001,radius/500);camera.far=Math.max(100,radius*100);camera.updateProjectionMatrix();orbit.update();}
async function loadModel(id){
 lockUi(1);clipRequest++;try{
 if(!eventLoading)clearEvent();
 if(!pairLoading)clearPair();
 const token=++request,start=performance.now();$('error').textContent='';const item=catalog.models.find(x=>x.id===id);if(!item)throw Error('Unknown model');
 const native=await get('/api/model?id='+encodeURIComponent(id));
 if(token!==request)return;
 let entry=modelMemo.get(id);
 if(!entry){
  const root=new THREE.Group(),bb=native.bones.map(b=>{const o=new THREE.Bone();o.name='bone'+b.id;o.position.fromArray(b.relative);return o});
  const origin=new THREE.Bone();origin.name='bone-1';bb.push(origin);root.add(origin);
  native.bones.forEach((b,i)=>{(b.parent>=0?bb[b.parent]:b.parent===-1?origin:root).add(bb[i])});root.updateMatrixWorld(true);
  const skeleton=new THREE.Skeleton(bb);skeleton.calculateInverses();
  for(const m of native.meshes){
   const g=new THREE.BufferGeometry();g.setAttribute('position',attribute(m.positions,3));if(m.normals.length)g.setAttribute('normal',attribute(m.normals,3));
   if(m.uvs.length)g.setAttribute('uv',attribute(m.uvs,2));g.setIndex(m.indices);if(!m.normals.length)g.computeVertexNormals();
   let map=null;if(item.texture&&m.texture>=0)try{map=await texture(item,m.texture)}catch(e){fail(e)}
   const mat=new THREE.MeshBasicMaterial({map,color:map?0xffffff:0xc7c3b7,side:THREE.DoubleSide,alphaTest:0});
   const obj=m.joints.length?new THREE.SkinnedMesh(g,mat):new THREE.Mesh(g,mat);obj.name=m.name;
   if(m.joints.length){g.setAttribute('skinIndex',attribute(m.joints,4,Uint16Array));g.setAttribute('skinWeight',attribute(m.weights,4));root.add(obj);obj.bind(skeleton,new THREE.Matrix4())}else root.add(obj);
   obj.frustumCulled=false;obj.sourceMat=mat;obj.litMat=new THREE.MeshStandardMaterial({map,color:mat.color,side:THREE.DoubleSide,roughness:.9,alphaTest:0});obj.normalMat=new THREE.MeshNormalMaterial({side:THREE.DoubleSide});obj.userData.enabled=!native.hiddenMeshes?.includes(m.name);
  }
  colorize(root,item,native);
  if(item.custom?.colorTexture)try{const ct=await texture(item,0,'color');root.traverse(o=>{if(o.isMesh&&!(native.meshes.find(m=>m.name===o.name)?.texture>0))for(const k of ['colorMat','colorLitMat']){o[k].map=ct;o[k].vertexColors=false;o[k].needsUpdate=true}})}catch(e){fail(e)}
  for(const mesh of native.meshes){const obj=root.getObjectByName(mesh.name),program=catalog.materialPrograms?.profiles?.[mesh.renderFlags&0x2000?'white':mesh.renderFlags&0x1000?'black':''];let ab=null;if(item.textureAB&&mesh.texture>=0)try{ab=await texture(item,mesh.texture,'ab')}catch(e){fail(e)}obj.inkMat=inkMaterial(obj.sourceMat,ab,mesh,program);obj.colorInkMat=inkMaterial(obj.colorMat,ab,mesh,program);}
  if(native.muzzles&&item.custom?.fxTexture)try{await addMuzzleFx(root,bb,native,item)}catch(e){fail(e)}
  entry={root,bones:bb,native};modelMemo.set(id,entry);
 }
 if(token!==request)return;
 if(actor)scene.remove(actor);if(helper)scene.remove(helper);
 selected=item;data=native;actor=entry.root;bones=entry.bones;scene.add(actor);helper=new THREE.SkeletonHelper(actor);helper.visible=$('skeleton').checked;scene.add(helper);
 clip=null;clipItem=null;time=0;loadMs=performance.now()-start;
 $('title').textContent=item.displayName||item.name;
 $('notice').textContent=item.custom?`Added character, not a disc resource: ${item.identityAuthority}. Rigged on ${item.custom.skeleton} bones; animations bound to the ${item.custom.motionFamily} motion family. See ${item.custom.notes}.`:item.review==='reviewed'?(item.identity.startsWith('Unidentified')?'Rendered woman; proper identity remains unresolved.':'Identity reviewed from rendered appearance; native identity links and animation ownership are separately documented.'):'Unreviewed native model. Character identity and animation ownership are unresolved.';
 $('meshes').replaceChildren();actor.traverse(o=>{if(o.isMesh&&!o.userData.fxPart){const l=document.createElement('label'),c=document.createElement('input');c.type='checkbox';c.checked=o.userData.enabled!==false;c.onchange=()=>{o.userData.enabled=c.checked;meshVisibility()};l.append(c,document.createTextNode(o.name));$('meshes').append(l)}});
 $('hands-wrap').hidden=!native.handPoses;if(native.handPoses){$('hands').replaceChildren(new Option('auto (clip)',''),...Object.keys(native.handPoses).map(k=>new Option(k,k)));$('hands').value=native.handPose||''}
 setMaterials();meshVisibility();pose(0);frame();menus();eventMenus();cards();details();window.study.ready=true;return item;
 }finally{lockUi(-1)}
}
function compatible(c,native=data){const ids=new Set(native.bones.map(b=>b.id)),ignored=new Set(native.ignoreMotionBones||[]);return c.bones.every(b=>b===-1||b>=0xf60||ids.has(b)||ignored.has(b))}
function motionFamily(item){return item.custom?.motionFamily||item.name.replace(/(?:_LOD\d)?\.mdb$/,'')}
function motionBank(item){return item.custom?item.custom.skeleton.split('#')[0]:item.bank}
function menus(){if(!selected)return;const needle=$('motion-search').value.toLowerCase(),mode=$('scope').value,stem=motionFamily(selected),bank=motionBank(selected);const choices=catalog.clips.filter(c=>(mode==='compatible'?compatible(c):mode==='bank'?c.bank===bank:mode==='reviewed'?c.reviewedLabel&&c.name.startsWith(stem+'_'):c.name.startsWith(stem+'_')&&compatible(c))&&(c.reviewedLabel||c.name).toLowerCase().includes(needle));$('motion-empty').hidden=choices.length>0;$('motion-empty').textContent=needle?'No matching source clips.':mode==='reviewed'?'No reviewed descriptions for this source family.':'No decoded single clips match this source filter. All compatible animations remain available for experimental inspection.';choices.sort((a,b)=>Number(!!b.reviewedLabel)-Number(!!a.reviewedLabel)||Number(b.bank===bank)-Number(a.bank===bank)||a.name.localeCompare(b.name));const old=$('motion').value;$('motion').replaceChildren(new Option('Bind pose',''));if(selected.custom&&data?.overlays){const og=document.createElement('optgroup');og.label='Secondary weapon: handgun';for(const [id,o] of Object.entries(data.overlays).sort((x,y)=>(x[1].style==='realistic'?0:1)-(y[1].style==='realistic'?0:1))){if(!needle||o.label.toLowerCase().includes(needle))og.append(new Option(o.label,id))}if(og.children.length)$('motion').append(og)}for(const c of choices)$('motion').add(new Option(`${c.reviewedLabel||c.name} · ${c.frames} frames`,c.id));if(!selected.custom)for(const[k,p]of Object.entries(PAIRS))if(pairState||p.who.includes(selected.identity))$('motion').add(new Option(p.label,'pair:'+k));for(const e of selected.custom?[]:eventIndex?.events||[]){if(!e.actors.some(a=>a.model===selected.id||(selected.identity==='Jack Cayman'&&a.object==='pl0030')))continue;for(const cut of e.cuts){const bindings=e.castsByCut.get(cut.index)||[];if(bindings.length<2)continue;const label=`Native cast · ${e.id} / cut ${cut.index} · ${bindings.map(a=>catalog.models.find(m=>m.id===a.model)?.identity?.replace('Unreviewed native model',a.object)||a.object).join(' + ')}`;if(!needle||label.toLowerCase().includes(needle))$('motion').add(new Option(label,`event:${e.id}:${cut.index}`));}}if([...$('motion').options].some(o=>o.value===old))$('motion').value=old;}
async function loadClip(id){
 if(!eventLoading)clearEvent();
 if(id?.startsWith('pair:'))return loadPair(id.slice(5));
 if(id?.startsWith('event:')){const [,event,cut]=id.split(':');return loadEvent(event,+cut);}
 lockUi(1);const token=++clipRequest,modelAtStart=selected?.id;
 try{if(!pairLoading)clearPair();$('error').textContent='';
  if(!id){clip=clipItem=null;time=0;pose(0);details();return}
  $('clock').textContent='Loading motion…';
  const item=catalog.clips.find(c=>c.id===id.split('|')[0]),result=await get(item?.url?item.url+'?sha='+item.sha256:'/api/clip?id='+encodeURIComponent(item?item.id:id));
  if(token!==clipRequest||selected?.id!==modelAtStart)return;
  const ovk=data?.overlays?.[id];let res=result;if(ovk?.range){const [ra,rb]=ovk.range;res={...result,frames:rb-ra+1,tracks:result.tracks.map(tr=>({...tr,values:tr.values.slice(ra,rb+1)}))}}else if(!ovk)res.id=item.id;res.id=id;clip=res;clipItem=item;time=0;$('seek').max=res.frames-1;$('seek').value=0;pose(0);details();return result;
 }finally{lockUi(-1)}
}

// Added-character muzzle FX: em0400.eff TEX sheet (blast streak + ring) on the gun bone, timed by the overlay's shots.
async function addMuzzleFx(root,bb,native,item){const tex=await texture(item,0,'fx');
 const quad=(pts,uvs)=>{const g=new THREE.BufferGeometry();g.setAttribute('position',new THREE.BufferAttribute(new Float32Array(pts.flat()),3));g.setAttribute('uv',new THREE.BufferAttribute(new Float32Array(uvs.flat()),2));g.setIndex([0,1,2,0,2,3]);return g};
 const mat=()=>new THREE.MeshBasicMaterial({map:tex,transparent:true,depthWrite:false,side:THREE.DoubleSide,color:0xffffff});
 const L=.34,W=.17,R=.2;
 for(const [name,p] of Object.entries(native.muzzles.points)){const m=native.meshes.find(x=>x.name===name);if(!m)continue;
  const g=new THREE.Group();g.position.fromArray(p);g.userData.fx={mesh:name,side:/_L_/.test(name)?'L':'R'};g.visible=false;
  const bu=[[.5,0],[0,0],[0,.5],[.5,.5]];   // streak quadrant: muzzle bar at u=.5, blast toward u=0
  for(const plane of [[[-W/2,0,0],[-W/2,-L,0],[W/2,-L,0],[W/2,0,0]],[[0,0,-W/2],[0,-L,-W/2],[0,-L,W/2],[0,0,W/2]]]){const b=new THREE.Mesh(quad(plane,[[.5,0],[0,0],[0,.5],[.5,.5]]),mat());b.userData.fxPart='blast';g.add(b)}
  const ring=new THREE.Mesh(quad([[-R/2,-.03,-R/2],[R/2,-.03,-R/2],[R/2,-.03,R/2],[-R/2,-.03,R/2]],[[.5,.5],[1,.5],[1,1],[.5,1]]),mat());ring.userData.fxPart='ring';g.add(ring);
  bb[m.joints[0][0]].add(g)}
}
function updateMuzzleFx(actor,data,clip,t){if(!actor||!data?.muzzles)return;const ov=clip&&data.overlays?.[clip.id],tint=$('colorize').checked?0xffd890:0xffffff;
 actor.traverse(o=>{const fx=o.userData.fx;if(!fx)return;const mesh=actor.getObjectByName(fx.mesh),shot=ov?.shots.find(s=>s.gun===fx.side&&t>=s.frame&&t<s.frame+4);
  o.visible=!!shot&&mesh?.userData.enabled!==false;if(!o.visible)return;const age=t-shot.frame,a=1-age/4;
  for(const c of o.children){c.material.opacity=a;c.material.color.setHex(tint);if(c.userData.fxPart==='ring')c.scale.setScalar(.5+age*.45);else c.scale.set(1,.7+age*.25,1)}});
}
function applyPoseOffsets(data,bones,role){const o=window.__mwOffsets?.[role];if(!o||!data)return;for(const[id,d]of Object.entries(o)){const i=data.bones.findIndex(b=>b.id===+id);if(i<0)continue;const r=bones[i].rotation;r.x+=d[0]||0;r.y+=d[1]||0;r.z+=d[2]||0;}}
function severActor(root,data,ids){/* Added-animation dismemberment: split skinning at the given bones so no vertex blends the severed subtree with the rest of the body. Geometry is cloned; source model data is untouched. */const cut=new Set();for(let i=0;i<data.bones.length;i++){let j=i;while(j>=0){if(ids.includes(data.bones[j].id)){cut.add(i);break}j=data.bones[j].parent;if(j===undefined||j===null)break}}let split=0;root.traverse(o=>{if(!o.isSkinnedMesh)return;const g=o.geometry.clone(),si=g.attributes.skinIndex,sw=g.attributes.skinWeight;for(let v=0;v<si.count;v++){let a=0,b=0;for(let k=0;k<4;k++){const w=sw.getComponent(v,k);if(!w)continue;if(cut.has(si.getComponent(v,k)))a+=w;else b+=w}if(!a||!b)continue;split++;const keepCut=a>=b;for(let k=0;k<4;k++){if(cut.has(si.getComponent(v,k))!==keepCut)sw.setComponent(v,k,0)}let t=0;for(let k=0;k<4;k++)t+=sw.getComponent(v,k);for(let k=0;k<4;k++)sw.setComponent(v,k,sw.getComponent(v,k)/t)}sw.needsUpdate=true;const side=v=>{let a=0,b=0;for(let k=0;k<4;k++){const w=sw.getComponent(v,k);if(cut.has(si.getComponent(v,k)))a+=w;else b+=w}return a>b};if(g.index){const ix=g.index.array,keep=[];for(let t=0;t+2<ix.length;t+=3){const s0=side(ix[t]);if(side(ix[t+1])===s0&&side(ix[t+2])===s0)keep.push(ix[t],ix[t+1],ix[t+2]);else split++}g.setIndex(keep)}else{const n=g.attributes.position.count,keep=[];for(let t=0;t+2<n;t+=3){const s0=side(t);if(side(t+1)===s0&&side(t+2)===s0)keep.push(t,t+1,t+2);else split++}g.setIndex(keep)}o.geometry=g});return split}
// Added characters: left<->right bone pairs for mirrored overlay clips (x positions and y/z ZYX angles flip sign)
const MIRROR_BONES=Object.fromEntries([[5,10],[6,11],[7,12],[8,13],[9,14],[16,20],[17,21],[18,22],[19,23],[40,41],[50,51],[100,101]].flatMap(([a,b])=>[[a,b],[b,a]]));
function sampleActor(data,bones,actor,clip,t,inplace=false){if(!data)return;for(let i=0;i<bones.length;i++){bones[i].position.fromArray(i<data.bones.length?data.bones[i].relative:[0,0,0]);bones[i].rotation.set(0,0,0,'ZYX');bones[i].scale.set(1,1,1)}
 const map=new Map(data.bones.map((x,i)=>[x.id,i]));map.set(-1,bones.length-1);const rt=data.retarget,ov=clip&&data.overlays?.[clip.id],hands=data.handPoses?.[data.handPose||ov?.handPose||data.defaultHandPose];
 if(hands)for(const [id,q] of Object.entries(hands)){const i=map.get(+id);if(i!==undefined)bones[i].quaternion.fromArray(q)}
 for(const id of data.propBones||[]){const i=map.get(id);if(i!==undefined)bones[i].scale.set(0,0,0)}
 if(clip){const a=Math.min(clip.frames-1,Math.floor(t)),b=Math.min(clip.frames-1,a+1),f=t-a,rs=(ov&&rt?.rootScaleByFamily?.[ov.family])??rt?.rootScale,mir=ov?.mirror;
 for(const track of clip.tracks){let id=track.bone,ch=track.channel,sg=1;if(mir){id=MIRROR_BONES[id]??id;if(ch===0||ch===4||ch===5)sg=-1}const i=map.get(id);if(i===undefined)continue;const bone=bones[i],root=id===-1||id===0;let v=(track.values[a]*(1-f)+track.values[b]*f)*sg;if(ch<3){if(rt?.rotationOnly&&!root)continue;if(id===-1&&ov?.rootOrigin)v-=ov.rootOrigin[ch]*(mir&&ch===0?-1:1);bone.position.setComponent(ch,root&&rt?v*rs:v)}else if(ch<6)bone.rotation[['x','y','z'][ch-3]]=v;else if(ch>=7&&ch<=9)bone.scale.setComponent(ch-7,v)}
 if(ov?.rootLift!=null){const L=ov.rootLift,lv=Array.isArray(L)?L[a]*(1-f)+L[b]*f:L;bones[bones.length-1].position.y+=lv}
 if(ov)for(const track of ov.tracks){const i=map.get(track.bone);if(i===undefined)continue;const bone=bones[i],v=track.values[a]*(1-f)+track.values[b]*f,ch=track.channel;if(ch<3)bone.position.setComponent(ch,v);else if(ch<6)bone.rotation[['x','y','z'][ch-3]]=v;else if(ch>=7&&ch<=9)bone.scale.setComponent(ch-7,v)}
 if(inplace){bones[bones.length-1].position.x=0;bones[bones.length-1].position.z=0;bones[0].position.x=data.bones[0].relative[0];bones[0].position.z=data.bones[0].relative[2]}
 }
 actor.updateMatrixWorld(true);
}
function pose(t){if(eventState){poseEvent(t);return}sampleActor(data,bones,actor,clip,t,$('inplace').checked);applyPoseOffsets(data,bones,'main');updateMuzzleFx(actor,data,clip,t);if(pairState){sampleActor(pairState.data,pairState.bones,pairState.actor,pairState.clip,t,$('inplace').checked);applyPoseOffsets(pairState.data,pairState.bones,'pair');if($('pair-follow')?.checked&&!$('inplace').checked){const jr=bones[bones.length-1],rr=pairState.bones[pairState.bones.length-1];jr.position.x+=rr.position.x;jr.position.z+=rr.position.z;}pairState.helper.visible=$('skeleton').checked;pairState.helper.updateMatrixWorld(true)}helper?.updateMatrixWorld(true);$('clock').textContent=clip?`${t.toFixed(1)} / ${clip.frames-1} frames`:'Bind pose';$('seek').value=t;}
function clearPair(){if(!pairLoading){$('performer').value='';$('receiver').value=''}if(pairState){scene.remove(pairState.actor,pairState.helper);pairState=null}$('pair-info')?.setAttribute('hidden','');$('pair-controls')?.setAttribute('hidden','');}
async function loadPair(key){if(key)pairKey=key;const P=PAIRS[pairKey];lockUi(1);pairLoading=true;try{clearPair();const bank=P.bank;const a=catalog.models.find(m=>m.id===$('performer').value)||catalog.models.find(m=>m.bank===P.a[0]&&m.name===P.a[1]),b=catalog.models.find(m=>m.id===$('receiver').value)||catalog.models.find(m=>m.bank===P.b[0]&&m.name===P.b[1]);await loadModel(a.id);await loadClip(catalog.clips.find(c=>c.bank===bank&&c.name===P.ca).id);const primary={selected,data,actor,bones,clip,clipItem};const cloned=cloneSkeleton(actor);cloned.traverse(o=>{if(o.isMesh){const original=actor.getObjectByName(o.name);for(const key of ['sourceMat','litMat','normalMat','colorMat','colorLitMat','inkMat','colorInkMat'])o[key]=original[key]}});primary.actor=cloned;primary.bones=bones.map(b=>cloned.getObjectByName(b.name));await loadModel(b.id);await loadClip(catalog.clips.find(c=>c.bank===bank&&c.name===P.cb).id);pairState={data,actor,bones,clip,clipItem,helper,selected};({selected,data,actor,bones,clip,clipItem}=primary);if(P.sever?.main)pairState.severed=severActor(actor,data,P.sever.main);if(P.sever?.pair)severActor(pairState.actor,pairState.data,P.sever.pair);scene.add(actor);helper=new THREE.SkeletonHelper(actor);helper.visible=$('skeleton').checked;scene.add(helper);time=0;$('seek').max=Math.max(clip.frames,pairState.clip.frames)-1;menus();$('motion').value='pair:'+pairKey;$('title').textContent='Paired native-motion inspection';$('pair-info').removeAttribute('hidden');$('pair-controls').removeAttribute('hidden');$('performer').value=a.id;$('receiver').value=b.id;$('pair-info').textContent=P.label+' — '+(Math.max(clip.frames,pairState.clip.frames))+' source frames on one clock. Native dispatch, contact alignment and continuation are unverified. Role swaps are experimental; source details below retain both clips and missing bindings.';pose(0);setMaterials();frame();cards();details()}finally{pairLoading=false;lockUi(-1)}}
function details(){$('details').textContent=JSON.stringify({model:selected,clip:clipItem,bones:data?.bones.length,loadMilliseconds:+loadMs.toFixed(1),rotationOrder:'ZYX per upstream MadWorld reader; runtime render basis unverified',fps:'60 upstream preview; native timebase unverified',auxiliaryPolicy:'All native mesh parts visible by default; mesh suffixes do not establish draw policy',texturePolicy:'GX CMPR interpolation/intensity alpha corrected; native ink dispatch and per-mesh threshold recovered. Preview lighting, shadow depths, konst uniforms and visibility remain partial',colorMode:$('colorize').checked?colorProvenance:'Source textures; added color disabled',nativeInkMaterials:data?.meshes.map(m=>({mesh:m.name,renderFlags:m.renderFlags,threshold:m.inkThreshold,mode:m.renderFlags&0x2000?'white base':m.renderFlags&0x1000?'black base':'other'})),event:eventState?{id:eventState.descriptor.id,cut:eventState.cut,actors:eventState.roles.map(r=>({object:r.binding.object,instance:r.binding.instance,model:r.selected.id,clip:r.clipItem?.id})),timing:'Native base cut-step arithmetic; cadence and SEQ timing controls unverified',visibility:'Missing cuts hidden; SEQ opcodes preserved, not executed',failures:eventState.failures}:null,compatibility:'bone ID subset only; does not prove authored ownership',animationBinding:selected?.custom?{skeleton:selected.custom.skeleton,motionFamily:selected.custom.motionFamily,policy:'per-character binding in custom/characters.json; swap motionFamily when own clips exist',retarget:data?.retarget,ignoredMotionBones:data?.ignoreMotionBones,handPose:data?.handPose||data?.defaultHandPose}:undefined,unmappedMotionBones:clip?[...new Set(clip.tracks.filter(t=>t.bone!==-1&&!data.bones.some(b=>b.id===t.bone)).map(t=>t.bone))]:[],pairedDispatch:'unresolved',pair:pairState?{receiverModel:pairState.selected,receiverClip:pairState.clipItem,alignment:'source roots with zero added world offset; native attachment/facing unresolved',receiverUnmapped:pairState.clip.tracks.filter(t=>t.bone!==-1&&!pairState.data.bones.some(b=>b.id===t.bone)).length}:null},null,2)}
function eventMenus(){
 if(!eventIndex||!selected)return;
 const old=$('event').value,stem=selected.name.replace(/(?:_LOD\d)?\.mdb$/,''),choices=eventIndex.events.filter(e=>e.actors.some(a=>a.object===stem||(selected.identity==='Jack Cayman'&&a.object==='pl0030')));
 $('event').replaceChildren(new Option(choices.length?'Choose native event':'No event actor binding',''));
 for(const e of choices)$('event').add(new Option(`${e.id} · ${e.cuts.length} cuts · ${e.actors.filter(a=>/^(em|pl)/.test(a.object)).map(a=>a.object).join(', ')}`,e.id));
 if(choices.some(e=>e.id===old))$('event').value=old;
}
function cloneRole(source){
 const root=cloneSkeleton(source);
 root.traverse(o=>{if(o.isMesh){const original=source.getObjectByName(o.name);for(const key of ['sourceMat','litMat','normalMat','colorMat','colorLitMat','inkMat','colorInkMat'])o[key]=original[key]}});
 return root;
}
function clearEvent(){
 if(eventLoading)return;
 if(eventState){for(const r of eventState.roles)scene.remove(r.actor,r.helper);eventState=null}
 if(actor)actor.visible=true;$('export').textContent='Export native rig + clip GLB';
 orbit.enabled=true;$('event-info').hidden=true;
}
function eventBounds(){const box=new THREE.Box3();for(const r of eventState.roles)if(r.actor.visible)box.union(bounds(r.actor));return box}
function poseEvent(t){
 for(const r of eventState.roles){sampleActor(r.data,r.bones,r.actor,r.clip,t,false);r.helper.visible=$('skeleton').checked;r.helper.updateMatrixWorld(true)}
 if($('event-camera').checked&&eventState.camera){
  const samples=eventState.camera.samples,a=Math.min(samples.length-1,Math.floor(t)),b=Math.min(samples.length-1,a+1),f=t-a,v=samples[a].map((x,i)=>x*(1-f)+samples[b][i]*f);
  camera.position.fromArray(v);camera.up.set(Math.sin(v[6]),Math.cos(v[6]),0);camera.lookAt(new THREE.Vector3(...v.slice(3,6)));camera.fov=THREE.MathUtils.radToDeg(v[7]);camera.aspect=16/9;camera.near=.01;camera.far=10000;camera.updateProjectionMatrix();orbit.enabled=false;
 }else{orbit.enabled=true;camera.up.set(0,1,0);camera.aspect=view.clientWidth/view.clientHeight;camera.updateProjectionMatrix()}
 $('clock').textContent=`${eventState.descriptor.id} / cut ${eventState.cut} · ${t.toFixed(1)} / ${eventState.frames-1} frames`;$('seek').value=t;
}
async function loadEvent(id,cut=0){
 if(!id)return;lockUi(1);clearEvent();clearPair();eventLoading=true;
 const original=selected,descriptor=eventIndex.events.find(e=>e.id===id),roles=[],failures=[];
 try{
  const span=descriptor.cuts[cut];if(!span)throw Error('Missing camera cut');
  for(const binding of descriptor.actors){
   const item=eventIndex.clips.find(c=>binding.clips.includes(c.id)&&c.cut===cut);if(!item)continue;
   if(!binding.model){failures.push({actor:binding.object,reason:'No exact native MDB binding'});continue}
   try{
    await loadModel(binding.model);const motion=await get('/api/clip?id='+encodeURIComponent(item.id));
    if(!compatible(item)){failures.push({actor:binding.object,reason:'Motion has unmapped bones'});continue}
    const root=cloneRole(actor),bb=bones.map(b=>root.getObjectByName(b.name)),sh=new THREE.SkeletonHelper(root);roles.push({selected,data,actor:root,bones:bb,clip:motion,clipItem:item,binding,helper:sh});
   }catch(e){failures.push({actor:binding.object,reason:String(e)})}
  }
  await loadModel(original.id);actor.visible=false;clip=null;clipItem=null;if(helper)helper.visible=false;
  const cam=await get(`/api/event-camera?id=${encodeURIComponent(id)}&cut=${cut}`);
  eventState={descriptor,cut,roles,failures,camera:cam,frames:span.frames};for(const r of roles)scene.add(r.actor,r.helper);
  time=0;$('seek').max=span.frames-1;$('event').value=id;$('event-cut').replaceChildren();for(const c of descriptor.cuts)$('event-cut').add(new Option(`${c.index} · frames ${c.start}–${c.start+c.frames-1}`,String(c.index)));$('event-cut').value=String(cut);
  $('event-info').hidden=false;$('event-info').textContent=`${id}: ${roles.length} native actor bindings in cut ${cut}. ${span.frames} camera samples; ${failures.length} unresolved bindings. Native cut IDs and actor motions; SEQ effects/visibility are not yet executed. Actors without this cut are omitted.`;
  $('title').textContent=`${id} · Native event cast · Cut ${cut}`;menus();$('motion').value=`event:${id}:${cut}`;$('export').textContent='Export current event cut GLB';setMaterials();pose(0);if(!$('event-camera').checked)frame();details();prefetchEvent(descriptor,(cut+1)%descriptor.cuts.length);
 }catch(e){for(const r of roles)scene.remove(r.actor,r.helper);if(actor)actor.visible=true;throw e}finally{eventLoading=false;lockUi(-1)}
}
function prefetchEvent(descriptor,cut){const items=eventIndex.clips.filter(c=>c.event===descriptor.id&&c.cut===cut);Promise.allSettled(items.map(c=>get('/api/clip?id='+encodeURIComponent(c.id)))).then(()=>{});}
function advanceEvent(){if(eventBoundaryPending||!eventState)return;const {descriptor,cut}=eventState;let next=cut+1;if(next>=descriptor.cuts.length){if($('loop').checked)next=0;else{playing=false;$('play').textContent='Play';time=eventState.frames-1;pose(time);return}}eventBoundaryPending=true;loadEvent(descriptor.id,next).catch(fail).finally(()=>{eventBoundaryPending=false});}
$('event-load').onclick=()=>loadEvent($('event').value).catch(fail);
$('event-cut').onchange=()=>eventState&&loadEvent(eventState.descriptor.id,+$('event-cut').value).catch(fail);
$('event-leave').onclick=()=>{clearEvent();$('motion').value='';pose(0);frame();details();if(selected)$('title').textContent=selected.displayName||selected.name};
$('event-camera').onchange=()=>{pose(time);if(!$('event-camera').checked)frame()};
function cards(){const q=$('search').value.toLowerCase(),mode=$('roster').value;let items=catalog.models.filter(m=>mode==='all'||m.custom||mode==='reviewed'&&m.review==='reviewed'||mode==='actors'&&m.bones>15&&/^(em|pl|ev)\//.test(m.bank));if(mode==='reviewed'&&!items.length)items=catalog.models.filter(m=>m.bank==='pl/pl0000.dat'&&!m.name.includes('LOD'));items=items.filter(m=>(m.id+' '+(m.displayName||'')).toLowerCase().includes(q));$('cards').replaceChildren();for(const item of items){const b=document.createElement('button');b.className='card'+(selected?.id===item.id?' active':'');b.append(document.createTextNode(item.displayName||item.name));b.append(document.createElement('br'));const s=document.createElement('small');s.textContent=`${item.bones} bones · ${item.bank}`;b.append(s);b.onclick=()=>loadModel(item.id).catch(fail);$('cards').append(b)}}
$('search').oninput=cards;$('roster').onchange=cards;$('scope').onchange=menus;$('motion-search').oninput=menus;$('motion').onchange=()=>loadClip($('motion').value).catch(fail);
for(const [id,step] of [['prev',-1],['next',1]])$(id).onclick=()=>{const s=$('motion');s.selectedIndex=(s.selectedIndex+step+s.options.length)%s.options.length;loadClip(s.value).catch(fail)};
$('play').onclick=()=>{playing=!playing;$('play').textContent=playing?'Pause':'Play'};$('frame').onclick=()=>{if(eventState)$('event-camera').checked=false;frame()};$('skeleton').onchange=()=>{helper.visible=$('skeleton').checked;if(pairState)pairState.helper.visible=$('skeleton').checked;if(eventState)pose(time)};$('inplace').onchange=()=>pose(time);if($('pair-follow'))$('pair-follow').onchange=()=>pose(time);$('material').onchange=setMaterials;$('hands').onchange=()=>{if(data?.handPoses){data.handPose=$('hands').value||undefined;pose(time);details()}};$('colorize').onchange=setMaterials;$('bind').onclick=()=>{clearEvent();clearPair();clip=null;clipItem=null;$('motion').value='';time=0;pose(0);details()};$('seek').oninput=()=>{time=+$('seek').value;pose(time)};
$('export').onclick=async()=>{const wasPlaying=playing;playing=false;lockUi(1);try{const result=await exportNative(eventState?eventState.roles.filter(r=>r.actor.visible):[{data,bones,actor,clip,clipItem,selected},...(pairState?[pairState]:[])],sampleActor,+$('fps').value);$('export-status').textContent='Saved on D: '+result.path+' / reimport matrix error '+result.validation.matrixError.toExponential(2);$('export-status').href=result.url;$('export-status').dataset.validation=JSON.stringify(result);details()}catch(e){fail(e)}finally{playing=wasPlaying;lockUi(-1);pose(time)}};
function renderView(){
 if(eventState&&$('event-camera').checked){const w=view.clientWidth,h=view.clientHeight,vw=Math.min(w,h*16/9),vh=vw*9/16,x=(w-vw)/2,y=(h-vh)/2;renderer.setScissorTest(false);renderer.setClearColor(0x111518,1);renderer.clear();renderer.setViewport(x,y,vw,vh);renderer.setScissor(x,y,vw,vh);renderer.setScissorTest(true);renderer.render(scene,camera);renderer.setScissorTest(false);renderer.setViewport(0,0,w,h)}else renderer.render(scene,camera);
}
let last=performance.now();function tick(now){requestAnimationFrame(tick);const dt=Math.min(.1,(now-last)/1000);last=now;if(playing&&!uiLocks&&(clip||eventState)){time+=dt*+$('fps').value*+$('speed').value;const count=eventState?eventState.frames:clip.frames;if(time>=count){if(eventState&&$('event-continue').checked){advanceEvent();return}time=$('loop').checked?time%count:count-1}pose(Math.min(time,count-1))}orbit.update();if(!uiLocks)renderView()}requestAnimationFrame(tick);
window.study={ready:false,loadModel,loadClip,pose,frame,get selected(){return selected},get actor(){return actor},get catalog(){return catalog},get data(){return data},setPlaying(v){playing=v},setTime(v){time=v;pose(v)},get renderer(){return renderer},get camera(){return camera},get orbit(){return orbit}};
async function audit(page){
 playing=false;
 const seen=new Set(),candidates=catalog.models.filter(m=>m.bones>15&&!seen.has(m.sha256)&&seen.add(m.sha256)),batch=candidates.slice(page*20,page*20+20);
 const gallery=document.createElement('div');gallery.style='display:grid;grid-template-columns:repeat(4,240px);gap:8px;padding:16px;background:#111518;position:absolute;top:0;left:0;z-index:10';document.body.append(gallery);
 view.style='width:240px;height:270px;min-height:270px';resize();
 for(const item of batch){
  const cell=document.createElement('div');cell.style='color:#eddec0;font:12px system-ui;width:240px;min-height:300px';gallery.append(cell);
  try{await loadModel(item.id);pose(0);frame();orbit.enableDamping=false;orbit.update();renderer.render(scene,camera);const image=new Image();image.width=240;image.height=270;image.src=renderer.domElement.toDataURL();cell.append(image,document.createTextNode(`${item.bank} / ${item.name} / ${item.bones} bones`))}catch(e){cell.textContent=item.id+' '+e;}
 }
 document.body.dataset.auditComplete='true';gallery.append(document.createTextNode(`Audit page ${page+1}; ${candidates.length} unique rigged resources`));
}
async function motionAudit(modelName,start=0,count=12){
 playing=false;
 const item=catalog.models.find(m=>m.name===modelName+'.mdb'&&(m.custom||!catalog.models.some(x=>x.custom&&x.name===m.name)));await loadModel(item.id);
 if(new URLSearchParams(location.search).has('color')){$('colorize').checked=true;setMaterials()}
 if(new URLSearchParams(location.search).has('material')){$('material').value=new URLSearchParams(location.search).get('material');setMaterials()}
 const names=new URLSearchParams(location.search).get('clips')?.split(','),bank=motionBank(item);
 const items=names?names.map(n=>catalog.clips.filter(c=>c.name===n+'.mot'&&compatible(c)).sort((a,b)=>Number(b.bank===bank)-Number(a.bank===bank))[0]).filter(Boolean):catalog.clips.filter(c=>c.name.startsWith(motionFamily(item)+'_')&&compatible(c)).slice(start,start+count);
 const gallery=document.createElement('div');gallery.style='display:flex;flex-direction:column;gap:6px;padding:12px;background:#111518;position:absolute;top:0;left:0;z-index:10';document.body.append(gallery);
 view.style='width:200px;height:230px;min-height:230px';resize();orbit.enableDamping=false;
 for(const c of items){
  const row=document.createElement('div');row.style='display:flex;flex-wrap:wrap;width:800px;min-height:260px;color:#eddec0';gallery.append(row);
  try{await loadClip(c.id);$('motion').value=c.id;const box=new THREE.Box3();for(let k=0;k<4;k++){pose((clip.frames-1)*k/3);actor.traverse(o=>{if(o.isMesh&&o.visible&&!o.userData.fxPart){o.computeBoundingBox();box.union(o.boundingBox.clone().applyMatrix4(o.matrixWorld))}})}const sz=box.getSize(new THREE.Vector3()),center=box.getCenter(new THREE.Vector3()),radius=Math.max(sz.x,sz.y,sz.z,.1);orbit.target.copy(center);camera.position.copy(center).add(new THREE.Vector3(radius*.85,radius*.25,radius*1.65));orbit.update();
   for(let k=0;k<4;k++){pose((clip.frames-1)*k/3);renderer.render(scene,camera);const image=new Image();image.width=200;image.height=230;image.src=renderer.domElement.toDataURL();image.alt=`${c.id} frame ${((clip.frames-1)*k/3).toFixed(1)}`;row.append(image)}row.append(document.createTextNode(c.name+' / '+clip.frames+' frames'));
  }catch(e){row.textContent=c.name+' '+e}
 }
 document.body.dataset.auditComplete='true';
}
async function pairAudit(){
 playing=false;await loadPair();orbit.enableDamping=false;
 const gallery=document.createElement('div');gallery.style='display:grid;grid-template-columns:repeat(3,240px);gap:8px;padding:12px;background:#111518;position:absolute;top:0;left:0;z-index:10';document.body.append(gallery);view.style='width:240px;height:290px;min-height:290px';resize();
 const frames=[0,30,60,90,120,177],box=new THREE.Box3();
 for(const f of frames){pose(f);box.union(bounds(actor));box.union(bounds(pairState.actor))}
 const c=box.getCenter(new THREE.Vector3()),sz=box.getSize(new THREE.Vector3()),r=Math.max(sz.x,sz.y,sz.z);orbit.target.copy(c);camera.position.copy(c).add(new THREE.Vector3(r*.85,r*.25,r*1.65));orbit.update();
 for(const f of frames){pose(f);renderer.render(scene,camera);const cell=document.createElement('div'),im=new Image();im.width=240;im.height=290;im.src=renderer.domElement.toDataURL();im.alt='Unverified paired candidate frame '+f;cell.append(im,document.createTextNode(im.alt));gallery.append(cell)}
 document.body.dataset.auditComplete='true';
}
try{catalog=await get('/api/catalog');eventIndex=catalog.events||{events:[],clips:[]};eventIndex.clipById=new Map(eventIndex.clips.map(c=>[c.id,c]));for(const e of eventIndex.events){e.castsByCut=new Map(e.cuts.map(c=>[c.index,e.actors.filter(a=>a.clips.some(id=>eventIndex.clipById.get(id)?.cut===c.index))]));}catalog.clips.push(...eventIndex.clips);try{const cc=await get('custom/clips/index.json?t='+Date.now());catalog.clips.push(...cc.clips)}catch(e){}for(const id of ['performer','receiver']){for(const m of catalog.models.filter(m=>m.review==='reviewed'))$(id).add(new Option(m.displayName,m.id));$(id).value='';$(id).onchange=()=>loadPair().catch(fail)}$('counts').textContent=`${catalog.models.length} model resources · ${catalog.clips.length} nonempty motions`;$('coverage').textContent=JSON.stringify(catalog.coverage||{status:'Coverage report unavailable'},null,2)+'\n\nOther .dat resources not parsed: '+catalog.failures.length+'. Gameplay dispatch, original shader composition and remaining proper identities are unresolved. Native event actor/cut bindings are indexed separately; sequence effects remain partial.';cards();await loadModel(catalog.models.find(m=>m.bank==='pl/pl0000.dat'&&!m.name.includes('LOD')).id)}catch(e){fail(e)}

if(new URLSearchParams(location.search).has('audit'))await audit(+new URLSearchParams(location.search).get('audit'));

if(new URLSearchParams(location.search).has('motion-audit'))await motionAudit(new URLSearchParams(location.search).get('motion-audit'),+(new URLSearchParams(location.search).get('start')||0),+(new URLSearchParams(location.search).get('count')||12));

if(new URLSearchParams(location.search).has('pair-audit'))await pairAudit();

// Deep link for captures: ?model=<id>&clip=<id>&t=<frame>&color=1 (also usable by hand)
{const q=new URLSearchParams(location.search);if(q.has('model')){try{await loadModel(q.get('model'));if(q.has('color')){$('colorize').checked=true;setMaterials()}if(q.has('hands')&&data.handPoses){data.handPose=q.get('hands');$('hands').value=q.get('hands')}if(q.has('material')){$('material').value=q.get('material');setMaterials()}if(q.has('clip')){await loadClip(q.get('clip'));$('motion').value=q.get('clip');}if(q.has('t')){playing=false;$('play').textContent='Play';time=+q.get('t');pose(time)}if(q.has('focus')){const i=data.bones.findIndex(b=>b.id===+q.get('focus'));if(i>=0){const p=bones[i].getWorldPosition(new THREE.Vector3()),d=+(q.get('dist')||.5);orbit.target.copy(p);camera.position.copy(p).add(new THREE.Vector3(d*.45,d*.25,d*.85));camera.near=.005;camera.updateProjectionMatrix();orbit.update()}}if(q.has('orbit')){orbit.enableDamping=false;const v=camera.position.clone().sub(orbit.target).applyAxisAngle(new THREE.Vector3(0,1,0),THREE.MathUtils.degToRad(+q.get('orbit')));camera.position.copy(orbit.target).add(v);orbit.update()}document.body.dataset.deepLink='ready'}catch(e){fail(e);document.body.dataset.deepLink='error'}}}
