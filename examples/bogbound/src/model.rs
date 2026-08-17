use serde::{Deserialize, Serialize};

pub const WORLD_SIZE: f32 = 1600.0;
pub const PLAYER_RADIUS: f32 = 13.0;
pub const ENEMY_CAP: usize = 180;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Skin {
    #[default]
    Sun,
    Tide,
    Storm,
    Grove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Element {
    Ember,
    Frost,
    Storm,
    #[default]
    Thorn,
}

impl Element {
    pub const ALL: [Self; 4] = [Self::Ember, Self::Frost, Self::Storm, Self::Thorn];
    pub fn label(self) -> &'static str {
        match self {
            Self::Ember => "Ember",
            Self::Frost => "Frost",
            Self::Storm => "Storm",
            Self::Thorn => "Thorn",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Form {
    #[default]
    Bolt,
    Orbit,
    Swarm,
    Nova,
}

impl Form {
    pub const ALL: [Self; 4] = [Self::Bolt, Self::Orbit, Self::Swarm, Self::Nova];
    pub fn label(self) -> &'static str {
        match self {
            Self::Bolt => "Bolt",
            Self::Orbit => "Orbit",
            Self::Swarm => "Swarm",
            Self::Nova => "Nova",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnemyKind {
    Mireling,
    Midge,
    Spitter,
    Brute,
    Colossus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharmKind {
    Damage,
    Speed,
    Magnet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpgradeKind {
    Damage,
    Cooldown,
    Speed,
    MaxHealth,
    Magnet,
    Revive,
    PartyDamage,
    PartySpeed,
    PartyMagnet,
}

impl UpgradeKind {
    pub fn title(self) -> &'static str {
        match self {
            Self::Damage => "Heavy Pollen",
            Self::Cooldown => "Quick Current",
            Self::Speed => "Reef Shoes",
            Self::MaxHealth => "Heartfruit",
            Self::Magnet => "Shell Magnet",
            Self::Revive => "Rescue Charm",
            Self::PartyDamage => "Campfire Charm",
            Self::PartySpeed => "Trade Wind",
            Self::PartyMagnet => "Tide Bell",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Damage => "+15% damage",
            Self::Cooldown => "10% faster attacks",
            Self::Speed => "+8% movement speed",
            Self::MaxHealth => "+20 max health and heal",
            Self::Magnet => "+25% pickup radius",
            Self::Revive => "+25% revive speed",
            Self::PartyDamage => "+3% damage for everyone",
            Self::PartySpeed => "+3% speed for everyone",
            Self::PartyMagnet => "+3% pickup radius for everyone",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn distance(self, other: Self) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
    pub fn normalized(self) -> Self {
        let len = (self.x * self.x + self.y * self.y).sqrt();
        if len > 0.001 {
            Self::new(self.x / len, self.y / len)
        } else {
            Self::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerView {
    pub id: String,
    pub name: String,
    pub skin: Skin,
    pub pos: Vec2,
    pub hp: f32,
    pub max_hp: f32,
    pub level: u32,
    pub xp: u32,
    pub xp_next: u32,
    pub downed: bool,
    pub connected: bool,
    pub element: Element,
    pub form: Form,
    pub kills: u32,
    pub damage: u64,
    pub revives: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyView {
    pub id: u64,
    pub kind: EnemyKind,
    pub pos: Vec2,
    pub hp: f32,
    pub max_hp: f32,
    pub boss: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectileView {
    pub id: u64,
    pub pos: Vec2,
    pub element: Element,
    pub hostile: bool,
    pub size: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickupView {
    pub id: u64,
    pub pos: Vec2,
    pub kind: String,
    pub owner_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegraphView {
    pub id: u64,
    pub pos: Vec2,
    pub radius: f32,
    pub remaining_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeChoice {
    pub id: u64,
    pub kind: UpgradeKind,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeDraft {
    pub id: u64,
    pub choices: Vec<UpgradeChoice>,
    pub expires_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuneDraft {
    pub id: u64,
    pub incantation: String,
    pub element: Element,
    pub form: Form,
    pub spell_name: String,
    pub description: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    #[default]
    Lobby,
    Running,
    Victory,
    Defeat,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BogMetrics {
    pub fold_commits: u64,
    pub materialized_players: i64,
    pub materialized_enemies: i64,
    pub hnsw_queries: u64,
    pub ese_compiles: u64,
    pub enemy_counts: Vec<(EnemyKind, i64)>,
    pub charms: Vec<(CharmKind, i64)>,
    pub top_damage: Vec<(String, u64)>,
    pub party_hp_milli: i64,
    pub party_max_hp_milli: i64,
    pub downed_players: i64,
    pub total_damage: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DirectorView {
    pub mode: String,
    pub pressure: u8,
    pub spawn_interval_ms: u64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSnapshot {
    pub tick: u64,
    pub phase: Phase,
    pub run_id: u64,
    pub elapsed_ms: u64,
    pub remaining_ms: u64,
    pub wave: u8,
    pub wave_label: String,
    pub join_url: String,
    pub connected_players: usize,
    pub you: Option<String>,
    pub players: Vec<PlayerView>,
    pub enemies: Vec<EnemyView>,
    pub projectiles: Vec<ProjectileView>,
    pub pickups: Vec<PickupView>,
    pub telegraphs: Vec<TelegraphView>,
    pub boss_hp: Option<(f32, f32)>,
    pub upgrade_draft: Option<UpgradeDraft>,
    pub rune_draft: Option<RuneDraft>,
    pub announcement: Option<String>,
    pub metrics: BogMetrics,
    pub director: DirectorView,
    #[serde(skip)]
    pub private_upgrades: Vec<(String, UpgradeDraft)>,
    #[serde(skip)]
    pub private_runes: Vec<(String, RuneDraft)>,
}

impl GameSnapshot {
    pub fn lobby(join_url: String) -> Self {
        Self {
            tick: 0,
            phase: Phase::Lobby,
            run_id: 0,
            elapsed_ms: 0,
            remaining_ms: 300_000,
            wave: 0,
            wave_label: "Gather at the shore".into(),
            join_url,
            connected_players: 0,
            you: None,
            players: vec![],
            enemies: vec![],
            projectiles: vec![],
            pickups: vec![],
            telegraphs: vec![],
            boss_hp: None,
            upgrade_draft: None,
            rune_draft: None,
            announcement: Some("Choose a traveler and enter the bog".into()),
            metrics: BogMetrics::default(),
            director: DirectorView::default(),
            private_upgrades: vec![],
            private_runes: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FactKey {
    Player(String),
    Enemy(u64),
    Loadout(String),
    Damage(String),
    Charm(String, CharmKind),
    Run,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Fact {
    Player {
        id: String,
        hp_milli: i64,
        max_hp_milli: i64,
        level: u32,
        downed: bool,
    },
    Enemy {
        id: u64,
        kind: EnemyKind,
        hp_milli: i64,
    },
    Loadout {
        player_id: String,
        element: Element,
        form: Form,
        description: String,
    },
    Damage {
        player_id: String,
        total: u64,
    },
    Charm {
        player_id: String,
        kind: CharmKind,
        stacks: i64,
    },
    Run {
        phase: Phase,
        wave: u8,
    },
}
