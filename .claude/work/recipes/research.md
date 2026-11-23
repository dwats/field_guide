# Recipes: how wikis and planners handle a recipe

Research for the recipes planning interview, 2026-11-23. Five researchers read
the sites below. "Recipe" is the working word; the name is not decided.

Marks: (S) seen on a loaded page. (M) from the researcher's memory, not
loaded.

## What could not be loaded

- Icy Veins refused every request (HTTP 403). Every Icy Veins claim is (M).
- Wowhead pages loaded, but the "Reagent for" and "Created by" tabs and Quick
  Facts did not render. Claims about them are (M).
- Satisfactory Calculator refused requests. FactorioLab and Satisfactory Tools
  returned a shell page; their repositories and documentation were read
  instead, and screen descriptions are partly (M).
- ftb.fandom.com refused requests; ftbwiki.org was read instead.

## Conclusions across all sites

1. Every site keeps a recipe as a structured record, apart from the page's
   prose. None stores it as plain sentences or as generic page links.
2. Two generations exist.
   - Typed by hand on the output item's page as a template call: Minecraft
     Wiki, Vintage Story wiki, Warcraft Wiki.
   - Central data that pages only refer to: Grounded (one template holding
     about 830 recipes), Satisfactory (JSON from the game's files), Factorio
     (infobox pages a bot rewrites from game data), Wowhead (a database; the
     recipe is its own page, a spell, apart from the item it makes).
3. "Used in" on an ingredient's page is either generated or wrong. Generated:
   Minecraft (the template writes a database row on render, and
   `{{crafting usage}}` queries it), Factorio, Satisfactory, Wowhead (M).
   Typed by hand: Vintage Story (the Stick page hard-codes "46 recipes use
   sticks"), Grounded, Warcraft Wiki.
4. A recipe has its own identity. One item has several recipes on every site:
   Satisfactory "Caterium Circuit Board" makes Circuit Board; Grounded invents
   keys such as "Crude Rope Alt"; Wowhead gives each recipe a spell page.
5. Display is the same everywhere: an icon per item, the amount on or beside
   the icon, the name on hover, a click opens the item's page. Minecraft and
   Vintage Story add a text list of ingredients beside the grid.
6. A tree of sub-recipes is rare. No wiki except Wowhead has one. Wowhead's
   Classic spell page expands every craftable path with multiplied totals,
   including routes no player would use (S). The in-game viewer EMI has a
   tree with totals, leftovers, and one default recipe per item chosen by the
   user (S).
7. No web site offers a checklist for several crafts. In-game add-ons do: a
   queue of several recipes and one combined shopping list, saved (Skillet,
   CraftSim, TradeSkillMaster). FactorioLab has a check box on each step row
   (M).

## What a recipe record must hold

Found on at least one site each.

- Inputs with amounts. Outputs with amounts; the output amount is often more
  than 1 (Stick 4, Copper cable 2).
- More than one output: byproducts (Satisfactory), returned containers
  (Minecraft lava bucket leaves a bucket; Vintage Story honey bowl returns a
  bowl), one input to many outputs (stonecutting).
- Grid positions: column letter and row number (A1 to C3; A1 to I9 for a 9 by
  9 grid on ftbwiki.org). A shaped or shapeless flag.
- A slot that accepts any of several items: Minecraft "Any Planks" (about
  1,500 aliases) and semicolon lists; Vintage Story wildcards (`log-*`);
  Grounded smoothie base; Warcraft optional and finishing reagents.
- A station, and often a list of stations (furnace, blast furnace, smoker;
  Assembler or Crafting Bench). Stations that are not objects: "Crafted by
  Hand", "Crafted in World" (Grounded), knapping on the ground (Vintage
  Story).
- Things required and not consumed: tools (Warcraft "Blacksmith Hammer",
  Vintage Story hammer with a durability cost), molds.
- Time. Minecraft and Grounded keep it on the station; Factorio and
  Satisfactory keep a duration on the recipe and a speed on the station.
- Requirements in words or numbers: profession, skill level, the four
  skill-up thresholds, unlock rules joined by "and" and "or", technology,
  temperature, experience gained.
- A category or type per station. Minecraft and ftbwiki.org use one template
  per station; ftbwiki.org has 403 grid templates.

## What a simple record misses

- Amounts that are not a count: liters (Vintage Story barrel), ratio ranges
  (tin bronze is copper 88 to 92 percent, tin 8 to 12 percent), optional
  slots with a range (0 to 2 vegetables).
- Shapes in place of slots: Vintage Story knapping, clay forming, and
  smithing use voxel patterns. The wiki shows them as screenshots and prose,
  not as data.
- Variants: the output's material follows the input (planks of the same wood
  as the log). Minecraft pairs alternatives by list position.
