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
        Observed(){super(128,128,42L,0,null,false);}
        @Override public ItemEntity dropItem(int x,int y,Item item){
            if(!item.getName().equalsIgnoreCase("Raw Pork")) throw new AssertionError(item.getName());
            pork+=item instanceof StackableItem?((StackableItem)item).count:1;
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
        System.out.println("PASS original Infdev Pig: actual constructor health, seeded Normal death drops and120 real AI ticks");
    }
}
