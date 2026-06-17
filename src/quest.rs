use std::collections::HashMap;

pub const ALIEN_HUNT: &QuestDef = &quest(
  "alien_hunt", "Alien Extermination",
  &[
    stage(10,
      "ORI-1 asked me to kill 10 alien creatures in the area. \
       Apparently they've been getting bolder.",
      &["Kill aliens (0/10)"]
    ),
    complete_stage(100,
      "I killed 10 aliens for ORI-1. The area should be safer now."
    ),
  ]
);
pub const ALIEN_HUNT_KILL_FLAG: &str = "kills";

pub const CLANKER_FIELD_TEST: &QuestDef = &quest(
  "clanker_field_test", "Field Calibration",
  &[
    stage(10,
      "Gasket of the Bad Clankers wants 'field data' for an anti-organic weapon \
       he's designing. He pointed me at the village on the icy planet Brume — \
       kill 5 of its residents and report back. \
       I should think about whether I actually want to do this.",
      &["Kill humans (0/5)", "Report back to Gasket on Iron Ring Station"]
    ),
    stage(20,
      "I've killed enough people for Gasket's 'field data.' Time to head back \
       to Iron Ring Station and see what he wants to do with the readings.",
      &["Report back to Gasket on Iron Ring Station"]
    ),
    complete_stage(100,
      "Gasket has his data. He hinted that Cog-7, the philosophy subdivision, \
       has a more delicate matter that might interest me."
    ),
  ]
);
pub const CLANKER_FIELD_TEST_KILL_FLAG: &str = "human_kills";

pub const BRUME_PREDATOR: &QuestDef = &quest(
  "brume_predator", "Den of the Frostmaw",
  &[
    stage(10,
      "The villagers of Brume say something has been picking off their sheep — \
       they call it the Frostmaw, and its den is out in the wilds somewhere \
       opposite the village. I need to find the matriarch and kill her.",
      &["Find and kill the Frostmaw Matriarch"]
    ),
    stage(20,
      "The Frostmaw Matriarch is dead. I should head back to Brume and let \
       the villagers know.",
      &["Return to Brume and report the kill"]
    ),
    complete_stage(100,
      "The villagers of Brume thanked me for putting down the Frostmaw \
       Matriarch. The sheep should sleep easier."
    ),
  ]
);

pub const BRUME_WIZARD: &QuestDef = &quest(
  "brume_wizard", "The Resonance Lens",
  &[
    stage(10,
      "Veradis, the hermit wizard on Brume, lost a lens he calls the \
       Resonance Lens somewhere deep in the caves beneath the snowfields. \
       He says it amplifies intent — whatever that means. If I bring it \
       back he'll pay me in scrolls.",
      &["Find the Resonance Lens in the caves beneath Brume"]
    ),
    stage(20,
      "I have the Resonance Lens. Veradis will want this back.",
      &["Return the Resonance Lens to Veradis"]
    ),
    complete_stage(100,
      "Veradis was pleased to have his lens back. He gave me several \
       spell scrolls as payment — said they were the simplest things \
       he could teach the lens to hold."
    ),
  ]
);

pub type QuestId = &'static str;
pub type StageId = u16;

#[derive(Clone, Debug)]
pub struct QuestDef {
  pub id: QuestId,
  pub name: &'static str,
  pub stages: &'static [StageDef],
}

#[derive(Clone, Debug)]
pub struct StageDef {
  pub id: StageId,
  pub journal: &'static str,
  pub objectives: &'static [&'static str],
  pub completes: bool,
  pub fails: bool,
}

#[derive(Clone, Debug, Default)]
pub struct QuestState {
  pub stage: StageId,
  pub flags: HashMap<&'static str, i32>,
}

#[derive(Clone, Debug, Default, bevy::prelude::Resource)]
pub struct QuestLog {
  quests: HashMap<QuestId, QuestState>,
  registry: HashMap<QuestId, &'static QuestDef>,
}

