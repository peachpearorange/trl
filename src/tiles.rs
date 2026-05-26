use enum_assoc::Assoc;
use num_enum::TryFromPrimitive;

/// Describes how a tile is rendered in graphical mode.
pub enum TileRenderMode {
  SolidColor,
  Sprite(&'static str, [f32; 3], [f32; 3]),
  /// All 8 flipped/rotated variants of each texture in the pack are added to the
  /// tileset as separate layers, and one is picked per-tile by position hash.
  SpritePackRandom(&'static [&'static str], [f32; 3], [f32; 3]),
  /// Connected wall tileset: seven base textures
  /// `[iso, end, straight, L, T, cross, reverse_L]` in their canonical
  /// orientations (iso: no neighbors; end: connects south; straight: connects
  /// east+west; L: connects north+east; T: connects N+E+W; cross: connects
  /// all; reverse_L: connects N+W — the mirror of L, needed because L's
  /// parity-preserving orbit only covers two of the four diagonal corners).
  /// The baker expands these into 16 layers, one per 4-bit cardinal-neighbor
  /// mask (bit 0 = N (y-1), 1 = E (x+1), 2 = S (y+1), 3 = W (x-1)); the
  /// renderer picks the layer matching same-tile neighbors.
  ConnectedSprite(&'static [&'static str; 7], [f32; 3], [f32; 3]),
  /// Connected border: a single texture with a 1px border. The baker generates
  /// 16 variants by removing border pixels on connected sides (keeping corner
  /// pixels) and clearing inner corner reinforcement.
  ConnectedBorder(&'static str, [f32; 3], [f32; 3])
}

impl TileRenderMode {
  pub fn colors(&self) -> ([f32; 3], [f32; 3]) {
    match self {
      Self::SolidColor => ([0.5; 3], [0.5; 3]),
      Self::Sprite(_, p, s)
      | Self::SpritePackRandom(_, p, s)
      | Self::ConnectedSprite(_, p, s)
      | Self::ConnectedBorder(_, p, s) => (*p, *s)
    }
  }
}

#[derive(Assoc, Clone, Copy, PartialEq, Eq, Debug, TryFromPrimitive)]
#[repr(u16)]
#[func(pub fn glyph(&self) -> &'static str)]
#[func(pub fn color(&self) -> [f32; 3])]
#[func(pub fn walkable(&self) -> bool)]
#[func(pub fn opaque(&self) -> bool { false })]
#[func(pub fn causes_falling(&self) -> bool { false })]
#[func(pub fn name(&self) -> &'static str)]
#[func(pub fn has_atmosphere(&self) -> bool { true })]
#[func(pub fn render_mode(&self) -> TileRenderMode { TileRenderMode::SolidColor })]
#[func(pub fn is_liquid(&self) -> bool { false })]
pub enum Tile {
  // --- Atmosphere / void ---
  #[assoc(glyph = " ", color = [0.0, 0.0, 0.0], walkable = true, causes_falling = true, name = "Air", has_atmosphere = false)]
  Air,
  #[assoc(glyph = " ", color = [0.0, 0.0, 0.0], walkable = true, name = "Vacuum", has_atmosphere = false, render_mode = TileRenderMode::SpritePackRandom(&["textures/space_qud/stars1.png", "textures/space_qud/stars2.png", "textures/space_qud/stars3.png", "textures/space_qud/stars4.png"], [1.0, 1.0, 1.0], [0.62, 0.72, 0.92]))]
  Vacuum,
  /// Transparent filler for ship bounding-box corners — skipped when merging
  /// the ship into a docked zone so it never overwrites the destination tiles.
  #[assoc(glyph = " ", color = [0.0, 0.0, 0.0], walkable = true, name = "Blank", has_atmosphere = false)]
  Blank,

