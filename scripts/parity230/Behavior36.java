// GPL-3.0-only. Actual dev2 room/Sheep methods, independent of World probes.
package minicraft.level;

import java.nio.file.*;
import java.util.*;
import org.json.*;
import minicraft.core.*;
import minicraft.core.io.Settings;
import minicraft.entity.*;
import minicraft.entity.furniture.*;
import minicraft.entity.mob.*;
import minicraft.item.*;
import minicraft.level.tile.Tiles;
import minicraft.saveload.Save;

public final class Behavior36 {
    public static void run(Path out) throws Exception {
        Level[] old=Game.levels.clone();Player oldPlayer=Game.player;
        Level dungeon=new Level(128,128,7L,-4,null,false);Game.levels[5]=dungeon;
        Structure.dungeonBossRoom.draw(dungeon.chunkManager,64,64);
        for(int y=60;y<=68;y++)for(int x=60;x<=68;x++)dungeon.setData(x,y,99);
        dungeon.setTile(69,64,Tiles.get("Ornate Wood"));
        Chest chest=new Chest();chest.getInventory().add(Items.get("Wood_7"));dungeon.add(chest,1064,1032);
        Player p=EntityState35.player(dungeon);
        KnightStatue statue=new KnightStatue(1731);dungeon.add(statue,1032,1032);ObsidianKnight.active=false;
        for(int i=0;i<3;i++)statue.interact(p,null,Direction.NONE);
        dungeon.tick(false);
        JSONArray cells=new JSONArray();
        for(int y=60;y<=68;y++)for(int x=60;x<=68;x++)cells.put(new JSONArray().put(x).put(y).put(dungeon.getTile(x,y).name).put(dungeon.getData(x,y)));
        JSONObject room=new JSONObject().put("cells",cells).put("outside",dungeon.getTile(69,64).name)
            .put("chest",Save.writeEntity(chest,true)).put("boss",Save.writeEntity(ObsidianKnight.entity,true));
        p.remove();ObsidianKnight.entity.tick();dungeon.tick(false);
        JSONArray afterDeath=new JSONArray();for(Entity e:dungeon.getEntitiesToSave())if(e instanceof KnightStatue)afterDeath.put(Save.writeEntity(e,true));
        room.put("afterRemovedPlayer",afterDeath);
        Oracle230.write(out,"boss-room36.json",room);

        JSONArray sheep=new JSONArray();Settings.set("mode","minicraft.settings.mode.survival");
        for(int diff=0;diff<3;diff++)for(DyeItem.DyeColor color:DyeItem.DyeColor.values()) {
            Settings.getEntry("diff").setSelection(diff);
            Level level=EntityState35.level(0);Player player=EntityState35.player(level);
            Sheep mob=new Sheep();level.add(mob,120,120);
            Item dye=Items.get(color.toString().replace('_',' ')+" Dye_2");
            boolean dyed=mob.interact(player,dye,Direction.NONE);
            ToolItem shears=(ToolItem)Items.get("Shears_1");
            ((Random)EntityState35.field(Entity.class,"random").get(mob)).setSeed(1234);
            mob.interact(player,shears,Direction.NONE);
            JSONObject cutDrops=EntityState35.drops(level);
            // Fresh color-matched mob tests real death, including sheared state.
            Level death=EntityState35.level(0);EntityState35.player(death);
            Sheep dying=new Sheep(color);death.add(dying,120,120);
            ((Random)EntityState35.field(Entity.class,"random").get(dying)).setSeed(1234);dying.die();
            JSONObject uncutDrops=EntityState35.drops(death);
            Level cutDeath=EntityState35.level(0);EntityState35.player(cutDeath);
            Sheep cut=new Sheep(color);cut.cut=true;cutDeath.add(cut,120,120);
            ((Random)EntityState35.field(Entity.class,"random").get(cut)).setSeed(1234);cut.die();
            sheep.put(new JSONObject().put("difficulty",diff).put("color",color.ordinal()).put("dyed",dyed)
                .put("dyeRemaining",Items.getCount(dye)).put("colorAfter",mob.color.ordinal()).put("sheared",mob.cut)
                .put("shearsDurability",shears.dur).put("shearDrops",cutDrops).put("deathDrops",uncutDrops)
                .put("shearedDeathDrops",EntityState35.drops(cutDeath)).put("saved",Save.writeEntity(mob,true)));
        }
        Oracle230.write(out,"sheep36.json",new JSONObject().put("javaCommit","4dc00332e24a727b96be7b885d5cbbdd9c85263c").put("cases",sheep));
        // Observe actual Level.trySpawn color choice. Its own constructor RNG
        // is used; never replace it or infer historical color from Java Save.
        Settings.getEntry("diff").setSelection(1);Updater.tickCount=0;Updater.pastDay1=false;
        Level natural=new Level(128,128,42L,0,null,false);EntityState35.player(natural);natural.tick(false);
        Set<Entity> seen=Collections.newSetFromMap(new IdentityHashMap<>());JSONArray colors=new JSONArray();
        java.lang.reflect.Method spawn=Level.class.getDeclaredMethod("trySpawn");spawn.setAccessible(true);
        for(int i=0;i<800;i++) {
            spawn.invoke(natural);
            for(Entity e:natural.getEntitiesToSave())if(e instanceof Sheep && seen.add(e)) {
                long state=WorldStages36.randomState(natural),mask=(1L<<48)-1;
                // Read-only inversion of the last LCG transition, so the actual
                // nextDouble consumed by trySpawn is observable without replay.
                long previous=((state-11)*0xdfe05bcb1365L)&mask;
                double roll=(((previous>>22)<<27)+(state>>21))/(double)(1L<<53);
                colors.put(new JSONObject().put("roll",roll).put("color",((Sheep)e).color.ordinal()));
            }
        }
        if(colors.length()==0)throw new AssertionError("No actual sheep spawned");
        Oracle230.write(out,"sheep-natural36.json",new JSONObject().put("source","unmodified Level.trySpawn; read-only original Random state after each sheep").put("samples",colors));
        System.arraycopy(old,0,Game.levels,0,old.length);Game.player=oldPlayer;ObsidianKnight.active=false;
        Settings.getEntry("diff").setSelection(1);Game.currentLevel=3;
    }
    public static void main(String[] args) throws Exception {
        minicraft.core.Game.gameDir=Paths.get(args[0]).resolve("behavior-game").toString();
        Tiles.initTileList();run(Paths.get(args[0]));System.exit(0);
    }
}
