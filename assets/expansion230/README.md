# Java 2.3.0-dev selected resources

These 106 original PNGs were copied byte-for-byte from Minicraft+ commit
[`4dc00332e24a727b96be7b885d5cbbdd9c85263c`](https://github.com/MinicraftPlus/minicraft-plus-revived/tree/4dc00332e24a727b96be7b885d5cbbdd9c85263c/src/client/resources/assets/textures),
branch `2.3.0-dev` (the traditional fixed-map development line). They are covered
by the upstream GPL-3.0 license; retain the project root LICENSE and upstream
attribution. This is separate from the experimental Infdev branch.

`manifest.json` records each source path and SHA-256. The files comprise:

- 60 item sheets: 16 dyes, 10 additional wool colors, 16 colored beds, Dye Vat,
  and 17 additional flowers.
- 17 furniture sheets: 16 colored beds and Dye Vat.
- 29 tile sheets: 10 additional wool colors and all 19 individual flower variants.

No existing `assets/assets` or `assets/resources` file is replaced. The six
original wool tile sheets, existing rose/daisy item sheets, and canonical old
white wool item (`wool.png`) retain their existing bytes. Legacy flower tile IDs
keep their original two shape sheets; new flowers use the new tile type and
ordinal data. Legacy bed state zero retains `bed.png`; colored states use the
new bed sheets. No additional animation metadata is needed by these sources.

The physical namespace keeps resource provenance and old assets separate.
Runtime sheet keys retain upstream names, so resource packs can override
`assets/textures/item/black_dye.png`, `assets/textures/tile/allium.png`,
`assets/textures/entity/blue_bed.png`, and `assets/textures/{item,entity}/dyevat.png`
using the same validated folder/ZIP loader and pack ordering as other sheets.
Missing new keys in older packs fall back to these embedded defaults.

`recipe_unlocks.json` contains 93 selected advancement definitions, with their
values unchanged from the pinned `src/client/resources/resources/recipes.json`.
It covers only the recipes ported in this stage. Rust namespaces these owners
at load time to keep legacy 2.2.4 completion history separate. It is also under
the upstream GPL-3.0 license. The PNG manifest deliberately describes images;
this selected JSON is recorded here separately.
