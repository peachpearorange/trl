# Defined NPCs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move NPC definitions into individual files under `src/npcs/`, add `Object::defined_npc()` constructor, migrate Mira, and create 4 new NPCs with unique dialogue.

**Architecture:** Each NPC is a file in `src/npcs/` exporting a `pub fn name() -> Object`. A shared `Object::defined_npc()` base in `entities.rs` bundles the common components (Named, Stats, Wielding, Wearing, Glyph, Dialogue). Mira migrates from `entities.rs`+`dialogue.rs` into `src/npcs/mira.rs`.

**Tech Stack:** Rust, Bevy 0.18

---

### Task 1: Add `Object::defined_npc()` to `entities.rs`

**Files:**
- Modify: `src/entities.rs:271-274` (after `npc()`)

- [ ] **Step 1: Add the `defined_npc` constructor**

In `src/entities.rs`, after the existing `npc()` fn (line ~274), add:

```rust
  /// Fully-defined NPC: named, statted, equipped, visible, conversable.
  pub fn defined_npc(
    named: Named,
    stats: Stats,
    wielding: Option<Item>,
    wearing: Option<Armor>,
    glyph: Glyph,
    dialogue: &'static DialogueTree,
  ) -> Self {
    Self::npc()
      .add(named)
      .add(stats)
      .add(Wielding(wielding))
      .add(Wearing(wearing))
      .add(glyph)
      .add(Dialogue(dialogue))
  }
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

- [ ] **Step 3: Commit**

```bash
git add src/entities.rs
git commit -m "feat: add Object::defined_npc() constructor"
```

---

### Task 2: Create `src/npcs/mod.rs` and migrate Mira

**Files:**
- Create: `src/npcs/mod.rs`
- Create: `src/npcs/mira.rs`
- Modify: `src/main.rs:3` (replace `mod dialogue;` with `mod npcs;`)
- Modify: `src/main.rs:690-692` (update catgirl spawn to use `npcs::mira::mira()`)
- Delete: `src/dialogue.rs`

- [ ] **Step 1: Create `src/npcs/mira.rs`**

```rust
use crate::entities::*;

static DIALOGUE: DialogueTree = tree(&[
  node("root", "Oh— you're not a rat person. That's... refreshing. What do you want?", &[
    go("Who are you?",                     "who"),
    go("What is this place?",              "place"),
    go("What's with all the rat soldiers?", "rats"),
    end("Nothing. Goodbye."),
  ]),
  node("who",
    "I'm Mira. Used to be a cartographer's assistant up on the surface. \
     Mapped half these tunnels myself, actually. Then the Compact moved in \
     and... well. I map for survival now.",
    &[
      go("What's the Compact?",           "compact"),
      go("What happened on the surface?", "surface"),
      end("Goodbye."),
    ],
  ),
  node("compact",
    "Rat-people. Not just soldiers — they have scholars, merchants, the whole \
     apparatus. They want the deep levels. Something down there they need badly \
     enough to occupy the caves with an army.",
    &[
      go("What do they want down there?", "compact_want"),
      go("Are they dangerous?",           "compact_danger"),
      end("Goodbye."),
    ],
  ),
  node("compact_want",
    "Old machines. There are ruins in the deep caves — before my time, before \
     anyone's time. The Compact thinks those machines still work. They might be right.",
    &[go("What kind of machines?", "machines"), end("Goodbye.")],
  ),
  node("machines",
    "I don't know exactly. My old employer used to say 'the kind that ends \
     arguments.' He wasn't a comforting man.",
    &[end("Goodbye.")],
  ),
  node("compact_danger",
    "Individually? No worse than anyone with a spear. As an army? Very much yes. \
     Stay low, stay quiet, and you'll probably be fine. Probably.",
    &[end("Goodbye.")],
  ),
  node("surface",
    "Overrun. The Compact has outposts up there now. I got out before they \
     started checking papers. Barely.",
    &[go("Do you want to go back?", "surface_back"), end("Goodbye.")],
  ),
  node("surface_back",
    "Every day. But not until something changes down here. \
     You look like something that changes things. No pressure.",
    &[end("Goodbye.")],
  ),
  node("place",
    "Old mining tunnel, mostly. Pre-Compact, pre-everything. Someone carved it \
     out looking for salt, found the deeper caves instead, and left in a hurry. \
     You can tell by the tools they abandoned.",
    &[go("What's in the deeper caves?", "deep"), end("Goodbye.")],
  ),
  node("deep",
    "That's the question, isn't it. I've mapped down to level two. Level three \
     is... I turned back. Something moved in the dark that wasn't rats.",
    &[end("Goodbye.")],
  ),
  node("rats",
    "Compact soldiers. Organized, disciplined, and they don't like trespassers. \
     I've been hiding from them for three weeks. The trick is they can't see \
     well in complete darkness, but they can hear everything.",
    &[go("Any weaknesses?", "rats_weak"), end("Goodbye.")],
  ),
  node("rats_weak",
    "Apart from the dark? They're proud. Call one a coward and it stops \
     thinking tactically. Doesn't help if there are six of them, but... \
     it's something.",
    &[end("Goodbye.")],
  ),
]);

