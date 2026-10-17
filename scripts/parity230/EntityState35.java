// GPL-3.0-only. Reflection seeds RNG / invokes protected entry points, never edits upstream sources.
package minicraft.level;

import java.nio.file.*;
import java.lang.reflect.*;
import java.util.*;
import org.json.*;
import minicraft.core.*;
import minicraft.core.io.Settings;
import minicraft.entity.*;
import minicraft.entity.mob.*;
import minicraft.entity.furniture.*;
import minicraft.item.*;
import minicraft.level.tile.*;
import minicraft.saveload.Save;

public final class EntityState35 {
    static EnemyMob mob(String name, int level) throws Exception {
        return (EnemyMob)Class.forName("minicraft.entity.mob."+name).getConstructor(int.class).newInstance(level);
    }
    static Field field(Class<?> type, String name) throws Exception {
        for(Class<?> c=type;c!=null;c=c.getSuperclass())try { Field f=c.getDeclaredField(name);f.setAccessible(true);return f; }catch(NoSuchFieldException ignored){}
        throw new NoSuchFieldException(name);
    }
    static Level level(int depth) {
        Level level=new Level(16,16,42L,depth,null,false);
        for(int y=0;y<16;y++)for(int x=0;x<16;x++)level.setTile(x,y,Tiles.get("Dirt"));
        return level;
    }
    @SuppressWarnings("unchecked")
    static Player player(Level level) throws Exception {
        Player p=new Player(null,null);Game.player=p;p.health=100;
        level.add(p,104,120);
        ((Set<Player>)field(Level.class,"players").get(level)).add(p);
        return p;
    }
    static JSONObject drops(Level level) {
        TreeMap<String,Integer> counts=new TreeMap<>();
        for(Entity e:level.getEntitiesToSave())if(e instanceof ItemEntity) {
            Item i=((ItemEntity)e).item;
            counts.merge(i.getName(),Items.getCount(i),Integer::sum);
        }
        return new JSONObject(counts);
    }
    public static void run(Path out) throws Exception {
        Player savedPlayer=Game.player;Level[] savedLevels=Game.levels.clone();
        JSONArray samples=new JSONArray();
        Settings.set("mode","minicraft.settings.mode.survival");
        for(int diff=0;diff<3;diff++)for(String name:new String[]{"Slime","Zombie","Skeleton","Creeper","Snake","Knight"})for(int lvl=1;lvl<=4;lvl++) {
            Settings.getEntry("diff").setSelection(diff);
            Level level=level(-1);Player p=player(level);
            EnemyMob mob=mob(name,lvl);level.add(mob,120,120);
            Method touch;
            try {touch=mob.getClass().getDeclaredMethod("touchedBy",Entity.class);}
            catch(NoSuchMethodException e){touch=EnemyMob.class.getDeclaredMethod("touchedBy",Entity.class);}
            touch.setAccessible(true);touch.invoke(mob,p);
            int damage=100-p.health;
            // A fresh unfused instance tests the actual species die/drop method.
            EnemyMob dying=mob(name,lvl);level.add(dying,152,120);
            ((Random)field(Entity.class,"random").get(dying)).setSeed(1234L);
            dying.die();
            samples.put(new JSONObject().put("species",name).put("level",lvl).put("difficulty",diff)
                .put("maxHealth",mob.maxHealth).put("contactDamage",damage).put("speed",mob.speed)
                .put("dropsSeed",1234).put("drops",drops(level)));
        }
        JSONArray passiveSamples=new JSONArray();
        for(int diff=0;diff<3;diff++)for(String name:new String[]{"Cow","Pig","Sheep"}) {
            Settings.getEntry("diff").setSelection(diff);
            Level level=level(0);Player p=player(level);
            PassiveMob passive=(PassiveMob)Class.forName("minicraft.entity.mob."+name).getConstructor().newInstance();
            level.add(passive,120,120);
            Method touch=Entity.class.getDeclaredMethod("touchedBy",Entity.class);touch.setAccessible(true);
            touch.invoke(passive,p);
            passiveSamples.put(new JSONObject().put("species",name).put("difficulty",diff)
                .put("initialHealth",passive.health).put("maxHealth",passive.maxHealth)
                .put("contactDamage",100-p.health).put("speed",passive.speed));
        }
        Settings.getEntry("diff").setSelection(1);
        JSONArray spawnerSamples=new JSONArray();
        for(String scenario:new String[]{"underground","day","night","torch","far","full","blocked"}) {
            int depth=scenario.equals("day")||scenario.equals("night")?0:-1;
            Level level=level(depth);Player p=player(level);
            Spawner sp=new Spawner(new Zombie(3));level.add(sp,104,120);
            field(Spawner.class,"spawnTick").setInt(sp,1);
            ((Random)field(Entity.class,"random").get(sp)).setSeed(1234L);
            ((Random)field(Spawner.class,"rnd").get(sp)).setSeed(42L);
            Updater.tickCount=scenario.equals("day")?20000:50000;
            if(scenario.equals("far"))p.x=400;
            if(scenario.equals("torch"))level.setTile(6,7,Tiles.get("Torch"));
            if(scenario.equals("blocked"))for(int y=6;y<=8;y++)for(int x=5;x<=7;x++)level.setTile(x,y,Tiles.get("Rock"));
            level.mobCount=scenario.equals("full")?level.maxMobCount:0;
            sp.tick();
            JSONArray spawned=new JSONArray();for(Entity e:level.getEntitiesToSave())if(e instanceof Zombie) {
                EnemyMob m=(EnemyMob)e;spawned.put(new JSONObject().put("level",m.lvl).put("maxHealth",m.maxHealth).put("x",m.x).put("y",m.y));
            }
            int before=field(Spawner.class,"health").getInt(sp);
            ToolItem tool=(ToolItem)Items.get("Iron Pickaxe_17");
            ((Random)field(Entity.class,"random").get(sp)).setSeed(55L);
            sp.interact(p,tool,Direction.NONE);
            spawnerSamples.put(new JSONObject().put("scenario",scenario).put("spawned",spawned)
                .put("resetTimer",field(Spawner.class,"spawnTick").getInt(sp)).put("healthBeforeHit",before)
                .put("healthAfterHit",field(Spawner.class,"health").getInt(sp)).put("toolDurability",tool.dur));
        }
        Level dungeon=level(-4);Game.levels[5]=dungeon;Player p=player(dungeon);
        KnightStatue statue=new KnightStatue(1731);dungeon.add(statue,104,120);ObsidianKnight.active=false;
        JSONArray taps=new JSONArray();for(int i=0;i<3;i++)taps.put(statue.interact(p,null,Direction.NONE));
        JSONObject statueState=new JSONObject().put("persisted",Save.writeEntity(statue,true)).put("bossHealth",statue.getBossHealth())
            .put("taps",taps).put("bossHealthAfterAwakening",ObsidianKnight.entity.health).put("phase",field(ObsidianKnight.class,"phase").getInt(ObsidianKnight.entity));
        // The actual constructor starts phase zero; first tick changes phase for <2500 HP.
        ObsidianKnight.entity.tick();statueState.put("phaseAfterTick",field(ObsidianKnight.class,"phase").getInt(ObsidianKnight.entity));
        Oracle230.write(out,"entity-state35.json",new JSONObject().put("javaCommit","4dc00332e24a727b96be7b885d5cbbdd9c85263c")
            .put("enemies",samples).put("passiveMobs",passiveSamples).put("spawners",spawnerSamples).put("statue",statueState));
        System.arraycopy(savedLevels,0,Game.levels,0,savedLevels.length);Game.player=savedPlayer;Game.currentLevel=3;
        ObsidianKnight.active=false;
        // Actual official Save with multiple levels/health values; retain all
        // prior colors, independent cans, chests and advancement state.
        int i=0;
        for(String name:new String[]{"Slime","Zombie","Skeleton","Creeper","Snake","Knight"})for(int lvl:new int[]{1,3,4}) {
            EnemyMob m=mob(name,lvl);m.health=Math.max(1,m.maxHealth-2);
            int index=1+(i%3);Game.levels[index].add(m,400+(i%6)*32,400+(i/6)*32);i++;
        }
        Game.levels[2].add(new Spawner(new Zombie(3)),280,248);
        Game.levels[1].add(new Spawner(new Skeleton(4)),312,248);
        Game.levels[3].add(new Spawner(new Sheep()),344,248);
        Game.levels[5].add(new KnightStatue(1731),400,400);
        new Save("migration-entities35");
        Migration230.zip(Paths.get(Game.gameDir).resolve("saves/migration-entities35"),out.resolve("migration-entities35.zip"));
    }
}
