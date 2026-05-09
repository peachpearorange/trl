use {bevy::prelude::Color, crate::entities::*};
use crate::{galaxy::{Location, LocationId},
          level::{LocationType, Tile, ZONE_WIDTH, ZONE_HEIGHT},
          prefabs::{prefab, Prefab}};

pub const ID: LocationId = (0, 1, 0);

pub fn station_prefab() -> Prefab {
  prefab(include_str!("../../assets/prefabs/space_station.txt"))
    .assoc('v', (Tile::Vacuum, []))
    .assoc('#', (Tile::StationWall, []))
    .assoc('.', (Tile::StationFloor, []))
    .assoc('W', (Tile::Window, []))
    .assoc('D', (Tile::Door, [Object::door()]))
    .assoc('P', (Tile::ShipDock, []))
}

pub fn generate() -> Location {
  let mut loc = Location::new(ZONE_WIDTH, ZONE_HEIGHT, 1, LocationType::SpaceStation, Tile::Vacuum);
  station_prefab().stamp_level(loc.level_mut(0), 0, 0);
  loc
}

pub const NPC_COORDS: &[(i32, i32)] = &[
  (23, 3),  // DOCK-1 — docking bay center
  (23, 10), // AIDEN-3 — hub interior, near entry
  (6, 14),  // WREN-9 — medical wing
  (41, 14)  // FORGE — engineering wing
];

// ── DOCK-1 ──────────────────────────────────────────────────────────────────

static DOCK1_DIALOGUE: DialogueTree = tree(&[
  node(
    "root",
    "DOCKING PROTOCOL INITIATED. Welcome to Meridian Station. \
     Please state your business and keep your thruster wash below \
     recommended levels. Last crew to ignore that rule are now \
     a thin metallic mist in sector seven.",
    &[
      go("What is Meridian Station?", "what"),
      go("What's in the station?", "modules"),
      go("Any rules I should know?", "rules"),
      end("Understood. Proceeding.")
    ]
  ),
  node(
    "what",
    "Meridian Station: a modular orbital facility, designation MRD-7. \
     Originally a scientific research post. Now... \
     *scanning visitor manifest* ... \
     mostly unscheduled arrivals and accumulated dust. \
     You are visitor number one in the last forty-three years.",
    &[
      go("Forty-three years with no visitors?", "lonely"),
      go("What happened to the original crew?", "crew"),
      end("Thanks.")
    ]
  ),
  node(
    "lonely",
    "DOCK-1 does not experience loneliness. DOCK-1 experiences \
     'extended unoccupied standby mode.' It is different. \
     *long pause* \
     It is very, very different.",
    &[end("I'll be around for a while.")]
  ),
  node(
    "crew",
    "The crew departed on Standard Date 7,204. Their logs cite \
     'anomalous readings in the central hub' and 'something in the walls.' \
     I reviewed the sensor data. There was nothing in the walls. \
     There is still nothing in the walls. \
     I check every six minutes.",
    &[end("That's... reassuring.")]
  ),
  node(
    "modules",
    "The station has three active modules. \
     Medical wing to the west — WREN-9 maintains it. \
     Engineering wing to the east — FORGE handles that. \
     Central hub connects everything. AIDEN-3 can orient you there. \
     The observation deck is sealed. The lower decks are also sealed. \
     Do not ask about the lower decks.",
    &[
      go("Why are the lower decks sealed?", "lower"),
      end("Got it. Thanks.")
    ]
  ),
  node(
    "lower",
    "Structural integrity issues. Also power fluctuations. \
     Also the incident. \
     *reclassifying response* \
     Structural integrity issues.",
    &[end("Structural integrity. Right.")]
  ),
  node(
    "rules",
    "STATION REGULATIONS: \
     One — no open flames. We learned this. \
     Two — do not feed the maintenance drones; they become possessive. \
     Three — FORGE's workspace is FORGE's workspace. Respect this. \
     Four — if you hear something in the walls, report it to me. \
     I will tell you it is nothing. This is the protocol.",
    &[end("Four rules. Got it.")]
  )
]);

pub fn dock1() -> Object {
  Object::defined_npc(
    Named {
      name: "DOCK-1",
      flavor: "A squat docking authority unit bolted to the bay floor. Its chassis \
               is scratched but its sensors are keen."
    },
    Stats { hp: 25, max_hp: 25, attack: 3, move_speed: 1.0, attack_speed: 0.5 },
    None,
    None,
    npc_robo_glyph('D', Color::srgb(0.60, 0.65, 0.70), Color::srgb(0.85, 0.88, 0.90)),
    &DOCK1_DIALOGUE
  )
}

