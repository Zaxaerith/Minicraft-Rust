// GPL-3.0. Observer of unmodified fixed Infdev passive mobs and Zombie/MobAi.
package minicraft.entity.mob;
import minicraft.core.Game;
import minicraft.core.io.Settings;
import minicraft.entity.Entity;
import minicraft.entity.ItemEntity;
import minicraft.item.*;
import minicraft.level.Level;
import minicraft.level.tile.Tiles;
import minicraft.util.AdvancementElement;
import java.nio.file.*;
import java.lang.reflect.Field;
import java.util.*;
import org.json.*;

public final class AnimalOracle {
    static class Observed extends Level {
        int pork;
        final TreeMap<String,Integer> loot=new TreeMap<>();
        Observed(){this(0);}
        Observed(int depth){super(128,128,42L,depth,null,false);}
        @Override public ItemEntity dropItem(int x,int y,Item item){
            int n=item instanceof StackableItem?((StackableItem)item).count:1;
            loot.merge(item.getName(),n,Integer::sum);
            if(item.getName().equalsIgnoreCase("Raw Pork")) pork+=n;
            return super.dropItem(x,y,item);
        }
    }
    static void seed(Entity e,long seed)throws Exception {
        Field f=Entity.class.getDeclaredField("random");f.setAccessible(true);((Random)f.get(e)).setSeed(seed);
    }
    public static void main(String[] args){try{run(Path.of(args[0]));System.exit(0);}catch(Throwable e){e.printStackTrace();System.exit(1);}}
    static void run(Path out)throws Exception {
        Game.gameDir=out.toAbsolutePath().toString();Settings.set("sound",false);Tiles.initTileList();
        AdvancementElement.AdvancementTrigger.getTrigger("inventory_changed");
        JSONArray health=new JSONArray();for(int d=0;d<3;d++){Settings.getEntry("diff").setSelection(d);health.put(new Pig().maxHealth);}
        Settings.getEntry("diff").setSelection(1);
        Observed level=new Observed();
        for(int x=0;x<64;x++)for(int y=0;y<64;y++)level.setTile(x,y,Tiles.get("Grass"));
        JSONArray drops=new JSONArray();
        for(long value:new long[]{0,42,256,-1,731}){
            Pig pig=new Pig();pig.setLevel(level,512,512);seed(pig,value);level.pork=0;pig.die();
            if(level.pork<1||level.pork>2)throw new AssertionError("pork count");
            drops.put(new JSONObject().put("seed",value).put("pork",level.pork));
        }
        Pig pig=new Pig();pig.setLevel(level,512,512);seed(pig,42);
        JSONArray walk=new JSONArray();
        for(int i=0;i<120;i++){
            pig.tick();
            walk.put(new JSONArray(new int[]{pig.x,pig.y,pig.xmov,pig.ymov,pig.randomWalkTime,pig.walkDist}));
        }
        JSONObject result=new JSONObject().put("healthByDifficulty",health).put("normalDrops",drops).put("walkSeed42",walk);
        Files.writeString(out.resolve("animals.json"),result.toString(2)+"\n");
        passive(out);
        hostile(out);
        environment(out);
        System.out.println("PASS original Infdev Pig: actual constructor health, seeded Normal death drops and120 real AI ticks");
    }
    static void passive(Path out)throws Exception {
        JSONObject result=new JSONObject();
        JSONArray health=new JSONArray();
        for(int d=0;d<3;d++){
            Settings.getEntry("diff").setSelection(d);
            health.put(new JSONArray(new int[]{new Cow().maxHealth,new Pig().maxHealth,new Sheep().maxHealth}));
        }
        Settings.getEntry("diff").setSelection(1);
        result.put("healthCowPigSheep",health);
        Observed level=new Observed();
        for(int x=0;x<64;x++)for(int y=0;y<64;y++)level.setTile(x,y,Tiles.get("Dirt"));
        Player player=new Player(null,null);Game.player=player;level.add(player,0,0);level.tick(false);
        JSONArray drops=new JSONArray(),interactions=new JSONArray();
        for(long value:new long[]{0,42,256,-1,731}){
            Cow cow=new Cow();cow.setLevel(level,512,512);seed(cow,value);level.loot.clear();cow.die();
            drops.put(new JSONObject().put("seed",value).put("kind","Cow").put("loot",new JSONObject(level.loot)));
            for(boolean cut:new boolean[]{false,true}){
                Sheep sheep=new Sheep(DyeItem.DyeColor.RED);sheep.cut=cut;sheep.setLevel(level,512,512);seed(sheep,value);
                level.loot.clear();sheep.die();
                drops.put(new JSONObject().put("seed",value).put("kind","Sheep").put("cut",cut).put("loot",new JSONObject(level.loot)));
            }
        }
        for(DyeItem.DyeColor color:DyeItem.DyeColor.values()){
            Sheep sheep=new Sheep();sheep.setLevel(level,512,512);seed(sheep,42);
            DyeItem dye=(DyeItem)Items.get(color.toString().replace('_',' ')+" Dye");dye.count=2;
            boolean dyed=sheep.interact(player,dye,minicraft.entity.Direction.RIGHT);
            ToolItem shears=new ToolItem(ToolType.Shears);shears.dur=2;level.loot.clear();
            boolean sheared=sheep.interact(player,shears,minicraft.entity.Direction.RIGHT);
            JSONObject loot=new JSONObject(level.loot);
            boolean twice=sheep.interact(player,shears,minicraft.entity.Direction.RIGHT);
            interactions.put(new JSONObject().put("color",color.ordinal()).put("dyed",dyed).put("dyeRemaining",dye.count)
                .put("sheared",sheared).put("cut",sheep.cut).put("secondShear",twice).put("durability",shears.dur).put("loot",loot));
        }
        result.put("normalDrops",drops).put("interactions",interactions);
        minicraft.core.Updater.changeTimeOfDay(minicraft.core.Updater.Time.Day);
        Field levelRandom=Level.class.getDeclaredField("random");levelRandom.setAccessible(true);
        java.lang.reflect.Method trySpawn=Level.class.getDeclaredMethod("trySpawn");trySpawn.setAccessible(true);
        JSONArray spawns=new JSONArray();
        for(long input=0;input<32;input++){
            Observed habitat=new Observed();
            for(int x=-64;x<64;x++)for(int y=-64;y<64;y++)habitat.setTile(x,y,Tiles.get("Grass"));
            Player observer=new Player(null,null);Game.player=observer;habitat.add(observer,8,8);habitat.tick(false);
            ((Random)levelRandom.get(habitat)).setSeed(input);
            trySpawn.invoke(habitat);habitat.tick(false);
            JSONArray found=new JSONArray();
            for(Entity entity:habitat.getEntityArray())if(entity instanceof PassiveMob){
                found.put(new JSONObject().put("kind",entity.getClass().getSimpleName()).put("x",entity.x).put("y",entity.y)
                    .put("color",entity instanceof Sheep?((Sheep)entity).color.ordinal():15));
            }
            spawns.put(new JSONObject().put("seed",input).put("animals",found));
        }
        result.put("dayGrassFirstSpawn",spawns);

        JSONObject walks=new JSONObject();
        for(MobAi mob:new MobAi[]{new Cow(),new Sheep()}){
            mob.setLevel(level,512,512);seed(mob,42);JSONArray walk=new JSONArray();
            for(int i=0;i<120;i++){
                mob.tick();walk.put(new JSONArray(new int[]{mob.x,mob.y,mob.xmov,mob.ymov,mob.randomWalkTime,mob.walkDist}));
            }
            walks.put(mob.getClass().getSimpleName(),walk);
        }
        result.put("dirtWalkSeed42",walks);
        long grazingSeed=0;
        // Select an input seed, then execute the original unmodified tick.
        for(;;grazingSeed++){
            Random r=new Random(grazingSeed);
            if(r.nextInt(40)!=0 && r.nextInt(1000)==0) break;
        }
        JSONArray grazing=new JSONArray();
        for(MobAi mob:new MobAi[]{new Cow(),new Sheep()}){
            level.setTile(32,32,Tiles.get("Grass"));mob.setLevel(level,512,512);seed(mob,grazingSeed);
            if(mob instanceof Sheep)((Sheep)mob).cut=true;
            mob.tick();
            grazing.put(new JSONObject().put("kind",mob.getClass().getSimpleName()).put("seed",grazingSeed)
                .put("tile",level.getTile(32,32).name).put("cut",mob instanceof Sheep && ((Sheep)mob).cut));
        }
        result.put("grazing",grazing);
        Cow knock=new Cow();knock.setLevel(level,512,512);seed(knock,42);knock.doHurt(1,minicraft.entity.Direction.RIGHT);
        JSONArray recoil=new JSONArray();for(int i=0;i<10;i++){knock.tick();recoil.put(new JSONArray(new int[]{knock.x,knock.y,knock.hurtTime}));}
        result.put("originalHurtMovement",recoil);
        Cow far=new Cow();far.setLevel(level,5000,5000);seed(far,42);
        result.put("originalFarDistance",level.distanceOfClosestPlayer(far));far.handleDespawn();result.put("originalFarRemoved",far.isRemoved());
        Sheep expired=new Sheep();expired.setLevel(level,512,512);expired.age=18000;seed(expired,42);expired.tick();
        result.put("expiredAwayRemoved",expired.isRemoved());
        Files.writeString(out.resolve("passive-animals.json"),result.toString(2)+"\n");
        System.out.println("PASS original Cow/Sheep health, drops, 16 dye/shear interactions, 120 AI ticks, grazing and despawn observations");
    }

