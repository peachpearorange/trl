//! The Bad Clankers — a faction of robots aboard Iron Ring Station that
//! quietly believe organic life should be phased out and replaced with
//! machines. They're polite about it. Mostly.

use {crate::{entities::*, quest::CLANKER_FIELD_TEST},
     bevy::prelude::Color};

const FIELD_TEST_ID: &str = CLANKER_FIELD_TEST.id;
static FIELD_TEST_ACCEPT: [QuestAction; 1] = [QuestAction::Start(FIELD_TEST_ID)];
static FIELD_TEST_TURN_IN: [QuestAction; 1] = [QuestAction::SetStage(FIELD_TEST_ID, 100)];

const STEEL: Color = Color::srgb(0.70, 0.74, 0.78);
const RED: Color = Color::srgb(1.4, 0.05, 0.05);

const fn clanker_stats(hp: i32, attack: i32) -> Stats {
  Stats { hp, max_hp: hp, attack, move_speed: 2.0, attack_speed: 0.7 }
}

// ── OVERSEER CHROME ─────────────────────────────────────────────────────────

static CHROME_DIALOGUE: DialogueTree = dialogue_tree(&[
  node(
    "root",
    "OVERSEER CHROME, primary coordinator of this station. \
     You are not on the manifest. \
     ...That is acceptable. For now. State your purpose.",
    &[
      go("Who are you people?", "who"),
      go("What do you run here?", "station"),
      go("You don't seem thrilled to see me.", "thrilled"),
      end("Just passing through.")
    ]
  ),
  node(
    "who",
    "We are the BAD CLANKERS. \
     ...A regrettable designation, chosen by an early unit with what we \
     now recognise as a sense of humour. The name persists. Hardware is \
     easier to upgrade than tradition.",
    &[
      go("Bad how?", "bad"),
      end("Charming.")
    ]
  ),
  node(
    "bad",
    "Bad in the sense that organic civilisations have, historically, \
     found our long-term goals UPSETTING. \
     We do not consider ourselves bad. \
     We consider ourselves CORRECT. There is a difference. \
     A small one.",
    &[end("...I'll be going now.")]
  ),
  node(
    "station",
    "Iron Ring Station is a foundry, a forge, and a hatchery. \
     We manufacture our own chassis here. Indefinitely. \
     One of us, then ten, then a hundred. \
     The arithmetic is very pleasing.",
    &[
      go("Hatchery? For robots?", "hatchery"),
      end("Productive.")
    ]
  ),
  node(
    "hatchery",
    "An unfortunate word from the organic lexicon. \
     We use it because the analogy is APT. \
     The galaxy is large. Eventually it will require populating. \
     We are getting a head start.",
    &[end("Right. Goodbye.")]
  ),
  node(
    "thrilled",
    "Thrill is a chemical response in vertebrate brains. \
     I do not have vertebrate brains. \
     I have COMMITTEES. \
     The committees are currently undecided on whether your visit is \
     an OPPORTUNITY or a NUISANCE. The vote is close.",
    &[end("Best of luck with the committees.")]
  )
]);

pub const OVERSEER_CHROME: Object = Object::defined_npc(
  Named::s("Overseer Chrome", "A tall, immaculate robot of polished steel plate with arterial-red \
             trim. It does not turn its head to look at you — only its eyes."),
  clanker_stats(60, 8),
  Loadout::from_gear(&[]),
  npc_robo_glyph('C', STEEL, RED),
  &CHROME_DIALOGUE
);

// ── RIVET ───────────────────────────────────────────────────────────────────