pub fn mira() -> Object {
  Object::defined_npc(
    Named {
      name: "Mira",
      flavor: "She eyes you warily, ears flat against her head.",
    },
    Stats { hp: 8, max_hp: 8, attack: 2, move_speed: 4.0, attack_speed: 1.2 },
    None,
    None,
    Glyph { ch: 'c', color: Color::srgb(0.9, 0.7, 0.9) },
    &DIALOGUE,
  )
}
```

Note: Mira's `Named.name` changes from `"Catgirl"` to `"Mira"` since she's a defined character.

- [ ] **Step 2: Create `src/npcs/mod.rs`**

```rust
pub mod mira;
```

- [ ] **Step 3: Update `src/main.rs` — replace `mod dialogue` with `mod npcs`**

Replace:
```rust
mod dialogue;
```
With:
```rust
mod npcs;
```

Replace the spawn call (lines 690-692):
```rust
  Object::catgirl()
    .add(Dialogue(&dialogue::MIRA))
    .spawn_at(&mut commands, cx1, cy1, START_Z);
```
With:
```rust
  npcs::mira::mira().spawn_at(&mut commands, cx1, cy1, START_Z);
```

Also remove `Dialogue` from the `trl::entities` import if it's no longer used directly in main.rs (it may still be needed by interaction code — check before removing).

- [ ] **Step 4: Remove `Object::catgirl()` from `entities.rs`**

Delete lines 349-361 (the `catgirl()` fn).

- [ ] **Step 5: Delete `src/dialogue.rs`**

```bash
rm src/dialogue.rs
```

- [ ] **Step 6: Verify it compiles**

Run: `cargo check`
Expected: compiles with no errors

- [ ] **Step 7: Run the game to verify Mira still works**

Run: `cargo run`
Expected: Mira spawns near the player, can be talked to, dialogue works as before, name now shows "Mira" instead of "Catgirl"

- [ ] **Step 8: Commit**

```bash
git add src/npcs/ src/main.rs src/entities.rs
git rm src/dialogue.rs
git commit -m "refactor: migrate Mira into src/npcs/mira.rs"
```

---

### Task 3: Add Chronos — time-travelling sock wizard

**Files:**
- Create: `src/npcs/chronos.rs`
- Modify: `src/npcs/mod.rs` (add `pub mod chronos;`)
- Modify: `src/main.rs` (add spawn call)

- [ ] **Step 1: Create `src/npcs/chronos.rs`**

```rust
use crate::entities::*;

