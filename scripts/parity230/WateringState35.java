// GPL-3.0-only. Calls the unmodified pinned dev2 item/inventory/Save methods.
package minicraft.level;

import java.nio.file.*;
import java.util.*;
import org.json.*;
import minicraft.core.Game;
import minicraft.core.io.Settings;
import minicraft.entity.Direction;
import minicraft.entity.furniture.DeathChest;
import minicraft.entity.mob.Player;
import minicraft.item.*;
import minicraft.level.tile.Tiles;
import minicraft.saveload.Save;
import minicraft.util.AdvancementElement;

public final class WateringState35 {
    private static WateringCanItem can(int content) {
        return (WateringCanItem) Items.get("Watering Can_" + content);
    }
    private static JSONArray contents(Inventory inventory) {
        JSONArray result = new JSONArray();
        for (Item item : inventory.getItems())
            if (item instanceof WateringCanItem) result.put(((WateringCanItem) item).content);
        return result;
    }
    private static JSONObject trace(String operation, WateringCanItem first,
            WateringCanItem second, Level level, boolean result) throws Exception {
        return new JSONObject().put("operation", operation).put("result", result)
            .put("contents", new JSONArray(Arrays.asList(first.content, second.content)))
            .put("cropData", level.getData(2, 2))
            .put("spriteKeys", new JSONArray(Arrays.asList(Oracle230.key(((Item) first).sprite), Oracle230.key(((Item) second).sprite))));
    }
    public static void run(Path out) throws Exception {
        Player savedPlayer = Game.player;
        Level savedSurface = Game.levels[3];
        int savedCurrent = Game.currentLevel;
        Object savedMode = Settings.get("mode");
        try {
            Settings.set("mode", "minicraft.settings.mode.survival");
            Player player = new Player(null, null);
            Game.player = player;
            Game.currentLevel = 3;
            Level level = new Level(16, 16, 42L, 0, null, false);
            Game.levels[3] = level;
            level.add(player, 24, 40);
            level.setTile(2, 2, Tiles.get("Wheat"), 0);
            level.setTile(3, 2, Tiles.get("Water"));
            WateringCanItem first = can(731), second = can(23);
            JSONArray operations = new JSONArray();
            JSONObject initial = trace("initial", first, second, level, true);
            initial.remove("result");
            operations.put(initial);
            operations.put(trace("firstWaterCrop", first, second, level,
                first.interactOn(level.getTile(2, 2), level, 2, 2, player, Direction.RIGHT)));
            operations.put(trace("secondWaterCrop", first, second, level,
                second.interactOn(level.getTile(2, 2), level, 2, 2, player, Direction.RIGHT)));
            operations.put(trace("secondRefill", first, second, level,
                second.interactOn(level.getTile(3, 2), level, 3, 2, player, Direction.RIGHT)));
            level.setTile(4, 2, Tiles.get("Farmland"), 0);
            WateringCanItem farmlandCan = can(9);
            boolean farmlandResult = farmlandCan.interactOn(level.getTile(4, 2), level, 4, 2, player, Direction.RIGHT);
            WateringCanItem empty = can(0);
            boolean emptyResult = empty.interactOn(level.getTile(2, 2), level, 2, 2, player, Direction.RIGHT);

            Inventory bag = new Inventory(), chest = new Inventory();
            WateringCanItem bagFirst = can(731), bagSecond = can(23);
            bag.add(bagFirst); bag.add(bagSecond);
            Item removed = bag.remove(1);
            Item transferRemainder = chest.add(removed);
            JSONObject transfer = new JSONObject().put("bagContents", contents(bag)).put("chestContents", contents(chest))
                .put("removedSameInstance", removed == bagSecond).put("addedSameInstance", chest.get(0) == bagSecond)
                .put("noRemainder", transferRemainder == null);
            Inventory full = new Inventory();
            for (int i = 0; i < full.getMaxSlots(); i++) full.add(can(0));
            Item fullRemainder = full.add(bagFirst);
            JSONObject overflow = new JSONObject().put("slots", full.invSize())
                .put("returnsOriginalInstance", fullRemainder == bagFirst)
                .put("remainder", fullRemainder.getData()).put("sourceContent", bagFirst.content);

            WateringCanItem copied = (WateringCanItem) bagFirst.copy();
            Inventory copiedBulk = new Inventory();
            List<Item> bulkRemainder = copiedBulk.add(bagFirst, 2);
            Inventory removeByName = new Inventory();
            removeByName.add(can(731)); removeByName.add(can(23));
            removeByName.removeItems(can(23), 1);
            JSONObject copy = new JSONObject().put("original", bagFirst.getData()).put("copied", copied.getData())
                .put("sameInstance", copied == bagFirst).put("bulkAddContents", contents(copiedBulk))
                .put("bulkAddRemainder", bulkRemainder.size()).put("removeItems23Remaining", contents(removeByName));

            player.getInventory().clearInv(); player.activeItem = null;
            player.getInventory().add(can(731)); player.getInventory().add(can(23));
            DeathChest death = new DeathChest(player);
            copy.put("deathChestCopiedContents", contents(death.getInventory()));
            player.activeItem = player.getInventory().remove(0);
            player.getInventory().add(Items.get("Iron Pickaxe_17"));
            List<String> serialized = new ArrayList<>();
            Save.writeInventory(player, serialized);
            Inventory restored = new Inventory();
            restored.updateInv(String.join(":", serialized));
            JSONObject serialization = new JSONObject().put("writeInventory", new JSONArray(serialized))
                .put("restoredContents", contents(restored)).put("restoredData", restored.getItemData());

            JSONObject result = new JSONObject().put("javaCommit", "4dc00332e24a727b96be7b885d5cbbdd9c85263c")
                .put("sourceMethods", new JSONArray(Arrays.asList("WateringCanItem.interactOn", "WateringCanItem.copy",
                    "Inventory.add", "Inventory.remove(int)", "Inventory.removeItems", "Inventory.updateInv",
                    "DeathChest(Player)", "Save.writeInventory")))
                .put("operations", operations).put("capacity", first.CAPACITY)
                .put("bareFarmland", new JSONObject().put("result", farmlandResult)
                    .put("contentAfter", farmlandCan.content).put("dataAfter", level.getData(4, 2)))
                .put("emptyWaterCrop", new JSONObject().put("result", emptyResult).put("content", empty.content))
                .put("exactTransfer", transfer).put("fullInventory", overflow)
                .put("upstreamCopyAndNameRemoval", copy).put("serialization", serialization)
                .put("scope", "Direct official item and inventory methods; no GUI events or random flower outcomes. "
                    + "copy() and copy-based death/container paths lose water in upstream dev2; Rust transfers deliberately preserve it.");
            Oracle230.write(out, "watering-instances35.json", result);
        } finally {
            Game.player = savedPlayer;
            Game.levels[3] = savedSurface;
            Game.currentLevel = savedCurrent;
            Settings.set("mode", savedMode);
        }
    }
    public static void main(String[] args) {
        try {
            Path out = Paths.get(args[0]).toAbsolutePath();
            Files.createDirectories(out);
            Game.gameDir = out.toString();
            Settings.set("sound", false);
            Tiles.initTileList();
            AdvancementElement.AdvancementTrigger.getTrigger("inventory_changed");
            run(out);
            System.exit(0);
        } catch (Throwable error) {
            error.printStackTrace();
            System.exit(1);
        }
    }
}
