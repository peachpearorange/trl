# The Rat Lands — Design Document

## Elevator Pitch

Turn-based space mercenary RPG in the tradition of Caves of Qud and Space Station 13. You are a bounty hunter in a mysterious, dangerous galaxy. Fly between handcrafted planets and stations, take jobs from bounty boards and cantina NPCs, explore sprawling alien worlds, recruit crew, upgrade your ship, and get better gear. Save-on-ship-only creates tension. The world is dark and strange — there's magic no one fully understands, factions with hidden agendas, and things in the deep caves that shouldn't exist.

## Simulation Model

Tile-based, turn-based. The game runs at 60fps but only every Nth frame (currently 8) is a **sim step** where gameplay actually advances — entities move, AI acts, status effects tick, combat resolves. The other frames are purely for rendering: linear interpolation of entity positions between tiles, particle effects, animations, and collecting player input. This means movement and actions look smooth despite the discrete tile grid.

Two time modes:
- **Turn-based** (default): sim steps only advance when the player spends a turn (moves, attacks, uses an ability). The world waits for you.
- **Real-time:** sim steps tick automatically every N render frames regardless of player input. Used for specific contexts where the world should keep moving.

The game will primarily run in WASM which is single-threaded, so you can't do any multi-threading and shouldn't try to enable that.

## Tone

Mysterious. The player pursues their goals but doesn't know what's really going on. The world seems dark and serious, but many NPCs don't take things so seriously — you can't tell how bad things really are until you're in too deep. Magic exists but it's just another thing life throws at you, and what's behind it is unknowable. Underrail-esque atmosphere.

## Core Loop

1. Visit a station or hub — pick up quests from bounty boards, cantina NPCs, or other quest-givers
2. Fly to a planet or location — explore, fight, complete objectives
3. Return to ship to save progress — walk back physically, creating tension
4. Turn in quests, sell loot, buy/craft gear
5. Upgrade ship or recruit new crew — unlock new areas, new capabilities
6. Repeat with escalating difficulty and deeper mysteries

## Quest Types

- **Bounty:** Kill a target at a specific location
- **Rescue:** Free someone captured in a hostile area
- **Fetch:** Retrieve an item from a dangerous place
- **Delivery:** Transport goods between locations
- **Investigation:** Figure out what happened / what's going on somewhere

Quests come from bounty boards on stations and from NPCs in cantinas, bars, and hub areas on planets. Some quests chain — completing one unlocks the next. Some quests reward crew recruitment.

## World Structure

~30 large handcrafted levels. Each level starts from proc-gen Python output, then hand-edited in text drawing program, then baked into code as a fixed template.

### Level types:
- **Planet surfaces:** Large, sprawling (much bigger than current 80x80). Points of interest spread across open terrain. Can have sub-areas (cave entrances leading to cave levels, ruins leading to interior levels).
- **Stations:** Smaller, dense, handcrafted. Social hubs with shops, bounty boards, cantinas, docking bays.
- **Dungeons/interiors:** Sub-areas accessed from planet surfaces. Caves, facilities, ruins.
- **Ship interiors:** Player's ship (upgrades to bigger ships over time). Crew lives here.

### Navigation:
- Galaxy is a sparse 3D coordinate system
- Ship travels between locations, costing fuel
- Better ships can reach farther/more dangerous locations (gating progression)

## Combat

### Basics:
- Tile-based, turn-based (see Simulation Model above)
- Melee: bump-attack (current system)
- Ranged/abilities: select with number key, click target tile with mouse
- Some abilities are untargeted (self-buffs, AoE around player, etc.)

### Damage & Defense (New Vegas-style DR/DT):
- Damage Resistance (DR): percentage reduction
- Damage Threshold (DT): flat reduction
- Some enemies are immune or resistant to specific attack types (e.g. EM attacks don't work on organic creatures)
- No complex damage type matrix — keep it intuitive

### Abilities (gained from gear):
- Shield — temporary damage absorption
- Invisibility — enemies can't see you
- Stun grenade — AoE stun
- Laser gun — ranged energy weapon
- Jetpack dash — move multiple tiles in one turn
- Various grenade types
- Status effects (stun, burn, poison, etc.)

### Gear Philosophy:
- Gear defines your build — what you equip determines your abilities
- Everything has tradeoffs: more grenade slots means less armor, jetpack is heavy, etc.
- Stats affected by gear: damage, defense, speed, ability slots, switch time, special movement
- Progression through finding/buying/crafting incrementally better gear
- Tiered but also lateral — some gear is situationally better, not strictly better

## Economy

- Currency: space credits (not gold coins)
- Stations have shops: buy and sell gear, consumables, crafting materials, ship parts
- Bounties and quests pay credits
- Ships cost serious money — major progression milestones
- Fuel costs money
- Crafting supplements buying — some items only craftable, some only purchasable

## Ship Progression

- Player starts in a small starter ship (alone, or with a robot companion)
- Over time, earn enough to buy bigger/better ships
- Better ships: more crew capacity, longer range, more storage, maybe built-in facilities (medbay, workshop)
- Ship replacement gates access to distant/dangerous regions of the galaxy

## Crew

- Player starts alone (or with one robot companion)
- Recruit crew members by finding them in the world — often as quest rewards
- Crew members are Skyrim-style followers: can accompany player on missions
- Crew lives on the ship when not following
- Growing crew over time — each has personality, dialogue, maybe unique abilities
- Whether crew can die: TBD (leaning toward "can be downed but not permanently killed" like Skyrim essential NPCs)

## Dialogue

- Skyrim-style: talk to NPC, choose from dialogue options, branching conversations
- NPCs give quests, sell things, share lore, give hints
- No keyword/encyclopedia system — keep it conversational and natural

## Saving

- Save only on your ship (must physically walk back and dock)
- Death reloads last save
- Creates meaningful tension on long expeditions into dangerous areas
- Player must weigh risk: push deeper or retreat to save?

## Progression Arc

1. **Early game:** Small ship, alone, easy bounties near starting area. Learn the ropes.
2. **Mid game:** Crew of 3-4, better ship, traveling to more dangerous planets. Quests start revealing bigger mysteries.
3. **Late game:** Full crew, top-tier ship, accessing the most dangerous and remote locations. The mysteries deepen — what's behind the magic, what's in the deep caves, what are the factions really doing?

## Current State vs. Target

| System | Current | Target |
|--------|---------|--------|
| Levels | 6 small locations (48x58 to 82x82) | ~30 large levels, some very sprawling |
| Quests | None | Bounty board + NPC quest-givers, 5 quest types |
| Combat | Melee bump-attack, enemy AI | Ranged abilities, gear-based builds, DR/DT |
| Economy | Gold coins, basic loot | Space credits, shops, buying/selling |
| Ship | Static interior, fuel/navigation | Ship tiers, progression gating |
| Crew | 5 fixed NPCs with dialogue | Recruitable followers, growing roster |
| Gear | Basic weapons/armor, crafting | Ability-granting gear with tradeoffs, deep stats |
| Saving | None | Save-on-ship-only |
| Dialogue | Branching trees | Same but with quest integration |
