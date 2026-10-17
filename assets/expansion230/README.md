# Java 2.3.0-dev selected resources

These 108 original PNGs were copied byte-for-byte from Minicraft+ commit
[`4dc00332e24a727b96be7b885d5cbbdd9c85263c`](https://github.com/MinicraftPlus/minicraft-plus-revived/tree/4dc00332e24a727b96be7b885d5cbbdd9c85263c/src/client/resources/assets/textures),
branch `2.3.0-dev` (the traditional fixed-map development line). They are covered
by the upstream GPL-3.0 license; retain the project root LICENSE and upstream
attribution. This is separate from the experimental Infdev branch.

`manifest.json` records each source path and SHA-256. The files comprise:

- 60 item sheets: 16 dyes, 10 additional wool colors, 16 colored beds, Dye Vat,
  and 17 additional flowers.
- 18 entity sheets: 16 colored beds, Dye Vat, and the tintable sheep sheet
  (added in Phase 3.6; the original 2.2.4 sheep file remains untouched).
- 30 tile sheets: 10 additional wool colors, all 19 individual flower variants,
  and Ornate Wood (added in Phase 3; its item reuses the original plank sheet).

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

`recipe_unlocks.json` contains 94 selected advancement definitions, with their
values unchanged from the pinned `src/client/resources/resources/recipes.json`.
It covers only the recipes ported in this stage. Rust namespaces these owners
at load time to keep legacy 2.2.4 completion history separate. It is also under
the upstream GPL-3.0 license. The PNG manifest deliberately describes images;
this selected JSON is recorded here separately.

Phase 3.6 uses the dev2 sheep sheet under the normal `assets/textures/entity/sheep.png`
pack key. Its pure-white wool pixels enable the original Java color tint; the old
2.2.4 sheet uses gray wool pixels. Resource-pack precedence is unchanged.
