use {bevy::prelude::Color, crate::entities::*};
use crate::{galaxy::{Location, LocationId},
          level::{LocationType, Tile},
          prefabs::{prefab, Prefab}};

pub const ID: LocationId = (0, 2, 0);

pub fn station_prefab() -> Prefab {
  prefab(
"..................................................................................
......wwsssww.....................................................................
......wfffffw.....................................................................
......sfffffs........wwwwPwwwww...................................................
......sfffffs........wlfffffllw...................................................
......wff1ffw........wlfffffffw......wwwwwwwww....................................
......wfffffwwsssssswwfrrffrrfw......wccfffccw....................................
......wwwdwwwffffffffwfrrffrrfwsssssswfffffffw....................................
......wfffffdff2fffffdfrrf0rrfdffffffdfffffffw....................................
......wfffffwwwwwwwwwwfrrffrrfwsssssswfffffffw....................................
......wffwwwwttttwfffdfrrffrrfw......wwwwdwwww....................................
......wffwtffffffsfffdffffffffw......wbfffbffw....................................
......wffwtffffffdfffdffffffffw......wfffffffw....................................
......wffwtfftttfsfffwwwwwdwwww......wfffffffw....................................
......wffwtfftttfsfffw..wfffw........wff5ffffw....................................
......wffwtf4ffffsfffw..wfffw........wwwwdwwww....................................
......wffwsssdssswfffw..wfffw...........wffffw....................................
......sffwfffffffffffw..wwwww...........wffffw....................................
......sffdffff3ffffffw..................wffffw.....................................
......sffwfffffffffffw...............wwwwwdwww.....................................
......wffwfffwwwwwwwww...............wcff8ffcw....................................
......wffwfffw.......................wcfffffcw....................................
......wffwfffw.......................wcfffffcw....................................
......wffwfffw.......................wwwwwwwww....................................
......wffwfffwwwwwwwwww...........................................................
......wffwffffffffffffs...........................................................
......wffwfffffff7ffffs...........................................................
......wffwwwwwwdwwwwwww....................................................... ...
......wffw....wfffffw.............................................................
......wffw....wfffffw.............................................................
......wffwwwwwdfffffw.............................................................
......wfffffffffffffw.............................................................
......wwwwwwwwwwwwwww.............................................................
..................................................................................
..................................................................................
..................................................................................
..................................................................................
..................................................................................
..................................................................................
..................................................................................
..................................................................................
..................................................................................
..................................................................................
..................................................................................
..................................................................................
..................................................................................
..................................................................................
..................................................................................
..................................................................................
..................................................................................
"
  )
  .assoc('.', (Tile::Vacuum, []))
  .assoc('w', (Tile::StationWall, []))
  .assoc('s', (Tile::Window, []))
  .assoc('f', (Tile::StationFloor, []))
  .assoc('d', (Tile::StationFloor, [Object::airlock_door()]))
  .assoc('P', (Tile::ShipDock, []))
  .assoc('l', (Tile::StationFloor, [Object::locker()]))
  .assoc('t', (Tile::StationFloor, [Object::table()]))
  .assoc('b', (Tile::StationFloor, [Object::bed()]))
  .assoc('c', (Tile::StationFloor, [Object::crate_obj()]))
  .assoc('r', (Tile::StationFloor, [Object::chair()]))
  .assoc('0', (Tile::StationFloor, [dock_master()]))
  .assoc('1', (Tile::StationFloor, [hub1()]))
  .assoc('2', (Tile::StationFloor, [medic2()]))
  .assoc('3', (Tile::StationFloor, [engineer5()]))
  .assoc('4', (Tile::StationFloor, [guard3()]))
  .assoc('5', (Tile::StationFloor, [analyst4()]))
  .assoc('6', (Tile::StationFloor, [steward6()]))
  .assoc('7', (Tile::StationFloor, [cargo8()]))
}

pub fn generate() -> Location {
  Location::from_prefab("Gamma Station", station_prefab(), LocationType::SpaceStation, Tile::Vacuum)
}

pub const NPC_COORDS: &[(i32, i32)] = &[
  (74, 24),
  (74, 74),
  (49, 49),
  (99, 74),
  (24, 24),
  (124, 24),
  (24, 74),
  (124, 74),
  (74, 99),
];

