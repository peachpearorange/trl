#!/usr/bin/env python3
"""
Generate src/locations/gamma_station.rs

Station layout — 5×5 room grid, each room 24×24 interior (26×26 with walls),
sharing walls with neighbours.  Station occupies cols/rows 12–137 (126 cells).
The remaining 12 cells on each side are vacuum.

Grid (col, row):
  (0,0) Security HQ   (1,0) Guard Post    (2,0) Bridge+Dock  (3,0) Comms      (4,0) Research Lab
  (0,1) Armory        (1,1) Medical Bay   (2,1) Ops Center   (3,1) Science     (4,1) Xenobiology
  (0,2) Crew Quarters (1,2) Cafeteria     (2,2) Atrium       (3,2) Engineering (4,2) Cargo Bay
  (0,3) Brig          (1,3) Maintenance   (2,3) Reactor Core (3,3) Fabrication (4,3) Cargo Store
  (0,4) W Airlock Hub (1,4) Waste Proc.   (2,4) Engine Deck  (3,4) Power Sys.  (4,4) E Airlock Hub
"""

W, H = 150, 150
grid = [['v'] * W for _ in range(H)]

# ── helpers ──────────────────────────────────────────────────────────────────

def s(x, y, c):
    if 0 <= x < W and 0 <= y < H:
        grid[y][x] = c

def g(x, y):
    if 0 <= x < W and 0 <= y < H:
        return grid[y][x]
    return 'v'

def fill(x1, y1, x2, y2, c):
    for y in range(y1, y2 + 1):
        for x in range(x1, x2 + 1):
            s(x, y, c)

# ── room grid geometry ────────────────────────────────────────────────────────

ORIGIN_X = 12
ORIGIN_Y = 12
ROOM_STEP = 25   # rooms are 25 wide/tall (sharing one wall with neighbour)
ROOM_W = 26      # outer wall to outer wall
NCOLS, NROWS = 5, 5

def room_bounds(col, row):
    """Return (x1, y1, x2, y2) for the outer walls of room (col,row)."""
    x1 = ORIGIN_X + col * ROOM_STEP
    y1 = ORIGIN_Y + row * ROOM_STEP
    return x1, y1, x1 + ROOM_W - 1, y1 + ROOM_W - 1

def room_interior(col, row):
    """Interior bounds (floor region, no walls)."""
    x1, y1, x2, y2 = room_bounds(col, row)
    return x1 + 1, y1 + 1, x2 - 1, y2 - 1

def room_center(col, row):
    x1, y1, x2, y2 = room_bounds(col, row)
    return (x1 + x2) // 2, (y1 + y2) // 2

# Draw all rooms: walls then floor
for c in range(NCOLS):
    for r in range(NROWS):
        x1, y1, x2, y2 = room_bounds(c, r)
        fill(x1, y1, x2, y2, '#')   # walls first
        ix1, iy1, ix2, iy2 = room_interior(c, r)
        fill(ix1, iy1, ix2, iy2, '.') # floor

# ── internal doors between adjacent rooms ─────────────────────────────────────
# Every adjacent pair gets a door opening (2 tiles wide).
# Omit some to create controlled access (brig, reactor, security).

SEALED = {
    # (from_col, from_row, direction) where direction is 'E' or 'S'
    (0, 0, 'S'),   # Security HQ → Armory: no direct access
    (2, 3, 'S'),   # Reactor → Engine Deck: airlock-only
    (2, 1, 'S'),   # Ops Center → Atrium: open intentionally (left connected)
}

def place_door_h(x, cy):
    """Door in a horizontal wall at column x, centred on cy."""
    s(x, cy - 1, 'D')
    s(x, cy,     'D')

def place_door_v(cx, y):
    """Door in a vertical wall at row y, centred on cx."""
    s(cx - 1, y, 'D')
    s(cx,     y, 'D')

for c in range(NCOLS):
    for r in range(NROWS):
        cx, cy = room_center(c, r)
        x1, y1, x2, y2 = room_bounds(c, r)

        # East neighbour
        if c + 1 < NCOLS and (c, r, 'E') not in SEALED:
            wall_x = x2   # shared wall column
            place_door_h(wall_x, cy)

        # South neighbour
        if r + 1 < NROWS and (c, r, 'S') not in SEALED:
            wall_y = y2   # shared wall row
            place_door_v(cx, wall_y)