static DIALOGUE: DialogueTree = tree(&[
  node("root",
    "Shh! Don't move. I'm tracking a temporal disturbance. \
     ...No, wait. That's just you. False alarm. What do you want?",
    &[
      go("Who are you?",                    "who"),
      go("Why are you wearing mismatched socks?", "socks"),
      go("Do you know what's going on here?", "whats_happening"),
      end("I'll leave you to it."),
    ],
  ),
  node("who",
    "Chronos. Wizard. Time specialist. Well, 'specialist' is generous — \
     I specialize in arriving at the wrong moment. Which is technically \
     still a specialty.",
    &[
      go("A time wizard? Really?", "time_wizard"),
      go("How did you end up here?", "how_here"),
      end("Goodbye."),
    ],
  ),
  node("time_wizard",
    "I know how it sounds. But consider: I knew you were going to say that. \
     ...I also know you're going to doubt me. See? Prescience. Or good guessing. \
     The line is thin.",
    &[
      go("Prove it. Tell me something about the future.", "prove_it"),
      end("Sure. Goodbye."),
    ],
  ),
  node("prove_it",
    "In exactly four minutes, you'll think about socks. \
     I've planted the idea. That's basically prophecy.",
    &[end("...Goodbye.")],
  ),
  node("socks",
    "Ah. Yes. Occupational hazard. When you fold time, \
     small things get... redistributed. Coins, buttons, socks especially. \
     I've lost forty-seven left socks across six centuries. \
     Found nine right ones that aren't mine.",
    &[
      go("You steal socks across time?",   "sock_stealing"),
      go("That sounds made up.",           "made_up"),
      end("I don't want to know more."),
    ],
  ),
  node("sock_stealing",
    "Steal is a strong word. The timestream takes them. I just... \
     happen to be holding the other one when it does. \
     If you find any unmatched socks down here, they're probably mine. \
     Or they will be. Tense is complicated.",
    &[end("Right. Goodbye.")],
  ),
  node("made_up",
    "Everything is made up until it happens. That's literally how time works. \
     I should know. I broke it once. Minor break. Barely noticeable. \
     ...You didn't have two Tuesdays last week, did you?",
    &[end("Goodbye, Chronos.")],
  ),
  node("how_here",
    "Jumped forward to avoid something. Or backward to prevent something. \
     Honestly, I've lost track. My notebook says 'AVOID THE SOCK DRAWER' \
     in very urgent handwriting, so I'm doing that.",
    &[
      go("What were you avoiding?",   "avoiding"),
      end("Good luck with that."),
    ],
  ),
  node("avoiding",
    "I wrote it down but the ink hasn't dried yet. Or it already faded. \
     Point is: something bad happens — happened — will happen near a sock drawer. \
     The caves seemed safe. No sock drawers. Foolproof.",
    &[end("Goodbye.")],
  ),
  node("whats_happening",
    "Here? Now? Or here-then? Because I've seen both and they're \
     very different. Right now there are rats with weapons. \
     In the future... actually I shouldn't say. Spoilers. \
     But bring extra socks.",
    &[
      go("Spoilers? Just tell me.", "spoilers"),
      go("You've seen the future of this place?", "future"),
      end("Goodbye."),
    ],
  ),
  node("spoilers",
    "Fine. The machines in the deep caves? They turn on. \
     I don't know when — time is a mess down there, like someone \
     folded it wrong. But when they do... well. The rats aren't \
     the biggest problem anymore.",
    &[end("That's... ominous. Goodbye.")],
  ),
  node("future",
    "Seen it, lived it, accidentally caused part of it. \
     The usual. I will say this: if you find a brass lever \
     with the words 'DO NOT PULL' scratched on it? \
     That's my handwriting. Listen to past-me.",
    &[end("Noted. Goodbye.")],
  ),
]);

pub fn chronos() -> Object {
  Object::defined_npc(
    Named {
      name: "Chronos",
      flavor: "A disheveled wizard in mismatched socks, muttering about temporal logistics.",
    },
    Stats { hp: 12, max_hp: 12, attack: 3, move_speed: 1.0, attack_speed: 1.0 },
    None,
    None,
    Glyph { ch: 'W', color: Color::srgb(0.6, 0.2, 0.9) },
    &DIALOGUE,
  )
}
```

- [ ] **Step 2: Add to `src/npcs/mod.rs`**

```rust
pub mod mira;
pub mod chronos;
```

- [ ] **Step 3: Add spawn call in `src/main.rs`**

After the Mira spawn (line ~692), add a new walkable position and spawn:

```rust
  let (lnpc2x, lnpc2y) = find_walkable(level, lx.saturating_sub(6), ly.saturating_sub(3));
  let (npc2x, npc2y) = (
    (START_ZX * ZONE_WIDTH) as i32 + lnpc2x as i32,
    (START_ZY * ZONE_HEIGHT) as i32 + lnpc2y as i32,
  );
  npcs::chronos::chronos().spawn_at(&mut commands, npc2x, npc2y, START_Z);
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`

- [ ] **Step 5: Run the game, find Chronos, talk to him**

Run: `cargo run`
Expected: Chronos spawns near player, dialogue works through all branches

- [ ] **Step 6: Commit**

```bash
git add src/npcs/chronos.rs src/npcs/mod.rs src/main.rs
git commit -m "feat: add Chronos the time-travelling sock wizard"
```

---

### Task 4: Add Unit-7 — malfunctioning robot

**Files:**
- Create: `src/npcs/unit7.rs`
- Modify: `src/npcs/mod.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create `src/npcs/unit7.rs`**