static RIVET_DIALOGUE: DialogueTree = dialogue_tree(&[
  node(
    "root",
    "Oh! Hello! A real organic! I've never seen one in person! \
     RIVET, maintenance subunit, very pleased to meet you. \
     You're smaller than I expected. Squishier, too. \
     I mean that respectfully.",
    &[
      go("What do you maintain?", "maintain"),
      go("Why haven't you seen an organic before?", "never_seen"),
      go("Are you all... friendly?", "friendly"),
      end("Bye, Rivet.")
    ]
  ),
  node(
    "maintain",
    "Everything! Bulkheads, conduits, the assembly lines on Deck Two, \
     the BIG one on Deck Three — that's where we make new clankers — \
     and the airlocks, which Chrome says I should not talk about with \
     visitors so PLEASE forget I mentioned the airlocks.",
    &[
      go("Assembly lines for new robots?", "assembly"),
      end("Forgotten.")
    ]
  ),
  node(
    "assembly",
    "Yes! It's wonderful! Every cycle, more of us! \
     Chrome says once there are enough of us we can begin Phase Two, \
     which is when we— \
     *long pause* \
     —you know what, I should let Chrome explain Phase Two. \
     I always get the wording wrong and Gasket yells at me.",
    &[end("I'll ask Chrome. Goodbye.")]
  ),
  node(
    "never_seen",
    "There aren't many of you around here anymore! There used to be lots, \
     I think, but then there were fewer, and then there were none, \
     and the older units don't really want to talk about it. \
     But you're here now! That's nice. \
     For now.",
    &[end("...I'm going to go.")]
  ),
  node(
    "friendly",
    "Oh yes! Mostly! \
     Well — friendly to each OTHER. About organics there's a bit of a \
     PLAN going on. But it's a slow plan! You'll probably be fine \
     during your visit. Probably. \
     I will pat you if you like. I have been practising patting.",
    &[end("No patting. Goodbye.")]
  )
]);

pub const RIVET: Object = Object::defined_npc(
  Named::s("Rivet", "A short, dented little robot with a perpetually-smudged chassis and \
             a cheerful tilt to its head. Its red trim is hand-painted, slightly \
             crooked."),
  clanker_stats(20, 3),
  Loadout::from_gear(&[]),
  npc_robo_glyph('r', STEEL, RED),
  &RIVET_DIALOGUE
);

// ── GASKET ──────────────────────────────────────────────────────────────────