# ── windows on outer hull ─────────────────────────────────────────────────────

def add_outer_windows():
    """Replace outer hull walls facing vacuum with windows."""
    for y in range(H):
        for x in range(W):
            if g(x, y) != '#':
                continue
            for dx, dy in ((-1,0),(1,0),(0,-1),(0,1)):
                if g(x+dx, y+dy) == 'v':
                    s(x, y, 'W')
                    break

add_outer_windows()

# ── ship docking bay (north of Bridge room col=2, row=0) ──────────────────────

DOCK_COL, DOCK_ROW = 2, 0
dx1, dy1, dx2, dy2 = room_bounds(DOCK_COL, DOCK_ROW)
dock_cx = (dx1 + dx2) // 2
dock_bay_top = dy1 - 8   # 8 rows of vacuum bay above the north wall
dock_bay_left = dock_cx - 5
dock_bay_right = dock_cx + 5

# Vacuum notch
fill(dock_bay_left, dock_bay_top, dock_bay_right, dy1 - 1, 'v')
# Bay walls (sides of the notch)
for y in range(dock_bay_top, dy1):
    s(dock_bay_left,  y, '#')
    s(dock_bay_right, y, '#')
# ShipDock tiles on the inner lip (row dy1, inside the notch)
for x in range(dock_bay_left + 1, dock_bay_right):
    s(x, dy1, 'P')
# Airlock at dock entrance
s(dock_bay_left + 1, dy1, 'A')
s(dock_bay_right - 1, dy1, 'A')

# ── space airlocks on outer hull ──────────────────────────────────────────────

def airlock_on(x, y):
    """Place an airlock door tile on the hull at (x,y)."""
    if g(x, y) in ('#', 'W'):
        s(x, y, 'A')

# North hull — row=0 rooms (skip the docking bay area)
for c in (0, 1, 3, 4):
    _, y1, _, _ = room_bounds(c, 0)
    cx, _ = room_center(c, 0)
    airlock_on(cx, y1)

# South hull — row=4 rooms
for c in range(NCOLS):
    _, _, _, y2 = room_bounds(c, 4)
    cx, _ = room_center(c, 4)
    airlock_on(cx, y2)

# West hull — col=0 rooms
for r in range(NROWS):
    x1, _, _, _ = room_bounds(0, r)
    _, cy = room_center(0, r)
    airlock_on(x1, cy)

# East hull — col=4 rooms
for r in range(NROWS):
    _, _, x2, _ = room_bounds(4, r)
    _, cy = room_center(4, r)
    airlock_on(x2, cy)

# ── NPC positions ─────────────────────────────────────────────────────────────

NPC_COORDS = []

npc_rooms = [
    (2, 0, "DOCK-MASTER"),    # Bridge
    (2, 2, "HUB-1"),          # Atrium
    (1, 1, "MEDIC-2"),        # Medical Bay
    (3, 2, "ENGINEER-5"),     # Engineering
    (0, 0, "GUARD-3"),        # Security HQ
    (4, 0, "ANALYST-4"),      # Research Lab
    (0, 2, "STEWARD-6"),      # Crew Quarters
    (4, 2, "CARGO-8"),        # Cargo Bay
    (2, 3, "REACTOR-7"),      # Reactor Core
]

for c, r, label in npc_rooms:
    cx, cy = room_center(c, r)
    NPC_COORDS.append((cx, cy, label))

# ── build output ──────────────────────────────────────────────────────────────

map_rows = [''.join(row) for row in grid]

# Each row must be exactly W chars
assert all(len(r) == W for r in map_rows), "Row length mismatch"
assert len(map_rows) == H, "Row count mismatch"

map_str = '\n'.join(map_rows)

npc_coords_str = '\n  '.join(f'({x}, {y}),' for x, y, _ in NPC_COORDS)

