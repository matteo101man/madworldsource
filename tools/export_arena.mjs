// Export the bounded native SCR decode; game data remains local on D:.
import fs from 'node:fs/promises';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import * as THREE from 'three';
import {GLTFExporter} from 'three/addons/exporters/GLTFExporter.js';
import {GLTFLoader} from 'three/addons/loaders/GLTFLoader.js';
// MW_ARENA_DIR (relative to the repo, default assets/native/world) and MW_ROOM (default c201) pick the room
const root=path.resolve(process.argv[2]),folder=path.join(root,process.env.MW_ARENA_DIR||'assets/native/world'),room=process.env.MW_ROOM||'c201';
const canvas=await import(pathToFileURL('D:/Games/gamesfordecomp/exports/madworld/tools/node-review/node_modules/@napi-rs/canvas/index.js'));
globalThis.self=globalThis;globalThis.ImageBitmap=canvas.Image;globalThis.createImageBitmap=async blob=>canvas.loadImage(Buffer.from(await blob.arrayBuffer()));
globalThis.OffscreenCanvas=class{constructor(w,h){return canvas.createCanvas(w,h)}};globalThis.ImageData=canvas.ImageData;
globalThis.document={createElement:type=>{if(type!=='canvas')throw Error(type);const c=canvas.createCanvas(1,1);c.toBlob=cb=>cb(new Blob([c.toBuffer('image/png')],{type:'image/png'}));return c}};
globalThis.FileReader=class{readAsArrayBuffer(b){b.arrayBuffer().then(x=>{this.result=x;this.onloadend?.()})}readAsDataURL(b){b.arrayBuffer().then(x=>{this.result='data:'+b.type+';base64,'+Buffer.from(x).toString('base64');this.onloadend?.()})}};
const data=JSON.parse(await fs.readFile(path.join(folder,room+'.json'),'utf8'));
if(data.failures.length)throw Error('Incomplete native world decode');
const textures=[];for(let i=0;i<data.textures;i++){const im=await canvas.loadImage(await fs.readFile(path.join(folder,process.env.MW_ARENA_TEXTURES||'','texture-'+String(i).padStart(3,'0')+'.png'))),t=new THREE.Texture(im);t.colorSpace=THREE.SRGBColorSpace;t.flipY=false;t.wrapS=t.wrapT=THREE.RepeatWrapping;textures.push(t)}
const scene=new THREE.Group();scene.userData.provenance={source:data.source,sha256:data.sourceSha256,authority:data.authority};let total=0;
for(let i=0;i<data.models.length;i++){
 const row=data.models[i];if(row.bones.length)throw Error('Unexpected world skin');const group=new THREE.Group();group.name='world_'+i+'_'+row.name;scene.add(group);
 // the SCR part transform: translation, rotation in radians (only Y is set; stage_arena.py checks), uniform scale;
 // (native T.R.S, PSMTXRotRad yaw, 8020F9F8: sign +1; MW_ARENA_YAW_SIGN=-1 is an inspection flip only)
 const t=row.transform||[0,0,0,0,0,0,1,1,1];group.position.set(t[0],t[1],t[2]);group.rotation.set(t[3],t[4]*(+(process.env.MW_ARENA_YAW_SIGN||1)),t[5]);group.scale.set(t[6],t[7],t[8]);
 for(let j=0;j<row.meshes.length;j++){
  const m=row.meshes[j],g=new THREE.BufferGeometry();g.setAttribute('position',new THREE.Float32BufferAttribute(m.positions.flat(),3));g.setIndex(m.indices);
  if(m.normals.length)g.setAttribute('normal',new THREE.Float32BufferAttribute(m.normals.flat(),3));else g.computeVertexNormals();
  if(m.uvs.length)g.setAttribute('uv',new THREE.Float32BufferAttribute(m.uvs.flat(),2));
  if(m.colors.length)g.setAttribute('color',new THREE.Float32BufferAttribute(m.colors.flatMap(c=>c.slice(0,3).map(v=>v/255)),3));
  const map=textures[m.texture]||null,material=new THREE.MeshBasicMaterial({map,color:map?0xffffff:0xc7c3b7,side:THREE.DoubleSide,vertexColors:!!m.colors.length});
  material.userData.nativeInk={renderFlags:m.renderFlags,threshold:m.inkThreshold,authority:'source atlas fallback; native lighting and visibility not reproduced'};
  const mesh=new THREE.Mesh(g,material);mesh.name='world_'+i+'_mesh_'+j;group.add(mesh);total++;
 }
}
const binary=await new GLTFExporter().parseAsync(scene,{binary:true,onlyVisible:true});const imported=await new GLTFLoader().parseAsync(binary,'');let error=0,checked=0;
scene.traverse(m=>{if(!m.isMesh)return;const actual=imported.scene.getObjectByName(m.name);if(!actual?.isMesh)throw Error('Lost world mesh '+m.name);for(const key of ['position','uv','color']){const a=m.geometry.getAttribute(key),b=actual.geometry.getAttribute(key);if(!a&&!b)continue;if(!a||!b||a.array.length!==b.array.length)throw Error('World attribute mismatch');for(let i=0;i<a.array.length;i++){error=Math.max(error,Math.abs(a.array[i]-b.array[i]));checked++}}if(m.geometry.index.count!==actual.geometry.index.count)throw Error('World triangle mismatch');for(let i=0;i<m.geometry.index.count;i++)if(m.geometry.index.array[i]!==actual.geometry.index.array[i])throw Error('World index changed')});
if(error>1e-6)throw Error('World attribute error '+error);await fs.writeFile(path.join(folder,process.env.MW_ARENA_OUT||room+'.glb'),Buffer.from(binary));
const report={models:data.models.length,meshes:total,bytes:binary.byteLength,checkedAttributes:checked,maxAttributeError:error,authority:'actual GLTFLoader reimport; geometry/UV/colors/indices only; native collision, visibility and visual fidelity separate'};await fs.writeFile(path.join(root,'evidence/'+(process.env.MW_ARENA_OUT?'world-export-color.json':'world-export.json')),JSON.stringify(report,null,2));console.log(JSON.stringify(report));