static DOCK_MASTER_DIALOGUE: DialogueTree = tree(&[
  node(
    "root",
    "Gamma Station docking control. Identify your vessel and state your business.",
    &[go("Tell me more.", "detail"), end("Duly noted.")]
  ),
  node(
    "detail",
    "This station handles transit cargo and deep-space survey relay. Keep your weapons stowed in the dock.",
    &[end("Understood.")]
  )
]);

static HUB1_DIALOGUE: DialogueTree = tree(&[
  node(
    "root",
    "Welcome to Gamma Station central hub. I coordinate between all departments. Ask me anything.",
    &[go("Tell me more.", "detail"), end("Thanks.")]
  ),
  node(
    "detail",
    "Traffic through here has been light since the relay grid went offline. We manage.",
    &[end("Understood.")]
  )
]);

static MEDIC2_DIALOGUE: DialogueTree = tree(&[
  node(
    "root",
    "Medical station MEDIC-2. Are you injured? I can run a full diagnostic.",
    &[go("Tell me more.", "detail"), end("I'll keep that in mind.")]
  ),
  node(
    "detail",
    "This facility handles minor trauma and radiation treatment. I haven't had a patient in months. Equipment is spotless.",
    &[end("Understood.")]
  )
]);

static ENGINEER5_DIALOGUE: DialogueTree = tree(&[
  node(
    "root",
    "Engineering. If you're not here about the coupling fault on deck three, I'm busy.",
    &[go("Tell me more.", "detail"), end("Good luck with that.")]
  ),
  node(
    "detail",
    "The station's power draw has been unstable. Reactor output is fine — something upstream is pulling extra load. I'm tracking it.",
    &[end("Understood.")]
  )
]);

static GUARD3_DIALOGUE: DialogueTree = tree(&[
  node(
    "root",
    "Security checkpoint. You're cleared to proceed, but I'm watching.",
    &[go("Tell me more.", "detail"), end("I'll stay out of trouble.")]
  ),
  node(
    "detail",
    "We had an incident last cycle. Cargo manifest didn't match what came through the airlock. Still investigating.",
    &[end("Understood.")]
  )
]);

static ANALYST4_DIALOGUE: DialogueTree = tree(&[
  node(
    "root",
    "Research division. I'm in the middle of a signal analysis — can this wait?",
    &[go("Tell me more.", "detail"), end("Interesting. Good luck.")]
  ),
  node(
    "detail",
    "We've been picking up a repeating pattern from the outer belt. Not a known beacon. Not random noise. Something in between.",
    &[end("Understood.")]
  )
]);

static STEWARD6_DIALOGUE: DialogueTree = tree(&[
  node(
    "root",
    "Crew quarters steward. The bunks are assigned, the schedule is posted, and the coffee is hot.",
    &[go("Tell me more.", "detail"), end("Good to hear.")]
  ),
  node(
    "detail",
    "We've got eleven permanent crew and rotating contract workers. Morale is acceptable. Better than last year.",
    &[end("Understood.")]
  )
]);

static CARGO8_DIALOGUE: DialogueTree = tree(&[
  node(
    "root",
    "Cargo management. Everything gets logged, everything gets weighed. No exceptions.",
    &[go("Tell me more.", "detail"), end("Hands off, understood.")]
  ),
  node(
    "detail",
    "Shipment in bay four is flagged for secondary inspection. Don't touch it — that's not a suggestion.",
    &[end("Understood.")]
  )
]);

static REACTOR7_DIALOGUE: DialogueTree = tree(&[
  node(
    "root",
    "Reactor core is restricted. You have thirty seconds to explain why you're here.",
    &[go("Tell me more.", "detail"), end("I'll leave you to it.")]
  ),
  node(
    "detail",
    "Output is nominal. The fluctuations you may have heard about are contained. We have procedures.",
    &[end("Understood.")]
  )
]);

pub fn dock_master() -> Object {
  Object::defined_npc(
    Named {
      name: "DOCK-MASTER",
      flavor: "A stern docking authority unit permanently bolted to the approach console. Blinking amber status lights."
    },
    Stats { hp: 25, max_hp: 25, attack: 3, move_speed: 1.0, attack_speed: 0.5 },
    Loadout::default(),
    npc_robo_glyph('D', Color::srgb(0.55, 0.60, 0.65), Color::srgb(0.80, 0.85, 0.90)),
    &DOCK_MASTER_DIALOGUE
  )
}