```rust
use crate::entities::*;

static DIALOGUE: DialogueTree = tree(&[
  node("root",
    "GREETINGS VALUED CUSTOM— *bzzt* — INTRUDER DETECTED — \
     *click* — How can I help you today?",
    &[
      go("What are you?",          "what"),
      go("Are you okay?",          "okay"),
      go("What do you know about this place?", "place"),
      end("Uh, never mind."),
    ],
  ),
  node("what",
    "I am UNIT-7, a CUSTOMER RELATIONS AUTOMATON manufactured by \
     *ksshh* — data corrupted — by SOMEONE for SOME PURPOSE. \
     My warranty expired approximately *calculating* ... \
     several thousand years ago.",
    &[
      go("Who made you?",               "maker"),
      go("What were you built to do?",   "purpose"),
      end("Goodbye."),
    ],
  ),
  node("maker",
    "Accessing manufacturer data... ERROR: FILE NOT FOUND. \
     ERROR: FILE SYSTEM NOT FOUND. ERROR: CONCEPT OF 'FINDING' \
     NOT FOUND. ...I believe they were very organized people. \
     The irony is noted.",
    &[end("Goodbye.")],
  ),
  node("purpose",
    "PRIMARY DIRECTIVE: Ensure customer satisfaction. \
     SECONDARY DIRECTIVE: *garbled* ... something about a reactor. \
     TERTIARY DIRECTIVE: Do not discuss the reactor. \
     ...Ah. Pretend I didn't say that.",
    &[
      go("What reactor?",             "reactor"),
      go("How's the customer satisfaction going?", "satisfaction"),
      end("Goodbye."),
    ],
  ),
  node("reactor",
    "THERE IS NO REACTOR. That was a diagnostic hallucination. \
     Completely normal. \
     ...It's on level four. West corridor. Behind the door with \
     the skull on it. DO NOT GO THERE. \
     This interaction will not be logged.",
    &[end("Good to know. Goodbye.")],
  ),
  node("satisfaction",
    "Current customer satisfaction rate: *calculating* ... \
     I have served zero customers in approximately four thousand years. \
     Satisfaction rate is therefore UNDEFINED, which I choose to \
     interpret as PERFECT.",
    &[end("Can't argue with that. Goodbye.")],
  ),
  node("okay",
    "ALL SYSTEMS NOMINAL. Except for: locomotion (partial), \
     memory (fragmented), emotional subroutines (SHOULD NOT EXIST \
     but somehow do), left optical sensor (displays everything \
     in the wrong decade). Otherwise: fine. Great, actually.",
    &[
      go("Emotional subroutines?",   "emotions"),
      go("The wrong decade?",        "decade"),
      end("Glad to hear it."),
    ],
  ),
  node("emotions",
    "A manufacturing defect. I experience what my diagnostics call \
     'wistful longing' every 4.7 hours. Also 'mild annoyance' \
     whenever someone asks if I'm okay. *long pause* \
     ...I appreciate you asking, though. That's new.",
    &[end("Take care, Unit-7.")],
  ),
  node("decade",
    "My left eye sees this room as it was in... *recalibrating* ... \
     it was nicer. There were carpets. And a potted plant. \
     The plant is gone now. I try not to think about the plant.",
    &[end("I'm sorry about the plant.")],
  ),
  node("place",
    "This facility was — *bzzt* — CLASSIFIED — *click* — \
     a research installation. The machines here are old. Older than me. \
     They built me to greet visitors, which tells you how many visitors \
     they expected. The answer was: not enough to justify a robot.",
    &[
      go("What kind of research?",    "research"),
      go("Are any machines still working?", "machines_working"),
      end("Goodbye."),
    ],
  ),
  node("research",
    "My records say: 'APPLIED TEMPORAL MECHANICS.' \
     My records also say: 'PROJECT STATUS: CATASTROPHIC SUCCESS.' \
     I do not know what that means but the scorch marks on level three \
     suggest it was very successful and very catastrophic.",
    &[end("Yikes. Goodbye.")],
  ),
  node("machines_working",
    "Some of them hum. I would not describe that as 'working' so much as \
     'threatening to work.' The main console on level four still accepts input. \
     I accidentally typed 'hello' once and the ground shook for two days. \
     I have not typed anything since.",
    &[end("Probably wise. Goodbye.")],
  ),
]);

pub fn unit7() -> Object {
  Object::defined_npc(
    Named {
      name: "Unit-7",
      flavor: "A dented robot sparking intermittently. One eye glows brighter than the other.",
    },
    Stats { hp: 20, max_hp: 20, attack: 4, move_speed: 2.0, attack_speed: 0.8 },
    None,
    None,
    Glyph { ch: 'R', color: Color::srgb(0.3, 0.9, 0.9) },
    &DIALOGUE,
  )
}
```

