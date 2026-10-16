// GPL-3.0. Runs unmodified Minicraft+ sources pinned by run.ps1, without starting the GUI.
package minicraft.level;

import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.lang.reflect.Field;
import java.util.*;
import org.json.*;
import minicraft.core.Game;
import minicraft.core.io.Settings;
import minicraft.entity.*;
import minicraft.entity.furniture.*;
import minicraft.entity.mob.Player;
import minicraft.gfx.SpriteLinker.LinkedSprite;
import minicraft.item.*;
import minicraft.level.tile.*;
import minicraft.saveload.*;
import minicraft.util.AdvancementElement;

public final class Oracle230 {
    static void write(Path out, String name, Object value) throws Exception {
        String body = value instanceof JSONObject ? ((JSONObject)value).toString(2) : ((JSONArray)value).toString(2);
        Files.write(out.resolve(name), (body + "\n").getBytes(StandardCharsets.UTF_8));
    }
    static String key(LinkedSprite sprite) throws Exception {
        Field field = LinkedSprite.class.getDeclaredField("key");
        field.setAccessible(true);
        return (String)field.get(sprite);
    }
    static int radius(Entity entity, String name) throws Exception {
        Field field=Entity.class.getDeclaredField(name); field.setAccessible(true);
        return field.getInt(entity);
    }
    static JSONObject recipe(Recipe value) {
        return new JSONObject().put("product", value.getProduct().getName()).put("amount", value.getAmount()).put("costs", new JSONObject(new TreeMap<>(value.getCosts())));
    }
    static JSONArray recipes(List<Recipe> values) {
        JSONArray result = new JSONArray();
        for (Recipe value : values) result.put(recipe(value));
        return result;
    }
    static JSONArray drops(Level level) {
        JSONArray result = new JSONArray();
        for (Entity value : level.getEntitiesToSave()) if (value instanceof ItemEntity) result.put(((ItemEntity)value).item.getData());
        return result;
    }
    public static void main(String[] args) {
        try { run(args); System.exit(0); } catch(Throwable error) { error.printStackTrace(); System.exit(1); }
    }
    static void run(String[] args) throws Exception {
        Path out = Paths.get(args[0]).toAbsolutePath();
        Files.createDirectories(out);
        Game.gameDir = out.toString();
        Settings.set("sound", false);
        Tiles.initTileList();
        AdvancementElement.AdvancementTrigger.getTrigger("inventory_changed");
        Game.player = new Player(null, null);
        Player player = Game.player;
        Game.currentLevel = 3;
        Level level = new Level(16, 16, 42L, 0, null, false);
        Game.levels[3] = level;
        level.add(player, 104, 120);
        JSONArray colors = new JSONArray();
        JSONArray beds = new JSONArray();
        JSONArray wool = new JSONArray();
        for (DyeItem.DyeColor color : DyeItem.DyeColor.values()) {
            String stem = color.toString().toLowerCase();
            String name = minicraft.util.MyUtils.capitalizeFully(stem.replace('_', ' '));
            Item dye = Items.get(name + " Dye");
            Item woolItem = Items.get(name + " Wool");
            colors.put(new JSONObject().put("ordinal", color.ordinal()).put("name", name).put("packedColor", color.color)
                .put("dye", dye.getName()).put("dyeData", dye.getData()).put("dyeSprite", key(dye.sprite))
                .put("wool", woolItem.getName()).put("woolData", woolItem.getData()).put("woolSprite", key(woolItem.sprite)));
            Level woolLevel = new Level(16,16,42L,0,null,false);
            woolLevel.setTile(2,2,Tiles.get("Hole"));
            Item placed = Items.get(woolItem.getName());
            boolean success = placed.interactOn(woolLevel.getTile(2,2),woolLevel,2,2,player,Direction.NONE);
            JSONObject woolEntry = new JSONObject().put("color", color.ordinal()).put("placed", success)
                .put("tile",woolLevel.getTile(2,2).name).put("tileId",woolLevel.getTile(2,2).id).put("data",woolLevel.getData(2,2))
                .put("remaining",Items.getCount(placed));
            ToolItem shears=(ToolItem)Items.get("Shears"); shears.dur=17; player.stamina=10;
            boolean harvested=woolLevel.getTile(2,2).interact(woolLevel,2,2,player,shears,Direction.NONE);
            woolEntry.put("harvested",harvested).put("afterTile",woolLevel.getTile(2,2).name).put("durability",shears.dur).put("drops",drops(woolLevel));
            wool.put(woolEntry);
            FurnitureItem bedItem = (FurnitureItem)Items.get(name + " Bed");
            Level bedLevel = new Level(16,16,42L,0,null,false);
            Game.levels[3]=bedLevel;
            bedLevel.setTile(3,3,Tiles.get("Grass"));
            boolean bedPlaced = bedItem.interactOn(bedLevel.getTile(3,3),bedLevel,3,3,player,Direction.NONE);
            Bed bed=(Bed)bedItem.furniture;
            String saved = Save.writeEntity(bed,true);
            Bed restored=(Bed)Load.loadEntity(saved,Game.VERSION,true);
            beds.put(new JSONObject().put("color",color.ordinal()).put("name",bed.name).put("placed",bedPlaced)
                .put("itemDepleted",bedItem.isDepleted()).put("entitySprite",key(bed.sprite)).put("itemSprite",key(bedItem.sprite))
                .put("xr",radius(bed,"xr")).put("yr",radius(bed,"yr")).put("copyColor",((Bed)bed.copy()).color.ordinal()).put("save",saved).put("restoredColor",restored.color.ordinal()).put("restoredName",restored.name));
        }
        write(out,"colors.json",colors); write(out,"wool.json",wool); write(out,"beds.json",beds);
        JSONArray flowers=new JSONArray();
        for (FlowerTile.FlowerVariant variant : FlowerTile.FlowerVariant.values()) {
            Level flowerLevel=new Level(16,16,42L,0,null,false);
            flowerLevel.setTile(2,2,Tiles.get("Grass"));
            Item item=Items.get(variant.getName());
            boolean success=item.interactOn(flowerLevel.getTile(2,2),flowerLevel,2,2,player,Direction.NONE);
            JSONObject entry=new JSONObject().put("ordinal",variant.ordinal()).put("name",variant.getName()).put("sprite",key(item.sprite))
                .put("placed",success).put("tile",flowerLevel.getTile(2,2).name).put("data",flowerLevel.getData(2,2)).put("remaining",Items.getCount(item));
            boolean harvested=flowerLevel.getTile(2,2).hurt(flowerLevel,2,2,player,1,Direction.NONE);
            entry.put("harvested",harvested).put("afterTile",flowerLevel.getTile(2,2).name).put("drops",drops(flowerLevel));
            flowers.put(entry);
        }
        write(out,"flowers.json",flowers);
        JSONObject tables=new JSONObject().put("dyeVat",recipes(Recipes.dyeVatRecipes)).put("loom",recipes(Recipes.loomRecipes));
        JSONArray workbench=new JSONArray();
        for (Recipe value : Recipes.workbenchRecipes) if(value.getProduct().getName().equals("Dye Vat")) workbench.put(recipe(value));
        tables.put("workbenchDyeVat",workbench);
        write(out,"recipes.json",tables);
        Game.levels[3]=level;
        Crafter vat=new Crafter(Crafter.Type.DyeVat); level.add(vat,56,56);
        String saved=Save.writeEntity(vat,true);
        Crafter restored=(Crafter)Load.loadEntity(saved,Game.VERSION,true);
        write(out,"furniture.json",new JSONObject().put("dyeVatSave",saved).put("dyeVatRestored",restored.type.name())
            .put("dyeVatName",restored.name).put("dyeVatXr",radius(vat,"xr")).put("dyeVatYr",radius(vat,"yr")).put("dyeVatSprite",key(vat.sprite)).put("dyeVatItemSprite",key(vat.itemSprite))
            .put("defaultBedColor",new Bed().color.ordinal()).put("version",Game.VERSION.toString()));
        JSONObject operations=new JSONObject();
        player.getInventory().clearInv(); player.getInventory().add(Items.get("Rose_3"));
        Recipe red=Recipes.dyeVatRecipes.stream().filter(r -> r.getCosts().containsKey("ROSE")).findFirst().get();
        operations.put("redDyeCrafted",red.craft(player)).put("rosesAfterCraft",player.getInventory().count(Items.get("Rose")))
            .put("redDyeAfterCraft",player.getInventory().count(Items.get("Red Dye")));
        Bed whiteBed=new Bed(DyeItem.DyeColor.WHITE); level.add(whiteBed,104,120);
        minicraft.core.Updater.tickCount=minicraft.core.Updater.sleepStartTime;
        player.x=104;player.y=120;
        operations.put("bedUse",whiteBed.use(player)).put("sleeping",Bed.inBed(player)).put("spawnx",player.spawnx).put("spawny",player.spawny);
        Bed.restorePlayers();
        Level shovelLevel=new Level(16,16,42L,0,null,false);
        shovelLevel.setTile(2,2,Tiles.get("Flower"),FlowerTile.FlowerVariant.VIOLET.ordinal());
        ToolItem shovel=(ToolItem)Items.get("Iron Shovel");shovel.dur=17;player.stamina=10;
        operations.put("violetShovelHarvest",shovelLevel.getTile(2,2).interact(shovelLevel,2,2,player,shovel,Direction.NONE))
            .put("violetShovelDrops",drops(shovelLevel));
        write(out,"operations.json",operations);
        generatedFlora(out);
        wateringFlora(out, player);
        Migration230.run(out);
        System.out.println("Pinned 2.3 source oracle: 16 colors, 16 wool placements/shears, 16 bed placements/save/load, 19 flower placements/harvests, actual recipe tables.");
    }
    static void generatedFlora(Path out) throws Exception {
        JSONArray maps = new JSONArray();
        String[] themes = {"normal", "forest", "desert", "plain", "hell"};
        int[][] scenarios = {{128,42,0}, {128,256,0}, {128,42,1}, {128,42,2}, {128,42,3}, {128,42,4}, {256,42,0}, {512,42,0}};
        Settings.set("Type", "minicraft.settings.type.island");
        for (int[] scenario : scenarios) {
            int size=scenario[0], seed=scenario[1], theme=scenario[2];
            Settings.set("Theme", "minicraft.settings.theme." + themes[theme]);
            ChunkManager map=LevelGen.createAndValidateMap(size,size,0,seed);
            int[] counts=new int[19];
            JSONArray flowers=new JSONArray();
            StringBuilder rows=new StringBuilder();
            long hash=0xcbf29ce484222325L;
            for (int y=0;y<size;y++) for (int x=0;x<size;x++) {
                Tile tile=map.getTile(x,y);int data=map.getData(x,y);
                rows.append(tile.name).append('\t').append(data).append('\n');
                String canonical=tile.name.replaceAll("[^A-Za-z0-9]", "").toLowerCase(Locale.ROOT)+":"+data+";";
                for (byte value : canonical.getBytes(StandardCharsets.UTF_8)) hash=(hash^(value&255))*0x100000001b3L;
                if (tile==Tiles.get("Flower")) {
                    counts[data]++;
                    if (size==128 && theme==0) flowers.put(new JSONArray().put(x).put(y).put(data));
                }
            }
            Files.write(out.resolve("flora-"+size+"-"+seed+"-"+theme+".tsv"),rows.toString().getBytes(StandardCharsets.UTF_8));
            maps.put(new JSONObject().put("size",size).put("seed",seed).put("theme",theme)
                .put("counts",new JSONArray(counts)).put("flowers",flowers).put("fnv1aNamesData",Long.toHexString(hash))
                .put("validationTrace",surfaceCandidates(size,seed,map)));
        }
        Settings.set("Theme","minicraft.settings.theme.normal");
        write(out,"generated-flora.json",maps);
    }
    // Replay the unmodified private candidate builder after resetting its actual
    // Random. Stop at the exact map returned by the real validation entry point;
    // no Rust expectations or duplicate validation criteria choose the endpoint.
    static JSONObject surfaceCandidates(int size, long seed, ChunkManager accepted) throws Exception {
        Field field=LevelGen.class.getDeclaredField("random");field.setAccessible(true);
        ((Random)field.get(null)).setSeed(seed);
        java.lang.reflect.Method create=LevelGen.class.getDeclaredMethod("createTopMap",int.class,int.class);
        create.setAccessible(true);
        int firstStairs=0;
        for(int attempt=1;attempt<=10000;attempt++) {
            ChunkManager candidate=(ChunkManager)create.invoke(null,size,size);
            boolean same=true;int stairs=0;
            for(int y=0;y<size;y++) for(int x=0;x<size;x++) {
                if(candidate.getTile(x,y)==Tiles.get("Stairs Down")) stairs++;
                if(candidate.getTile(x,y)!=accepted.getTile(x,y) || candidate.getData(x,y)!=accepted.getData(x,y)) same=false;
            }
            if(attempt==1) firstStairs=stairs;
            if(same) return new JSONObject().put("candidates",attempt).put("firstCandidateStairs",firstStairs).put("acceptedStairs",stairs);
        }
        throw new AssertionError("accepted surface candidate not reproduced");
    }
    static void wateringFlora(Path out, Player player) throws Exception {
        Level level=new Level(16,16,42L,0,null,false);
        level.setTile(8,8,Tiles.get("Grass"));
        boolean[] seen=new boolean[19];int remaining=19;
        // WateringCan constructs new Random per call, so this proves reachable
        // outcomes, not a deterministic cross-language trajectory. Bound work.
        for(int trial=0;trial<50000 && remaining>0;trial++) {
            WateringCanItem can=(WateringCanItem)Items.get("Watering Can");can.content=1;
            can.interactOn(level.getTile(8,8),level,8,8,player,Direction.NONE);
            if(level.getTile(8,8)==Tiles.get("Flower")) {
                int data=level.getData(8,8);
                if(!seen[data]) { seen[data]=true;remaining--; }
                level.setTile(8,8,Tiles.get("Grass"));
            }
        }
        if(remaining!=0) throw new AssertionError("Watering oracle did not observe every flower within bounded work");
        JSONArray variants=new JSONArray();
        for(int i=0;i<seen.length;i++) variants.put(i);
        write(out,"watering-flora.json",new JSONObject().put("observedVariants",variants)
            .put("capacity",((WateringCanItem)Items.get("Watering Can")).CAPACITY).put("chanceDenominator",60).put("rng", "new Random per interaction; reachability only"));
    }

}
