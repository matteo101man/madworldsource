import * as THREE from 'three';

const colorNames={'prev.rgb':'pv.rgb','prev.a':'ivec3(pv.a)','reg0.rgb':'c0.rgb','reg0.a':'ivec3(c0.a)','reg1.rgb':'c1.rgb','reg1.a':'ivec3(c1.a)','reg2.rgb':'c2.rgb','reg2.a':'ivec3(c2.a)','texture.rgb':'tx.rgb','texture.a':'ivec3(tx.a)','raster.rgb':'rs.rgb','raster.a':'ivec3(rs.a)','one':'ivec3(255)','half':'ivec3(128)','konst.rgb':'kc','zero':'ivec3(0)'};
const alphaNames={'prev.a':'pv.a','reg0.a':'c0.a','reg1.a':'c1.a','reg2.a':'c2.a','texture.a':'tx.a','raster.a':'rs.a','konst.a':'ka','zero':'0'};
const registers={prev:'pv',reg0:'c0',reg1:'c1',reg2:'c2'};
const swizzle=a=>a.map(i=>'rgba'[i]).join('');
function konst(selector,alpha=false){
 if(selector<8)return alpha?String([255,223,191,159,128,96,64,32][selector]):`ivec3(${[255,223,191,159,128,96,64,32][selector]})`;
 if(!alpha&&selector>=12&&selector<16)return `k${selector-12}.rgb`;
 if(selector>=16){const component='rgba'[Math.floor((selector-16)/4)],reg=(selector-16)%4;return alpha?`k${reg}.${component}`:`ivec3(k${reg}.${component})`}
 return alpha?'0':'ivec3(0)';
}
export function tevCode(program){
 let body='ivec4 pv=ivec4(0),c0=ivec4(0),c1=ivec4(0),c2=ivec4(0); c1.a=int(mwThreshold);\n';
 body+='ivec4 k0=ivec4(255),k1=ivec4(127,127,127,255),k2=ivec4(0),k3=ivec4(0);\n';
 for(const stage of program.stages){
  const c=stage.color,a=stage.alpha,C=c.inputs,A=a.inputs;
  const colorArgs='ABCD'.split('').map(k=>colorNames[C[k]]),alphaArgs='ABCD'.split('').map(k=>alphaNames[A[k]]);
  body+=`{ivec4 tx=s${stage.textureMap}.${swizzle(stage.textureSwap)},rs=ra${stage.rasterChannel===1?1:0}.${swizzle(stage.rasterSwap)}; ivec3 kc=${konst(stage.konstColor)}; int ka=${konst(stage.konstAlpha,true)};\n`;
  body+=`ivec3 co=mwColor(${colorArgs.join(',')},${c.bias},${+c.subtract},${c.scale}); int al=mwAlpha(${alphaArgs.join(',')},${a.bias},${+a.subtract},${a.scale});\n`;
  body+=`${registers[c.destination]}.rgb=clamp(co,ivec3(${c.clamp?0:-1024}),ivec3(${c.clamp?255:1023})); ${registers[a.destination]}.a=clamp(al,${a.clamp?0:-1024},${a.clamp?255:1023});}\n`;
 }
 return body+'return clamp(pv,ivec4(0),ivec4(255));';
}
export const tevMath=`
int mwMix(int a,int b,int c,int d,int bias,int sub,int scale){
 int x=(a<<8)+(b-a)*(c+(c>>7)); int v=(d<<8)+(sub==1?-x:x)+(bias==1?32768:bias==2?-32768:0);
 if(scale<3)v=v<<scale;else v=v>>1;return (v+128)>>8;
}
ivec3 mwColor(ivec3 a,ivec3 b,ivec3 c,ivec3 d,int bias,int sub,int scale){
 if(bias==3){if(scale==3)return d+ivec3(sub==1?equal(a,b):greaterThan(a,b))*c;
 int av=a.r,bv=b.r;if(scale>=1){av+=a.g<<8;bv+=b.g<<8;}if(scale==2){av+=a.b<<16;bv+=b.b<<16;}
 return d+((sub==1?av==bv:av>bv)?c:ivec3(0));}
 return ivec3(mwMix(a.r,b.r,c.r,d.r,bias,sub,scale),mwMix(a.g,b.g,c.g,d.g,bias,sub,scale),mwMix(a.b,b.b,c.b,d.b,bias,sub,scale));
}
int mwAlpha(int a,int b,int c,int d,int bias,int sub,int scale){if(bias==3)return d+((sub==1?a==b:a>b)?c:0);return mwMix(a,b,c,d,bias,sub,scale);}
vec3 mwEncode(vec3 x){return mix(x*12.92,1.055*pow(max(x,vec3(0.0)),vec3(1.0/2.4))-.055,step(vec3(.0031308),x));}
vec3 mwDecode(vec3 x){return mix(x/12.92,pow((max(x,vec3(0.0))+.055)/1.055,vec3(2.4)),step(vec3(.04045),x));}
`;

export function inkMaterial(source,ab,mesh,program){
 if(!program||!ab)return source;
 const mat=source.clone();
 mat.userData={nativeInk:{program:program.id,renderFlags:mesh.renderFlags,threshold:mesh.inkThreshold,authority:'native mesh flag dispatch and captured GX TEV program',preview:'camera-relative diffuse light; unoccluded depth inputs; scene light/konst values unresolved'},exportFallback:'Source atlas + vertex palette; native GX program recorded in metadata'};
 const preamble=`uniform sampler2D mwAB; uniform float mwThreshold; varying float mwIllumination;\n${tevMath}\nivec4 mwEvaluate(ivec4 s0,ivec4 s1,ivec4 s2,ivec4 s3,ivec4 ra0,ivec4 ra1){${tevCode(program)}}\n`;
 mat.onBeforeCompile=shader=>{
  shader.uniforms.mwAB={value:ab};shader.uniforms.mwThreshold={value:mesh.inkThreshold??128};
  shader.vertexShader='varying float mwIllumination;\n'+shader.vertexShader;
  shader.vertexShader=shader.vertexShader.replace('#include <begin_vertex>',`#include <begin_vertex>
   vec3 mwN=normal;
   #ifdef USE_SKINNING
    mwN=vec4(skinMatrix*vec4(mwN,0.0)).xyz;
   #endif
   mwIllumination=max(0.0,dot(normalize(normalMatrix*mwN),normalize(vec3(.25,.8,.6))));`);
  shader.fragmentShader=preamble+shader.fragmentShader;
  shader.fragmentShader=shader.fragmentShader.replace('#include <map_fragment>',`#include <map_fragment>
   #ifdef USE_MAP
    vec4 mwSource=texture2D(map,vMapUv),mwB=texture2D(mwAB,vMapUv);
    ivec4 mwS=ivec4(mwEncode(mwSource.rgb)*255.0+.5,mwSource.a*255.0+.5),mwShadow=ivec4(mwB*255.0+.5);
    if(mwThreshold==0.0)mwShadow.a=0;
    ivec4 mwOut=mwEvaluate(ivec4(0),ivec4(255),mwS,mwShadow,ivec4(255),ivec4(int(mwIllumination*255.0+.5)));
    diffuseColor.rgb=mwDecode(vec3(mwOut.rgb)/255.0)*diffuse;
    #ifdef USE_COLOR
     diffuseColor.rgb*=vColor;
    #endif
   #endif`);
 };
 mat.customProgramCacheKey=()=>program.id+'-'+Number(source.vertexColors);
 return mat;
}
