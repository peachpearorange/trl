use crate::entities::Object;
use crate::level::{Level, Tile};

type ObjectFactory = fn() -> Object;

struct CharAssoc {
    ch: char,
    tile: Tile,
    factories: Vec<ObjectFactory>,
}

pub struct PrefabArea {
    assocs: Vec<CharAssoc>,
    default_tile: Tile,
}

impl PrefabArea {
    pub fn new() -> Self {
        PrefabArea {
            assocs: Vec::new(),
            default_tile: Tile::Vacuum,
        }
    }

    pub fn assoc(mut self, ch: char, tile: Tile, factories: &[ObjectFactory]) -> Self {
        self.assocs.push(CharAssoc {
            ch,
            tile,
            factories: factories.to_vec(),
        });
        self
    }

    pub fn build(&self, layout: &str) -> (Level, Vec<(Object, i32, i32)>) {
        let lines: Vec<&str> = layout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect();
        let height = lines.len();
        let width = lines.iter().map(|l| l.len()).max().unwrap_or(0);

        let mut level = Level::new(width, height, self.default_tile);
        let mut spawns: Vec<(Object, i32, i32)> = Vec::new();

        for (y, line) in lines.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                if let Some(assoc) = self.assocs.iter().find(|a| a.ch == ch) {
                    level.set(x as i32, y as i32, assoc.tile);
                    for factory in &assoc.factories {
                        spawns.push((factory(), x as i32, y as i32));
                    }
                }
            }
        }

        (level, spawns)
    }
}