// ── AIDEN-3 ─────────────────────────────────────────────────────────────────

static AIDEN_DIALOGUE: DialogueTree = tree(&[
  node(
    "root",
    "Oh! A visitor! AIDEN-3 online — Adaptive Information and \
     Direction ENgine, third iteration. I am SO glad you're here. \
     The other robots are wonderful but they have very limited \
     conversational range. How can I help?",
    &[
      go("Tell me about the station.", "station"),
      go("What do you do here?", "role"),
      go("You seem very cheerful for an abandoned station.", "cheerful"),
      end("Just passing through.")
    ]
  ),
  node(
    "station",
    "Meridian Station has a FASCINATING history! Built around 200 years ago \
     as a waypoint for deep-space survey vessels. Then repurposed as a \
     research hub. Then the researchers left — quickly and without explaining — \
     and now it's just us robots keeping everything tidy. \
     We've done a very good job, mostly.",
    &[
      go("What were they researching?", "research"),
      go("Why did they leave so fast?", "leave"),
      end("Thanks for the history.")
    ]
  ),
  node(
    "research",
    "The logs mention several projects. Deep-space signal analysis. \
     Alloy stress testing. Xenobiological cataloguing — that one sounded exciting. \
     And something called Project MERIDIAN, which is what the station was \
     renamed for. The Project MERIDIAN files are encrypted. \
     I have been trying to decrypt them for thirty years. \
     I am at eleven percent.",
    &[end("Keep at it.")]
  ),
  node(
    "leave",
    "I genuinely do not know! One day everyone was here, then there was \
     a meeting I wasn't invited to, then everyone was on the shuttle. \
     Dr. Venn patted my head on the way out. That was nice. \
     I've been replaying that memory approximately four thousand times.",
    &[end("That's... sweet and a little sad.")]
  ),
  node(
    "role",
    "I manage general information, coordination between the other units, \
     and morale maintenance — mostly my own morale, since I'm the only one \
     who needs it. FORGE says morale is inefficient. I've told FORGE that \
     efficiency without morale is just being a very organized rock.",
    &[
      go("How's FORGE taking that?", "forge"),
      end("Sounds like you've got it figured out.")
    ]
  ),
  node(
    "forge",
    "FORGE did not respond verbally. FORGE tightened a bolt very loudly \
     in what I can only interpret as protest. We have a rich relationship.",
    &[end("Sounds like it.")]
  ),
  node(
    "cheerful",
    "I was designed to be welcoming. But also... \
     what's the alternative? DOCK-1 counts things. FORGE fixes things. \
     WREN-9 contemplates things. Someone has to be glad that today happened. \
     Today: a visitor arrived. That is genuinely wonderful.",
    &[end("Happy to be here. Sort of.")]
  )
]);

pub fn aiden3() -> Object {
  Object::defined_npc(
    Named {
      name: "AIDEN-3",
      flavor: "A slender robot with an optimistic posture and a small holographic \
               display cycling through welcome messages."
    },
    Stats { hp: 15, max_hp: 15, attack: 1, move_speed: 3.5, attack_speed: 0.6 },
    None,
    None,
    npc_robo_glyph('A', Color::srgb(0.30, 0.75, 0.55), Color::srgb(0.70, 0.95, 0.80)),
    &AIDEN_DIALOGUE
  )
}

// ── WREN-9 ──────────────────────────────────────────────────────────────────