    static void hostile(Path out)throws Exception {
        JSONObject result=new JSONObject();JSONArray health=new JSONArray(),contact=new JSONArray(),drops=new JSONArray();
        Observed floor=new Observed(-1);for(int x=0;x<64;x++)for(int y=0;y<64;y++)floor.setTile(x,y,Tiles.get("Dirt"));
        Player p=new Player(null,null);Game.player=p;floor.add(p,560,560);floor.tick(false);
        for(int d=0;d<3;d++){
            Settings.getEntry("diff").setSelection(d);JSONArray levels=new JSONArray();
            for(int level=1;level<=3;level++){
                Zombie z=new Zombie(level);levels.put(z.maxHealth);z.setLevel(floor,512,512);seed(z,42);
                p.health=10;p.hurtTime=0;z.touchedBy(p);
                contact.put(new JSONObject().put("difficulty",d).put("level",level).put("health",p.health).put("hurtTime",p.hurtTime));
            }
            health.put(levels);
        }
        Settings.getEntry("diff").setSelection(1);
        for(int input=0;input<128;input++){
            Zombie z=new Zombie(1);z.setLevel(floor,512,512);seed(z,input);floor.loot.clear();z.die();
            drops.put(new JSONObject().put("seed",input).put("loot",new JSONObject(floor.loot)));
        }
        JSONObject walks=new JSONObject();
        for(int distance:new int[]{48,300}){
            p.x=512+distance;p.y=512+distance;p.health=10;p.hurtTime=0;
            Zombie z=new Zombie(1);z.setLevel(floor,512,512);seed(z,42);JSONArray walk=new JSONArray();
            for(int tick=0;tick<80;tick++){z.tick();walk.put(new JSONArray(new int[]{z.x,z.y,z.xmov,z.ymov,z.randomWalkTime,z.walkDist}));}
            walks.put(Integer.toString(distance),walk);
        }
        result.put("healthByDifficultyLevel",health).put("contact",contact).put("normalDrops",drops).put("walkSeed42",walks);
        minicraft.core.Updater.changeTimeOfDay(minicraft.core.Updater.Time.Day);
        Field random=Level.class.getDeclaredField("random");random.setAccessible(true);
        java.lang.reflect.Method spawn=Level.class.getDeclaredMethod("trySpawn");spawn.setAccessible(true);
        JSONArray spawns=new JSONArray();
        for(int depth=-1;depth>=-3;depth--){for(int input=0;input<16;input++){
            Observed habitat=new Observed(depth);for(int x=-64;x<64;x++)for(int y=-64;y<64;y++)habitat.setTile(x,y,Tiles.get("Dirt"));
            Player observer=new Player(null,null);Game.player=observer;habitat.add(observer,8,8);habitat.tick(false);((Random)random.get(habitat)).setSeed(input);
            spawn.invoke(habitat);habitat.tick(false);JSONArray found=new JSONArray();
            for(Entity e:habitat.getEntityArray())if(e instanceof EnemyMob){EnemyMob enemy=(EnemyMob)e;found.put(new JSONObject().put("kind",e.getClass().getSimpleName()).put("level",enemy.lvl).put("health",enemy.health).put("x",e.x).put("y",e.y));}
            spawns.put(new JSONObject().put("depth",depth).put("seed",input).put("enemies",found));
        }}
        result.put("undergroundFirstSpawn",spawns);
        Observed arrival=new Observed(-1);for(int x=30;x<35;x++)for(int y=30;y<35;y++)arrival.setTile(x,y,Tiles.get("Dirt"));
        Player incoming=new Player(null,null);Game.player=incoming;arrival.add(incoming,512,512);
        Zombie occupant=new Zombie(1);arrival.add(occupant,512,512);arrival.tick(false);
        boolean escaped=incoming.move(1,0);
        result.put("overlapEscape",new JSONObject().put("moved",escaped).put("playerX",incoming.x).put("playerY",incoming.y)
            .put("enemyX",occupant.x).put("enemyY",occupant.y).put("enemyHealth",occupant.health));

        Files.writeString(out.resolve("enemies.json"),result.toString(2)+"\n");
        System.out.println("PASS original Zombie: three difficulties/levels, actual contact/drops/AI and private underground trySpawn (all original species retained)");
    }

