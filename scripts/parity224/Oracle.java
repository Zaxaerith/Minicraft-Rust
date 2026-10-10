// GPL-3.0-only. Executes the unmodified, pinned Minicraft+ 2.2.4 classes.
package minicraft.level;

import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import org.json.*;
import minicraft.core.Game;
import minicraft.core.io.Settings;
import minicraft.entity.furniture.DeathChest;
import minicraft.entity.mob.Player;
import minicraft.item.*;
import minicraft.level.tile.Tiles;
import minicraft.saveload.Save;
import minicraft.screen.CraftingDisplay;
import minicraft.screen.QuestsDisplay;
import minicraft.screen.TutorialDisplayHandler;
import minicraft.util.AdvancementElement;

public final class Oracle {
    static void write(Path out, String name, String body) throws Exception {
        Files.write(out.resolve(name), body.getBytes(StandardCharsets.UTF_8));
    }
    static JSONObject completed(String criterion) {
        return new JSONObject().put("done", true).put("criteria",
            new JSONObject().put(criterion, "2024-06-01T12:34:56.123"));
    }
    public static void main(String[] args) {
        try { run(args); } catch (Throwable error) { error.printStackTrace(); System.exit(1); }
    }
    static void run(String[] args) throws Exception {
        Path out = Paths.get(args[0]).toAbsolutePath();
        Files.createDirectories(out);
        Game.gameDir = out.toString();
        Settings.set("sound", false);
        Tiles.initTileList();
        AdvancementElement.AdvancementTrigger.getTrigger("inventory_changed");
        Game.player = new Player(null, null);
        Game.currentLevel = 3;
        Player player = Game.player;
        player.x = 104; player.y = 120; player.spawnx = 2; player.spawny = 3;
        player.shirtColor = 0x010000CC;
        player.health = 7; player.extraHealth = 5; player.hunger = 6;
        WateringCanItem can = (WateringCanItem) Items.get("Watering Can");
        can.content = 731;
        player.activeItem = can;
        ToolItem pick = (ToolItem) Items.get("Iron Pickaxe");
        pick.dur = 17;
        player.getInventory().add(pick);
        player.getInventory().add(Items.get("Wood_9"));
        CraftingDisplay.loadUnlockedRecipes(Arrays.asList(new Recipe("Plank_2", "Wood_1")));
        List<String> fields = new ArrayList<>();
        Save.writePlayer(player, fields);
        write(out, "Player.miniplussave", String.join(",", fields) + ",");
        Save.writeInventory(player, fields);
        write(out, "Inventory.miniplussave", String.join(",", fields) + ",");

        Level fixtureLevel = new Level(16, 16, 42L, 0, null, false);
        DeathChest fixtureChest = new DeathChest(player);
        fixtureChest.time = 123;
        fixtureLevel.add(fixtureChest, 104, 120);
        write(out, "Entities.miniplussave", Save.writeEntity(fixtureChest, true) + ",");

        JSONObject progress = new JSONObject().put("Version", "2.2.4")
            .put("CurrentOngoingTutorial", "minicraft.tutorial.getting_rocks")
            .put("minicraft.tutorial.start_getting_wood", completed("has_wood"))
            .put("minicraft.tutorial.getting_rocks", new JSONObject().put("done", false)
                .put("criteria", new JSONObject().put("has_rock", "2024-06-02T01:02:03")))
            .put("minicraft.quest.farming", new JSONObject().put("unlocked", true)
                .put("unlockingCriteria", new JSONObject().put("has_seeds", "2024-06-01T12:34:56")))
            .put("minicraft.quest.farming.crafting_hoe", completed("has_hoe"))
            .put("minicraft.advancements.recipes.plank", completed("has_wood"));
        TutorialDisplayHandler.load(progress);
        QuestsDisplay.load(progress);
        AdvancementElement.loadRecipeUnlockingElements(progress);
        JSONObject saved = new JSONObject().put("Version", "2.2.4");
        TutorialDisplayHandler.save(saved);
        QuestsDisplay.save(saved);
        AdvancementElement.saveRecipeUnlockingElements(saved);
        write(out, "advancements.json", saved.toString(2));

        JSONArray registeredTiles = new JSONArray();
        for (short id = 0; id < 59; id++) {
            minicraft.level.tile.Tile tile = Tiles.get(id);
            registeredTiles.put(new JSONObject().put("id", id).put("name", tile.name).put("data", tile.getDefaultData()));
        }
        fixtureLevel.setTile(1, 1, Tiles.get("Torch"), Tiles.get("Obsidian").id);
        write(out, "tiles.json", new JSONObject().put("tiles", registeredTiles)
            .put("torch", new JSONObject().put("name", fixtureLevel.getTile(1, 1).name)
                .put("data", fixtureLevel.getData(1, 1))).toString(2));

        JSONObject reference = new JSONObject();
        JSONArray lifetimes = new JSONArray();
        for (int difficulty = 0; difficulty < 3; difficulty++) {
            Settings.getEntry("diff").setSelection(difficulty);
            lifetimes.put(new DeathChest().time);
        }
        reference.put("deathChestTicks", lifetimes);
        pick.dur = 1;
        reference.put("lastToolUse", pick.payDurability());
        reference.put("afterLastUse", pick.dur);
        reference.put("depletedToolUse", pick.payDurability());
        Recipe recipe = new Recipe("Wood Door_1", "Plank_5");
        reference.put("craftInsufficient", recipe.craft(player));
        reference.put("woodAfterRejectedCraft", player.getInventory().count(Items.get("Wood")));
        Recipe vanillaPlank = Recipes.craftRecipes.stream().filter(r -> r.getProduct().getName().equalsIgnoreCase("Plank")).findFirst().get();
        reference.put("craftPlank", vanillaPlank.craft(player));
        reference.put("plankAfterCraft", player.getInventory().count(Items.get("Plank")));
        reference.put("woodAfterCraft", player.getInventory().count(Items.get("Wood")));
        write(out, "reference.json", reference.toString(2));

        java.lang.reflect.Field randomField = LevelGen.class.getDeclaredField("random");
        randomField.setAccessible(true);
        Random generator = (Random) randomField.get(null);
        for (int feature : new int[]{16, 32}) {
            generator.setSeed(42L);
            LevelGen noise = new LevelGen(128, 128, feature);
            StringBuilder bits = new StringBuilder();
            for (double value : noise.values) bits.append(Long.toHexString(Double.doubleToLongBits(value))).append('\n');
            write(out, "noise-" + feature + ".txt", bits.toString());
        }

        // Raw LevelGen arrays, before Level structures/stair links. Each seed is
        // explicitly reset by createAndValidateMap; compare names, not Rust IDs.
        JSONArray generationHashes = new JSONArray();
        for (long seed : new long[]{42L, 256L}) {
            for (int depth : new int[]{1, 0, -1, -2, -3, -4}) {
                short[][] map = LevelGen.createAndValidateMap(128, 128, depth, seed);
                StringBuilder rows = new StringBuilder();
                long hash = 0xcbf29ce484222325L;
                for (int i = 0; i < map[0].length; i++) {
                    rows.append(Tiles.get(map[0][i]).name).append('\t').append(map[1][i]).append('\n');
                    for (int value : new int[]{map[0][i] & 255, map[1][i] & 255, (map[1][i] >>> 8) & 255})
                        hash = (hash ^ value) * 0x100000001b3L;
                }
                write(out, "level-" + (seed == 42L ? "" : seed + "-") + depth + ".tsv", rows.toString());
                generationHashes.put(new JSONObject().put("seed", seed).put("depth", depth).put("fnv1aTileData", Long.toHexString(hash)));
            }
        }
        write(out, "generation.json", new JSONObject().put("version", "2.2.4").put("width", 128).put("height", 128).put("maps", generationHashes).toString(2));
        // Java advancement trigger executors may still be alive.
        System.exit(0);
    }
}