pub fn hub1() -> Object {
  Object::defined_npc(
    Named {
      name: "HUB-1",
      flavor: "A slender coordination unit at the atrium centre, its display cycling station-wide status feeds."
    },
    Stats { hp: 18, max_hp: 18, attack: 2, move_speed: 2.5, attack_speed: 0.6 },
    Loadout::default(),
    npc_robo_glyph('H', Color::srgb(0.40, 0.70, 0.55), Color::srgb(0.70, 0.90, 0.78)),
    &HUB1_DIALOGUE
  )
}

pub fn medic2() -> Object {
  Object::defined_npc(
    Named {
      name: "MEDIC-2",
      flavor: "A compact medical unit, its diagnostic arm held at the ready. The medbay gleams."
    },
    Stats { hp: 20, max_hp: 20, attack: 2, move_speed: 2.0, attack_speed: 0.7 },
    Loadout::default(),
    npc_robo_glyph('M', Color::srgb(0.75, 0.30, 0.30), Color::srgb(0.95, 0.70, 0.70)),
    &MEDIC2_DIALOGUE
  )
}

pub fn engineer5() -> Object {
  Object::defined_npc(
    Named {
      name: "ENGINEER-5",
      flavor: "A heavyset engineering unit trailing a bundle of diagnostic cables. Smells faintly of ozone."
    },
    Stats { hp: 22, max_hp: 22, attack: 3, move_speed: 2.0, attack_speed: 0.6 },
    Loadout::default(),
    npc_robo_glyph('E', Color::srgb(0.35, 0.55, 0.80), Color::srgb(0.65, 0.80, 0.95)),
    &ENGINEER5_DIALOGUE
  )
}

pub fn guard3() -> Object {
  Object::defined_npc(
    Named {
      name: "GUARD-3",
      flavor: "A security unit with a dented chassis and an active stun baton. It watches you."
    },
    Stats { hp: 30, max_hp: 30, attack: 5, move_speed: 3.0, attack_speed: 0.8 },
    Loadout::default(),
    npc_robo_glyph('G', Color::srgb(0.30, 0.35, 0.55), Color::srgb(0.55, 0.60, 0.80)),
    &GUARD3_DIALOGUE
  )
}

pub fn analyst4() -> Object {
  Object::defined_npc(
    Named {
      name: "ANALYST-4",
      flavor: "A research unit surrounded by floating holographic spectrographs. It's annotating something."
    },
    Stats { hp: 15, max_hp: 15, attack: 1, move_speed: 2.5, attack_speed: 0.4 },
    Loadout::default(),
    npc_robo_glyph('A', Color::srgb(0.50, 0.40, 0.70), Color::srgb(0.75, 0.65, 0.90)),
    &ANALYST4_DIALOGUE
  )
}

pub fn steward6() -> Object {
  Object::defined_npc(
    Named {
      name: "STEWARD-6",
      flavor: "A crew welfare unit with a calm demeanor and a tray of coffee bulbs clipped to its chassis."
    },
    Stats { hp: 16, max_hp: 16, attack: 2, move_speed: 2.5, attack_speed: 0.5 },
    Loadout::default(),
    npc_robo_glyph('S', Color::srgb(0.55, 0.55, 0.40), Color::srgb(0.80, 0.80, 0.60)),
    &STEWARD6_DIALOGUE
  )
}

pub fn cargo8() -> Object {
  Object::defined_npc(
    Named {
      name: "CARGO-8",
      flavor: "A squat cargo management unit with a barcode scanner fused to its left forearm."
    },
    Stats { hp: 20, max_hp: 20, attack: 3, move_speed: 1.5, attack_speed: 0.5 },
    Loadout::default(),
    npc_robo_glyph('C', Color::srgb(0.45, 0.38, 0.30), Color::srgb(0.72, 0.62, 0.50)),
    &CARGO8_DIALOGUE
  )
}

pub fn reactor7() -> Object {
  Object::defined_npc(
    Named {
      name: "REACTOR-7",
      flavor: "A heavy reactor technician unit running noticeably hot. Radiation insignia on both pauldrons."
    },
    Stats { hp: 28, max_hp: 28, attack: 4, move_speed: 1.5, attack_speed: 0.5 },
    Loadout::default(),
    npc_robo_glyph('R', Color::srgb(0.60, 0.45, 0.20), Color::srgb(0.85, 0.70, 0.40)),
    &REACTOR7_DIALOGUE
  )
}
