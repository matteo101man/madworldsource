"""Synthetic scalar FPU result check: production emitter against Unicorn750CL.
Not a Broadway paired-single/FPSCR/estimate or gameplay equivalence claim.
"""
import argparse,json,math,random,struct,subprocess,sys
from pathlib import Path
from unicorn import Uc,UcError,UC_ARCH_PPC,UC_MODE_PPC32,UC_MODE_BIG_ENDIAN
from unicorn import ppc_const as P
PORT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(PORT/'recomp'))
from recomp import Recompiler,Function
from ppc_decode import decode

def specifications():
    rows=[]
    for primary,suffix in ((63,''),(59,'s')):
        for root,xo in (('fadd',21),('fsub',20),('fdiv',18),('fmul',25),('fmadd',29),('fmsub',28),('fnmadd',31),('fnmsub',30)):
            b=0 if root=='fmul' else 3;c=4 if root in ('fmul','fmadd','fmsub','fnmadd','fnmsub') else 0
            rows.append((root+suffix,(primary<<26)|(1<<21)|(2<<16)|(b<<11)|(c<<6)|(xo<<1)))
    for name,xo in (('frsp',12),('fctiw',14),('fctiwz',15),('fmr',72),('fneg',40),('fabs',264),('fnabs',136)):
        rows.append((name,(63<<26)|(1<<21)|(3<<11)|(xo<<1)))
    for name,word in rows:assert decode(word).op==name,(name,decode(word))
    return rows

def generate():
    rows=specifications();emitter=Recompiler(None,[]);f=Function('synthetic_fpu',0x81000000,4)
    cases=[]
    for index,(name,word) in enumerate(rows):cases.append('case '+str(index)+': { '+emitter.emit_insn(f,f.addr,decode(word))+' break; } // '+name)
    (PORT/'tests/fpu_fixture.inc').write_text('// Synthetic instructions from the production emitter.\n'+'\n'.join(cases)+'\n',encoding='utf-8')
    return rows

def bits(x):return int.from_bytes(struct.pack('>d',x),'big')

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--generate-only',action='store_true');ap.add_argument('--cases',type=int,default=500);a=ap.parse_args();rows=generate()
    if a.generate_only:print('Generated',len(rows),'synthetic scalar instructions');return
    evidence=PORT.parents[2]/'evidence';inputs=evidence/'fpu-synthetic-input.bin';outputs=evidence/'fpu-synthetic-output.bin'
    rng=random.Random(20261108);records=[]
    for index,(name,_) in enumerate(rows):
        values=[(0.,1.,-1.),(-0.,-0.,-0.),(1.,-1.,1.),(1.5,2.5,-2.5),(2.5,-2.5,1.)]
        values += [tuple(math.ldexp(rng.uniform(-1,1),rng.randrange(-100,101)) for _ in range(3)) for _ in range(a.cases)]
        for av,bv,cv in values:
            if name.startswith('fdiv') and bv==0.:bv=1.
            for mode in (range(4) if name in ('fctiw','fctiwz') else (0,)):
                records.append((index,mode,bits(av),bits(bv),bits(cv)))
    inputs.write_bytes(b''.join(struct.pack('<IIQQQ',*row) for row in records))
    subprocess.run([str(PORT/'build/fpu_math.exe'),str(inputs),str(outputs)],check=True)
    actual=outputs.read_bytes();assert len(actual)==len(records)*8
    cpu=Uc(UC_ARCH_PPC,UC_MODE_PPC32|UC_MODE_BIG_ENDIAN);cpu.ctl_set_cpu_model(P.UC_CPU_PPC32_750CL_V2_0);cpu.mem_map(0x1000,0x1000)
    for i,(_,word) in enumerate(rows):cpu.mem_write(0x1000+i*4,word.to_bytes(4,'big'))
    cpu.reg_write(P.UC_PPC_REG_MSR,0x2000)
    mismatches=[];unavailable={};counts={name:0 for name,_ in rows};matched=0;full_bit_mismatches=0;by_operation={};excluded_conversion_inputs=0
    for i,(index,fpscr,av,bv,cv) in enumerate(records):
        name=rows[index][0]
        if name in unavailable:continue
        for reg,value in ((2,av),(3,bv),(4,cv)):cpu.reg_write(getattr(P,'UC_PPC_REG_FPR'+str(reg)),value)
        cpu.reg_write(P.UC_PPC_REG_FPSCR,fpscr)
        try:cpu.emu_start(0x1000+index*4,0x1004+index*4,count=1)
        except UcError as error:unavailable[name]=str(error);continue
        source=struct.unpack('>d',bv.to_bytes(8,'big'))[0]
        if name in ('fctiw','fctiwz') and not -2147483648<=source<=2147483647:
            excluded_conversion_inputs+=1
            continue  # Generic Unicorn positive-overflow result differs from Broadway.
        expected=cpu.reg_read(P.UC_PPC_REG_FPR1);result=struct.unpack_from('<Q',actual,i*8)[0];counts[name]+=1
        if result!=expected:full_bit_mismatches+=1
        # Generic Unicorn conversion has a different upper-word convention.
        # Verify its integer32 result; Broadway marker bits have independent
        # native-source checks in the C++ harness, not a Unicorn oracle.
        mask=0xffffffff if name in ('fctiw','fctiwz') else 0xffffffffffffffff
        if (result&mask)==(expected&mask):matched+=1
        else:
            by_operation[name]=by_operation.get(name,0)+1
            mismatches.append(dict(operation=name,roundingMode=fpscr,inputs=[hex(av),hex(bv),hex(cv)],native=hex(result),unicorn=hex(expected)))
    report=dict(authority='Synthetic scalar FPR0 result comparison against Unicorn750CL; not original Broadway hardware evidence',cases=len(records),compared=sum(counts.values()),matched=matched,operations=counts,unavailable=unavailable,mismatchCount=len(mismatches),mismatchOperations=by_operation,fullRegisterBitDifferences=full_bit_mismatches,excludedConversionInputs=excluded_conversion_inputs,conversionComparison="Integer32 result only; Broadway upper marker tested independently against pinned Dolphin HW-tested behavior",examples=mismatches[:40],notVerified=['FPSCR flags/exception semantics','Non-default arithmetic rounding modes (conversion RN selectors alone tested)' ,'Paired-single/quantized operations','Reciprocal estimates','Original gameplay equivalence'])
    (evidence/'fpu-synthetic-results.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({key:report[key] for key in ('compared','matched','mismatchCount','unavailable')}))
if __name__=='__main__':main()
