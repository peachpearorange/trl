use enum_assoc::Assoc;

#[derive(Assoc, Clone, Copy, PartialEq, Eq, Debug)]
#[func(pub fn glyph(&self) -> &'static str)]
#[func(pub fn color(&self) -> [f32; 3])]
#[func(pub fn texture_path(&self) -> Option<&'static str> { None })]
#[func(pub fn walkable(&self) -> bool)]
#[func(pub fn opaque(&self) -> bool { false })]
#[func(pub fn causes_falling(&self) -> bool { false })]
#[func(pub fn name(&self) -> &'static str)]
#[func(pub fn has_atmosphere(&self) -> bool { true })]
#[func(pub fn space_qud_sprite(&self) -> Option<(&'static str, [f32; 3], [f32; 3])>)]
pub enum Tile {
  #[assoc(glyph = " ", color = [0.0, 0.0, 0.0], walkable = true, causes_falling = true, name = "Air", has_atmosphere = false)]
  Air,
  #[assoc(glyph = ".", color = [0.50, 0.56, 0.64], walkable = true, name = "Deck Plate", space_qud_sprite = ("textures/space_qud/floor3.png", [0.42, 0.50, 0.60], [0.72, 0.80, 0.90]))]
  DeckPlate,
  #[assoc(glyph = "#", color = [0.4, 0.4, 0.4], walkable = false, opaque = true, name = "Wall")]
  Wall,
  #[assoc(glyph = "#", color = [0.5, 0.5, 0.5], walkable = false, opaque = true, name = "Cobblestone Wall")]
  CobblestoneWall,
  #[assoc(glyph = "#", color = [0.6, 0.3, 0.2], walkable = false, opaque = true, name = "Brick Wall")]
  BrickWall,
  #[assoc(glyph = "\"", color = [0.2, 0.6, 0.2], walkable = true, name = "Grass", space_qud_sprite = ("textures/space_qud/grass.png", [0.22, 0.48, 0.18], [0.52, 0.72, 0.28]))]
  Grass,
  #[assoc(glyph = "~", color = [0.2, 0.3, 0.8], walkable = false, name = "Water", space_qud_sprite = ("textures/space_qud/liquid tile.png", [0.08, 0.18, 0.55], [0.28, 0.52, 0.88]))]
  Water,
  #[assoc(glyph = ",", color = [0.8, 0.7, 0.4], walkable = true, name = "Sand", space_qud_sprite = ("textures/space_qud/wavy.png", [0.72, 0.62, 0.38], [0.92, 0.86, 0.62]))]
  Sand,
  #[assoc(glyph = "<", color = [0.9, 0.9, 0.2], walkable = true, name = "Stairs Up")]
  StairsUp,
  #[assoc(glyph = ">", color = [0.9, 0.9, 0.2], walkable = true, name = "Stairs Down")]
  StairsDown,
  #[assoc(glyph = "+", color = [0.6, 0.3, 0.1], walkable = true, name = "Door")]
  Door,
  #[assoc(glyph = "\"", color = [0.25, 0.65, 0.25], walkable = true, name = "Tall Grass", space_qud_sprite = ("textures/space_qud/grass.png", [0.22, 0.48, 0.18], [0.52, 0.72, 0.28]))]
  TallGrass,
  #[assoc(glyph = "%", color = [0.15, 0.45, 0.15], walkable = false, name = "Bush")]
  Bush,
  #[assoc(glyph = ".", color = [0.55, 0.53, 0.5], walkable = true, name = "Ash", space_qud_sprite = ("textures/space_qud/ground.png", [0.32, 0.30, 0.28], [0.55, 0.53, 0.50]))]
  Ash,
  #[assoc(glyph = "~", color = [0.9, 0.3, 0.05], walkable = false, name = "Lava", space_qud_sprite = ("textures/space_qud/liquid tile.png", [0.72, 0.18, 0.04], [0.95, 0.52, 0.08]))]
  Lava,
  #[assoc(glyph = "~", color = [0.3, 0.5, 0.85], walkable = true, name = "Shallow Water", space_qud_sprite = ("textures/space_qud/wavy.png", [0.18, 0.42, 0.62], [0.45, 0.68, 0.88]))]
  ShallowWater,
  #[assoc(glyph = "≈", color = [0.1, 0.15, 0.6], walkable = false, name = "Deep Water", space_qud_sprite = ("textures/space_qud/liquid tile.png", [0.04, 0.08, 0.42], [0.12, 0.28, 0.68]))]
  DeepWater,
  #[assoc(glyph = "·", color = [0.45, 0.4, 0.35], walkable = true, name = "Road")]
  Road,
  #[assoc(glyph = "#", color = [0.45, 0.3, 0.15], walkable = false, opaque = true, name = "Wooden Wall")]
  WoodWall,
  #[assoc(glyph = ".", color = [0.55, 0.4, 0.25], walkable = true, name = "Wooden Floor", space_qud_sprite = ("textures/space_qud/floor4.png", [0.45, 0.32, 0.18], [0.72, 0.58, 0.32]))]
  WoodFloor,
  #[assoc(glyph = ".", color = [0.42, 0.28, 0.14], walkable = true, name = "Wood Tile", space_qud_sprite = ("textures/space_qud/tiles1.png", [0.28, 0.16, 0.08], [0.62, 0.44, 0.24]))]
  WoodTile,
  #[assoc(glyph = "+", color = [0.5, 0.35, 0.2], walkable = false, name = "Fence")]
  Fence,
  #[assoc(glyph = "#", color = [0.3, 0.28, 0.25], walkable = false, opaque = true, name = "Cave Wall", space_qud_sprite = ("textures/space_qud/cobble tile.png", [0.28, 0.26, 0.24], [0.48, 0.46, 0.42]))]
  CaveWall,
  #[assoc(glyph = ".", color = [0.4, 0.38, 0.35], walkable = true, name = "Cave Floor", space_qud_sprite = ("textures/space_qud/floor3.png", [0.30, 0.28, 0.25], [0.48, 0.44, 0.38]))]
  CaveFloor,
  #[assoc(glyph = "*", color = [0.5, 0.8, 0.95], walkable = false, name = "Crystal Formation")]
  CrystalFormation,
  // --- Space tiles ---
  #[assoc(glyph = "#", color = [0.45, 0.47, 0.50], walkable = false, opaque = true, name = "Bulkhead", space_qud_sprite = ("textures/space_qud/wall hashtag.png", [0.28, 0.30, 0.34], [0.48, 0.52, 0.56]))]
  Bulkhead,
  #[assoc(glyph = "o", color = [0.2, 0.25, 0.7], walkable = false, name = "Window", space_qud_sprite = ("textures/space_qud/window (1).png", [0.22, 0.32, 0.52], [0.62, 0.76, 0.94]))]
  Window,
  #[assoc(glyph = "+", color = [0.7, 0.65, 0.3], walkable = true, name = "Airlock Door")]
  AirlockDoor,
  #[assoc(glyph = ".", color = [0.55, 0.58, 0.62], walkable = true, name = "Station Floor", space_qud_sprite = ("textures/space_qud/floor4.png", [0.52, 0.56, 0.62], [0.88, 0.90, 0.94]))]
  StationFloor,
  #[assoc(glyph = "#", color = [0.5, 0.52, 0.55], walkable = false, opaque = true, name = "Station Wall", space_qud_sprite = ("textures/space_qud/wall hashtag.png", [0.28, 0.30, 0.34], [0.48, 0.52, 0.56]))]
  StationWall,
  #[assoc(glyph = ".", color = [0.35, 0.33, 0.3], walkable = true, name = "Derelict Floor", space_qud_sprite = ("textures/space_qud/floor2.png", [0.28, 0.26, 0.22], [0.42, 0.38, 0.32]))]
  DerelictFloor,
  #[assoc(glyph = "#", color = [0.3, 0.28, 0.25], walkable = false, opaque = true, name = "Derelict Wall", space_qud_sprite = ("textures/space_qud/wall hashtag.png", [0.28, 0.30, 0.34], [0.48, 0.52, 0.56]))]
  DerelictWall,
  #[assoc(glyph = "=", color = [0.6, 0.55, 0.2], walkable = true, name = "Conduit", space_qud_sprite = ("textures/space_qud/grid.png", [0.40, 0.28, 0.14], [0.88, 0.62, 0.22]))]
  Conduit,
  #[assoc(glyph = "#", color = [0.4, 0.35, 0.3], walkable = false, opaque = true, name = "Asteroid Rock", space_qud_sprite = ("textures/space_qud/wall hashtag.png", [0.42, 0.38, 0.36], [0.58, 0.54, 0.52]))]
  AsteroidRock,
  #[assoc(glyph = ".", color = [0.5, 0.45, 0.4], walkable = true, name = "Asteroid Floor", space_qud_sprite = ("textures/space_qud/ground.png", [0.48, 0.46, 0.44], [0.72, 0.70, 0.68]))]
  AsteroidFloor,
  #[assoc(glyph = ",", color = [0.55, 0.5, 0.45], walkable = true, name = "Regolith", space_qud_sprite = ("textures/space_qud/ground.png", [0.48, 0.46, 0.44], [0.72, 0.70, 0.68]))]
  Regolith,
  #[assoc(glyph = " ", color = [0.0, 0.0, 0.0], walkable = true, name = "Vacuum", has_atmosphere = false, space_qud_sprite = ("textures/space_qud/space background.png", [1.0, 1.0, 1.0], [0.62, 0.72, 0.92]))]
  Vacuum,
  #[assoc(glyph = ",", color = [0.7, 0.75, 0.85], walkable = true, name = "Ice Floor")]
  IceFloor,
  #[assoc(glyph = "#", color = [0.5, 0.55, 0.7], walkable = false, opaque = true, name = "Ice Wall")]
  IceWall,
  #[assoc(glyph = ",", color = [0.45, 0.35, 0.55], walkable = true, name = "Alien Soil")]
  AlienSoil,
  #[assoc(glyph = "\"", color = [0.3, 0.55, 0.3], walkable = true, name = "Alien Grass", space_qud_sprite = ("textures/space_qud/grass.png", [0.38, 0.16, 0.52], [0.68, 0.52, 0.88]))]
  AlienGrass,
  #[assoc(glyph = "*", color = [0.5, 0.8, 0.95], walkable = false, name = "Crystal Growth")]
  CrystalGrowth,
  #[assoc(glyph = "~", color = [0.5, 0.3, 0.7], walkable = true, name = "Alien Fluid", space_qud_sprite = ("textures/space_qud/liquid tile.png", [0.35, 0.12, 0.52], [0.68, 0.32, 0.88]))]
  AlienFluid,
  #[assoc(glyph = "P", color = [0.85, 0.72, 0.1], walkable = true, name = "Ship Dock", has_atmosphere = false)]
  ShipDock,
  /// Transparent filler for ship bounding-box corners — skipped when merging
  /// the ship into a docked zone so it never overwrites the destination tiles.
  #[assoc(glyph = " ", color = [0.0, 0.0, 0.0], walkable = true, name = "Blank", has_atmosphere = false)]
  Blank
}
