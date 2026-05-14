use {bevy::prelude::Color, crate::entities::*};

static DIALOGUE: DialogueTree = tree(&[
  node(
    "root",
    "GREETINGS VALUED CUSTOM— *bzzt* — INTRUDER DETECTED — \
     *click* — How can I help you today?",
    &[
      go("What are you?", "what"),
      go("Are you okay?", "okay"),
      go("What do you know about this place?", "place"),
      end("Uh, never mind.")
    ]
  ),
  node(
    "what",
    "I am UNIT-7, a CUSTOMER RELATIONS AUTOMATON manufactured by \
     *ksshh* — data corrupted — by SOMEONE for SOME PURPOSE. \
     My warranty expired approximately *calculating* ... \
     several thousand years ago.",
    &[
      go("Who made you?", "maker"),
      go("What were you built to do?", "purpose"),
      end("Goodbye.")
    ]
  ),
  node(
    "maker",
    "Accessing manufacturer data... ERROR: FILE NOT FOUND. \
     ERROR: FILE SYSTEM NOT FOUND. ERROR: CONCEPT OF 'FINDING' \
     NOT FOUND. ...I believe they were very organized people. \
     The irony is noted.",
    &[end("Goodbye.")]
  ),
  node(
    "purpose",
    "PRIMARY DIRECTIVE: Ensure customer satisfaction. \
     SECONDARY DIRECTIVE: *garbled* ... something about a reactor. \
     TERTIARY DIRECTIVE: Do not discuss the reactor. \
     ...Ah. Pretend I didn't say that.",
    &[
      go("What reactor?", "reactor"),
      go("How's the customer satisfaction going?", "satisfaction"),
      end("Goodbye.")
    ]
  ),
  node(
    "reactor",
    "THERE IS NO REACTOR. That was a diagnostic hallucination. \
     Completely normal. \
     ...It's on level four. West corridor. Behind the door with \
     the skull on it. DO NOT GO THERE. \
     This interaction will not be logged.",
    &[end("Good to know. Goodbye.")]
  ),
  node(
    "satisfaction",
    "Current customer satisfaction rate: *calculating* ... \
     I have served zero customers in approximately four thousand years. \
     Satisfaction rate is therefore UNDEFINED, which I choose to \
     interpret as PERFECT.",
    &[end("Can't argue with that. Goodbye.")]
  ),
  node(
    "okay",
    "ALL SYSTEMS NOMINAL. Except for: locomotion (partial), \
     memory (fragmented), emotional subroutines (SHOULD NOT EXIST \
     but somehow do), left optical sensor (displays everything \
     in the wrong decade). Otherwise: fine. Great, actually.",
    &[
      go("Emotional subroutines?", "emotions"),
      go("The wrong decade?", "decade"),
      end("Glad to hear it.")
    ]
  ),
  node(
    "emotions",
    "A manufacturing defect. I experience what my diagnostics call \
     'wistful longing' every 4.7 hours. Also 'mild annoyance' \
     whenever someone asks if I'm okay. *long pause* \
     ...I appreciate you asking, though. That's new.",
    &[end("Take care, Unit-7.")]
  ),
  node(
    "decade",
    "My left eye sees this room as it was in... *recalibrating* ... \
     it was nicer. There were carpets. And a potted plant. \
     The plant is gone now. I try not to think about the plant.",
    &[end("I'm sorry about the plant.")]
  ),
  node(
    "place",
    "This facility was — *bzzt* — CLASSIFIED — *click* — \
     a research installation. The machines here are old. Older than me. \
     They built me to greet visitors, which tells you how many visitors \
     they expected. The answer was: not enough to justify a robot.",
    &[
      go("What kind of research?", "research"),
      go("Are any machines still working?", "machines_working"),
      end("Goodbye.")
    ]
  ),
  node(
    "research",
    "My records say: 'APPLIED TEMPORAL MECHANICS.' \
     My records also say: 'PROJECT STATUS: CATASTROPHIC SUCCESS.' \
     I do not know what that means but the scorch marks on level three \
     suggest it was very successful and very catastrophic.",
    &[end("Yikes. Goodbye.")]
  ),
  node(
    "machines_working",
    "Some of them hum. I would not describe that as 'working' so much as \
     'threatening to work.' The main console on level four still accepts input. \
     I accidentally typed 'hello' once and the ground shook for two days. \
     I have not typed anything since.",
    &[end("Probably wise. Goodbye.")]
  )
]);

pub fn unit7() -> Object {
  Object::defined_npc(
    Named {
      name: "Unit-7",
      flavor: "A dented robot sparking intermittently. One eye glows brighter than the other."
    },
    Stats { hp: 20, max_hp: 20, attack: 4, move_speed: 2.0, attack_speed: 0.8 },
    None,
    None,
    npc_robo_glyph('R', Color::srgb(0.22, 0.78, 0.88), Color::srgb(0.55, 0.95, 0.98)),
    &DIALOGUE
  )
  .as_follower()
}
