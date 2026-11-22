// Expand the actual pinned Three.js shader chunks and the explorer's material hook.
import fs from 'node:fs/promises';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
import * as THREE from 'three';
const root=path.resolve(process.argv[2]);
const {inkMaterial}=await import(pathToFileURL(path.join(root,'ink.js')));
const programs=JSON.parse(await fs.readFile(path.join(root,'evidence/material-programs.json'),'utf8'));
function expand(source){return source.replace(/#include <([\w]+)>/g,(_,name)=>{if(!(name in THREE.ShaderChunk))throw Error(name);return expand(THREE.ShaderChunk[name])})}
for(const [kind,program] of Object.entries(programs.profiles)){
 const source=new THREE.MeshBasicMaterial({map:new THREE.Texture(),vertexColors:true});
 const mat=inkMaterial(source,new THREE.Texture(),{renderFlags:kind==='white'?0x2000:0x1000,inkThreshold:135},program);
 const shader={vertexShader:THREE.ShaderLib.basic.vertexShader,fragmentShader:THREE.ShaderLib.basic.fragmentShader,uniforms:{}};mat.onBeforeCompile(shader);
 const common='#version 300 es\nprecision highp float;precision highp int;\n#define USE_MAP\n#define USE_COLOR\n#define MAP_UV uv\n#define average mwAverage\n#define NUM_CLIPPING_PLANES 0\n#define UNION_CLIPPING_PLANES 0\n#define NUM_SPOT_LIGHT_COORDS 0\n#define NUM_SPOT_LIGHT_MAPS 0\n#define NUM_SPOT_LIGHT_SHADOWS 0\n#define NUM_DIR_LIGHT_SHADOWS 0\n#define NUM_POINT_LIGHT_SHADOWS 0\n';
 const vertex=common+`#define attribute in
#define varying out
#define USE_SKINNING
uniform mat4 modelMatrix,modelViewMatrix,projectionMatrix,viewMatrix;
uniform mat3 normalMatrix;uniform vec3 cameraPosition;uniform bool isOrthographic;
in vec3 position,normal,color;in vec2 uv;in vec4 skinIndex,skinWeight;
`+expand(shader.vertexShader);
 const fragment=common+`#define varying in
#define texture2D texture
#define gl_FragColor pc_fragColor
#define OPAQUE
out vec4 pc_fragColor;uniform mat4 viewMatrix;uniform vec3 cameraPosition;uniform bool isOrthographic;
vec4 linearToOutputTexel(vec4 value){return value;}
`+expand(shader.fragmentShader);
 await fs.writeFile(path.join(root,'evidence/ink-'+kind+'.vert'),vertex);await fs.writeFile(path.join(root,'evidence/ink-'+kind+'.frag'),fragment);
}
console.log('Expanded native ink shader variants for offline GLSL checks');