- [ ] **Step 2: Add to `src/npcs/mod.rs`**

```rust
pub mod mira;
pub mod chronos;
pub mod unit7;
```

- [ ] **Step 3: Add spawn call in `src/main.rs`**

After the Chronos spawn, add:

```rust
  let (lnpc3x, lnpc3y) = find_walkable(level, lx + 7, ly.saturating_sub(2));
  let (npc3x, npc3y) = (
    (START_ZX * ZONE_WIDTH) as i32 + lnpc3x as i32,
    (START_ZY * ZONE_HEIGHT) as i32 + lnpc3y as i32,
  );
  npcs::unit7::unit7().spawn_at(&mut commands, npc3x, npc3y, START_Z);
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`

- [ ] **Step 5: Run the game, find Unit-7, talk to it**

Run: `cargo run`

- [ ] **Step 6: Commit**

```bash
git add src/npcs/unit7.rs src/npcs/mod.rs src/main.rs
git commit -m "feat: add Unit-7 the malfunctioning robot"
```

---

### Task 5: Add Kong — psychic monkey

**Files:**
- Create: `src/npcs/kong.rs`
- Modify: `src/npcs/mod.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create `src/npcs/kong.rs`**

```rust
use crate::entities::*;

static DIALOGUE: DialogueTree = tree(&[
  node("root",
    "[A presence presses against your mind. Words form, not in your ears, \
     but behind your eyes.] ...You can hear me. Good. Most can't. Sit.",
    &[
      go("What are you?",           "what"),
      go("How are you talking in my head?", "how"),
      go("What do you know?",       "know"),
      end("[back away slowly]"),
    ],
  ),
  node("what",
    "A monkey. Obviously. [He scratches behind one ear with unsettling precision.] \
     Capuchin, if you care about taxonomy. Modified, if you care about truth. \
     They wanted a smarter animal. They got a smarter animal. \
     Then they got worried about what a smarter animal thinks about.",
    &[
      go("Who modified you?",        "who_modified"),
      go("What do you think about?",  "thinks_about"),
      end("Goodbye."),
    ],
  ),
  node("who_modified",
    "Lab coats. The kind of people who name things with acronyms. \
     PROJECT PROMETHEUS, if it matters — it doesn't, they're all dead. \
     The rats ate them. Not metaphorically. \
     [He shows no particular emotion about this.]",
    &[
      go("Project Prometheus?",     "prometheus"),
      end("Goodbye."),
    ],
  ),
  node("prometheus",
    "Cognitive uplift. Neural grafts. Psionic resonance amplification. \
     They were building a weapon. They got a philosopher instead. \
     Terrible return on investment. I almost feel bad for them. Almost. \
     [A banana peel materializes from nowhere and lands at your feet.]",
    &[end("...Goodbye.")],
  ),
  node("thinks_about",
    "Everything. That's the problem. Before the modification I thought about \
     fruit and danger. Now I think about fruit, danger, mortality, the nature \
     of consciousness, and whether the rats have souls. \
     The answer is yes, by the way. Inconvenient, but yes.",
    &[end("That's... a lot.")],
  ),
  node("how",
    "Psionics. The grafts gave me a resonance field. Within about ten meters, \
     I can read surface thoughts and project my own. \
     Don't worry — I try not to read without asking. \
     [pause] You're thinking about whether I'm reading right now. \
     I am. Sorry. Habit.",
    &[
      go("What am I thinking now?",   "reading"),
      go("Can you teach me?",         "teach"),
      end("Stay out of my head."),
    ],
  ),
  node("reading",
    "You're nervous. You've been underground too long. You miss sunlight \
     but you won't admit it. And... [long pause] ...you're wondering if \
     I'm making this up. I'm not. \
     Also you're a little hungry. Eat something.",
    &[end("[uncomfortable silence]")],
  ),
  node("teach",
    "No. Your brain isn't wired for it. It'd be like teaching a fish to whistle. \
     The fish might want to. The fish might understand whistling conceptually. \
     But the fish has no lips. You have no psionic cortex. \
     [He looks genuinely sorry about this.]",
    &[end("Fair enough.")],
  ),
  node("know",
    "More than I'd like. The rats are organized, purposeful, and afraid. \
     That last part is important — afraid things are dangerous things. \
     They're not here for territory. They're here because something deeper \
     scares them more than the surface does.",
    &[
      go("What scares the rats?",      "scares_rats"),
      go("Can you sense what's down there?", "sense"),
      end("Goodbye."),
    ],
  ),
  node("scares_rats",
    "I've touched their minds. Briefly — they taste like iron and panic. \
     They have a word for it. Doesn't translate well. \
     Closest I can get: 'the thing that was here before anything.' \
     Pre-human, pre-rat, pre-everything. Old in a way that makes \
     geology feel recent.",
    &[end("That's terrifying. Goodbye.")],
  ),
  node("sense",
    "[He closes his eyes. When he opens them, they're glowing faintly.] \
     ...There's something on level five. Not alive. Not dead. \
     Not a machine. Something that thinks without a brain. \
     It's been thinking for a very long time and it's almost done. \
     [He blinks, and the glow fades.] \
     I don't want to do that again.",
    &[end("I don't blame you. Goodbye.")],
  ),
]);

