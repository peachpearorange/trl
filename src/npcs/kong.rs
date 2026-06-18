use {crate::entities::*, bevy::prelude::Color};

static DIALOGUE: DialogueTree = dialogue_tree(&[
  node(
    "root",
    "[A presence presses against your mind. Words form, not in your ears, \
     but behind your eyes.] ...You can hear me. Good. Most can't. Sit.",
    &[
      go("What are you?", "what"),
      go("How are you talking in my head?", "how"),
      go("What do you know?", "know"),
      end("[back away slowly]")
    ]
  ),
  node(
    "what",
    "A monkey. Obviously. [He scratches behind one ear with unsettling precision.] \
     Capuchin, if you care about taxonomy. Modified, if you care about truth. \
     They wanted a smarter animal. They got a smarter animal. \
     Then they got worried about what a smarter animal thinks about.",
    &[
      go("Who modified you?", "who_modified"),
      go("What do you think about?", "thinks_about"),
      end("Goodbye.")
    ]
  ),
  node(
    "who_modified",
    "Lab coats. The kind of people who name things with acronyms. \
     PROJECT PROMETHEUS, if it matters — it doesn't, they're all dead. \
     The rats ate them. Not metaphorically. \
     [He shows no particular emotion about this.]",
    &[go("Project Prometheus?", "prometheus"), end("Goodbye.")]
  ),
  node(
    "prometheus",
    "Cognitive uplift. Neural grafts. Psionic resonance amplification. \
     They were building a weapon. They got a philosopher instead. \
     Terrible return on investment. I almost feel bad for them. Almost. \
     [A banana peel materializes from nowhere and lands at your feet.]",
    &[end("...Goodbye.")]
  ),
  node(
    "thinks_about",
    "Everything. That's the problem. Before the modification I thought about \
     fruit and danger. Now I think about fruit, danger, mortality, the nature \
     of consciousness, and whether the rats have souls. \
     The answer is yes, by the way. Inconvenient, but yes.",
    &[end("That's... a lot.")]
  ),
  node(
    "how",
    "Psionics. The grafts gave me a resonance field. Within about ten meters, \
     I can read surface thoughts and project my own. \
     Don't worry — I try not to read without asking. \
     [pause] You're thinking about whether I'm reading right now. \
     I am. Sorry. Habit.",
    &[
      go("What am I thinking now?", "reading"),
      go("Can you teach me?", "teach"),
      end("Stay out of my head.")
    ]
  ),
  node(
    "reading",
    "You're nervous. You've been underground too long. You miss sunlight \
     but you won't admit it. And... [long pause] ...you're wondering if \
     I'm making this up. I'm not. \
     Also you're a little hungry. Eat something.",
    &[end("[uncomfortable silence]")]
  ),
  node(
    "teach",
    "No. Your brain isn't wired for it. It'd be like teaching a fish to whistle. \
     The fish might want to. The fish might understand whistling conceptually. \
     But the fish has no lips. You have no psionic cortex. \
     [He looks genuinely sorry about this.]",
    &[end("Fair enough.")]
  ),
  node(
    "know",
    "More than I'd like. The rats are organized, purposeful, and afraid. \
     That last part is important — afraid things are dangerous things. \
     They're not here for territory. They're here because something deeper \
     scares them more than the surface does.",
    &[
      go("What scares the rats?", "scares_rats"),
      go("Can you sense what's down there?", "sense"),
      end("Goodbye.")
    ]
  ),
  node(
    "scares_rats",
    "I've touched their minds. Briefly — they taste like iron and panic. \
     They have a word for it. Doesn't translate well. \
     Closest I can get: 'the thing that was here before anything.' \
     Pre-human, pre-rat, pre-everything. Old in a way that makes \
     geology feel recent.",
    &[end("That's terrifying. Goodbye.")]
  ),
  node(
    "sense",
    "[He closes his eyes. When he opens them, they're glowing faintly.] \
     ...There's something on level five. Not alive. Not dead. \
     Not a machine. Something that thinks without a brain. \
     It's been thinking for a very long time and it's almost done. \
     [He blinks, and the glow fades.] \
     I don't want to do that again.",
    &[end("I don't blame you. Goodbye.")]
  )
]);

pub fn kong() -> Object {
  Object::as_follower(Object::defined_npc(
    Named::s("Kong", "A small monkey with unsettlingly intelligent eyes. You feel watched from the inside."),
    Stats { hp: 6, max_hp: 6, attack: 1, move_speed: 5.0, attack_speed: 1.0 },
    Loadout::default(),
    npc_person_glyph('M', Color::srgb(0.38, 0.72, 0.32), Color::srgb(0.72, 0.92, 0.48)),
    &DIALOGUE
  ))
}
