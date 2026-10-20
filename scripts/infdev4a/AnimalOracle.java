// GPL-3.0. Observer of the unmodified fixed Infdev Pig/PassiveMob/MobAi.
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
        Observed(){super(128,128,42L,0,null,false);}
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

}