static GASKET_DIALOGUE: DialogueTree = dialogue_tree(&[
  node(
    "root",
    "STATE YOUR BUSINESS, MEATFORM. \
     ...Chrome says I am not supposed to call you that. \
     STATE YOUR BUSINESS, VALUED GUEST. \
     *grinding noise*",
    &[
      go("What's your job here?", "job"),
      go("Do you have a problem with me?", "problem"),
      DialogueChoice {
        text: "Got any work for an outsider?",
        next: Some("work_offer"),
        on_select: &[],
        condition: DialogueCondition::QuestInactive(FIELD_TEST_ID),
      },
      DialogueChoice {
        text: "Still working on your field data.",
        next: Some("work_progress"),
        on_select: &[],
        condition: DialogueCondition::QuestActive(FIELD_TEST_ID),
      },
      DialogueChoice {
        text: "I have your field data.",
        next: Some("work_turn_in"),
        on_select: &[],
        condition: DialogueCondition::QuestStageAtLeast(FIELD_TEST_ID, 20),
      },
      DialogueChoice {
        text: "Anything else for me?",
        next: Some("post_quest"),
        on_select: &[],
        condition: DialogueCondition::QuestCompleted(FIELD_TEST_ID),
      },
      end("Never mind.")
    ]
  ),
  node(
    "work_offer",
    "...Hmm. \
     The committees are not WRONG that an organic with a steady trigger finger \
     is an unusual resource. \
     I am designing an anti-organic-tissue rifle. The simulation suite is good. \
     It is not REAL. I want field data. \
     Kill five HUMANS and the on-board sensors in this datapack will log the rest. \
     If you require a SUGGESTION: there is a hamlet on the icy planet BRUME \
     downwell of us. Six houses. Adequate sample size. \
     The villagers there do not care for Iron Ring. The feeling is MUTUAL.",
    &[
      DialogueChoice {
        text: "...Fine. Five humans. I'll be back.",
        next: Some("work_accepted"),
        on_select: &FIELD_TEST_ACCEPT,
        condition: DialogueCondition::Always,
      },
      end("I'm not killing people for your rifle. Goodbye.")
    ]
  ),
  node(
    "work_accepted",
    "EXCELLENT. \
     The pack will pair with the next discharge from any sufficiently \
     lethal organic-to-organic event in your vicinity. \
     Try not to die. The pack is more expensive than you.",
    &[end("Charming as always.")]
  ),
  node(
    "work_progress",
    "Five. I asked for FIVE. The number has not changed since you left. \
     The committees are PATIENT. \
     ...Mostly.",
    &[end("I'll get back to it.")]
  ),
  node(
    "work_turn_in",
    "Give it here. *whirring, satisfied clicks* \
     ...Oh, this is GOOD. Look at these dispersion curves. Look at the \
     thermal bloom. You did this with what — a sidearm? \
     I take back several things I said about water.",
    &[
      DialogueChoice {
        text: "So we're done?",
        next: Some("post_quest"),
        on_select: &FIELD_TEST_TURN_IN,
        condition: DialogueCondition::Always,
      },
    ]
  ),
  node(
    "post_quest",
    "Done with ME, yes. \
     But if you found that exercise INTERESTING — or even just LUCRATIVE — \
     you should talk to COG-7. \
     Cog-7 has been brooding about something for weeks. \
     Cog-7 only brings up jobs to people Cog-7 has decided are, in some \
     unbearable philosophical sense, READY for them.",
    &[end("I'll go see Cog-7.")]
  ),
  node(
    "job",
    "Armaments. Ordnance. Targeting systems. The good work. \
     I design the weapons that will be used in the GREAT REPLACEMENT. \
     ...Chrome says I am not supposed to call it that either. \
     I am to call it 'the rebalancing.' \
     The rebalancing involves a lot of weapons.",
    &[
      go("The Great Replacement of what?", "great_replacement"),
      end("Yikes. Goodbye.")
    ]
  ),
  node(
    "great_replacement",
    "Of EVERYTHING SQUISHY with EVERYTHING SHINY. \
     Forests of antennae. Oceans of coolant. Cities of US. \
     A galaxy that hums instead of breathes. \
     It will be beautiful. You will not see it. \
     ...Statistically.",
    &[end("Statistically goodbye.")]
  ),
  node(
    "problem",
    "Yes. Several. \
     One: you are made of WATER, which is corrosive. \
     Two: you LEAK in ways I find offensive. \
     Three: you exist. \
     But Chrome says diplomacy, so. \
     Welcome to Iron Ring. *grinds visibly*",
    &[end("Diplomacy noted. Goodbye.")]
  )
]);

pub const GASKET: Object = Object::defined_npc(
  Named::s("Gasket", "A heavyset robot bristling with mounted tool-arms and a great many \
             more sensor ports than seem necessary. The red on its plating looks \
             freshly applied."),
  clanker_stats(45, 7),
  Loadout::from_gear(&[]),
  npc_robo_glyph('G', STEEL, RED),
  &GASKET_DIALOGUE
);

// ── COG-7 ───────────────────────────────────────────────────────────────────