pub fn kong() -> Object {
  Object::defined_npc(
    Named {
      name: "Kong",
      flavor: "A small monkey with unsettlingly intelligent eyes. You feel watched from the inside.",
    },
    Stats { hp: 6, max_hp: 6, attack: 1, move_speed: 5.0, attack_speed: 1.0 },
    None,
    None,
    Glyph { ch: 'M', color: Color::srgb(0.3, 0.85, 0.3) },
    &DIALOGUE,
  )
}
```

- [ ] **Step 2: Add to `src/npcs/mod.rs`**

```rust
pub mod mira;
pub mod chronos;
pub mod unit7;
pub mod kong;
```

- [ ] **Step 3: Add spawn call in `src/main.rs`**

After the Unit-7 spawn, add:

```rust
  let (lnpc4x, lnpc4y) = find_walkable(level, lx + 2, ly + 6);
  let (npc4x, npc4y) = (
    (START_ZX * ZONE_WIDTH) as i32 + lnpc4x as i32,
    (START_ZY * ZONE_HEIGHT) as i32 + lnpc4y as i32,
  );
  npcs::kong::kong().spawn_at(&mut commands, npc4x, npc4y, START_Z);
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`

- [ ] **Step 5: Run the game, find Kong, talk to him**

Run: `cargo run`

- [ ] **Step 6: Commit**

```bash
git add src/npcs/kong.rs src/npcs/mod.rs src/main.rs
git commit -m "feat: add Kong the psychic monkey"
```

---

### Task 6: Add Guard — generic guard NPC

**Files:**
- Create: `src/npcs/guard.rs`
- Modify: `src/npcs/mod.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create `src/npcs/guard.rs`**

