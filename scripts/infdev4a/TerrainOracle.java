// GPL-3.0: executes the original pinned LevelGen / biomes / ChunkManager.
package minicraft.level;

import java.nio.file.*;
import java.io.*;
import java.lang.reflect.Field;
import java.util.*;
import minicraft.core.Game;
import minicraft.core.io.Settings;
import minicraft.level.biome.*;
import minicraft.level.noise.LevelNoise;
import minicraft.level.tile.*;

public final class TerrainOracle {
    static int kind(String name) {
        switch(name.toLowerCase()) {
            case "grass": return 0; case "water": return 1; case "sand": return 2;
            case "rock": return 3; case "tree": return 4; case "flower": return 5;
            case "cactus": return 6; case "stairs down": return 7;
            default: throw new IllegalArgumentException(name);
        }
    }
    static boolean eligible(Biome b, LevelNoise n, int x, int y) {
        double v=Math.abs(n.getScale64Noise(x,y,0)-n.getScale64Noise(x,y,1));
        double m=Math.abs(Math.abs(n.getScale16Noise(x,y,0)-n.getScale16Noise(x,y,1))-n.getScale16Noise(x,y,2));
        if(b instanceof SurfaceBiome) return v>0.85 && m<0.25;
        if(b instanceof DesertBiome) return v>0.65 && m<0.35;
        return b instanceof RockBiome && n.getHeight(x,y)>0.35;
    }
    public static void main(String[] args) throws Exception {
        Path out=Path.of(args[0]);Files.createDirectories(out);
        Game.gameDir=out.toAbsolutePath().toString();Settings.set("sound",false);
        Tiles.initTileList();Biomes.initBiomeList();
        Field f=Biomes.BiomeCollection.class.getDeclaredField("biomes");f.setAccessible(true);
        Map<String,Biome> map=(Map<String,Biome>)f.get(Biomes.getLayerBiomes(0));
        Files.writeString(out.resolve("biome-order.txt"),String.join("\n",map.keySet())+"\n");
        // Cases span adjacent +/- chunks, far terrain and different world seeds.
        long[][] initial={{42,0,0},{42,-1,0},{42,0,-1},{42,-1,-1},{0,8,-7},
            {-1,-8,7},{256,23,31},{42,20,20},{42,-200,100},{Long.MIN_VALUE,3,-4},
            {42,1000,-1000},{256,-256,256},
            {42,33554431,-1},{42,-33554432,0}};
        List<long[]> cases=new ArrayList<>(Arrays.asList(initial));
        // Find a real naturally selected Rock biome, not a manually forced tile.
        Random scan=new Random(1337);
        boolean found=false;
        for(int i=0;i<4096;i++) {
            int x=scan.nextInt(131072)-65536,y=scan.nextInt(131072)-65536;
            long seed=i%2==0?42:256;
            LevelNoise n=new LevelNoise(seed,x,y,0,1,1);
            if(Biomes.getLayerBiomes(0).getClosestBiome(n,0,0) instanceof RockBiome) {
                cases.add(new long[]{seed,Math.floorDiv(x,64),Math.floorDiv(y,64)});
                found=true;break;
            }
        }
        if(!found)throw new IllegalStateException("No natural Rock sample found in fixed discovery scan");
        StringBuilder summary=new StringBuilder();
        for(long[] c:cases) {
            long seed=c[0];int cx=(int)c[1],cy=(int)c[2];
            ChunkManager cm=new ChunkManager();
            // Exact original stage sequence: prime -> assign biomes -> generate.
            for(int stage=0;stage<2;stage++) LevelGen.advanceChunk(cm,cx,cy,0,seed);
            LevelNoise observed=cm.getChunkNoise(cx,cy); // stage3 releases this reference
            LevelGen.advanceChunk(cm,cx,cy,0,seed);
            int eligible=0,stairs=0;
            String filename="terrain-"+seed+"-"+cx+"-"+cy+".bin";
            try(DataOutputStream stream=new DataOutputStream(Files.newOutputStream(out.resolve(filename)))) {
                for(int y=0;y<64;y++) for(int x=0;x<64;x++) {
                    int tx=cx*64+x,ty=cy*64+y;
                    Biome b=cm.getBiome(tx,ty);LevelNoise n=observed;
                    int tile=kind(cm.getTile(tx,ty).name);
                    boolean random=eligible(b,n,tx,ty);
                    if(random)eligible++;if(tile==7)stairs++;
                    stream.writeByte(tile);stream.writeShort(cm.getData(tx,ty));
                    stream.writeByte(new ArrayList<>(map.values()).indexOf(b));
                    stream.writeBoolean(random);
                }
            }
            summary.append(seed+"\t"+cx+"\t"+cy+"\t"+filename+"\t"+eligible+"\t"+stairs+"\n");
        }
        Files.writeString(out.resolve("terrain-cases.tsv"),summary);
        System.out.println("PASS original staged Infdev surface: "+cases.size()+" full chunks, actual Tile/data/biome and time-RNG eligibility retained");
        System.exit(0);
    }
}