  // --- Liquids (keep contiguous for editor palette) ---
  #[assoc(glyph = "~", color = [0.2, 0.3, 0.8], walkable = false, name = "Water", is_liquid = true, render_mode = TileRenderMode::Sprite("textures/space_qud/liquid tile.png", [0.08, 0.18, 0.55], [0.28, 0.52, 0.88]))]
  Water,
  #[assoc(glyph = "~", color = [0.3, 0.5, 0.85], walkable = true, name = "Shallow Water", is_liquid = true, render_mode = TileRenderMode::Sprite("textures/space_qud/wavy.png", [0.18, 0.42, 0.62], [0.45, 0.68, 0.88]))]
  ShallowWater,
  #[assoc(glyph = "≈", color = [0.1, 0.15, 0.6], walkable = false, name = "Deep Water", is_liquid = true, render_mode = TileRenderMode::Sprite("textures/space_qud/liquid tile.png", [0.04, 0.08, 0.42], [0.12, 0.28, 0.68]))]
  DeepWater,
  #[assoc(glyph = "~", color = [0.9, 0.3, 0.05], walkable = false, name = "Lava", is_liquid = true, render_mode = TileRenderMode::Sprite("textures/space_qud/liquid tile.png", [0.72, 0.18, 0.04], [0.95, 0.52, 0.08]))]
  Lava,
  #[assoc(glyph = "~", color = [0.5, 0.3, 0.7], walkable = true, name = "Alien Fluid", is_liquid = true, render_mode = TileRenderMode::Sprite("textures/space_qud/liquid tile.png", [0.35, 0.12, 0.52], [0.68, 0.32, 0.88]))]
  AlienFluid,
  #[assoc(glyph = "~", color = [0.1, 0.75, 0.8], walkable = true, name = "Bioluminescent Pool", is_liquid = true, render_mode = TileRenderMode::Sprite("textures/space_qud/liquid tile.png", [0.04, 0.48, 0.62], [0.18, 0.88, 0.95]))]
  BioluminescentPool,
  #[assoc(glyph = "~", color = [0.65, 0.85, 0.1], walkable = true, name = "Acid Pool", is_liquid = true, render_mode = TileRenderMode::Sprite("textures/space_qud/liquid tile.png", [0.42, 0.62, 0.05], [0.72, 0.92, 0.22]))]
  AcidPool,
  #[assoc(glyph = "~", color = [0.75, 0.12, 0.18], walkable = true, name = "Crimson Pool", is_liquid = true, render_mode = TileRenderMode::Sprite("textures/space_qud/liquid tile.png", [0.52, 0.06, 0.08], [0.88, 0.28, 0.32]))]
  CrimsonPool,
  #[assoc(glyph = "~", color = [0.85, 0.52, 0.08], walkable = true, name = "Amber Pool", is_liquid = true, render_mode = TileRenderMode::Sprite("textures/space_qud/liquid tile.png", [0.62, 0.32, 0.04], [0.92, 0.68, 0.22]))]
  AmberPool,

