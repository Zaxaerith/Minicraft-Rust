// GPL-3.0. Executes the unmodified pinned Infdev DeathChest and Inventory.
package minicraft.entity.furniture;

import java.nio.file.*;
import java.util.*;
import org.json.*;
import minicraft.core.Game;
import minicraft.core.io.Settings;
import minicraft.entity.mob.Player;
import minicraft.item.*;
import minicraft.level.tile.Tiles;
import minicraft.util.AdvancementElement;

public final class DeathChestOracle {
    static List<String> contents(Inventory inventory) {
        ArrayList<String> result=new ArrayList<>();
        TreeMap<String,Integer> stacks=new TreeMap<>();
        for(Item item:inventory.getItems()) {
            if(item instanceof StackableItem) stacks.merge(item.getName(),((StackableItem)item).count,Integer::sum);
            else result.add(item.getData());
        }
        for(Map.Entry<String,Integer> row:stacks.entrySet()) result.add("STACK:"+row.getKey()+"_"+row.getValue());
        Collections.sort(result);return result;
    }
    static void require(boolean condition,String message) {if(!condition)throw new AssertionError(message);}
    public static void main(String[] args) {
        try {run(args);System.exit(0);} catch(Throwable e) {e.printStackTrace();System.exit(1);}
    }
    static void run(String[] args)throws Exception {
        Path out=Path.of(args[0]).toAbsolutePath();
        Game.gameDir=out.toString();Settings.set("sound",false);Tiles.initTileList();
        AdvancementElement.AdvancementTrigger.getTrigger("inventory_changed");
        JSONObject result=new JSONObject();JSONArray lifetime=new JSONArray();
        int[] expected={450*60,300*60,150*60};
        for(int difficulty=0;difficulty<3;difficulty++) {
            Settings.getEntry("diff").setSelection(difficulty);
            DeathChest chest=new DeathChest();
            require(chest.time==expected[difficulty],"difficulty lifetime");
            require(!chest.use(null),"cannot open as a normal chest");
            chest.take(null);
            lifetime.put(chest.time);
        }
        Settings.getEntry("diff").setSelection(1);
        Player source=new Player(null,null);Game.player=source;source.getInventory().clearInv();
        StackableItem wood=(StackableItem)Items.get("Wood");wood.count=731;source.getInventory().add(wood);
        ToolItem pick=(ToolItem)Items.get("Iron Pickaxe");pick.dur=23;source.getInventory().add(pick);
        WateringCanItem water=(WateringCanItem)Items.get("Watering Can");water.content=731;source.getInventory().add(water);
        WateringCanItem second=(WateringCanItem)Items.get("Watering Can");second.content=23;source.getInventory().add(second);
        DeathChest chest=new DeathChest(source);List<String> owned=contents(source.getInventory());
        List<String> copied=contents(chest.getInventory());
        require(!owned.equals(copied),"observe upstream watering copy loss instead of hiding it");
        int emptyCans=0;
        for(Item item:chest.getInventory().getItems()) {
            if(item instanceof WateringCanItem) {require(((WateringCanItem)item).content==0,"upstream copy resets water");emptyCans++;}
            if(item instanceof ToolItem) require(((ToolItem)item).dur==23,"tool durability preserved");
        }
        require(emptyCans==2,"separate can instances remain separate");
        Player target=new Player(null,null);target.getInventory().clearInv();
        for(int i=0;i<27;i++) require(target.getInventory().add(Items.get("Wood Axe"))==null,"fill bag");
        chest.touchedBy(target);require(copied.equals(contents(chest.getInventory())),"full bag retains chest inventory");
        target.getInventory().clearInv();chest.touchedBy(target);
        require(chest.getInventory().invSize()==0,"all items taken");
        require(copied.equals(contents(target.getInventory())),"saved chest content conserved on retrieval");
        chest.touchedBy(target);require(copied.equals(contents(target.getInventory())),"repeated touch no duplicate");
        result.put("lifetimeTicks",lifetime).put("sourceInstances",owned).put("constructorInstances",copied).put("upstreamWaterCopyResetsToZero",true).put("fullBagRetains",true).put("retrievalConserves",true);
        Files.writeString(out.resolve("death-chest.json"),result.toString(2)+"\n");
        System.out.println("PASS original Infdev DeathChest: lifetimes, observed water-copy loss, full bag, tool durability, touch/repeated touch");
    }
}