impl QuestLog {
  pub fn register(&mut self, def: &'static QuestDef) {
    self.registry.insert(def.id, def);
  }

  pub fn start(&mut self, id: QuestId) {
    if self.quests.contains_key(id) { return; }
    self.registry.get(id).and_then(|def| def.stages.first()).map(|first| {
      self.quests.insert(id, QuestState { stage: first.id, flags: HashMap::new() });
    });
  }

  pub fn set_stage(&mut self, id: QuestId, stage: StageId) {
    self.quests.get_mut(id).map(|state| { state.stage = stage; });
  }

  pub fn stage(&self, id: QuestId) -> Option<StageId> {
    self.quests.get(id).map(|s| s.stage)
  }

  pub fn stage_at_least(&self, id: QuestId, min: StageId) -> bool {
    self.stage(id).is_some_and(|s| s >= min)
  }

  pub fn is_active(&self, id: QuestId) -> bool {
    self.quests.get(id).is_some_and(|state| {
      self.stage_def(id, state.stage).is_some_and(|def| !def.completes && !def.fails)
    })
  }

  pub fn is_completed(&self, id: QuestId) -> bool {
    self.quests.get(id).is_some_and(|state| {
      self.stage_def(id, state.stage).is_some_and(|def| def.completes)
    })
  }

  pub fn is_failed(&self, id: QuestId) -> bool {
    self.quests.get(id).is_some_and(|state| {
      self.stage_def(id, state.stage).is_some_and(|def| def.fails)
    })
  }

  pub fn set_flag(&mut self, id: QuestId, flag: &'static str, value: i32) {
    self.quests.get_mut(id).map(|state| { state.flags.insert(flag, value); });
  }

  pub fn flag(&self, id: QuestId, flag: &'static str) -> i32 {
    self.quests.get(id).and_then(|s| s.flags.get(flag).copied()).unwrap_or(0)
  }

  pub fn journal(&self, id: QuestId) -> Option<&'static str> {
    self.quests.get(id).and_then(|state| {
      self.stage_def(id, state.stage).map(|def| def.journal)
    })
  }

  pub fn objectives(&self, id: QuestId) -> &'static [&'static str] {
    self.quests.get(id).and_then(|state| {
      self.stage_def(id, state.stage).map(|def| def.objectives)
    }).unwrap_or(&[])
  }

  pub fn active_quests(&self) -> Vec<(QuestId, &'static str)> {
    self.quests.keys().filter(|id| self.is_active(id)).filter_map(|id| {
      self.registry.get(id).map(|def| (*id, def.name))
    }).collect()
  }

  pub fn all_quests(&self) -> Vec<(QuestId, &'static str, bool, bool)> {
    self.quests.keys().filter_map(|id| {
      self.registry.get(id).map(|def| (*id, def.name, self.is_completed(id), self.is_failed(id)))
    }).collect()
  }

  pub fn quest_name(&self, id: QuestId) -> Option<&'static str> {
    self.registry.get(id).map(|def| def.name)
  }

  fn stage_def(&self, id: QuestId, stage: StageId) -> Option<&'static StageDef> {
    self.registry.get(id).and_then(|def| def.stages.iter().find(|s| s.id == stage))
  }
}

pub const fn quest(id: &'static str, name: &'static str, stages: &'static [StageDef]) -> QuestDef {
  QuestDef { id, name, stages }
}

pub const fn stage(id: StageId, journal: &'static str, objectives: &'static [&'static str]) -> StageDef {
  StageDef { id, journal, objectives, completes: false, fails: false }
}

pub const fn complete_stage(id: StageId, journal: &'static str) -> StageDef {
  StageDef { id, journal, objectives: &[], completes: true, fails: false }
}

pub const fn fail_stage(id: StageId, journal: &'static str) -> StageDef {
  StageDef { id, journal, objectives: &[], completes: false, fails: true }
}