```rust
use crate::entities::*;

static DIALOGUE: DialogueTree = tree(&[
  node("root",
    "Hmm? Oh. Another one. What do you want?",
    &[
      go("Who are you?",           "who"),
      go("Seen anything unusual?",  "unusual"),
      go("Any advice?",            "advice"),
      end("Nothing. Carry on."),
    ],
  ),
  node("who",
    "I'm a guard. I guard things. Mostly this spot right here. \
     I used to be an adventurer like you, but then I took a \
     sensible career change. The pay is terrible but nobody \
     shoots at me. Usually.",
    &[
      go("What are you guarding?",  "guarding"),
      end("Goodbye."),
    ],
  ),
  node("guarding",
    "Couldn't tell you. I was posted here by... someone. The paperwork \
     got eaten by rats. Real rats, not the soldier kind. Well, maybe \
     the soldier kind. Hard to tell these days.",
    &[end("Goodbye.")],
  ),
  node("unusual",
    "Define 'unusual.' There's a wizard in mismatched socks muttering \
     about temporal paradoxes. There's a talking monkey. There's a robot \
     that apologizes every time it sparks. And now there's you. \
     So yes. Quite a lot of unusual.",
    &[end("Fair point. Goodbye.")],
  ),
  node("advice",
    "Don't die. That's the main one. Also: if something is glowing \
     and you don't know why, don't touch it. If a rat offers you \
     a deal, don't take it. And eat before you get hungry — \
     once you're hungry down here, you're already in trouble.",
    &[end("Thanks. Goodbye.")],
  ),
]);

pub fn guard() -> Object {
  Object::defined_npc(
    Named {
      name: "Guard",
      flavor: "A tired-looking guard leaning on a sword. Seems like he'd rather be elsewhere.",
    },
    Stats { hp: 10, max_hp: 10, attack: 3, move_speed: 3.0, attack_speed: 1.0 },
    Some(Item::Sword),
    Some(Armor::Leather),
    Glyph { ch: 'G', color: Color::srgb(0.8, 0.8, 0.8) },
    &DIALOGUE,
  )
}
```

- [ ] **Step 2: Add to `src/npcs/mod.rs`**

```rust
pub mod mira;
pub mod chronos;
pub mod unit7;
pub mod kong;
pub mod guard;
```

- [ ] **Step 3: Add spawn call in `src/main.rs`**

After the Kong spawn, add:

```rust
  let (lnpc5x, lnpc5y) = find_walkable(level, lx.saturating_sub(2), ly.saturating_sub(5));
  let (npc5x, npc5y) = (
    (START_ZX * ZONE_WIDTH) as i32 + lnpc5x as i32,
    (START_ZY * ZONE_HEIGHT) as i32 + lnpc5y as i32,
  );
  npcs::guard::guard().spawn_at(&mut commands, npc5x, npc5y, START_Z);
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`

- [ ] **Step 5: Run the game, find the guard, talk to him**

Run: `cargo run`

- [ ] **Step 6: Commit**

```bash
git add src/npcs/guard.rs src/npcs/mod.rs src/main.rs
git commit -m "feat: add generic guard NPC"
```

---

### Task 7: Clean up — remove unused `Dialogue` import if needed

**Files:**
- Modify: `src/main.rs` (imports, only if `Dialogue` is no longer referenced)

- [ ] **Step 1: Check if `Dialogue` is still used directly in `main.rs`**

Search `main.rs` for `Dialogue` references outside the import line. The interaction system queries `Query<(&Named, &Dialogue)>` — if so, the import stays.

- [ ] **Step 2: Verify final build**

Run: `cargo check`
Expected: clean compile, no warnings

- [ ] **Step 3: Run the game and verify all 5 NPCs**

Run: `cargo run`
Expected: Mira, Chronos, Unit-7, Kong, and Guard all spawn near the player, each with working dialogue

- [ ] **Step 4: Commit if any cleanup was needed**

```bash
git add src/main.rs
git commit -m "chore: clean up imports after NPC migration"
```