    static void environment(Path out)throws Exception {
        JSONObject result=new JSONObject();JSONArray times=new JSONArray();
        for(int tick:new int[]{-1,0,16199,16200,32399,32400,48599,48600,64799,64800}){
            minicraft.core.Updater.pastDay1=false;minicraft.core.Updater.setTime(tick);
            times.put(new JSONObject().put("input",tick).put("tick",minicraft.core.Updater.tickCount).put("pastDayOne",minicraft.core.Updater.pastDay1).put("phase",minicraft.core.Updater.getTime().ordinal()));
        }
        result.put("times",times);JSONArray lights=new JSONArray();
        for(int origin:new int[]{-65,0,64}) {
            Observed floor=new Observed(-1);floor.setTile(origin,origin,minicraft.level.tile.Tiles.get("Torch"),0);
            for(int dx=-4;dx<=4;dx++)for(int dy=-4;dy<=4;dy++)lights.put(new JSONObject().put("origin",origin).put("dx",dx).put("dy",dy).put("lit",floor.isLight(origin+dx,origin+dy)).put("pixelCall",floor.isLight((origin+dx)*16+8,(origin+dy)*16+8)));
        }
        result.put("torches",lights);JSONArray lanterns=new JSONArray();
        for(int tile:new int[]{0,8,-65})for(minicraft.entity.furniture.Lantern.Type type:minicraft.entity.furniture.Lantern.Type.values()){
            Observed floor=new Observed(-1);minicraft.entity.furniture.Lantern lamp=new minicraft.entity.furniture.Lantern(type);floor.add(lamp,tile*16+8,tile*16+8);floor.tick(false);
            lanterns.put(new JSONObject().put("tile",tile).put("type",type.name()).put("radius",lamp.getLightRadius()).put("tileCall",floor.isLight(tile,tile)).put("pixelCall",floor.isLight(tile*16+8,tile*16+8)));
        }
        result.put("lanterns",lanterns);
        JSONArray hp=new JSONArray(),drops=new JSONArray(),contact=new JSONArray();
        Observed floor=new Observed(-1);for(int x=0;x<64;x++)for(int y=0;y<64;y++)floor.setTile(x,y,Tiles.get("Dirt"));
        Player player=new Player(null,null);Game.player=player;floor.add(player,800,800);floor.tick(false);
        for(int difficulty=0;difficulty<3;difficulty++){
            Settings.getEntry("diff").setSelection(difficulty);JSONArray row=new JSONArray();
            for(int level=1;level<=3;level++) {Slime slime=new Slime(level);row.put(slime.maxHealth);slime.setLevel(floor,512,512);player.health=10;player.hurtTime=0;slime.touchedBy(player);contact.put(new JSONObject().put("difficulty",difficulty).put("level",level).put("health",player.health).put("hurt",player.hurtTime));}
            hp.put(row);
        }
        Settings.getEntry("diff").setSelection(1);
        for(int seed=0;seed<32;seed++) {Slime slime=new Slime(1);slime.setLevel(floor,512,512);seed(slime,seed);floor.loot.clear();slime.die();drops.put(new JSONObject().put("seed",seed).put("loot",new JSONObject(floor.loot)));}
        Field jump=Slime.class.getDeclaredField("jumpTime");jump.setAccessible(true);
        JSONObject walks=new JSONObject();
        for(int distance:new int[]{24,300}) {
            Slime slime=new Slime(1);slime.setLevel(floor,512,512);seed(slime,42);JSONArray walk=new JSONArray();
            for(int tick=0;tick<160;tick++) {player.x=slime.x+distance;player.y=slime.y+distance;slime.tick();walk.put(new JSONArray(new int[]{slime.x,slime.y,slime.xmov,slime.ymov,slime.randomWalkTime,slime.walkDist,jump.getInt(slime)}));}
            walks.put(Integer.toString(distance),walk);
        }
        result.put("slimeHealth",hp).put("slimeContact",contact).put("slimeDrops",drops).put("slimeWalkSeed42MovingObserver",walks);
        JSONArray surfaceSpawns=new JSONArray();
        Field rng=Level.class.getDeclaredField("random");rng.setAccessible(true);
        java.lang.reflect.Method spawn=Level.class.getDeclaredMethod("trySpawn");spawn.setAccessible(true);
        for(int phase:new int[]{1,3})for(boolean past:new boolean[]{false,true})for(int input=0;input<16;input++) {
            minicraft.core.Updater.changeTimeOfDay(phase==1?minicraft.core.Updater.Time.Day:minicraft.core.Updater.Time.Night);minicraft.core.Updater.pastDay1=past;
            Observed habitat=new Observed(0);for(int x=-64;x<64;x++)for(int y=-64;y<64;y++)habitat.setTile(x,y,Tiles.get("Grass"));
            Player observer=new Player(null,null);Game.player=observer;habitat.add(observer,8,8);habitat.tick(false);((Random)rng.get(habitat)).setSeed(input);
            spawn.invoke(habitat);habitat.tick(false);JSONArray found=new JSONArray();
            for(Entity entity:habitat.getEntityArray())if(entity instanceof MobAi) {
                MobAi mob=(MobAi)entity;found.put(new JSONObject().put("kind",mob.getClass().getSimpleName()).put("x",mob.x).put("y",mob.y).put("health",mob.health).put("level",mob instanceof EnemyMob?((EnemyMob)mob).lvl:1));
            }
            surfaceSpawns.put(new JSONObject().put("phase",phase).put("pastDayOne",past).put("seed",input).put("mobs",found));
        }
        result.put("surfaceFirstSpawn",surfaceSpawns);
        Files.writeString(out.resolve("environment.json"),result.toString(2)+"\n");
        System.out.println("PASS original calendar/Torch/Lantern unit observations and Slime health/contact/drop/jump AI");
    }

}