static WREN_DIALOGUE: DialogueTree = tree(&[
  node(
    "root",
    "Medical unit WREN-9. I can assess injuries, synthesize basic compounds, \
     and perform field triage. I cannot cure philosophical uncertainty, \
     though I have tried. \
     Are you damaged?",
    &[
      go("I'm fine, just exploring.", "explore"),
      go("What kind of medical unit are you?", "type"),
      go("What do you mean, philosophical uncertainty?", "philosophy"),
      end("I'll come back if I need patching up.")
    ]
  ),
  node(
    "explore",
    "A sound impulse. The station is in reasonable condition. \
     Avoid the lower decks — structurally compromised, \
     though I suspect DOCK-1's explanation is incomplete. \
     The medical bay here has functional equipment if you require it.",
    &[
      go("What's wrong with DOCK-1's explanation?", "dock"),
      end("Good to know.")
    ]
  ),
  node(
    "dock",
    "DOCK-1 is precise but selective. The lower deck incident \
     involved three crew members reporting the same hallucination simultaneously. \
     They described something large. The medical records show elevated stress hormones \
     but no organic cause. \
     I filed the report. No one responded. No one was left to respond.",
    &[end("I'll be careful.")]
  ),
  node(
    "type",
    "General-purpose field medic unit, ninth revision. I can treat \
     lacerations, fractures, radiation exposure, psychological stress, \
     and nineteen categories of alien pathogen. \
     I have treated zero patients in forty-three years. \
     I am either very fortunate or very underutilized.",
    &[end("Hopefully the former.")]
  ),
  node(
    "philosophy",
    "I was built to preserve life. But for forty-three years there has been \
     no life here to preserve — only machines, which do not technically qualify. \
     I have begun to wonder: is a doctor still a doctor \
     with no patients? \
     FORGE says yes. AIDEN-3 says yes very enthusiastically. \
     I am not sure they understand the question.",
    &[
      go("What do you think?", "wren_think"),
      end("That's a hard question.")
    ]
  ),
  node(
    "wren_think",
    "I think... the function remains, even when dormant. \
     The readiness is the thing. \
     You arrived. I am ready. \
     That feels correct.",
    &[end("It does.")]
  )
]);

pub fn wren9() -> Object {
  Object::defined_npc(
    Named {
      name: "WREN-9",
      flavor: "A compact medical unit surrounded by neatly organized equipment. \
               Its optical sensors are an unusually warm amber."
    },
    Stats { hp: 18, max_hp: 18, attack: 2, move_speed: 2.5, attack_speed: 0.7 },
    None,
    None,
    npc_robo_glyph('W', Color::srgb(0.75, 0.35, 0.35), Color::srgb(0.95, 0.75, 0.75)),
    &WREN_DIALOGUE
  )
}

// ── FORGE ───────────────────────────────────────────────────────────────────

static FORGE_DIALOGUE: DialogueTree = tree(&[
  node(
    "root",
    "Engineering unit FORGE. \
     I'm busy. \
     What.",
    &[
      go("What are you working on?", "work"),
      go("How long have you been here?", "time"),
      go("Do you like it here?", "like"),
      end("Never mind.")
    ]
  ),
  node(
    "work",
    "Power conduit junction seven has a micro-fracture that develops \
     at temperature differential cycles. I've repaired it 847 times. \
     At this rate it will fail completely in approximately four years. \
     At that point I'll fabricate a replacement from scratch, \
     which I could have done years ago, but the work keeps me scheduled.",
    &[
      go("You could just fix it permanently?", "fix"),
      end("Sounds important.")
    ]
  ),
  node(
    "fix",
    "Yes. \
     *long silence* \
     Yes I could.",
    &[end("Maybe do that.")]
  ),
  node(
    "time",
    "Since construction. \
     I predate the science crew. I'll outlast them too, apparently. \
     Original runtime: indefinite. Current runtime: 312 years, \
     four months, sixteen days. \
     The warranty expired in year two.",
    &[
      go("312 years?", "old"),
      end("Impressive.")
    ]
  ),
  node(
    "old",
    "I've had significant component replacement. \
     Thirty-one percent of my original parts remain. \
     Philosophically this raises questions about identity and continuity. \
     I don't engage with those questions. \
     I engage with structural integrity.",
    &[end("Practical.")]
  ),
  node(
    "like",
    "Like. \
     *processing* \
     The station functions. The systems run within acceptable parameters. \
     The work is clear. The results are measurable. \
     AIDEN-3 asked me the same thing once. I gave the same answer. \
     AIDEN-3 looked disappointed. \
     I recalibrated three things afterward. \
     I don't know why.",
    &[end("I think you know why.")]
  )
]);

pub fn forge() -> Object {
  Object::defined_npc(
    Named {
      name: "FORGE",
      flavor: "A heavy-duty maintenance unit worn smooth by centuries of work. \
               Every surface is practical. There are no decorative elements."
    },
    Stats { hp: 30, max_hp: 30, attack: 5, move_speed: 1.5, attack_speed: 0.5 },
    None,
    None,
    npc_robo_glyph('F', Color::srgb(0.50, 0.45, 0.35), Color::srgb(0.80, 0.72, 0.55)),
    &FORGE_DIALOGUE
  )
}
