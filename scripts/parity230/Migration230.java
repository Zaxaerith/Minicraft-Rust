// GPL-3.0-only. Official dev2 Save path, never edited upstream game sources.
package minicraft.level;
import java.nio.file.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.zip.*;
import java.io.*;
import org.json.*;
import minicraft.core.*;
import minicraft.core.io.Settings;
import minicraft.entity.*;
import minicraft.entity.mob.*;
import minicraft.entity.furniture.*;
import minicraft.item.*;
import minicraft.level.tile.*;
import minicraft.saveload.*;
import minicraft.screen.*;
import minicraft.util.AdvancementElement;

public final class Migration230 {
    static void run(Path out) throws Exception {
        Path root=out.resolve("migration");Files.createDirectories(root);
        Game.gameDir=root.toString();Settings.set("size",128);
        Settings.set("mode","minicraft.settings.mode.survival");
        Settings.getEntry("diff").setSelection(1);
        Settings.set("quests",true);Settings.set("tutorials",true);
        java.lang.reflect.Field worldSeed=World.class.getDeclaredField("seed");worldSeed.setAccessible(true);worldSeed.setLong(null,42L);
        Game.currentLevel=3;Game.player=new Player(null,null);
        Player player=Game.player;
        TutorialDisplayHandler.reset(false);QuestsDisplay.resetGameQuests();AdvancementElement.resetRecipeUnlockingElements();
        for(int index=0;index<World.idxToDepth.length;index++) {
            int depth=World.idxToDepth[index];
            Level level=new Level(128,128,42L+index,depth,null,false);
            level.chunkManager=LevelGen.createAndValidateMap(128,128,depth,42L+index);
            Game.levels[index]=level;
        }
        // A real generated terrain source with a controlled safe arrival patch.
        for(int x=4;x<16;x++)for(int y=4;y<16;y++)Game.levels[3].setTile(x,y,Tiles.get("Grass"));
        player.x=104;player.y=120;player.spawnx=6;player.spawny=7;
        player.health=7;player.extraHealth=5;player.hunger=6;player.shirtColor=0x010000CC;
        Game.levels[3].add(player,player.x,player.y);
        Updater.tickCount=48600;Updater.gameTime=123456;
        Game.levels[4].checkAirWizard();
        new Save("migration-base");
        zip(root.resolve("saves/migration-base"),out.resolve("migration-base.zip"));

        // All official registry entries, without relying on Java integer IDs as Rust IDs.
        JSONArray registry=new JSONArray();
        for(short id=0;id<70;id++)registry.put(new JSONObject().put("id",id).put("name",Tiles.get(id).name));
        JSONArray items=new JSONArray();for(Item item:Items.getAll())items.put(item.getName());
        JSONObject catalog=new JSONObject().put("version",Game.VERSION.toString()).put("tiles",registry).put("items",items)
            .put("depths",new JSONArray(World.idxToDepth));
        Files.writeString(out.resolve("migration-registry.json"),catalog.toString(2)+"\n",StandardCharsets.UTF_8);
        // Local Save intentionally omits loose item/projectile/particle entities.
        Game.levels[3].add(new ItemEntity(Items.get("Rose_2"),88,88),88,88);
        player.activeItem=Items.get("Iron Pickaxe_17");
        player.getInventory().add(Items.get("Wood_999"));player.getInventory().add(Items.get("Wood_9"));
        player.getInventory().add(Items.get("Cyan Wool_4"));player.getInventory().add(Items.get("White Lily_2"));
        player.getInventory().add(Items.get("Red Dye_3"));
        CraftingDisplay.loadUnlockedRecipes(Arrays.asList(new Recipe("Red Dye_1","Rose_1"),new Recipe("Cyan Wool_1","White Wool_1","Cyan Dye_1")));
        JSONObject progress=new JSONObject().put("Version",Game.VERSION.toString()).put("CurrentOngoingTutorial","minicraft.tutorial.getting_rocks")
            .put("minicraft.tutorial.start_getting_wood",new JSONObject().put("done",true).put("criteria",new JSONObject().put("has_wood","2024-06-01T12:34:56")));
        progress.put("minicraft.advancements.recipes.red_dye_from_rose",new JSONObject().put("done",true).put("criteria",new JSONObject().put("has_rose","2024-06-02T01:02:03")));
        TutorialDisplayHandler.load(progress);QuestsDisplay.load(progress);AdvancementElement.loadRecipeUnlockingElements(progress);
        Level surface=Game.levels[3];
        surface.setTile(9,8,Tiles.get("Flower"),FlowerTile.FlowerVariant.VIOLET.ordinal());
        surface.setTile(10,8,Tiles.get("Cyan Wool"));
        surface.setTile(11,8,Tiles.get("Torch"),Tiles.get("Cyan Wool").id);
        surface.setTile(12,8,Tiles.get("Wood Door"),1);
        surface.setTile(13,8,Tiles.get("Sign"));
        surface.setTile(14,8,Tiles.get("Stairs Down"));Game.levels[2].setTile(14,8,Tiles.get("Stairs Up"));
        SignDisplay.loadSignTexts(Collections.singletonMap(new AbstractMap.SimpleImmutableEntry<>(0,new minicraft.gfx.Point(13,8)),Arrays.asList("Java230","keep","all","lines")));
        surface.add(new Bed(DyeItem.DyeColor.CYAN),104,136);
        surface.add(new Crafter(Crafter.Type.DyeVat),136,136);
        Chest chest=new Chest();chest.getInventory().add(Items.get("Cyan Wool_4"));chest.getInventory().add(Items.get("Iron Pickaxe_11"));surface.add(chest,168,136);
        DeathChest death=new DeathChest();death.time=123;death.getInventory().add(Items.get("White Lily_2"));death.getInventory().add(Items.get("Iron Sword_7"));surface.add(death,200,136);
        Sheep sheep=new Sheep();sheep.cut=true;sheep.health=4;surface.add(sheep,232,136);
        Cow cow=new Cow();cow.health=2;surface.add(cow,248,136);
        new Save("migration-colors");
        zip(root.resolve("saves/migration-colors"),out.resolve("migration-colors.zip"));
        // Execute the actual inventory loader, including the serialized held
        // instance. Java puts it in the bag and does not restore held selection.
        Player restored=new Player(null,null);
        Load loader=new Load("migration-colors",false);
        loader.loadInventory(restored.getInventory(),Arrays.asList(Files.readString(root.resolve("saves/migration-colors/Inventory.miniplussave")).split(",")));
        JSONArray loadedInventory=new JSONArray();for(Item i:restored.getInventory().getItems())loadedInventory.put(i.getData());
        Files.writeString(out.resolve("migration-operations.json"),new JSONObject().put("activeRestored",restored.activeItem!=null).put("loadedInventory",loadedInventory).toString(2)+"\n",StandardCharsets.UTF_8);
        AchievementsDisplay.unlockAchievements(new JSONArray().put("minicraft.achievement.planks"));
        java.lang.reflect.Constructor<Save> writer=Save.class.getDeclaredConstructor(File.class);writer.setAccessible(true);
        java.lang.reflect.Method writeUnlocks=Save.class.getDeclaredMethod("writeUnlocks");writeUnlocks.setAccessible(true);
        writeUnlocks.invoke(writer.newInstance(root.toFile()));
        Files.copy(root.resolve("Unlocks.json"),out.resolve("migration-unlocks.json"),StandardCopyOption.REPLACE_EXISTING);
        // Valid upstream data the first strict import profile must refuse.
        WateringCanItem first=(WateringCanItem)Items.get("Watering Can");first.content=731;
        WateringCanItem second=(WateringCanItem)Items.get("Watering Can");second.content=23;
        player.activeItem=first;chest.getInventory().add(second);
        surface.add(new Zombie(3),264,136);
        new Save("migration-instances");
        zip(root.resolve("saves/migration-instances"),out.resolve("migration-instances.zip"));
        // Compact all-depth hashes plus full ignored TSVs for raw generation.
        JSONArray generated=new JSONArray();
        for(int[] scenario:new int[][]{{128,42},{128,256},{256,42},{512,42}}) {
            for(int depth:World.idxToDepth) {
                int size=scenario[0];long seed=scenario[1];
                ChunkManager map=LevelGen.createAndValidateMap(size,size,depth,seed);
                StringBuilder rows=new StringBuilder();long hash=0xcbf29ce484222325L;
                for(int y=0;y<size;y++)for(int x=0;x<size;x++) {
                    String name=map.getTile(x,y).name;int data=map.getData(x,y);
                    rows.append(name).append('\t').append(data).append('\n');
                    String canonical=canonical(name)+":"+data+";";
                    for(byte b:canonical.getBytes(StandardCharsets.UTF_8))hash=(hash^(b&255))*0x100000001b3L;
                }
                Files.writeString(out.resolve("depth-"+size+"-"+seed+"-"+depth+".tsv"),rows,StandardCharsets.UTF_8);
                generated.put(new JSONObject().put("size",size).put("seed",seed).put("depth",depth).put("hash",Long.toHexString(hash)));
            }
        }
        Files.writeString(out.resolve("generated-depths.json"),generated.toString(2)+"\n",StandardCharsets.UTF_8);
    }
    static String canonical(String name) {
        String key=name.replaceAll("[^A-Za-z0-9]", "").toLowerCase(Locale.ROOT);
        switch(key) {case "obsidian":return "obsidianfloor";case "cloudcactus":return "cloudore";case "lapis":return "lapisore";case "woodplanks":return "woodfloor";case "stonebricks":return "stonefloor";default:return key;}
    }
    static void zip(Path source,Path destination) throws Exception {
        try(ZipOutputStream zip=new ZipOutputStream(Files.newOutputStream(destination))) {
            try(java.util.stream.Stream<Path> files=Files.walk(source)) {
                for(Path file:(Iterable<Path>)files.filter(Files::isRegularFile).sorted()::iterator) {
                    ZipEntry entry=new ZipEntry(source.relativize(file).toString().replace('\\','/'));entry.setTime(0);zip.putNextEntry(entry);
                    Files.copy(file,zip);zip.closeEntry();
                }
            }
        }
    }
}
