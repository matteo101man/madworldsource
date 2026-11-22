// CPU-only integration check of the explorer's actual exporter. No browser/desktop input.
// Run from a local output whose tools/node-review contains @napi-rs/canvas.
import fs from 'node:fs/promises';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import * as THREE from 'three';
const root=path.resolve(process.argv[2]),mode=process.argv[3]||'em0440';
const canvas=await import(pathToFileURL(path.join(root,'tools/node-review/node_modules/@napi-rs/canvas/index.js')));
globalThis.self=globalThis;
globalThis.ImageBitmap=canvas.Image;
globalThis.createImageBitmap=async blob=>canvas.loadImage(Buffer.from(await blob.arrayBuffer()));
globalThis.OffscreenCanvas=class{constructor(w,h){return canvas.createCanvas(w,h)}};
globalThis.ImageData=canvas.ImageData;
globalThis.document={createElement:type=>{if(type!=='canvas')throw Error(type);const c=canvas.createCanvas(1,1);c.toBlob=cb=>cb(new Blob([c.toBuffer('image/png')],{type:'image/png'}));return c}};
globalThis.FileReader=class{readAsArrayBuffer(blob){blob.arrayBuffer().then(b=>{this.result=b;this.onloadend?.()})}readAsDataURL(blob){blob.arrayBuffer().then(b=>{this.result='data:'+blob.type+';base64,'+Buffer.from(b).toString('base64');this.onloadend?.()})}};
const rawFetch=globalThis.fetch;globalThis.fetch=(url,options)=>rawFetch(typeof url==='string'&&url.startsWith('/')?'http://127.0.0.1:8781'+url:url,options);
const get=async url=>{const response=await fetch(url);if(!response.ok)throw Error(await response.text());return response.json()};
const {inkMaterial}=await import(pathToFileURL(path.join(root,'ink.js')));
const {exportNative}=await import(pathToFileURL(path.join(root,'export.js'))),{colorize}=await import(pathToFileURL(path.join(root,'color.js')));
const app=(await fs.readFile(path.join(root,'app.js'),'utf8')).replace(/\r/g,'');
const sample=new Function('THREE',app.slice(app.indexOf('function sampleActor('),app.indexOf('function pose('))+';return sampleActor;')(THREE);
const catalog=await get('/api/catalog');catalog.clips.push(...(catalog.events?.clips||[]));
const modelMemo=new Map();
async function texture(item,index,bank='primary'){const response=await fetch(`/api/texture?id=${encodeURIComponent(item.id)}&index=${index}&bank=${bank}`);const image=await canvas.loadImage(Buffer.from(await response.arrayBuffer())),t=new THREE.Texture(image);t.colorSpace=bank==='ab'?THREE.NoColorSpace:THREE.SRGBColorSpace;t.wrapS=t.wrapT=THREE.RepeatWrapping;t.flipY=false;return t}
const attribute=(values,width,type=Float32Array)=>new THREE.BufferAttribute(new type(values.flat()),width);
const prepare=new (Object.getPrototypeOf(async function(){}).constructor)('id','native','item','THREE','texture','modelMemo','colorize','attribute','fail','catalog','inkMaterial',app.slice(app.indexOf(' let entry=modelMemo.get(id);'),app.indexOf(' if(token!==request)return;\n if(actor)scene.remove(actor);'))+'\nreturn entry;');
async function state(stem,clipItem,modelId=null){
 const selected=catalog.models.find(m=>modelId?m.id===modelId:m.name===stem+'.mdb');if(!selected)throw Error(stem);
 const data=await get('/api/model?id='+encodeURIComponent(selected.id));
 const prepared=await prepare(selected.id,data,selected,THREE,texture,modelMemo,colorize,attribute,e=>{throw e},catalog,inkMaterial);
 const colored=process.env.MW_COLOR!=='0';prepared.root.traverse(o=>{if(o.isMesh)o.material=colored?o.colorInkMat:o.inkMat});prepared.root.userData.colorMode=colored?'added inspection palette':'source monochrome';
 const clip=clipItem?await get('/api/clip?id='+encodeURIComponent(clipItem.id)):null;
 return {selected,data,actor:prepared.root,bones:prepared.bones,clip,clipItem};
}
let states;
if(mode.startsWith('event')){
 const [,eventName='ev21',part='0']=mode.split(':');const e=catalog.events.events.find(e=>e.id===eventName);states=[];
 for(const binding of e.actors){const clip=catalog.events.clips.find(c=>c.actor===binding.id&&c.cut===+part);if(clip&&binding.model)states.push(await state(binding.object,clip,binding.model))}
}else{const clip=catalog.clips.find(c=>c.name.startsWith(mode+'_')&&!c.event);states=[await state(mode,clip)]}
const result=await exportNative(states,sample,60);
await fs.writeFile(path.join(root,'evidence/node-export-'+mode.replaceAll(':','-')+(process.env.MW_COLOR==='0'?'-source':'')+'.json'),JSON.stringify(result,null,2));
console.log(JSON.stringify(result));
