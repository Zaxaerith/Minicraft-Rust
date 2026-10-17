// GPL-3.0-only. Observes real pinned World.initWorld without consuming game RNG.
package minicraft.level;

import java.nio.file.*;
import java.io.*;
import java.lang.reflect.Field;
import java.util.*;
import java.util.concurrent.atomic.AtomicLong;
import org.json.*;
import minicraft.core.*;
import minicraft.core.io.Settings;
import minicraft.entity.Entity;
import minicraft.level.tile.Tiles;
import minicraft.saveload.Save;
import minicraft.screen.*;
import minicraft.screen.entry.InputEntry;
import minicraft.util.AdvancementElement;

public final class WorldStages36 {
    static Path output;
    static final JSONArray stages = new JSONArray();
    static final Field randomField, seedField;
    static {
        try {
            randomField=Level.class.getDeclaredField("random");randomField.setAccessible(true);
            seedField=Random.class.getDeclaredField("seed");seedField.setAccessible(true);
        } catch(Exception e) {throw new ExceptionInInitializerError(e);}
    }
    static long randomState(Level level) throws Exception {
        return ((AtomicLong)seedField.get(randomField.get(level))).get();
    }
    static void raw(ChunkManager map,Level level) { snapshot(level,map,"raw"); }
    public static void observe(Level level,String stage) { snapshot(level,level.chunkManager,stage); }
    static void snapshot(Level level,ChunkManager map,String stage) {
        if(output==null)return;
        try {
            String file="depth-"+level.depth+"-"+stage+".bin";
            JSONArray up=new JSONArray(),down=new JSONArray(),entities=new JSONArray();
            long hash=0xcbf29ce484222325L;
            try(DataOutputStream data=new DataOutputStream(new BufferedOutputStream(Files.newOutputStream(output.resolve(file))))) {
                for(int y=0;y<level.h;y++)for(int x=0;x<level.w;x++) {
                    String name=map.getTile(x,y).name;
                    data.writeShort(map.getTile(x,y).id);data.writeShort(map.getData(x,y));
                    String cell=Migration230.canonical(name)+":"+map.getData(x,y)+";";
                    for(byte b:cell.getBytes(java.nio.charset.StandardCharsets.UTF_8))hash=(hash^(b&255))*0x100000001b3L;
                    if(name.equalsIgnoreCase("Stairs Up"))up.put(new JSONArray().put(x).put(y));
                    if(name.equalsIgnoreCase("Stairs Down"))down.put(new JSONArray().put(x).put(y));
                }
            }
            ArrayList<String> saved=new ArrayList<>();
            for(Entity e:level.getEntitiesToSave()) {String value=Save.writeEntity(e,true);if(!value.isEmpty())saved.add(value);}
            Collections.sort(saved);for(String value:saved)entities.put(value);
            stages.put(new JSONObject().put("depth",level.depth).put("stage",stage).put("seed",level.getSeed())
                .put("rngState",randomState(level)).put("map",file).put("hash",String.format("%016x",hash))
                .put("stairsUp",up).put("stairsDown",down).put("entities",entities).put("chestCount",level.chestCount));
        } catch(Exception e) {throw new RuntimeException("Read-only World probe failed",e);}
    }
    public static void main(String[] args) throws Exception {
        output=Paths.get(args[0]).toAbsolutePath();Files.createDirectories(output);
        int size=Integer.parseInt(args[1]);long seed=Long.parseLong(args[2]);
        WorldAssembly35.offline();Game.gameDir=output.resolve("game").toString();Files.createDirectories(Paths.get(Game.gameDir));
        Settings.set("sound",false);Settings.set("size",size);Settings.set("mode","minicraft.settings.mode.survival");
        Settings.getEntry("diff").setSelection(1);Settings.set("quests",true);Settings.set("tutorials",true);
        Tiles.initTileList();AdvancementElement.AdvancementTrigger.getTrigger("inventory_changed");
        WorldSelectDisplay.setWorldName("world36",false);
        Field widget=WorldGenDisplay.class.getDeclaredField("worldSeed");widget.setAccessible(true);
        widget.set(null,new InputEntry("minicraft.displays.world_gen.world_seed","-?[0-9]+",20,Long.toString(seed)));
        World.initWorld();
        if(stages.length()!=30)throw new AssertionError("Expected30 real stage observations, found "+stages.length());
        JSONObject result=new JSONObject().put("javaCommit","4dc00332e24a727b96be7b885d5cbbdd9c85263c")
            .put("size",size).put("seed",seed).put("stages",stages).put("playerX",Game.player.x).put("playerY",Game.player.y)
            .put("spawnX",Game.player.spawnx).put("spawnY",Game.player.spawny).put("tickCount",Updater.tickCount).put("gameTime",Updater.gameTime)
            .put("observation","Read-only bytecode callbacks after original constructor calls; original RNGs never replaced or advanced by callbacks");
        // Derive seeds independently of the initialized world, checking signed nextLong.
        Random derivation=new Random(seed);
        for(int depth=1;depth>=-4;depth--)if(Game.levels[World.lvlIdx(depth)].getSeed()!=derivation.nextLong())throw new AssertionError("layer seed mismatch");
        new Save("world36");
        Oracle230.write(output,"world36.json",result);
        Migration230.zip(output.resolve("game/saves/world36"),output.resolve("official-save.zip"));
        // Small compressed source snapshots, not a copied Java checkout.
        Path pack=output.resolveSibling(output.getFileName()+".zip");
        try(java.util.zip.ZipOutputStream zip=new java.util.zip.ZipOutputStream(Files.newOutputStream(pack))) {
            ArrayList<Path> files=new ArrayList<>();files.add(output.resolve("world36.json"));
            for(String name:new String[]{"Game.miniplussave","Player.miniplussave","Inventory.miniplussave","advancements.json"})files.add(output.resolve("game/saves/world36").resolve(name));
            try(java.util.stream.Stream<Path> found=Files.list(output)) {found.filter(p->p.toString().endsWith(".bin")).sorted().forEach(files::add);}
            for(Path p:files){java.util.zip.ZipEntry entry=new java.util.zip.ZipEntry(p.getFileName().toString());entry.setTime(0);zip.putNextEntry(entry);Files.copy(p,zip);zip.closeEntry();}
        }
        System.out.println("PASS actual World.initWorld staged source: size="+size+" seed="+seed+" stages="+stages.length());
        System.exit(0);
    }
}
