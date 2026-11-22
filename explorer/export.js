import * as THREE from 'three';
import {GLTFExporter} from 'three/addons/exporters/GLTFExporter.js';
import {GLTFLoader} from 'three/addons/loaders/GLTFLoader.js';

export async function exportNative(states,sample,fps){
 const container=new THREE.Group(),restore=[],meshNames=[],geometryRestore=[],tracks=[],duration=Math.max(...states.map(s=>s.clip?.frames||1));
 container.userData.provenance={adapter:'MadWorld native study',timebase:'adjustable inspection rate; native runtime timebase unresolved',fps,roles:states.map(s=>({model:s.selected,clip:s.clipItem,colorMode:s.actor.userData.colorMode,colorProvenance:s.actor.userData.colorProvenance})),pairing:states.every(s=>s.clipItem?.event)?'Native event actor/cut bindings; scene effects and draw commands incomplete':states.length>1?'inspection candidate; native dispatch and alignment unverified':'single native clip'};
 const previousTime=states.map(s=>s.bones.map(b=>({position:b.position.clone(),quaternion:b.quaternion.clone(),scale:b.scale.clone(),name:b.name})));
 try{
  for(let role=0;role<states.length;role++){
   const s=states[role];restore.push({parent:s.actor.parent,name:s.actor.name});s.actor.name='role_'+role;container.add(s.actor);
   let meshIndex=0;s.actor.traverse(o=>{if(o.isMesh){meshNames.push([o,o.name]);o.name='role_'+role+'_mesh_'+meshIndex++;if(o.geometry.getAttribute('color')&&!o.material.vertexColors){geometryRestore.push([o,o.geometry]);o.geometry=o.geometry.clone();o.geometry.deleteAttribute('color')}}});
   s.bones.forEach((b,i)=>b.name='role_'+role+'_bone'+(i<s.data.bones.length?s.data.bones[i].id:-1));
   if(!s.clip)continue;
   const times=Float32Array.from({length:s.clip.frames},(_,f)=>f/fps),perBone=s.bones.map(()=>({p:[],q:[],s:[]}));
   for(let f=0;f<s.clip.frames;f++){
    sample(s.data,s.bones,s.actor,s.clip,f,false);
    s.bones.forEach((b,i)=>{perBone[i].p.push(...b.position);perBone[i].q.push(...b.quaternion);perBone[i].s.push(...b.scale)});
   }
   s.bones.forEach((b,i)=>{tracks.push(new THREE.VectorKeyframeTrack(b.name+'.position',times,perBone[i].p),new THREE.QuaternionKeyframeTrack(b.name+'.quaternion',times,perBone[i].q),new THREE.VectorKeyframeTrack(b.name+'.scale',times,perBone[i].s))});
   sample(s.data,s.bones,s.actor,s.clip,0,false);
  }
  const animation=tracks.length?new THREE.AnimationClip(states.map(s=>s.clipItem?.name||'bind').join(' + '),-1,tracks):null;
  const binary=await new GLTFExporter().parseAsync(container,{binary:true,animations:animation?[animation]:[],onlyVisible:true});
  const loaded=await new GLTFLoader().parseAsync(binary,'');
  const mixer=new THREE.AnimationMixer(loaded.scene);
  if(loaded.animations.length){const action=mixer.clipAction(loaded.animations[0]);action.setLoop(THREE.LoopOnce,1);action.clampWhenFinished=true;action.play()}
  let matrixError=0,weightError=0,attributeError=0,checkedAttributes=0,checkedWeights=0,checkedBones=0,checkedFrames=0;
  for(const f of [...new Set([0,Math.floor((duration-1)/3),Math.floor(2*(duration-1)/3),duration-1])]){
   mixer.setTime(f/fps);loaded.scene.updateMatrixWorld(true);
   for(const s of states){sample(s.data,s.bones,s.actor,s.clip,f,false);for(const bone of s.bones){const output=loaded.scene.getObjectByName(bone.name);if(!output)throw Error('Export lost bone '+bone.name);for(let k=0;k<16;k++)matrixError=Math.max(matrixError,Math.abs(output.matrixWorld.elements[k]-bone.matrixWorld.elements[k]));checkedBones++}}
   checkedFrames++;
  }
  container.traverse(mesh=>{if(!mesh.isMesh||!mesh.visible)return;const actual=loaded.scene.getObjectByName(mesh.name);if(!actual?.isMesh)throw Error('Export lost mesh '+mesh.name);if(!mesh.material.vertexColors&&actual.geometry.getAttribute('color'))throw Error('Export enabled an inactive color palette');
   for(const key of ['position','uv',...(mesh.material.vertexColors?['color']:[])]){const a=mesh.geometry.getAttribute(key),b=actual.geometry.getAttribute(key);if(!a&&!b)continue;if(!a||!b||a.array.length!==b.array.length)throw Error('Export lost vertex attribute '+key);for(let k=0;k<a.array.length;k++){attributeError=Math.max(attributeError,Math.abs(a.array[k]-b.array[k]));checkedAttributes++}}
   const sourceIndex=mesh.geometry.index,outputIndex=actual.geometry.index;if(sourceIndex){if(!outputIndex||sourceIndex.count!==outputIndex.count)throw Error('Export index count');for(let k=0;k<sourceIndex.count;k++){attributeError=Math.max(attributeError,Math.abs(sourceIndex.array[k]-outputIndex.array[k]));checkedAttributes++}}
   if(!mesh.isSkinnedMesh)return;if(!actual.isSkinnedMesh)throw Error('Export lost skin '+mesh.name);
   for(const key of ['skinWeight','skinIndex']){const a=mesh.geometry.getAttribute(key),b=actual.geometry.getAttribute(key);if(a.count!==b.count||a.itemSize!==b.itemSize)throw Error('Export influence shape '+mesh.name+': '+a.count+' / '+b.count);for(let k=0;k<a.array.length;k++){weightError=Math.max(weightError,Math.abs(a.array[k]-b.array[k]));checkedWeights++}}
  });
  if(matrixError>1e-4||weightError>1e-6||attributeError>1e-6)throw Error(`Export comparison failed: matrices ${matrixError}, influences ${weightError}, vertex attributes ${attributeError}`);
  const validation={checkedFrames,checkedBones,checkedWeights,checkedAttributes,matrixError,weightError,attributeError,authority:'actual GLTFLoader reimport; integer-frame native pose, positions/UVs/active vertex colors/indices and every influence; lighting and visual fidelity separate'};
  const response=await fetch('/api/export',{method:'POST',body:binary,headers:{'Content-Type':'model/gltf-binary','X-MadWorld-Validation':JSON.stringify(validation)}});if(!response.ok)throw Error('Cannot save GLB on D:');
  return {...await response.json(),validation};
 }finally{
  for(const [mesh,geometry] of geometryRestore){mesh.geometry.dispose();mesh.geometry=geometry}
  for(const [mesh,name] of meshNames)mesh.name=name;
  states.forEach((s,role)=>{s.actor.name=restore[role]?.name||'';restore[role]?.parent?.add(s.actor);s.bones.forEach((b,i)=>{const old=previousTime[role][i];b.name=old.name;b.position.copy(old.position);b.quaternion.copy(old.quaternion);b.scale.copy(old.scale)});s.actor.updateMatrixWorld(true)})
 }
}
