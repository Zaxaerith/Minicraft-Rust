// GPL-3.0-only. Calls the unmodified pinned dev2 World.initWorld and Save paths.
package minicraft.level;

import java.nio.file.*;
import java.nio.charset.StandardCharsets;
import java.lang.reflect.Field;
import java.util.*;
import java.util.concurrent.CompletableFuture;
import java.util.function.Function;
import org.json.*;
import kong.unirest.*;
import minicraft.core.*;
import minicraft.core.io.Settings;
import minicraft.entity.*;
import minicraft.entity.furniture.*;
import minicraft.item.Item;
import minicraft.level.tile.*;
import minicraft.saveload.Save;
import minicraft.screen.*;
import minicraft.screen.entry.InputEntry;
import minicraft.util.AdvancementElement;

public final class WorldAssembly35 {
    // World.initWorld always calls Analytics.WorldCreation.ping(). The oracle
    // supplies an offline transport, not a patched game method or live request.
    static void offline() {
        Unirest.config().asyncClient(new AsyncClient() {
            public <T> T getClient() { return null; }
            public void registerShutdownHook() { }
            public <T> CompletableFuture<HttpResponse<T>> request(HttpRequest request,
                Function<RawResponse,HttpResponse<T>> mapper, CompletableFuture<HttpResponse<T>> result) {
                result.completeExceptionally(new UnirestException("Oracle offline transport: request suppressed"));
                return result;
            }
        });
    }
    static JSONObject position(int x,int y) { return new JSONObject().put("x",x).put("y",y); }
    static String hash(Level level) {
        long value=0xcbf29ce484222325L;
        for(int y=0;y<level.h;y++)for(int x=0;x<level.w;x++) {
            String cell=Migration230.canonical(level.getTile(x,y).name)+":"+level.getData(x,y)+";";
            for(byte b:cell.getBytes(StandardCharsets.UTF_8))value=(value^(b&255))*0x100000001b3L;
        }
        return String.format("%016x",value);
    }
    static JSONObject rawDifference(Level level) {
        ChunkManager raw=LevelGen.createAndValidateMap(level.w,level.h,level.depth,level.getSeed());
        long value=0xcbf29ce484222325L;int changed=0;
        JSONObject first=null;TreeMap<String,Integer> edits=new TreeMap<>();
        for(int y=0;y<level.h;y++)for(int x=0;x<level.w;x++) {
            String oldName=raw.getTile(x,y).name,newName=level.getTile(x,y).name;
            int oldData=raw.getData(x,y),newData=level.getData(x,y);
            String cell=Migration230.canonical(oldName)+":"+oldData+";";
            for(byte b:cell.getBytes(StandardCharsets.UTF_8))value=(value^(b&255))*0x100000001b3L;
            if(!oldName.equals(newName)||oldData!=newData) {
                changed++;edits.merge(oldName+":"+oldData+" -> "+newName+":"+newData,1,Integer::sum);
                if(first==null)first=position(x,y).put("rawTile",oldName).put("rawData",oldData)
                    .put("worldTile",newName).put("worldData",newData);
            }
        }
        return new JSONObject().put("rawHash",String.format("%016x",value)).put("changedCells",changed)
            .put("firstDifference",first==null?JSONObject.NULL:first).put("edits",new JSONObject(edits));
    }
    static JSONObject describe(Level level,Path out) throws Exception {
        JSONArray up=new JSONArray(),down=new JSONArray(),entities=new JSONArray();
        TreeMap<String,Integer> counts=new TreeMap<>(),entityCounts=new TreeMap<>();
        StringBuilder tsv=new StringBuilder();
        for(int y=0;y<level.h;y++)for(int x=0;x<level.w;x++) {
            String name=level.getTile(x,y).name;
            counts.merge(name,1,Integer::sum);
            tsv.append(name).append('\t').append(level.getData(x,y)).append('\n');
            if(name.equalsIgnoreCase("Stairs Up"))up.put(position(x,y));
            if(name.equalsIgnoreCase("Stairs Down"))down.put(position(x,y));
        }
        // Ordering of Level's HashSet is not part of Java Save semantics.
        ArrayList<String> serialized=new ArrayList<>();
        for(Entity e:level.getEntitiesToSave()) {
            String saved=Save.writeEntity(e,true);if(!saved.isEmpty())serialized.add(saved);
        }
        Collections.sort(serialized);
        for(String saved:serialized) {
            entities.put(saved);entityCounts.merge(saved.substring(0,saved.indexOf('[')),1,Integer::sum);
        }
        Files.writeString(out.resolve("world-"+level.w+"-42-"+level.depth+".tsv"),tsv,StandardCharsets.UTF_8);
        return new JSONObject().put("depth",level.depth).put("index",World.lvlIdx(level.depth))
            .put("seed",level.getSeed()).put("hash",hash(level)).put("tiles",new JSONObject(counts))
            .put("stairsUp",up).put("stairsDown",down).put("chestCount",level.chestCount)
            .put("entities",entities).put("entityCounts",new JSONObject(entityCounts))
            .put("assemblyDeltaFromSameSeedRawMap",rawDifference(level));
    }
    static JSONObject chestStep(String action, boolean used, Level level,
        minicraft.entity.mob.Player player, DungeonChest a, DungeonChest b) {
        int apples=0;
        for(Entity e:level.getEntitiesToSave())if(e instanceof ItemEntity) {
            minicraft.item.Item item=((ItemEntity)e).item;
            if(item.getName().equals("Gold Apple"))apples+=minicraft.item.Items.getCount(item);
        }
        return new JSONObject().put("action",action).put("used",used).put("chestCount",level.chestCount)
            .put("keys",player.getInventory().count(minicraft.item.Items.get("Key")))
            .put("goldApples",apples).put("drops",Oracle230.drops(level))
            .put("locked",new JSONArray().put(a.isLocked()).put(b.isLocked()));
    }
    static void chestRewards(Path out) throws Exception {
        // Independent behavior oracle after both ordinary-world saves. Encode
        // real chest instances using the official Save writer, then load them
        // through official Load.loadEntity so chestCount is never invented.
        Level original=new Level(16,16,7L,-4,null,false);
        DungeonChest first=new DungeonChest(null),last=new DungeonChest(null);
        first.getInventory().add(minicraft.item.Items.get("Wood_2"));
        last.getInventory().add(minicraft.item.Items.get("Gem_3"));
        original.add(first,40,56);original.add(last,104,56);
        String savedFirst=Save.writeEntity(first,true),savedLast=Save.writeEntity(last,true);
        Level loaded=new Level(16,16,7L,-4,null,false);Game.levels[5]=loaded;Game.currentLevel=5;
        DungeonChest a=(DungeonChest)minicraft.saveload.Load.loadEntity(savedFirst,Game.VERSION,true);
        DungeonChest b=(DungeonChest)minicraft.saveload.Load.loadEntity(savedLast,Game.VERSION,true);
        minicraft.entity.mob.Player player=new minicraft.entity.mob.Player(null,Game.input);
        Game.player=player;loaded.add(player,40,40);loaded.tick(false);
        if(loaded.chestCount!=2)throw new AssertionError("Actual Load did not restore two locked chests");
        JSONArray cases=new JSONArray();
        cases.put(chestStep("no-key",a.use(player),loaded,player,a,b));Game.setDisplay(null);
        player.getInventory().add(minicraft.item.Items.get("Key_2"));
        cases.put(chestStep("first-unlock",a.use(player),loaded,player,a,b));Game.setDisplay(null);
        cases.put(chestStep("repeat-first",a.use(player),loaded,player,a,b));Game.setDisplay(null);
        cases.put(chestStep("last-unlock",b.use(player),loaded,player,a,b));Game.setDisplay(null);
        cases.put(chestStep("repeat-last",b.use(player),loaded,player,a,b));Game.setDisplay(null);
        if(cases.getJSONObject(0).getBoolean("used") || cases.getJSONObject(1).getInt("goldApples")!=0
            || cases.getJSONObject(2).getInt("keys")!=1 || cases.getJSONObject(3).getInt("goldApples")!=5
            || cases.getJSONObject(4).getInt("goldApples")!=5)throw new AssertionError("Chest use reward behavior differs");
        Oracle230.write(out,"dungeon-chest-reward35.json",new JSONObject().put("version",Game.VERSION.toString())
            .put("source","Save.writeEntity -> Load.loadEntity -> DungeonChest.use")
            .put("officialSavedEntities",new JSONArray().put(savedFirst).put(savedLast)).put("cases",cases));
    }
    static void runtime(Path out) throws Exception {
        // This is the same untouched World.initWorld result saved above. Move the
        // player only to real generated stairs, then call the actual transition
        // and Level tick entrypoints. No entity, tile, health, or RNG is edited.
        final int ticksPerVisit=60;
        // Level transitions unlock real achievements, whose official Save also
        // serializes key preferences. Initialize the ordinary input component;
        // no input events or player input ticks are generated.
        if(Game.input==null)Game.input=new minicraft.core.io.InputHandler();
        Path maps=out.resolve("runtime");Files.createDirectories(maps);
        JSONArray visits=new JSONArray(),transitions=new JSONArray();
        for(int visit=0;visit<5;visit++) {
            Level level=Game.levels[Game.currentLevel];
            int start=Updater.tickCount;
            for(int tick=0;tick<ticksPerVisit;tick++) {
                Updater.setTime(Updater.tickCount+1);Updater.gameTime++;
                level.tick(true);
            }
            JSONObject counts=new JSONObject();
            for(Entity e:level.getEntitiesToSave()) {
                String saved=Save.writeEntity(e,true);
                if(!saved.isEmpty())counts.increment(saved.substring(0,saved.indexOf('[')));
            }
            visits.put(new JSONObject().put("depth",level.depth).put("ticks",ticksPerVisit)
                .put("fromTick",start).put("toTick",Updater.tickCount).put("entities",counts)
                .put("playerHealth",Game.player.health));
            if(visit<4) {
                int sx=-1,sy=-1,best=Integer.MAX_VALUE;
                for(int y=0;y<level.h;y++)for(int x=0;x<level.w;x++)
                    if(level.getTile(x,y)==Tiles.get("Stairs Down")) {
                        int dx=(Game.player.x>>4)-x,dy=(Game.player.y>>4)-y;
                        int distance=dx*dx+dy*dy;
                        if(distance<best) { best=distance;sx=x;sy=y; }
                    }
                if(sx<0)throw new AssertionError("No generated stairs below depth "+level.depth);
                Game.player.x=sx*16+8;Game.player.y=sy*16+8;
                int from=level.depth;
                World.changeLevel(-1);
                // Drain the official queued removal. Tick(false) performs no
                // entity/tile update and no spawning in the abandoned layer.
                level.tick(false);
                Level lower=Game.levels[Game.currentLevel];
                if(lower.getTile(sx,sy)!=Tiles.get("Stairs Up"))throw new AssertionError("Bad transition");
                transitions.put(position(sx,sy).put("fromDepth",from).put("toDepth",lower.depth));
            }
        }
        // Flush the final natural-spawn addition without another full tick.
        Game.levels[Game.currentLevel].tick(false);
        JSONArray layers=new JSONArray();
        int enemies=0;
        for(int depth:World.idxToDepth) {
            Level level=Game.levels[World.lvlIdx(depth)];layers.put(describe(level,maps));
            for(Entity e:level.getEntitiesToSave())
                if(e instanceof minicraft.entity.mob.EnemyMob && !(e instanceof minicraft.entity.mob.AirWizard))enemies++;
        }
        if(enemies==0)throw new AssertionError("No ordinary enemies naturally spawned");
        JSONObject summary=new JSONObject().put("version",Game.VERSION.toString()).put("worldSeed",World.getWorldSeed())
            .put("size",128).put("ticksPerVisit",ticksPerVisit).put("totalLevelTicks",ticksPerVisit*5)
            .put("visits",visits).put("transitions",transitions).put("layers",layers)
            .put("currentLevel",Game.currentLevel).put("player",position(Game.player.x,Game.player.y))
            .put("playerHealth",Game.player.health).put("spawn",position(Game.player.spawnx,Game.player.spawny))
            .put("tickCount",Updater.tickCount).put("gameTime",Updater.gameTime).put("ordinaryEnemies",enemies)
            .put("sourcePath","World.initWorld -> Level.tick(true) + World.changeLevel -> official Save")
            .put("headless",true).put("playerTicks",false).put("inputRenderingAudioTicks",false)
            .put("positioning","scripted player positioning at nearest already-generated stairs; real World.changeLevel")
            .put("rng","unmodified Entity new Random runtime streams; no cross-JVM trajectory equality claim");
        new Save("normal-runtime-42");
        Migration230.zip(Paths.get(Game.gameDir,"saves/normal-runtime-42"),out.resolve("normal-runtime-42.zip"));
        Oracle230.write(out,"world-runtime-42.json",summary);
        Files.copy(Paths.get(Game.gameDir,"Unlocks.json"),out.resolve("normal-runtime-unlocks.json"),StandardCopyOption.REPLACE_EXISTING);
        System.out.println("Official Level runtime + Save: "+enemies+" ordinary enemies after "+ticksPerVisit*5+" level ticks");
    }
    static void run(Path out) throws Exception {
        Files.createDirectories(out);offline();
        Game.gameDir=out.resolve("normal-world").toString();Files.createDirectories(Paths.get(Game.gameDir));
        Settings.set("sound",false);Settings.set("size",128);
        Settings.set("mode","minicraft.settings.mode.survival");
        Settings.getEntry("diff").setSelection(1);
        Settings.set("quests",true);Settings.set("tutorials",true);
        Tiles.initTileList();AdvancementElement.AdvancementTrigger.getTrigger("inventory_changed");
        WorldSelectDisplay.setWorldName("normal-survival-42",false);
        Field seedInput=WorldGenDisplay.class.getDeclaredField("worldSeed");seedInput.setAccessible(true);
        seedInput.set(null,new InputEntry("minicraft.displays.world_gen.world_seed","-?[0-9]+",20,"42"));
        long started=System.nanoTime();
        World.initWorld();
        long elapsed=System.nanoTime()-started;
        JSONArray layers=new JSONArray();
        for(int depth:World.idxToDepth)layers.put(describe(Game.levels[World.lvlIdx(depth)],out));
        JSONArray connections=new JSONArray();
        for(int depth=World.maxLevelDepth;depth>World.minLevelDepth;depth--) {
            Level upper=Game.levels[World.lvlIdx(depth)],lower=Game.levels[World.lvlIdx(depth-1)];
            for(int y=0;y<upper.h;y++)for(int x=0;x<upper.w;x++)if(upper.getTile(x,y)==Tiles.get("Stairs Down")) {
                boolean connected=lower.getTile(x,y)==Tiles.get("Stairs Up");
                connections.put(position(x,y).put("upperDepth",depth).put("lowerDepth",depth-1).put("connected",connected));
                if(!connected)throw new AssertionError("World has an unconnected generated stair at "+depth+":"+x+","+y);
            }
        }
        JSONArray inventory=new JSONArray();for(Item item:Game.player.getInventory().getItems())inventory.put(item.getData());
        JSONObject summary=new JSONObject().put("version",Game.VERSION.toString()).put("worldSeed",World.getWorldSeed())
            .put("size",128).put("generationNanos",elapsed).put("mode",Settings.get("mode"))
            .put("difficulty",Settings.get("diff")).put("theme",Settings.get("theme")).put("type",Settings.get("type"))
            .put("currentLevel",Game.currentLevel).put("player",position(Game.player.x,Game.player.y))
            .put("spawn",position(Game.player.spawnx,Game.player.spawny)).put("inventory",inventory)
            .put("tickCount",Updater.tickCount).put("gameTime",Updater.gameTime).put("layers",layers).put("stairConnections",connections)
            .put("sourcePath","World.initWorld -> new Level(makeWorld=true) -> official Save")
            .put("headless",true).put("network","offline Unirest transport; no analytics sent");
        new Save("normal-survival-42");
        Migration230.zip(Paths.get(Game.gameDir,"saves/normal-survival-42"),out.resolve("normal-survival-42.zip"));
        Oracle230.write(out,"world-assembly-42.json",summary);
        // Derivation is separately asserted against real initialized layers.
        Random derivation=new Random(42L);
        for(int depth=World.maxLevelDepth;depth>=World.minLevelDepth;depth--)
            if(derivation.nextLong()!=Game.levels[World.lvlIdx(depth)].getSeed())throw new AssertionError("World layer seed derivation differs");
        System.out.println("Official World.initWorld + Save: "+out+" ("+elapsed/1_000_000+" ms)");
        runtime(out);
        chestRewards(out);
    }
    public static void main(String[] args) {
        try { run(Paths.get(args[0]).toAbsolutePath());System.exit(0); }
        catch(Throwable error) { error.printStackTrace();System.exit(1); }
    }
}
