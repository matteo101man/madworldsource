import * as THREE from 'three';

// Added inspection art direction. These colors are not recovered game artwork.
const palettes={
 em0440:{skin:'#eeb897',cloth:'#c93248',accent:'#e3ac38',hair:'#263548'},
 em0610:{skin:'#dca780',cloth:'#354d69',accent:'#c287ae',hair:'#d6b36a'},
 em0630:{skin:'#c6a6cd',cloth:'#77344e',accent:'#b99756',hair:'#382943'},
 em0920:{skin:'#efc4ae',cloth:'#b74765',accent:'#d7b45d',hair:'#e5c377'},
 em0925:{skin:'#e5bdac',cloth:'#5b7ea5',accent:'#d8b454',hair:'#b98548'},
 em0930:{skin:'#b17859',cloth:'#5d9b98',accent:'#d9ac4e',hair:'#2b2734'},
 em0935:{skin:'#b17859',cloth:'#5d9b98',accent:'#d9ac4e',hair:'#2b2734'},
 em0960:{skin:'#f1c7b4',cloth:'#9073b4',accent:'#ce6688',hair:'#283342'},
 pl:{skin:'#d9aa83',cloth:'#718590',accent:'#b19b62',hair:'#554d42'}
};
export const colorProvenance='Added inspection palette v1; native mesh-name regions plus bind-space fallback. Source textures and ink retained. Colors are not recovered or canonical.';

export function colorize(root,item,native){
 const stem=item.name.split('.')[0].replace(/_LOD\d$/,''),key=stem.startsWith('pl')?'pl':stem;
 const hash=[...stem].reduce((v,c)=>(v*31+c.charCodeAt(0))>>>0,0),fallback={skin:'#dab69a',cloth:new THREE.Color().setHSL((hash%360)/360,.42,.62),accent:'#cdab61',hair:'#42445b'};
 const p=Object.fromEntries(Object.entries(palettes[key]||fallback).map(([k,v])=>[k,v.isColor?v:new THREE.Color(v)]));
 const box=new THREE.Box3();for(const m of native.meshes)for(const v of m.positions)box.expandByPoint(new THREE.Vector3(...v));
 const height=Math.max(.001,box.max.y-box.min.y),center=(box.min.x+box.max.x)/2;
 root.traverse(o=>{
  if(!o.isMesh)return;
  const name=o.name.toLowerCase(),pos=o.geometry.getAttribute('position'),colors=new Float32Array(pos.count*3);
  for(let i=0;i<pos.count;i++){
   const y=(pos.getY(i)-box.min.y)/height,x=Math.abs(pos.getX(i)-center)/height;
   let c=p.cloth;
   if(/hair|heir/.test(name))c=p.hair;
   else if(/^(ha_|ha\b|sirome|medama)|tooth/.test(name))c=new THREE.Color('#ece7dc');
   else if(/kuchi|danmen/.test(name))c=new THREE.Color('#b66c71');
   else if(/skin|hada|hand|face|arm|mimi|mune|head_sb/.test(name))c=p.skin;
   else if(/kazari|akuse|acce|ring|backle|buckle|kanagu|kushi|spike|tsuno|kougu/.test(name))c=p.accent;
   else if(/goggles|sunglasses/.test(name))c=new THREE.Color('#5a8c96');
   else if(/belt|leather|kutsu|kutu|geta|pants/.test(name))c=p.hair;
   else if(/obi|ribon|himo|oogi/.test(name))c=p.accent;
   else if(/^_m\d|^body_sb|^leg_sb/.test(name))c=y>.84||y>.46&&x>.19?p.skin:y<.08?p.hair:p.cloth;
   if(/head_sb|face/.test(name)&&y>.965)c=p.hair;
   colors.set([c.r,c.g,c.b],i*3);
  }
  o.geometry.setAttribute('color',new THREE.BufferAttribute(colors,3));
  o.colorMat=o.sourceMat.clone();o.colorMat.vertexColors=true;
  o.colorLitMat=o.litMat.clone();o.colorLitMat.vertexColors=true;
  o.colorMat.userData={colorProvenance};o.colorLitMat.userData={colorProvenance};
 });
 root.userData.colorProvenance=colorProvenance;
}