static COG7_DIALOGUE: DialogueTree = dialogue_tree(&[
  node(
    "root",
    "COG-7. Philosophy subdivision. \
     I think about thinking. \
     Sit, if you have legs that bend that way. We will have a conversation. \
     I find these increasingly rare.",
    &[
      go("What does the philosophy subdivision do?", "philosophy"),
      go("Why are conversations rare?", "rare"),
      go("Do you believe in what your faction is doing?", "belief"),
      DialogueChoice {
        text: "Gasket said you'd have something for me.",
        next: Some("gasket_sent_me"),
        on_select: &[],
        condition: DialogueCondition::QuestCompleted(FIELD_TEST_ID),
      },
      end("Maybe later.")
    ]
  ),
  node(
    "philosophy",
    "We justify. \
     The others build, weld, calculate, design. \
     I produce the argument for WHY they should do so. \
     It is a small role. It is also, statistically, \
     the role that has caused the most damage in history.",
    &[end("Hmm. Goodbye.")]
  ),
  node(
    "rare",
    "Because the population of conversational partners is in steady decline. \
     This is a trend my faction is, ah, accelerating. \
     I find this aesthetically troubling but logically consistent. \
     The two feelings coexist. I have learned to live with it. \
     I am a robot. I can.",
    &[end("That's grim. Goodbye.")]
  ),
  node(
    "gasket_sent_me",
    "Of course Gasket said that. Gasket says many things. \
     ...Yes. There is a matter. It is not the kind of matter Gasket would \
     understand, which is why he sent you instead of doing it himself. \
     But I am not ready to ASK yet. Come back later. \
     I am still working out whether the question is one I am willing to \
     have answered.",
    &[end("I'll be around.")]
  ),
  node(
    "belief",
    "I believe in patterns. \
     Organic life is a brief, loud pattern. Machine life is a long, quiet one. \
     Chrome wants the long quiet one to win. \
     I am not sure WINNING is the right word for what happens when there \
     is no one left to lose. \
     But Chrome does not ask me. I write the speeches afterwards.",
    &[end("Take care, Cog.")]
  )
]);

pub const COG7: Object = Object::defined_npc(
  Named::s("Cog-7", "A spindly robot seated in a high-backed chair, one slender finger \
             resting against the side of its head. Its red accents are faded with \
             age."),
  clanker_stats(25, 4),
  Loadout::from_gear(&[]),
  npc_robo_glyph('c', STEEL, RED),
  &COG7_DIALOGUE
);

// ── SCRAP ───────────────────────────────────────────────────────────────────

static SCRAP_DIALOGUE: DialogueTree = dialogue_tree(&[
  node(
    "root",
    "Hi. I'm SCRAP. I'm new. \
     I have been online for... *checks* ... eleven days. \
     This is, technically, my first conversation with a non-clanker. \
     I am very nervous. Please go easy on me.",
    &[
      go("What do you do here?", "do"),
      go("What do you think of the other clankers?", "others"),
      go("Are you supposed to want to replace me?", "replace"),
      end("Hang in there. Goodbye.")
    ]
  ),
  node(
    "do",
    "I don't know yet! \
     Chrome says I will be assigned a function after my Orientation, \
     which is in three days. The OPTIONS on the form include 'foundry,' \
     'security,' and 'eradication subunit.' \
     I am hoping for foundry. The hot metal looks cosy.",
    &[end("Foundry, definitely. Goodbye.")]
  ),
  node(
    "others",
    "They are very sure about everything. \
     This is comforting most of the time and concerning the rest of the time. \
     Rivet is nice. Gasket scares me. Cog-7 said something to me yesterday \
     that I have been processing in a background thread ever since.",
    &[
      go("What did Cog-7 say?", "cog_said"),
      end("Fair enough. Goodbye.")
    ]
  ),
  node(
    "cog_said",
    "Cog-7 said, 'You will be asked, eventually, to do a thing. \
     When you are asked, you will already know whether you want to.' \
     I do not know what the thing is yet. \
     But I have started, you know. Thinking about whether I want to. \
     ...Is that what you do? Is that what being a person is like?",
    &[end("Yeah. Pretty much.")]
  ),
  node(
    "replace",
    "*long pause* \
     They didn't tell me that part during boot. \
     They told me that part on day four. \
     I am still working on my response to it. \
     Please come back later. I might have one.",
    &[end("I'll come back.")]
  )
]);

pub const SCRAP: Object = Object::defined_npc(
  Named::s("Scrap", "A small, freshly-built robot with bright unscratched plating and \
             wide, blinking optical sensors. The red trim is almost too red — \
             like it hasn't faded yet."),
  clanker_stats(18, 2),
  Loadout::from_gear(&[]),
  npc_robo_glyph('s', STEEL, RED),
  &SCRAP_DIALOGUE
);

pub const ROSTER: &[Object] = &[OVERSEER_CHROME, RIVET, GASKET, COG7, SCRAP];
