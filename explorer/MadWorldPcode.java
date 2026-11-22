// Bounded, asset-free DOL-to-Pcode research adapter. Generated evidence stays external.
// @category MadWorld
import ghidra.app.script.GhidraScript;
import ghidra.app.cmd.disassemble.DisassembleCommand;
import ghidra.app.decompiler.*;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.mem.*;
import ghidra.program.model.pcode.*;
import ghidra.program.model.symbol.SourceType;
import com.google.gson.*;
import java.nio.file.*;
import java.io.*;
import java.util.*;

public class MadWorldPcode extends GhidraScript {
    PrintWriter writer;Gson gson=new Gson();int nextValue=0;
    IdentityHashMap<Varnode,String> values;
    void node(String id,String kind,String label,Map<String,Object> attrs){writer.println(gson.toJson(Map.of("n",Map.of("id",id,"kind",kind,"label",label,"attrs",attrs))));}
    void edge(String a,String b,String kind,int seq){writer.println(gson.toJson(Map.of("e",Map.of("src",a,"dst",b,"kind",kind,"attrs",Map.of("seq",seq)))));}
    long word(byte[] b,int p){return Integer.toUnsignedLong(java.nio.ByteBuffer.wrap(b,p,4).getInt());}
    String value(Varnode v,String prefix){
        if(values.containsKey(v))return values.get(v);
        String id=prefix+":value:"+(nextValue++);values.put(v,id);Map<String,Object> a=new LinkedHashMap<>();
        a.put("space",v.getAddress().getAddressSpace().getName());a.put("offset",v.getOffset());a.put("size",v.getSize());a.put("constant",v.isConstant());
        String label=v.toString();
        MemoryBlock valueBlock=v.getOffset()>=0x80000000L&&v.getOffset()<0x81800000L?currentProgram.getMemory().getBlock(toAddr(v.getOffset())):null;
        if((v.isConstant()||v.isAddress())&&v.getOffset()>=0x80000000L&&v.getOffset()<0x81800000L&&valueBlock!=null&&!valueBlock.isExecute()){
            try{StringBuilder s=new StringBuilder();for(int i=0;i<100;i++){int c=getByte(toAddr(v.getOffset()+i))&255;if(c==0)break;if(c<32||c>126){s.setLength(0);break;}s.append((char)c);}if(s.length()>0){a.put("string",s.toString());label=s.toString();}}catch(Exception ignored){}
        }
        node(id,"pcode-value",label,a);return id;
    }
    public void run() throws Exception {
        String root=getScriptArgs()[0];byte[] dol=Files.readAllBytes(Path.of(root,"extracted/disc/DATA/sys/main.dol"));
        Memory mem=currentProgram.getMemory();for(MemoryBlock block:mem.getBlocks())mem.removeBlock(block,monitor);
        for(int i=0;i<18;i++){
            int j=i<7?i:i-7;int offsets=i<7?0:0x1c,addresses=i<7?0x48:0x64,sizes=i<7?0x90:0xac;
            long offset=word(dol,offsets+j*4),address=word(dol,addresses+j*4),size=word(dol,sizes+j*4);if(size==0)continue;
            MemoryBlock block=mem.createInitializedBlock((i<7?"text":"data")+j,toAddr(address),new ByteArrayInputStream(dol,(int)offset,(int)size),size,monitor,false);block.setRead(true);block.setWrite(i>=7);block.setExecute(i<7);
        }
        long bss=word(dol,0xd8),bssSize=word(dol,0xdc);AddressSet gaps=new AddressSet(toAddr(bss),toAddr(bss+bssSize-1));gaps.delete(mem);
        int gapIndex=0;for(AddressRange gap:gaps.getAddressRanges())mem.createUninitializedBlock("bss"+(gapIndex++),gap.getMinAddress(),gap.getLength(),false).setWrite(true);
        // Values from the original startup register initializer at 800064d8/e0.
        for(MemoryBlock block:mem.getBlocks()){
            currentProgram.getProgramContext().setValue(currentProgram.getRegister("r2"),block.getStart(),block.getEnd(),new java.math.BigInteger("80720720",16));
            currentProgram.getProgramContext().setValue(currentProgram.getRegister("r13"),block.getStart(),block.getEnd(),new java.math.BigInteger("8071f720",16));
        }
        List<Function> targets=new ArrayList<>();
        for(String line:Files.readAllLines(Path.of(root,"tools/ghidra-functions.tsv"))){
            String[] row=line.split("\t");long address=Long.parseUnsignedLong(row[0],16),size=Long.parseLong(row[1],16);String name=row[2];
            createLabel(toAddr(address),name.replaceAll("[^A-Za-z0-9_]","_"),true);
            Set<Long> drawTargets=Set.of(0x80256d38L,0x80258a88L,0x802576fcL,0x8011ea94L,0x8025f6c0L);
            if((address<0x80205000L||address>=0x80208b00L)&&!drawTargets.contains(address))continue;
            // Analysis-only ABI simplification: these proven SDK stubs solely
            // save/restore callee-saved GPRs using r11. They preserve input r3.
            // The source DOL and runtime executable are never modified.
            for(long p=address;p<address+size;p+=4){
                int instruction=mem.getInt(toAddr(p));
                if((instruction&0xfc000003)==0x48000001){
                    int displacement=(instruction&0x03fffffc);if((displacement&0x02000000)!=0)displacement|=0xfc000000;
                    long target=p+displacement;
                    if(target>=0x802e58dcL&&target<=0x802e5970L)mem.setInt(toAddr(p),0x60000000);
                }
            }
            AddressSet body=new AddressSet(toAddr(address),toAddr(address+size-1));new DisassembleCommand(toAddr(address),body,true).applyTo(currentProgram,monitor);
            Function f=currentProgram.getFunctionManager().createFunction(name,toAddr(address),body,SourceType.IMPORTED);targets.add(f);
        }
        writer=new PrintWriter(Path.of(root,"evidence/render-pcode.graph.jsonl").toFile(),"UTF-8");writer.println(gson.toJson(Map.of("gk_graph",1,"meta",Map.of("game","MadWorld","authority","Ghidra high Pcode, bounded renderer functions; generic PPC language may miss Gekko paired-float instructions","language",currentProgram.getLanguageID().toString()))));
        DecompInterface decomp=new DecompInterface();decomp.toggleCCode(false);decomp.toggleSyntaxTree(true);decomp.openProgram(currentProgram);
        List<Map<String,Object>> report=new ArrayList<>();
        for(Function f:targets){
            String fid="fn:"+f.getEntryPoint();node(fid,"native-function",f.getName(),Map.of("address",f.getEntryPoint().toString()));
            DecompileResults result=decomp.decompileFunction(f,30,monitor);HighFunction high=result.getHighFunction();report.add(Map.of("function",f.getName(),"address",f.getEntryPoint().toString(),"complete",result.decompileCompleted(),"error",result.getErrorMessage()));if(high==null)continue;
            values=new IdentityHashMap<>();Iterator<PcodeOpAST> ops=high.getPcodeOps();int seq=0;
            while(ops.hasNext()){
                PcodeOpAST op=ops.next();String oid=fid+":op:"+seq;node(oid,"pcode-op",op.getMnemonic(),Map.of("address",op.getSeqnum().getTarget().toString(),"block",op.getParent().getIndex()));edge(fid,oid,"contains",seq++);
                for(int i=0;i<op.getNumInputs();i++)edge(value(op.getInput(i),fid),oid,"feeds",i);
                if(op.getOutput()!=null)edge(oid,value(op.getOutput(),fid),"defines",0);
            }
        }
        writer.close();decomp.dispose();Files.writeString(Path.of(root,"evidence/render-pcode-report.json"),gson.toJson(report));println("RENDER_PCODE_FUNCTIONS="+targets.size());
    }
}