  // --- Ship & station ---
  #[assoc(glyph = ".", color = [0.50, 0.56, 0.64], walkable = true, name = "Deck Plate", render_mode = TileRenderMode::Sprite("textures/space_qud/floor3.png", [0.42, 0.50, 0.60], [0.72, 0.80, 0.90]))]
  DeckPlate,
  #[assoc(glyph = ".", color = [0.55, 0.58, 0.62], walkable = true, name = "Station Floor", render_mode = TileRenderMode::Sprite("textures/space_qud/floor4.png", [0.52, 0.56, 0.62], [0.88, 0.90, 0.94]))]
  StationFloor,
  #[assoc(glyph = "#", color = [0.5, 0.52, 0.55], walkable = false, opaque = true, name = "Station Wall", render_mode = TileRenderMode::Sprite("textures/space_qud/wall1.png", [0.28, 0.30, 0.34], [0.48, 0.52, 0.56]))]
  StationWall,
  #[assoc(glyph = "#", color = [0.55, 0.58, 0.62], walkable = false, opaque = true, name = "Ship Wall", render_mode = TileRenderMode::ConnectedSprite(&[
    "textures/space_qud/wall2 iso.png",
    "textures/space_qud/wall2 end.png",
    "textures/space_qud/wall2 straight.png",
    "textures/space_qud/wall2 L.png",
    "textures/space_qud/wall2 T.png",
    "textures/space_qud/wall2 cross.png",
    "textures/space_qud/wall2 reverse L.png",
  ], [0.30, 0.32, 0.36], [0.55, 0.60, 0.68]))]
  ShipWall,
  #[assoc(glyph = "#", color = [0.45, 0.47, 0.50], walkable = false, opaque = true, name = "Bulkhead", render_mode = TileRenderMode::Sprite("textures/space_qud/wall hashtag.png", [0.28, 0.30, 0.34], [0.48, 0.52, 0.56]))]
  Bulkhead,
  #[assoc(glyph = "o", color = [0.2, 0.25, 0.7], walkable = false, name = "Window", render_mode = TileRenderMode::ConnectedBorder("textures/space_qud/window (1).png", [0.22, 0.32, 0.52], [0.62, 0.76, 0.94]))]
  Window,
  #[assoc(glyph = ".", color = [0.35, 0.33, 0.3], walkable = true, name = "Derelict Floor", render_mode = TileRenderMode::Sprite("textures/space_qud/floor2.png", [0.28, 0.26, 0.22], [0.42, 0.38, 0.32]))]
  DerelictFloor,
  #[assoc(glyph = "#", color = [0.3, 0.28, 0.25], walkable = false, opaque = true, name = "Derelict Wall", render_mode = TileRenderMode::Sprite("textures/space_qud/wall hashtag.png", [0.28, 0.30, 0.34], [0.48, 0.52, 0.56]))]
  DerelictWall,
  #[assoc(glyph = "=", color = [0.6, 0.55, 0.2], walkable = true, name = "Conduit", render_mode = TileRenderMode::Sprite("textures/space_qud/grid.png", [0.40, 0.28, 0.14], [0.88, 0.62, 0.22]))]
  Conduit,
  #[assoc(glyph = "P", color = [0.85, 0.72, 0.1], walkable = true, name = "Ship Dock", has_atmosphere = false, render_mode = TileRenderMode::Sprite("textures/space_qud/diagonal lines tile.png", [0.62, 0.50, 0.05], [1.0, 0.92, 0.38]))]
  ShipDock,

  // --- Nature & planetary floors ---
  #[assoc(glyph = "\"", color = [0.2, 0.6, 0.2], walkable = true, name = "Grass", render_mode = TileRenderMode::Sprite("textures/space_qud/grass.png", [0.22, 0.48, 0.18], [0.52, 0.72, 0.28]))]
  Grass,
  #[assoc(glyph = ",", color = [0.8, 0.7, 0.4], walkable = true, name = "Sand", render_mode = TileRenderMode::Sprite("textures/space_qud/wavy.png", [0.72, 0.62, 0.38], [0.92, 0.86, 0.62]))]
  Sand,
  #[assoc(glyph = "\"", color = [0.25, 0.65, 0.25], walkable = true, name = "Tall Grass", render_mode = TileRenderMode::Sprite("textures/space_qud/grass.png", [0.22, 0.48, 0.18], [0.52, 0.72, 0.28]))]
  TallGrass,
  #[assoc(glyph = ".", color = [0.55, 0.53, 0.5], walkable = true, name = "Ash", render_mode = TileRenderMode::SpritePackRandom(&["textures/space_qud/ground.png"], [0.32, 0.30, 0.28], [0.55, 0.53, 0.50]))]
  Ash,
  #[assoc(glyph = "·", color = [0.45, 0.4, 0.35], walkable = true, name = "Road")]
  Road,
  #[assoc(glyph = ".", color = [0.55, 0.4, 0.25], walkable = true, name = "Wooden Floor", render_mode = TileRenderMode::Sprite("textures/space_qud/floor4.png", [0.45, 0.32, 0.18], [0.72, 0.58, 0.32]))]
  WoodFloor,
  #[assoc(glyph = ".", color = [0.42, 0.28, 0.14], walkable = true, name = "Wood Tile", render_mode = TileRenderMode::Sprite("textures/space_qud/tiles1.png", [0.28, 0.16, 0.08], [0.62, 0.44, 0.24]))]
  WoodTile,
  #[assoc(glyph = ".", color = [0.4, 0.38, 0.35], walkable = true, name = "Cave Floor", render_mode = TileRenderMode::Sprite("textures/space_qud/floor3.png", [0.30, 0.28, 0.25], [0.48, 0.44, 0.38]))]
  CaveFloor,
  #[assoc(glyph = ".", color = [0.5, 0.45, 0.4], walkable = true, name = "Asteroid Floor", render_mode = TileRenderMode::SpritePackRandom(&["textures/space_qud/ground.png"], [0.48, 0.46, 0.44], [0.72, 0.70, 0.68]))]
  AsteroidFloor,
  #[assoc(glyph = ",", color = [0.55, 0.5, 0.45], walkable = true, name = "Regolith", render_mode = TileRenderMode::SpritePackRandom(&["textures/space_qud/ground.png"], [0.48, 0.46, 0.44], [0.72, 0.70, 0.68]))]
  Regolith,
  #[assoc(glyph = ",", color = [0.7, 0.75, 0.85], walkable = true, name = "Ice Floor")]
  IceFloor,
  #[assoc(glyph = ",", color = [0.45, 0.35, 0.55], walkable = true, name = "Alien Soil", render_mode = TileRenderMode::SpritePackRandom(&["textures/space_qud/ground.png"], [0.28, 0.18, 0.38], [0.52, 0.38, 0.62]))]
  AlienSoil,
  #[assoc(glyph = "\"", color = [0.3, 0.55, 0.3], walkable = true, name = "Alien Grass", render_mode = TileRenderMode::Sprite("textures/space_qud/grass.png", [0.38, 0.16, 0.52], [0.68, 0.52, 0.88]))]
  AlienGrass,
  #[assoc(glyph = ".", color = [0.65, 0.78, 0.88], walkable = true, name = "Bright Ground", render_mode = TileRenderMode::SpritePackRandom(&["textures/space_qud/ground.png"], [0.58, 0.72, 0.85], [0.82, 0.90, 0.96]))]
  BrightGround,

  // --- Walls & barriers ---
  #[assoc(glyph = "#", color = [0.4, 0.4, 0.4], walkable = false, opaque = true, name = "Wall")]
  Wall,
  #[assoc(glyph = "#", color = [0.5, 0.5, 0.5], walkable = false, opaque = true, name = "Cobblestone Wall")]
  CobblestoneWall,
  #[assoc(glyph = "#", color = [0.6, 0.3, 0.2], walkable = false, opaque = true, name = "Brick Wall")]
  BrickWall,
  #[assoc(glyph = "#", color = [0.45, 0.3, 0.15], walkable = false, opaque = true, name = "Wooden Wall")]
  WoodWall,
  #[assoc(glyph = "#", color = [0.3, 0.28, 0.25], walkable = false, opaque = true, name = "Cave Wall", render_mode = TileRenderMode::Sprite("textures/space_qud/cobble tile.png", [0.28, 0.26, 0.24], [0.48, 0.46, 0.42]))]
  CaveWall,
  #[assoc(glyph = "#", color = [0.4, 0.35, 0.3], walkable = false, opaque = true, name = "Asteroid Rock", render_mode = TileRenderMode::Sprite("textures/space_qud/cobble tile.png", [0.28, 0.26, 0.24], [0.48, 0.46, 0.42]))]
  AsteroidRock,
  #[assoc(glyph = "#", color = [0.5, 0.55, 0.7], walkable = false, opaque = true, name = "Ice Wall")]
  IceWall,
  #[assoc(glyph = "#", color = [0.72, 0.76, 0.82], walkable = false, opaque = true, name = "Bright Cobble Wall", render_mode = TileRenderMode::Sprite("textures/space_qud/cobble tile.png", [0.68, 0.72, 0.78], [0.88, 0.92, 0.96]))]
  BrightCobbleWall,
  #[assoc(glyph = "+", color = [0.5, 0.35, 0.2], walkable = false, name = "Fence")]
  Fence,
  #[assoc(glyph = "%", color = [0.15, 0.45, 0.15], walkable = false, name = "Bush")]
  Bush,

  // --- Stairs & props ---
  #[assoc(glyph = "<", color = [0.9, 0.9, 0.2], walkable = true, name = "Stairs Up")]
  StairsUp,
  #[assoc(glyph = ">", color = [0.9, 0.9, 0.2], walkable = true, name = "Stairs Down")]
  StairsDown,
  #[assoc(glyph = "*", color = [0.5, 0.8, 0.95], walkable = false, name = "Crystal Formation", render_mode = TileRenderMode::Sprite("textures/space_qud/crystal.png", [0.28, 0.62, 0.82], [0.62, 0.88, 1.0]))]
  CrystalFormation,
  #[assoc(glyph = "*", color = [0.5, 0.8, 0.95], walkable = false, name = "Crystal Growth", render_mode = TileRenderMode::Sprite("textures/space_qud/crystal.png", [0.28, 0.62, 0.82], [0.62, 0.88, 1.0]))]
  CrystalGrowth,
}

impl Tile {
  pub fn all() -> impl Iterator<Item = Tile> {
    (0u16..).map_while(|i| Tile::try_from(i).ok())
  }

  pub fn from_save(s: &str) -> Option<Self> {
    Self::all().find(|t| format!("{t:?}") == s)
  }
}