def dialogue(var_name, npc_name, greeting, detail, exit_line):
    return f"""\
static {var_name}: DialogueTree = tree(&[
  node(
    "root",
    "{greeting}",
    &[go("Tell me more.", "detail"), end("{exit_line}")]
  ),
  node(
    "detail",
    "{detail}",
    &[end("Understood.")]
  )
]);
"""

dialogues = [
    ("DOCK_MASTER_DIALOGUE", "DOCK-MASTER",
     "Gamma Station docking control. Identify your vessel and state your business.",
     "This station handles transit cargo and deep-space survey relay. Keep your weapons stowed in the dock.",
     "Duly noted."),
    ("HUB1_DIALOGUE", "HUB-1",
     "Welcome to Gamma Station central hub. I coordinate between all departments. Ask me anything.",
     "Traffic through here has been light since the relay grid went offline. We manage.",
     "Thanks."),
    ("MEDIC2_DIALOGUE", "MEDIC-2",
     "Medical station MEDIC-2. Are you injured? I can run a full diagnostic.",
     "This facility handles minor trauma and radiation treatment. I haven't had a patient in months. Equipment is spotless.",
     "I'll keep that in mind."),
    ("ENGINEER5_DIALOGUE", "ENGINEER-5",
     "Engineering. If you're not here about the coupling fault on deck three, I'm busy.",
     "The station's power draw has been unstable. Reactor output is fine — something upstream is pulling extra load. I'm tracking it.",
     "Good luck with that."),
    ("GUARD3_DIALOGUE", "GUARD-3",
     "Security checkpoint. You're cleared to proceed, but I'm watching.",
     "We had an incident last cycle. Cargo manifest didn't match what came through the airlock. Still investigating.",
     "I'll stay out of trouble."),
    ("ANALYST4_DIALOGUE", "ANALYST-4",
     "Research division. I'm in the middle of a signal analysis — can this wait?",
     "We've been picking up a repeating pattern from the outer belt. Not a known beacon. Not random noise. Something in between.",
     "Interesting. Good luck."),
    ("STEWARD6_DIALOGUE", "STEWARD-6",
     "Crew quarters steward. The bunks are assigned, the schedule is posted, and the coffee is hot.",
     "We've got eleven permanent crew and rotating contract workers. Morale is acceptable. Better than last year.",
     "Good to hear."),
    ("CARGO8_DIALOGUE", "CARGO-8",
     "Cargo management. Everything gets logged, everything gets weighed. No exceptions.",
     "Shipment in bay four is flagged for secondary inspection. Don't touch it — that's not a suggestion.",
     "Hands off, understood."),
    ("REACTOR7_DIALOGUE", "REACTOR-7",
     "Reactor core is restricted. You have thirty seconds to explain why you're here.",
     "Output is nominal. The fluctuations you may have heard about are contained. We have procedures.",
     "I'll leave you to it."),
]

def npc_fn(fn_name, display_name, flavor, glyph_ch, c1r, c1g, c1b, c2r, c2g, c2b,
           hp, attack, move_speed, attack_speed, dialogue_var):
    return f"""\
pub fn {fn_name}() -> Object {{
  Object::defined_npc(
    Named {{
      name: "{display_name}",
      flavor: "{flavor}"
    }},
    Stats {{ hp: {hp}, max_hp: {hp}, attack: {attack}, move_speed: {move_speed}, attack_speed: {attack_speed} }},
    None,
    None,
    npc_robo_glyph('{glyph_ch}', Color::srgb({c1r:.2f}, {c1g:.2f}, {c1b:.2f}), Color::srgb({c2r:.2f}, {c2g:.2f}, {c2b:.2f})),
    &{dialogue_var}
  )
}}
"""

