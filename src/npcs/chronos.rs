use trl::entities::*;
use bevy::prelude::Color;

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
    npc_person_glyph(
      'W',
      Color::srgb(0.52, 0.22, 0.88),
      Color::srgb(0.82, 0.62, 0.95),
    ),
    &DIALOGUE,
  )
}
