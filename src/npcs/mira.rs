use {bevy::prelude::Color, trl::entities::*};

static DIALOGUE: DialogueTree = tree(&[
  node(
    "root",
    "Oh— you're not a rat person. That's... refreshing. What do you want?",
    &[
      go("Who are you?", "who"),
      go("What is this place?", "place"),
      go("What's with all the rat soldiers?", "rats"),
      end("Nothing. Goodbye.")
    ]
  ),
  node(
    "who",
    "I'm Mira. Used to be a cartographer's assistant up on the surface. \
     Mapped half these tunnels myself, actually. Then the Compact moved in \
     and... well. I map for survival now.",
    &[
      go("What's the Compact?", "compact"),
      go("What happened on the surface?", "surface"),
      end("Goodbye.")
    ]
  ),
  node(
    "compact",
    "Rat-people. Not just soldiers — they have scholars, merchants, the whole \
     apparatus. They want the deep levels. Something down there they need badly \
     enough to occupy the caves with an army.",
    &[
      go("What do they want down there?", "compact_want"),
      go("Are they dangerous?", "compact_danger"),
      end("Goodbye.")
    ]
  ),
  node(
    "compact_want",
    "Old machines. There are ruins in the deep caves — before my time, before \
     anyone's time. The Compact thinks those machines still work. They might be right.",
    &[go("What kind of machines?", "machines"), end("Goodbye.")]
  ),
  node(
    "machines",
    "I don't know exactly. My old employer used to say 'the kind that ends \
     arguments.' He wasn't a comforting man.",
    &[end("Goodbye.")]
  ),
  node(
    "compact_danger",
    "Individually? No worse than anyone with a spear. As an army? Very much yes. \
     Stay low, stay quiet, and you'll probably be fine. Probably.",
    &[end("Goodbye.")]
  ),
  node(
    "surface",
    "Overrun. The Compact has outposts up there now. I got out before they \
     started checking papers. Barely.",
    &[go("Do you want to go back?", "surface_back"), end("Goodbye.")]
  ),
  node(
    "surface_back",
    "Every day. But not until something changes down here. \
     You look like something that changes things. No pressure.",
    &[end("Goodbye.")]
  ),
  node(
    "place",
    "Old mining tunnel, mostly. Pre-Compact, pre-everything. Someone carved it \
     out looking for salt, found the deeper caves instead, and left in a hurry. \
     You can tell by the tools they abandoned.",
    &[go("What's in the deeper caves?", "deep"), end("Goodbye.")]
  ),
  node(
    "deep",
    "That's the question, isn't it. I've mapped down to level two. Level three \
     is... I turned back. Something moved in the dark that wasn't rats.",
    &[end("Goodbye.")]
  ),
  node(
    "rats",
    "Compact soldiers. Organized, disciplined, and they don't like trespassers. \
     I've been hiding from them for three weeks. The trick is they can't see \
     well in complete darkness, but they can hear everything.",
    &[go("Any weaknesses?", "rats_weak"), end("Goodbye.")]
  ),
  node(
    "rats_weak",
    "Apart from the dark? They're proud. Call one a coward and it stops \
     thinking tactically. Doesn't help if there are six of them, but... \
     it's something.",
    &[end("Goodbye.")]
  )
]);

pub fn mira() -> Object {
  Object::defined_npc(
    Named { name: "Mira", flavor: "She eyes you warily, ears flat against her head." },
    Stats { hp: 8, max_hp: 8, attack: 2, move_speed: 4.0, attack_speed: 1.2 },
    None,
    None,
    npc_person_glyph('c', Color::srgb(0.95, 0.55, 0.82), Color::srgb(0.48, 0.32, 0.62)),
    &DIALOGUE
  )
}