npcs = [
    # fn_name, display_name, flavor, glyph, c1(r,g,b), c2(r,g,b), hp, atk, spd, aspd, dialogue_var
    ("dock_master", "DOCK-MASTER",
     "A stern docking authority unit permanently bolted to the approach console. Blinking amber status lights.",
     'D', 0.55,0.60,0.65, 0.80,0.85,0.90, 25, 3, 1.0, 0.5, "DOCK_MASTER_DIALOGUE"),
    ("hub1", "HUB-1",
     "A slender coordination unit at the atrium centre, its display cycling station-wide status feeds.",
     'H', 0.40,0.70,0.55, 0.70,0.90,0.78, 18, 2, 2.5, 0.6, "HUB1_DIALOGUE"),
    ("medic2", "MEDIC-2",
     "A compact medical unit, its diagnostic arm held at the ready. The medbay gleams.",
     'M', 0.75,0.30,0.30, 0.95,0.70,0.70, 20, 2, 2.0, 0.7, "MEDIC2_DIALOGUE"),
    ("engineer5", "ENGINEER-5",
     "A heavyset engineering unit trailing a bundle of diagnostic cables. Smells faintly of ozone.",
     'E', 0.35,0.55,0.80, 0.65,0.80,0.95, 22, 3, 2.0, 0.6, "ENGINEER5_DIALOGUE"),
    ("guard3", "GUARD-3",
     "A security unit with a dented chassis and an active stun baton. It watches you.",
     'G', 0.30,0.35,0.55, 0.55,0.60,0.80, 30, 5, 3.0, 0.8, "GUARD3_DIALOGUE"),
    ("analyst4", "ANALYST-4",
     "A research unit surrounded by floating holographic spectrographs. It's annotating something.",
     'A', 0.50,0.40,0.70, 0.75,0.65,0.90, 15, 1, 2.5, 0.4, "ANALYST4_DIALOGUE"),
    ("steward6", "STEWARD-6",
     "A crew welfare unit with a calm demeanor and a tray of coffee bulbs clipped to its chassis.",
     'S', 0.55,0.55,0.40, 0.80,0.80,0.60, 16, 2, 2.5, 0.5, "STEWARD6_DIALOGUE"),
    ("cargo8", "CARGO-8",
     "A squat cargo management unit with a barcode scanner fused to its left forearm.",
     'C', 0.45,0.38,0.30, 0.72,0.62,0.50, 20, 3, 1.5, 0.5, "CARGO8_DIALOGUE"),
    ("reactor7", "REACTOR-7",
     "A heavy reactor technician unit running noticeably hot. Radiation insignia on both pauldrons.",
     'R', 0.60,0.45,0.20, 0.85,0.70,0.40, 28, 4, 1.5, 0.5, "REACTOR7_DIALOGUE"),
]

out = f'''\
use {{bevy::prelude::Color, crate::entities::*}};
use crate::{{galaxy::{{Location, LocationId}},
          level::{{LocationType, Tile}},
          prefabs::{{prefab, Prefab}}}};

pub const ID: LocationId = (0, 2, 0);

pub fn station_prefab() -> Prefab {{
  prefab(
"\\
{map_str}
"
  )
  .assoc('v', (Tile::Vacuum, []))
  .assoc('#', (Tile::StationWall, []))
  .assoc('.', (Tile::StationFloor, []))
  .assoc('W', (Tile::Window, []))
  .assoc('D', (Tile::Door, [Object::door()]))
  .assoc('A', (Tile::AirlockDoor, [Object::airlock_door()]))
  .assoc('P', (Tile::ShipDock, []))
}}

pub fn generate() -> Location {{
  Location::from_prefab(station_prefab(), LocationType::SpaceStation, Tile::Vacuum)
}}

pub const NPC_COORDS: &[(i32, i32)] = &[
  {npc_coords_str}
];

'''

for var, npc_name, greeting, detail, exit_line in dialogues:
    out += dialogue(var, npc_name, greeting, detail, exit_line) + '\n'

for fn_name, display_name, flavor, glyph_ch, c1r, c1g, c1b, c2r, c2g, c2b, hp, atk, spd, aspd, dvar in npcs:
    out += npc_fn(fn_name, display_name, flavor, glyph_ch,
                  c1r, c1g, c1b, c2r, c2g, c2b,
                  hp, atk, spd, aspd, dvar) + '\n'

with open('src/locations/gamma_station.rs', 'w') as f:
    f.write(out)

print(f"Wrote src/locations/gamma_station.rs ({H}x{W} map)")
print(f"NPCs: {[label for _, _, label in NPC_COORDS]}")

# Print a minimap for sanity check
print("\nMinimap (every 5th cell):")
for y in range(0, H, 5):
    print(''.join(grid[y][x] for x in range(0, W, 5)))
