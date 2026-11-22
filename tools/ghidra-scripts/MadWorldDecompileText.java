// Bounded DOL decompile-to-text research adapter for reading selected original
// functions. args: root, hex targets separated by "," or "+" (use "+" through analyzeHeadless.bat), output path.
// Same analysis memory setup as MadWorldPlayerStatePcode; emits C text only.
// @category MadWorld
import ghidra.app.script.GhidraScript;
import ghidra.app.cmd.disassemble.DisassembleCommand;
import ghidra.app.decompiler.*;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.mem.*;
import ghidra.program.model.symbol.SourceType;
import com.google.gson.*;
import java.nio.file.*;
import java.io.*;
import java.util.*;

public class MadWorldDecompileText extends GhidraScript {
    long word(byte[] b,int p){return Integer.toUnsignedLong(java.nio.ByteBuffer.wrap(b,p,4).getInt());}
    public void run() throws Exception {
        String[] args=getScriptArgs();String root=args[0];Set<Long> wanted=new LinkedHashSet<>();for(String t:args[1].split("[,+]"))wanted.add(Long.parseUnsignedLong(t.trim(),16));String out=args[2];
        byte[] dol=Files.readAllBytes(Path.of("D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/sys/main.dol"));
        List<Address> previous=new ArrayList<>();for(Function fn:currentProgram.getFunctionManager().getFunctions(true))previous.add(fn.getEntryPoint());for(Address a:previous)currentProgram.getFunctionManager().removeFunction(a);
        Memory mem=currentProgram.getMemory();for(MemoryBlock block:mem.getBlocks())mem.removeBlock(block,monitor);
        for(int i=0;i<18;i++){
            int j=i<7?i:i-7;int offsets=i<7?0:0x1c,addresses=i<7?0x48:0x64,sizes=i<7?0x90:0xac;
            long offset=word(dol,offsets+j*4),address=word(dol,addresses+j*4),size=word(dol,sizes+j*4);if(size==0)continue;
            MemoryBlock block=mem.createInitializedBlock((i<7?"text":"data")+j,toAddr(address),new ByteArrayInputStream(dol,(int)offset,(int)size),size,monitor,false);block.setRead(true);block.setWrite(i>=7);block.setExecute(i<7);
        }
        long bss=word(dol,0xd8),bssSize=word(dol,0xdc);AddressSet gaps=new AddressSet(toAddr(bss),toAddr(bss+bssSize-1));gaps.delete(mem);
        int gapIndex=0;for(AddressRange gap:gaps.getAddressRanges())mem.createUninitializedBlock("bss"+(gapIndex++),gap.getMinAddress(),gap.getLength(),false).setWrite(true);
        byte[] rso=Files.readAllBytes(Path.of(root,"tools/enemy-analysis.bin"));
        JsonObject module=JsonParser.parseString(Files.readString(Path.of(root,"research/enemy-link.json"))).getAsJsonObject();
        for(JsonElement item:module.getAsJsonArray("sections")){
            JsonObject section=item.getAsJsonObject();long address=section.get("address").getAsLong(),size=section.get("size").getAsLong();int offset=section.get("offset").getAsInt(),index=section.get("index").getAsInt();
            MemoryBlock block=section.get("zero").getAsBoolean()?mem.createUninitializedBlock("enemy.bss"+index,toAddr(address),size,false):mem.createInitializedBlock("enemy.s"+index,toAddr(address),new ByteArrayInputStream(rso,offset,(int)size),size,monitor,false);
            block.setRead(true);block.setWrite(index!=1);block.setExecute(index==1);
        }
        for(MemoryBlock block:mem.getBlocks()){
            currentProgram.getProgramContext().setValue(currentProgram.getRegister("r2"),block.getStart(),block.getEnd(),new java.math.BigInteger("80720720",16));
            currentProgram.getProgramContext().setValue(currentProgram.getRegister("r13"),block.getStart(),block.getEnd(),new java.math.BigInteger("8071f720",16));
        }
        List<Function> targets=new ArrayList<>();
        for(String line:Files.readAllLines(Path.of(root,"tools/ghidra-functions.tsv"))){
            String[] row=line.split("\t");long address=Long.parseUnsignedLong(row[0],16),size=Long.parseLong(row[1],16);String name=row[2];
            createLabel(toAddr(address),name.replaceAll("[^A-Za-z0-9_]","_"),true);
            if(!wanted.contains(address))continue;
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
        DecompInterface decomp=new DecompInterface();decomp.toggleCCode(true);decomp.openProgram(currentProgram);
        try(PrintWriter w=new PrintWriter(Path.of(out).toFile(),"UTF-8")){
            for(Function f:targets){
                DecompileResults r=decomp.decompileFunction(f,60,monitor);
                w.println("// ===== "+f.getName()+" @ "+f.getEntryPoint()+" complete="+r.decompileCompleted()+" err="+r.getErrorMessage());
                if(r.getDecompiledFunction()!=null)w.println(r.getDecompiledFunction().getC());
            }
        }
        decomp.dispose();println("wrote "+out);
    }
}