- Amounts that change by recipe rank (Warcraft). Outputs that are not a fixed
  item (Warcraft retail quality). Chance outputs (Factorio recycling 0.25).
- The same item as input and output (Factorio Kovarex). Loops between recipes
  (Satisfactory Recycled Plastic and Recycled Rubber). Wikis list these and do
  not solve them.
- Recipes that differ by game version or edition, side by side.
- Repair and upgrade costs: a second ingredient list per item (Grounded,
  Minecraft).

## Totals and plans

- Factorio "Total raw" is one flat row with fractional amounts and summed time
  (Electronic circuit: time 1.25, copper plate 1.5, iron plate 1). It stops
  at plates, not ore. The value comes from the game, not the wiki.
- "Raw" is a choice. The Kirk McDonald calculator lets a click on an item mark
  it "supplied from elsewhere", which stops the expansion there.
- A walk down the tree works only when each item has one chosen recipe. Loops
  and recipes with several outputs need a solver (FactorioLab uses simplex).
- Fractional totals do not suit a checklist. Whole crafts need rounding up,
  which leaves leftovers. EMI shows leftovers.
- Leveling guides (wow-professions, wow.gg) show one flat list of raw
  materials with icons and counts, and intermediate crafts as steps. No check
  boxes, nothing saved.
- Skillet: a queue of several recipes saved across sessions, one shopping list
  for the whole queue, owned stock subtracted, an ignored materials list.

## Per site, briefly

- Minecraft Wiki: `{{Crafting |B2= Any Planks |B3= Any Planks |Output= Stick,4
  |type= Material}}` on the output's page. One table row per recipe: name,
  ingredients as text, the icon grid with output, notes. Alternatives cycle
  inside a slot. Smelting, brewing, smithing, and stonecutting each have their
  own template and fields. In-game viewers: JEI shows "recipes" on click and
  "uses" on right-click, with a tab per station; EMI shows a tag as one tag
  icon.
- Vintage Story wiki: `{{Grid/Crafting |B2=... |B3=... |Output=...}}` typed by
  hand; no central table. Seven recipe kinds in the game's files: grid,
  knapping, clay forming, smithing, alloy, barrel, cooking. Other processes
  (casting, quern, firepit, kiln) are tables on the process page.
- Factorio wiki: an infobox row of icons with numbers and a clock for time;
  alternates in a body table (process, input, output, made in, technology).
- Satisfactory wiki: one table, a row per recipe: recipe name, ingredients,
  produced in (with seconds), products, unlocked by. Each cell shows amount,
  icon, name, and a rate per minute computed from the duration. Alternates
  carry a badge.
- Grounded wiki: a three-column table (item, materials, station), icon with a
  text link. Fields: item, amount, up to five materials with quantities,
  station. Unlock, tier, and repair cost sit in the infobox.
- Wowhead: a recipe page lists reagents with icon and count, tools, cast time,
  cooldown. The tooltip shows the item's stats and its direct reagents.
- Warcraft Wiki: `ProfessionBox` on the crafted item's page: profession,
  tools, up to eight items with amounts per rank, skill thresholds, optional
  and finishing slots, output amount.
