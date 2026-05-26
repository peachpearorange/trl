use {crate::entities::*, bevy::prelude::Color};

static DIALOGUE: DialogueTree = dialogue_tree(&[
  node(
    "root",
    "Ah — you're awake! Good morning. I am ORI-1, your orientation unit. \
     I maintain this outpost while travelers pass through.",
    &[
      go("Where am I?", "where"),
      go("What should I do?", "what_do"),
      go("Tell me about this place.", "about"),
      end("Thanks, I'll look around.")
    ]
  ),
  node(
    "where",
    "This is the Origin World — a waypoint at the edge of mapped space. \
     Your ship, the Mongoose, is docked just outside. \
     Follow the road south through the door and you'll reach the landing pad.",
    &[
      go("What should I do?", "what_do"),
      go("What's out there?", "out_there"),
      end("Got it. I'll head to the ship.")
    ]
  ),
  node(
    "what_do",
    "Board your ship and use the flight console to navigate. \
     There are stations to trade at, planets to explore, and — \
     well, things that need killing. You're a bounty hunter. \
     That's what bounty hunters do.",
    &[
      go("How do I fight?", "combat"),
      go("How do I use equipment?", "equipment"),
      go("What's out there?", "out_there"),
      end("Time to get moving.")
    ]
  ),
  node(
    "combat",
    "Walk into hostiles to attack with your melee weapon. \
     Ranged weapons fire automatically when enemies are in sight — \
     check the supply cache on your ship for firearms. \
     If things go badly, grenades help even the odds.",
    &[
      go("How do I use equipment?", "equipment"),
      go("What about healing?", "healing"),
      end("Thanks for the tips.")
    ]
  ),
  node(
    "equipment",
    "Press Q near the loadout console on your ship to manage gear. \
     You can equip weapons and armor from your inventory. \
     Salvage junk with G to get crafting materials, then craft \
     better equipment at a crafting table.",
    &[
      go("How do I fight?", "combat"),
      go("What about saving?", "saving"),
      end("I'll figure the rest out.")
    ]
  ),
  node(
    "healing",
    "Sleep in a bed to save your progress and restore health. \
     If you fall in battle, you'll wake up at the last bed \
     you slept in. There's one right here in this room, and \
     more on your ship.",
    &[go("What about equipment?", "equipment"), end("Good to know.")]
  ),
  node(
    "saving",
    "Interact with any bed to sleep and save. If you die, \
     you'll respawn there with your gear intact. \
     I'd recommend saving often — space is unforgiving.",
    &[go("How do I fight?", "combat"), end("Will do.")]
  ),
  node(
    "out_there",
    "Stations for resupply. Derelict ships full of salvage — \
     and hostiles. Planets with alien ruins. Asteroid fields. \
     The deeper you go, the worse it gets. \
     But the bounties get better too.",
    &[
      go("How do I fight?", "combat"),
      go("How do I use equipment?", "equipment"),
      end("Sounds like my kind of work.")
    ]
  ),
  node(
    "about",
    "A quiet outpost. Not much here — some grass, some alien crystal, \
     a landing pad. I keep the lights on and greet new arrivals. \
     You're the first in a while, actually.",
    &[
      go("Where am I exactly?", "where"),
      go("What should I do?", "what_do"),
      end("Peaceful place. See you around.")
    ]
  )
]);

pub fn ori1() -> Object {
  Object::defined_npc(
    Named {
      name: "ORI-1",
      flavor: "A small orientation robot. Its chassis is scuffed but its optics are bright."
    },
    Stats { hp: 30, max_hp: 30, attack: 1, move_speed: 2.0, attack_speed: 1.0 },
    Loadout::default(),
    npc_robo_glyph('O', Color::srgb(0.45, 0.75, 0.42), Color::srgb(0.82, 0.90, 0.78)),
    &DIALOGUE
  )
}
