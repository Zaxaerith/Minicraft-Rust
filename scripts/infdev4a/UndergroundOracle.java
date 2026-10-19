// GPL-3.0: observes original pinned Infdev LevelGen, biomes and Level.loadChunk.
package minicraft.level;

import java.nio.file.*;
import java.io.*;
import java.util.*;
import minicraft.core.Game;
import minicraft.core.io.Settings;
import minicraft.level.biome.*;
import minicraft.level.noise.LevelNoise;
import minicraft.level.tile.*;

public final class UndergroundOracle {
    static int kind(String name) {
        switch(name.toLowerCase()) {
            case "grass": return 0; case "water": return 1; case "sand": return 2;
            case "rock": return 3; case "tree": return 4; case "flower": return 5;
            case "cactus": return 6; case "stairs down": return 7;
            case "dirt": return 8; case "iron ore": return 9; case "gold ore": return 10;
            case "gem ore": return 11; case "lapis": return 12; case "lava": return 13;
            case "obsidian wall": return 14; case "obsidian": return 15;
            case "raw obsidian": return 16; case "ornate obsidian": return 17;
            case "stairs up": return 18; case "obsidian door": return 19;
            default: throw new IllegalArgumentException(name);
        }
    }
    // Read-only observer: every tile still passes through the original writer.
    // Retain final writer provenance so random pool overwrites are not mistaken
    // for deterministic terrain equality. The original algorithms are unchanged.
    static final class Observed extends ChunkManager {
        final HashSet<Long> decorations = new HashSet<>();
        final boolean observe;
        Observed(int depth) { observe=depth==-4; }
        static long key(int x,int y) { return ((long)x<<32) ^ (y&0xffffffffL); }
        @Override public void setTile(int x,int y,Tile tile,int data) {
            super.setTile(x,y,tile,data);
            if(!observe)return;
            boolean fromStructure=false;
            for(StackTraceElement frame:Thread.currentThread().getStackTrace()) {
                if(frame.getClassName().equals("minicraft.level.Structure")) {fromStructure=true;break;}
            }
            long key=key(x,y);
            if(fromStructure)decorations.add(key);else decorations.remove(key);
        }
    }
    static boolean eligible(LevelNoise n,int depth,int x,int y) {
        if(depth==-4)return Math.abs(n.getScale16Noise(x,y,0)-n.getScale16Noise(x,y,1))>=0.18;
        double val=Math.abs(n.getScale32Noise(x,y,0)-n.getScale32Noise(x,y,1));
        double m=Math.abs(Math.abs(n.getScale16Noise(x,y,0)-n.getScale16Noise(x,y,1))-n.getScale16Noise(x,y,2));
        double v=Math.abs(Math.abs(n.getScale16Noise(x,y,3)-n.getScale16Noise(x,y,4))-n.getScale16Noise(x,y,5));
        double w=Math.abs(Math.abs(n.getScale16Noise(x,y,6)-n.getScale16Noise(x,y,7))-n.getScale16Noise(x,y,8));
        if(val>0.25 && w<(depth==-1?0.35:0.45))return false;
        if(val>0.125 && m<0.20 || v<0.25)return false;
        if(n.getScale16Noise(x,y,9)>(depth==-1?0.65:depth==-2?0.675:0.7))return false;
        if(n.getScale32Noise(x,y,3)>(depth==-1?0.8:depth==-2?0.825:0.85))return false;
        return true;
    }
    static int biomeKind(Biome biome) {
        if(biome instanceof IronCaveBiome)return 1;
        if(biome instanceof GoldCaveBiome)return 2;
        if(biome instanceof GemCaveBiome)return 3;
        if(biome instanceof DungeonBiome)return 4;
        throw new IllegalArgumentException("Unexpected original underground biome: "+biome);
    }
    static void stages(ChunkManager cm,int cx,int cy,int depth,long seed) {
        for(int stage=0;stage<3;stage++)LevelGen.advanceChunk(cm,cx,cy,depth,seed);
        if(cm.getChunkStage(cx,cy)!=ChunkManager.CHUNK_STAGE_UNFINISHED_STAIRS)
            throw new AssertionError("Original staging changed");
    }
    public static void main(String[] args)throws Exception {
        Path out=Path.of(args[0]);Files.createDirectories(out);
        Game.gameDir=out.toAbsolutePath().toString();Settings.set("sound",false);
        Tiles.initTileList();Biomes.initBiomeList();
        long[][] cases={{42,0,0},{42,-1,0},{42,0,-1},{42,-1,-1},
            {256,23,31},{-1,-8,7},{Long.MIN_VALUE,3,-4},
            {42,33554431,-1},{42,-33554432,0}};
        StringBuilder summary=new StringBuilder();
        int totalDecor=0;
        for(int depth=-1;depth>=-4;depth--)for(long[] c:cases) {
            long seed=c[0];int cx=(int)c[1],cy=(int)c[2];
            Observed cm=new Observed(depth);
            for(int stage=0;stage<2;stage++)LevelGen.advanceChunk(cm,cx,cy,depth,seed);
            LevelNoise observed=cm.getChunkNoise(cx,cy);
            LevelGen.advanceChunk(cm,cx,cy,depth,seed);
            int random=0,decor=0;
            String filename="underground-"+depth+"-"+seed+"-"+cx+"-"+cy+".bin";
            try(DataOutputStream stream=new DataOutputStream(Files.newOutputStream(out.resolve(filename)))) {
                for(int y=0;y<64;y++)for(int x=0;x<64;x++) {
                    int tx=cx*64+x,ty=cy*64+y;
                    boolean eligible=eligible(observed,depth,tx,ty);
                    boolean decorated=cm.decorations.contains(Observed.key(tx,ty));
                    if(eligible)random++;if(decorated)decor++;
                    stream.writeByte(kind(cm.getTile(tx,ty).name));
                    stream.writeShort(cm.getData(tx,ty));
                    stream.writeByte(biomeKind(cm.getBiome(tx,ty)));stream.writeBoolean(eligible);stream.writeBoolean(decorated);
                }
            }
            totalDecor+=decor;
            summary.append(seed+"\t"+depth+"\t"+cx+"\t"+cy+"\t"+filename+"\t"+random+"\t"+decor+"\n");
        }
        Files.writeString(out.resolve("underground-cases.tsv"),summary);
        // Exercise the actual original Level.loadChunk after naturally generated
        // parent stairs. No tile is forced or deleted to make a connection.
        StringBuilder connections=new StringBuilder();
        int linkCount=0;
        for(int depth=-1;depth>=-4;depth--) {
            int cx=depth==-1?945:-1,cy=depth==-1?877:-1;long seed=42;
            Level parent=new Level(64,64,seed,depth+1,null,false);
            Level child=new Level(64,64,seed,depth,parent,false);
            stages(parent.chunkManager,cx,cy,depth+1,seed);
            stages(child.chunkManager,cx,cy,depth,seed);
            ArrayList<int[]> source=new ArrayList<>();
            for(int x=cx*64;x<cx*64+64;x++)for(int y=cy*64;y<cy*64+64;y++)
                if(parent.getTile(x,y)==Tiles.get("Stairs Down"))source.add(new int[]{x,y});
            child.loadChunk(cx,cy);
            if(child.chunkManager.getChunkStage(cx,cy)!=ChunkManager.CHUNK_STAGE_DONE)
                throw new AssertionError("Original parent/child stage did not finish");
            for(int[] stair:source) {
                if(child.getTile(stair[0],stair[1])!=Tiles.get("Stairs Up"))
                    throw new AssertionError("Original staircase disconnected");
                connections.append(seed+"\t"+depth+"\t"+stair[0]+"\t"+stair[1]+"\t"+child.getData(stair[0],stair[1])+"\n");
                linkCount++;
            }
        }
        Files.writeString(out.resolve("underground-connections.tsv"),connections);
        System.out.println("PASS original Infdev underground: "+(cases.length*4)+" chunks; actual time-pool final writes="+totalDecor+"; original Level.loadChunk natural staircase links="+linkCount);
        System.exit(0);
    }
}
