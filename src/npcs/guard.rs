use trl::entities::*;
use bevy::prelude::Color;

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
    npc_person_glyph(
      'G',
      Color::srgb(0.72, 0.76, 0.8),
      Color::srgb(0.42, 0.46, 0.52),
    ),
    &DIALOGUE,
  )
}
