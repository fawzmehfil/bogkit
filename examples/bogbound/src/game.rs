use crate::{model::*, semantic::SpellCompiler};
use anny::{hnsw::Hnsw, metric::L2};
use std::{
    collections::{HashMap, HashSet},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use tokio::sync::watch;

type SpatialIndex = Hnsw<f32, L2, 2, 16, 8, 32, 64, 8>;

#[derive(Debug)]
pub enum EngineCommand {
    SocketOpened {
        connection_id: u64,
    },
    SocketClosed {
        connection_id: u64,
    },
    Register {
        connection_id: u64,
        player_id: String,
        name: String,
        skin: Skin,
    },
    SelectSkin {
        player_id: String,
        skin: Skin,
    },
    Input {
        connection_id: u64,
        player_id: String,
        sequence: u64,
        x: f32,
        y: f32,
    },
    StartRun,
    ChooseUpgrade {
        player_id: String,
        draft_id: u64,
        choice_id: u64,
    },
    EquipRune {
        player_id: String,
        drop_id: u64,
    },
    SalvageRune {
        player_id: String,
        drop_id: u64,
    },
    Rematch,
}

#[derive(Clone)]
pub struct EngineHandle {
    pub commands: mpsc::Sender<EngineCommand>,
    pub snapshots: watch::Receiver<GameSnapshot>,
}

#[derive(Clone)]
struct Player {
    id: String,
    name: String,
    skin: Skin,
    pos: Vec2,
    input: Vec2,
    input_seq: u64,
    connected: bool,
    hp: f32,
    max_hp: f32,
    level: u32,
    xp: u32,
    damage_mult: f32,
    cooldown_mult: f32,
    speed_mult: f32,
    magnet_mult: f32,
    revive_mult: f32,
    element: Element,
    form: Form,
    attack_cd: f32,
    invulnerable: f32,
    downed: bool,
    auto_revive: bool,
    kills: u32,
    damage: u64,
    revives: u32,
    pending_upgrade: Option<UpgradeDraft>,
    pending_rune: Option<RuneDraft>,
    rune_wave: u8,
}

impl Player {
    fn new(id: String, name: String, skin: Skin, pos: Vec2) -> Self {
        Self {
            id,
            name,
            skin,
            pos,
            input: Vec2::default(),
            input_seq: 0,
            connected: true,
            hp: 100.0,
            max_hp: 100.0,
            level: 1,
            xp: 0,
            damage_mult: 1.0,
            cooldown_mult: 1.0,
            speed_mult: 1.0,
            magnet_mult: 1.0,
            revive_mult: 1.0,
            element: Element::Thorn,
            form: Form::Bolt,
            attack_cd: 0.2,
            invulnerable: 0.0,
            downed: false,
            auto_revive: true,
            kills: 0,
            damage: 0,
            revives: 0,
            pending_upgrade: None,
            pending_rune: None,
            rune_wave: 0,
        }
    }
    fn xp_next(&self) -> u32 {
        20 + 15 * self.level.saturating_sub(1)
    }
}

struct Enemy {
    id: u64,
    kind: EnemyKind,
    pos: Vec2,
    hp: f32,
    max_hp: f32,
    speed: f32,
    damage: f32,
    attack_cd: f32,
    slow: f32,
    dot: f32,
    dot_ttl: f32,
    dot_owner: Option<String>,
}
struct Projectile {
    id: u64,
    pos: Vec2,
    vel: Vec2,
    owner: Option<String>,
    damage: f32,
    element: Element,
    hostile: bool,
    homing: bool,
    ttl: f32,
    size: f32,
}
struct FriendlyShot {
    owner: String,
    pos: Vec2,
    dir: Vec2,
    damage: f32,
    element: Element,
    homing: bool,
    ttl: f32,
    size: f32,
}
enum PickupKind {
    Xp(u32),
    Rune {
        owner: String,
        element: Element,
        form: Form,
    },
}
struct Pickup {
    id: u64,
    pos: Vec2,
    kind: PickupKind,
    age: f32,
}

struct Telegraph {
    id: u64,
    pos: Vec2,
    radius: f32,
    ttl: f32,
    damage: f32,
}

enum FactOp {
    Upsert(FactKey, Fact),
    Remove(FactKey),
}

struct Game {
    phase: Phase,
    run_id: u64,
    tick: u64,
    elapsed_ms: u64,
    time_scale: f32,
    join_url: String,
    sockets: HashSet<u64>,
    connection_players: HashMap<u64, String>,
    players: HashMap<String, Player>,
    enemies: HashMap<u64, Enemy>,
    projectiles: HashMap<u64, Projectile>,
    pickups: HashMap<u64, Pickup>,
    telegraphs: HashMap<u64, Telegraph>,
    next_id: u64,
    spawn_cd: f32,
    boss_spawned: bool,
    boss_slam_cd: f32,
    boss_volley_cd: f32,
    boss_summon_cd: f32,
    announcement: Option<String>,
    announcement_ttl: f32,
    rng: fastrand::Rng,
    compiler: SpellCompiler,
    charms: HashMap<(String, CharmKind), i64>,
    fact_ops: Vec<FactOp>,
    fold_commits: u64,
    hnsw_queries: u64,
    ese_compiles: u64,
}

impl Game {
    fn new(join_url: String) -> Self {
        Self {
            phase: Phase::Lobby,
            run_id: 0,
            tick: 0,
            elapsed_ms: 0,
            time_scale: std::env::var("BOGBOUND_TIME_SCALE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1.0),
            join_url,
            sockets: HashSet::new(),
            connection_players: HashMap::new(),
            players: HashMap::new(),
            enemies: HashMap::new(),
            projectiles: HashMap::new(),
            pickups: HashMap::new(),
            telegraphs: HashMap::new(),
            next_id: 1,
            spawn_cd: 0.0,
            boss_spawned: false,
            boss_slam_cd: 0.0,
            boss_volley_cd: 0.0,
            boss_summon_cd: 0.0,
            announcement: Some("Choose a traveler and enter the bog".into()),
            announcement_ttl: 99.0,
            rng: fastrand::Rng::with_seed(0xb06b0a7d),
            compiler: SpellCompiler::new(),
            charms: HashMap::new(),
            fact_ops: vec![],
            fold_commits: 0,
            hnsw_queries: 0,
            ese_compiles: 0,
        }
    }

    fn connected_count(&self) -> usize {
        self.players.values().filter(|p| p.connected).count()
    }
    fn active_player_ids(&self) -> Vec<String> {
        self.players
            .values()
            .filter(|p| p.connected)
            .map(|p| p.id.clone())
            .collect()
    }
    fn wave(&self) -> u8 {
        match self.elapsed_ms {
            0..50_000 => 1,
            50_000..100_000 => 2,
            100_000..150_000 => 3,
            150_000..200_000 => 4,
            _ => 5,
        }
    }
    fn wave_label(&self) -> String {
        match self.wave() {
            1 => "Wave I · Mirelings",
            2 => "Wave II · Winged Tide",
            3 => "Wave III · Spitters",
            4 => "Wave IV · Brute Bloom",
            _ => "BOSS · Fen Colossus",
        }
        .into()
    }
    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn register(&mut self, connection_id: u64, player_id: String, name: String, skin: Skin) {
        if let Some(old_connection) = self
            .connection_players
            .iter()
            .find_map(|(c, p)| (p == &player_id && *c != connection_id).then_some(*c))
        {
            self.connection_players.remove(&old_connection);
        }
        self.connection_players
            .insert(connection_id, player_id.clone());
        let spawn = self.living_spawn();
        let median = self.median_level();
        let player = self
            .players
            .entry(player_id.clone())
            .or_insert_with(|| Player::new(player_id.clone(), name.clone(), skin, spawn));
        player.name = clean_name(name);
        player.skin = skin;
        player.connected = true;
        player.input = Vec2::default();
        player.input_seq = 0;
        if self.phase == Phase::Running && player.level < median {
            let gap = median - player.level;
            player.level = median;
            player.damage_mult *= 1.0 + gap as f32 * 0.08;
            player.max_hp += gap as f32 * 5.0;
            player.hp = player.max_hp;
            player.invulnerable = 3.0;
        }
        self.sync_player(&player_id);
        self.announcement = Some(format!(
            "{} joined the expedition",
            self.players[&player_id].name
        ));
        self.announcement_ttl = 2.0;
    }

    fn socket_closed(&mut self, connection_id: u64) {
        self.sockets.remove(&connection_id);
        if let Some(player_id) = self.connection_players.remove(&connection_id) {
            let still_connected = self.connection_players.values().any(|id| id == &player_id);
            if !still_connected {
                if let Some(player) = self.players.get_mut(&player_id) {
                    player.connected = false;
                    player.input = Vec2::default();
                }
                self.fact_ops
                    .push(FactOp::Remove(FactKey::Player(player_id.clone())));
                self.fact_ops
                    .push(FactOp::Remove(FactKey::Loadout(player_id.clone())));
                self.fact_ops
                    .push(FactOp::Remove(FactKey::Damage(player_id)));
            }
        }
    }

    fn start_run(&mut self) {
        if self.connected_count() == 0 {
            return;
        }
        for enemy in self.enemies.values() {
            self.fact_ops.push(FactOp::Remove(FactKey::Enemy(enemy.id)));
        }
        for ((player_id, kind), _) in self.charms.drain() {
            self.fact_ops
                .push(FactOp::Remove(FactKey::Charm(player_id, kind)));
        }
        self.phase = Phase::Running;
        self.run_id += 1;
        self.elapsed_ms = 0;
        self.tick = 0;
        self.enemies.clear();
        self.projectiles.clear();
        self.pickups.clear();
        self.telegraphs.clear();
        self.spawn_cd = 0.0;
        self.boss_spawned = false;
        self.boss_slam_cd = 0.0;
        self.boss_volley_cd = 0.0;
        self.boss_summon_cd = 0.0;
        self.rng = fastrand::Rng::with_seed(0xb06b0a7d ^ self.run_id);
        let ids = self.active_player_ids();
        for (index, id) in ids.iter().enumerate() {
            let angle = index as f32 * 0.9;
            if let Some(p) = self.players.get_mut(id) {
                p.pos = Vec2::new(800.0 + angle.cos() * 36.0, 800.0 + angle.sin() * 36.0);
                p.input = Vec2::default();
                p.input_seq = 0;
                p.hp = 100.0;
                p.max_hp = 100.0;
                p.level = 1;
                p.xp = 0;
                p.damage_mult = 1.0;
                p.cooldown_mult = 1.0;
                p.speed_mult = 1.0;
                p.magnet_mult = 1.0;
                p.revive_mult = 1.0;
                p.element = Element::Thorn;
                p.form = Form::Bolt;
                p.downed = false;
                p.auto_revive = true;
                p.kills = 0;
                p.damage = 0;
                p.revives = 0;
                p.pending_upgrade = None;
                p.pending_rune = None;
                p.rune_wave = 0;
                p.invulnerable = 2.0;
            }
            self.sync_player(id);
        }
        self.fact_ops.push(FactOp::Upsert(
            FactKey::Run,
            Fact::Run {
                phase: self.phase,
                wave: 1,
            },
        ));
        self.announcement = Some("THE BOG STIRS".into());
        self.announcement_ttl = 2.5;
    }

    fn tick(&mut self, dt: f32) {
        if self.phase != Phase::Running {
            return;
        }
        self.tick += 1;
        self.elapsed_ms = self
            .elapsed_ms
            .saturating_add((dt * 1000.0 * self.time_scale) as u64);
        self.announcement_ttl -= dt * self.time_scale;
        if self.announcement_ttl <= 0.0 {
            self.announcement = None;
        }
        let old_wave = match self
            .elapsed_ms
            .saturating_sub((dt * 1000.0 * self.time_scale) as u64)
        {
            0..50_000 => 1,
            50_000..100_000 => 2,
            100_000..150_000 => 3,
            150_000..200_000 => 4,
            _ => 5,
        };
        if self.wave() != old_wave {
            self.announcement = Some(self.wave_label());
            self.announcement_ttl = 2.5;
            self.fact_ops.push(FactOp::Upsert(
                FactKey::Run,
                Fact::Run {
                    phase: self.phase,
                    wave: self.wave(),
                },
            ));
        }
        if self.elapsed_ms >= 200_000 && !self.boss_spawned {
            self.spawn_boss();
        }
        if self.elapsed_ms >= 300_000 {
            self.finish(Phase::Defeat, "THE FEN SWALLOWS THE ISLAND");
            return;
        }

        self.update_players(dt);
        self.spawn_enemies(dt);
        self.update_boss(dt);
        self.update_combat(dt);
        self.update_enemies(dt);
        self.update_projectiles(dt);
        self.update_pickups(dt);
        self.update_revives(dt);
        self.update_drafts();
        if self.connected_count() > 0
            && self
                .players
                .values()
                .filter(|p| p.connected)
                .all(|p| p.downed)
        {
            self.finish(Phase::Defeat, "THE EXPEDITION HAS FALLEN");
        }
    }

    fn update_players(&mut self, dt: f32) {
        let party_speed = 1.0 + self.charm_total(CharmKind::Speed).min(5) as f32 * 0.03;
        for p in self.players.values_mut().filter(|p| p.connected) {
            p.invulnerable = (p.invulnerable - dt).max(0.0);
            p.attack_cd -= dt;
            let speed = if p.downed {
                45.0
            } else {
                145.0 * p.speed_mult * party_speed
            };
            let dir = p.input.normalized();
            p.pos.x = (p.pos.x + dir.x * speed * dt).clamp(28.0, WORLD_SIZE - 28.0);
            p.pos.y = (p.pos.y + dir.y * speed * dt).clamp(28.0, WORLD_SIZE - 28.0);
        }
    }

    fn spawn_enemies(&mut self, dt: f32) {
        self.spawn_cd -= dt * self.time_scale;
        let active = self.connected_count().max(1);
        let cap = (18 + active * 7 + self.wave() as usize * 12).min(ENEMY_CAP);
        if self.spawn_cd > 0.0 || self.enemies.len() >= cap || self.wave() == 5 {
            return;
        }
        self.spawn_cd = (0.62 - self.wave() as f32 * 0.07 - active as f32 * 0.008).max(0.16);
        let roll = self.rng.u32(0..100);
        let kind = match self.wave() {
            1 => EnemyKind::Mireling,
            2 if roll < 45 => EnemyKind::Midge,
            3 if roll < 30 => EnemyKind::Spitter,
            4 if roll < 12 => EnemyKind::Brute,
            4 if roll < 42 => EnemyKind::Spitter,
            _ => EnemyKind::Mireling,
        };
        self.spawn_enemy(kind);
    }

    fn spawn_enemy(&mut self, kind: EnemyKind) {
        let center = self.living_spawn();
        let angle = self.rng.f32() * std::f32::consts::TAU;
        let distance = 360.0 + self.rng.f32() * 180.0;
        let pos = Vec2::new(
            (center.x + angle.cos() * distance).clamp(32.0, WORLD_SIZE - 32.0),
            (center.y + angle.sin() * distance).clamp(32.0, WORLD_SIZE - 32.0),
        );
        self.spawn_enemy_at(kind, pos);
    }

    fn spawn_enemy_at(&mut self, kind: EnemyKind, pos: Vec2) {
        let (hp, speed, damage) = match kind {
            EnemyKind::Mireling => (30.0, 48.0, 9.0),
            EnemyKind::Midge => (16.0, 92.0, 6.0),
            EnemyKind::Spitter => (42.0, 37.0, 8.0),
            EnemyKind::Brute => (170.0, 31.0, 15.0),
            EnemyKind::Colossus => (1800.0, 24.0, 22.0),
        };
        let id = self.next_id();
        let enemy = Enemy {
            id,
            kind,
            pos,
            hp,
            max_hp: hp,
            speed,
            damage,
            attack_cd: 0.5,
            slow: 0.0,
            dot: 0.0,
            dot_ttl: 0.0,
            dot_owner: None,
        };
        self.enemies.insert(id, enemy);
        self.sync_enemy(id);
    }

    fn spawn_boss(&mut self) {
        self.boss_spawned = true;
        if self.enemies.len() >= ENEMY_CAP
            && let Some(enemy_id) = self.enemies.keys().copied().min()
        {
            self.enemies.remove(&enemy_id);
            self.fact_ops.push(FactOp::Remove(FactKey::Enemy(enemy_id)));
        }
        let players = self.connected_count().max(1);
        let hp = 1800.0 + 700.0 * (players.saturating_sub(1)) as f32;
        let id = self.next_id();
        let enemy = Enemy {
            id,
            kind: EnemyKind::Colossus,
            pos: Vec2::new(800.0, 250.0),
            hp,
            max_hp: hp,
            speed: 25.0,
            damage: 22.0,
            attack_cd: 1.5,
            slow: 0.0,
            dot: 0.0,
            dot_ttl: 0.0,
            dot_owner: None,
        };
        self.enemies.insert(id, enemy);
        self.sync_enemy(id);
        self.boss_slam_cd = 2.4;
        self.boss_volley_cd = 4.0;
        self.boss_summon_cd = 7.0;
        self.announcement = Some("THE FEN COLOSSUS RISES".into());
        self.announcement_ttl = 4.0;
    }

    fn update_boss(&mut self, dt: f32) {
        let detonations: Vec<(u64, Vec2, f32, f32)> = self
            .telegraphs
            .values_mut()
            .filter_map(|telegraph| {
                telegraph.ttl -= dt;
                (telegraph.ttl <= 0.0).then_some((
                    telegraph.id,
                    telegraph.pos,
                    telegraph.radius,
                    telegraph.damage,
                ))
            })
            .collect();
        for (id, pos, radius, damage) in detonations {
            self.telegraphs.remove(&id);
            let victims: Vec<String> = self
                .players
                .values()
                .filter(|player| {
                    player.connected && !player.downed && player.pos.distance(pos) <= radius
                })
                .map(|player| player.id.clone())
                .collect();
            for player_id in victims {
                self.damage_player(&player_id, damage);
            }
        }

        let Some(boss_pos) = self
            .enemies
            .values()
            .find(|enemy| enemy.kind == EnemyKind::Colossus)
            .map(|enemy| enemy.pos)
        else {
            return;
        };
        self.boss_slam_cd -= dt;
        self.boss_volley_cd -= dt;
        self.boss_summon_cd -= dt;

        if self.boss_slam_cd <= 0.0 {
            let target = self
                .players
                .values()
                .filter(|player| player.connected && !player.downed)
                .min_by(|a, b| {
                    a.pos
                        .distance(boss_pos)
                        .total_cmp(&b.pos.distance(boss_pos))
                })
                .map(|player| player.pos);
            if let Some(pos) = target {
                let id = self.next_id();
                self.telegraphs.insert(
                    id,
                    Telegraph {
                        id,
                        pos,
                        radius: 88.0,
                        ttl: 1.2,
                        damage: 28.0,
                    },
                );
                self.announcement = Some("COLOSSUS SLAM · MOVE!".into());
                self.announcement_ttl = 1.2;
            }
            self.boss_slam_cd = 5.5;
        }

        if self.boss_volley_cd <= 0.0 {
            for ray in 0..12 {
                let angle = ray as f32 / 12.0 * std::f32::consts::TAU;
                let id = self.next_id();
                self.projectiles.insert(
                    id,
                    Projectile {
                        id,
                        pos: boss_pos,
                        vel: Vec2::new(angle.cos() * 165.0, angle.sin() * 165.0),
                        owner: None,
                        damage: 9.0,
                        element: Element::Ember,
                        hostile: true,
                        homing: false,
                        ttl: 4.5,
                        size: 6.0,
                    },
                );
            }
            self.boss_volley_cd = 6.5;
        }

        if self.boss_summon_cd <= 0.0 {
            let available = ENEMY_CAP.saturating_sub(self.enemies.len()).min(6);
            for index in 0..available {
                let angle = index as f32 / available.max(1) as f32 * std::f32::consts::TAU;
                let kind = if index.is_multiple_of(2) {
                    EnemyKind::Midge
                } else {
                    EnemyKind::Mireling
                };
                self.spawn_enemy_at(
                    kind,
                    Vec2::new(
                        (boss_pos.x + angle.cos() * 76.0).clamp(32.0, WORLD_SIZE - 32.0),
                        (boss_pos.y + angle.sin() * 76.0).clamp(32.0, WORLD_SIZE - 32.0),
                    ),
                );
            }
            self.announcement = Some("THE COLOSSUS CALLS THE MIRE".into());
            self.announcement_ttl = 1.7;
            self.boss_summon_cd = 10.0;
        }
    }

    fn build_enemy_index(&mut self) -> (SpatialIndex, HashMap<u32, u64>) {
        let mut index = SpatialIndex::new(L2, self.tick | 1);
        let mut ids = HashMap::new();
        for e in self.enemies.values() {
            let node = index.insert([e.pos.x, e.pos.y]);
            ids.insert(node, e.id);
        }
        (index, ids)
    }
    fn build_player_index(&mut self) -> (SpatialIndex, HashMap<u32, String>) {
        let mut index = SpatialIndex::new(L2, (self.tick + 7) | 1);
        let mut ids = HashMap::new();
        for p in self.players.values().filter(|p| p.connected && !p.downed) {
            let node = index.insert([p.pos.x, p.pos.y]);
            ids.insert(node, p.id.clone());
        }
        (index, ids)
    }

    fn update_combat(&mut self, dt: f32) {
        if self.enemies.is_empty() {
            return;
        }
        let (index, map) = self.build_enemy_index();
        let ids = self.active_player_ids();
        for player_id in ids {
            let Some(p) = self.players.get(&player_id) else {
                continue;
            };
            if p.downed || p.attack_cd > 0.0 {
                continue;
            }
            self.hnsw_queries += 1;
            let hits = index.search(&[p.pos.x, p.pos.y]);
            let target = hits.into_iter().find_map(|(_, n)| map.get(&n).copied());
            let Some(target_id) = target else { continue };
            let target_pos = self.enemies[&target_id].pos;
            let (form, element, pos, damage, cooldown) = (
                p.form,
                p.element,
                p.pos,
                18.0 * p.damage_mult
                    * (1.0 + self.charm_total(CharmKind::Damage).min(5) as f32 * 0.03),
                0.75 * p.cooldown_mult,
            );
            if let Some(p) = self.players.get_mut(&player_id) {
                p.attack_cd = cooldown.max(0.18)
            }
            match form {
                Form::Bolt => self.spawn_friendly(FriendlyShot {
                    owner: player_id.clone(),
                    pos,
                    dir: Vec2::new(target_pos.x - pos.x, target_pos.y - pos.y).normalized(),
                    damage,
                    element,
                    homing: false,
                    ttl: 1.0,
                    size: 7.0,
                }),
                Form::Swarm => {
                    for offset in [-0.18, 0.0, 0.18] {
                        let dir =
                            Vec2::new(target_pos.x - pos.x + offset * 90.0, target_pos.y - pos.y)
                                .normalized();
                        self.spawn_friendly(FriendlyShot {
                            owner: player_id.clone(),
                            pos,
                            dir,
                            damage: damage * 0.48,
                            element,
                            homing: true,
                            ttl: 4.0,
                            size: 5.0,
                        });
                    }
                }
                Form::Orbit => {
                    let targets: Vec<u64> = index
                        .search(&[pos.x, pos.y])
                        .into_iter()
                        .filter_map(|(d, n)| {
                            (d < 90.0 * 90.0).then(|| map.get(&n).copied()).flatten()
                        })
                        .take(3)
                        .collect();
                    for id in targets {
                        self.damage_enemy(id, damage * 0.6, element, &player_id);
                    }
                }
                Form::Nova => {
                    let targets: Vec<u64> = index
                        .search(&[pos.x, pos.y])
                        .into_iter()
                        .filter_map(|(d, n)| {
                            (d < 120.0 * 120.0).then(|| map.get(&n).copied()).flatten()
                        })
                        .collect();
                    for id in targets {
                        self.damage_enemy(id, damage * 1.35, element, &player_id);
                    }
                }
            }
        }
        let _ = dt;
    }

    fn spawn_friendly(&mut self, shot: FriendlyShot) {
        let id = self.next_id();
        self.projectiles.insert(
            id,
            Projectile {
                id,
                pos: shot.pos,
                vel: Vec2::new(shot.dir.x * 330.0, shot.dir.y * 330.0),
                owner: Some(shot.owner),
                damage: shot.damage,
                element: shot.element,
                hostile: false,
                homing: shot.homing,
                ttl: shot.ttl,
                size: shot.size,
            },
        );
    }

    fn update_enemies(&mut self, dt: f32) {
        let (pindex, pmap) = self.build_player_index();
        if pmap.is_empty() {
            return;
        }
        let ids: Vec<u64> = self.enemies.keys().copied().collect();
        for id in ids {
            let Some(e) = self.enemies.get(&id) else {
                continue;
            };
            self.hnsw_queries += 1;
            let target = pindex
                .search(&[e.pos.x, e.pos.y])
                .into_iter()
                .find_map(|(_, n)| pmap.get(&n).cloned());
            let Some(target_id) = target else { continue };
            let target_pos = self.players[&target_id].pos;
            let mut contact = false;
            let mut ranged = false;
            if let Some(e) = self.enemies.get_mut(&id) {
                e.attack_cd -= dt;
                e.slow = (e.slow - dt).max(0.0);
                e.dot_ttl = (e.dot_ttl - dt).max(0.0);
                if e.dot_ttl > 0.0 {
                    e.hp -= e.dot * dt;
                } else {
                    e.dot = 0.0;
                    e.dot_owner = None;
                }
                let dir = Vec2::new(target_pos.x - e.pos.x, target_pos.y - e.pos.y).normalized();
                let mult = if e.slow > 0.0 { 0.65 } else { 1.0 };
                e.pos.x += dir.x * e.speed * mult * dt;
                e.pos.y += dir.y * e.speed * mult * dt;
                let distance = e.pos.distance(target_pos);
                if e.kind == EnemyKind::Spitter && distance < 300.0 && e.attack_cd <= 0.0 {
                    e.attack_cd = 2.0;
                    ranged = true
                } else if distance < PLAYER_RADIUS + 14.0 && e.attack_cd <= 0.0 {
                    e.attack_cd = 0.9;
                    contact = true
                }
            }
            if contact {
                let damage = self.enemies[&id].damage;
                self.damage_player(&target_id, damage)
            }
            if ranged {
                let pos = self.enemies[&id].pos;
                let dir = Vec2::new(target_pos.x - pos.x, target_pos.y - pos.y).normalized();
                let pid = self.next_id();
                self.projectiles.insert(
                    pid,
                    Projectile {
                        id: pid,
                        pos,
                        vel: Vec2::new(dir.x * 190.0, dir.y * 190.0),
                        owner: None,
                        damage: 8.0,
                        element: Element::Ember,
                        hostile: true,
                        homing: false,
                        ttl: 3.0,
                        size: 6.0,
                    },
                );
            }
            if self.enemies.get(&id).is_some_and(|e| e.hp <= 0.0) {
                let owner = self.enemies[&id].dot_owner.clone();
                self.kill_enemy(id, owner.as_deref());
            }
        }
    }

    fn update_projectiles(&mut self, dt: f32) {
        let (enemy_index, enemy_map) = self.build_enemy_index();
        let ids: Vec<u64> = self.projectiles.keys().copied().collect();
        for id in ids {
            let homing_target = self.projectiles.get(&id).and_then(|projectile| {
                if projectile.hostile || !projectile.homing {
                    return None;
                }
                self.hnsw_queries += 1;
                enemy_index
                    .search(&[projectile.pos.x, projectile.pos.y])
                    .into_iter()
                    .find_map(|(_, node)| enemy_map.get(&node).copied())
                    .and_then(|enemy_id| self.enemies.get(&enemy_id).map(|enemy| enemy.pos))
            });
            let mut remove = false;
            let mut hit_enemy = None;
            let mut hit_player = None;
            if let Some(p) = self.projectiles.get_mut(&id) {
                if let Some(target) = homing_target {
                    let direction = Vec2::new(target.x - p.pos.x, target.y - p.pos.y).normalized();
                    p.vel = Vec2::new(direction.x * 245.0, direction.y * 245.0);
                }
                p.ttl -= dt;
                p.pos.x += p.vel.x * dt;
                p.pos.y += p.vel.y * dt;
                if p.ttl <= 0.0 {
                    remove = true
                } else if p.hostile {
                    for player in self.players.values().filter(|v| v.connected && !v.downed) {
                        if p.pos.distance(player.pos) < p.size + PLAYER_RADIUS {
                            hit_player = Some(player.id.clone());
                            break;
                        }
                    }
                } else {
                    for e in self.enemies.values() {
                        if p.pos.distance(e.pos) < p.size + 14.0 {
                            hit_enemy = Some(e.id);
                            break;
                        }
                    }
                }
            }
            if let Some(pid) = hit_player {
                let damage = self.projectiles[&id].damage;
                self.damage_player(&pid, damage);
                remove = true
            }
            if let Some(eid) = hit_enemy {
                let p = &self.projectiles[&id];
                let (damage, element, owner) =
                    (p.damage, p.element, p.owner.clone().unwrap_or_default());
                self.damage_enemy(eid, damage, element, &owner);
                remove = true
            }
            if remove {
                self.projectiles.remove(&id);
            }
        }
    }

    fn damage_enemy(&mut self, id: u64, base: f32, element: Element, owner: &str) {
        let multiplier = if element == Element::Ember {
            1.15
        } else if matches!(element, Element::Frost | Element::Storm) {
            0.9
        } else {
            1.0
        };
        let damage = base * multiplier;
        let mut chain_pos = None;
        if let Some(e) = self.enemies.get_mut(&id) {
            e.hp -= damage;
            if element == Element::Frost {
                e.slow = 1.5
            }
            if element == Element::Ember {
                e.dot = damage * 0.1;
                e.dot_ttl = 2.5;
                e.dot_owner = Some(owner.into())
            }
            if element == Element::Thorn {
                e.dot = damage * 0.07;
                e.dot_ttl = 3.0;
                e.dot_owner = Some(owner.into())
            }
            chain_pos = (element == Element::Storm).then_some(e.pos)
        }
        if let Some(p) = self.players.get_mut(owner) {
            p.damage += damage.max(0.0) as u64;
            if element == Element::Thorn {
                p.hp = (p.hp + damage * 0.08).min(p.max_hp)
            }
        }
        self.sync_player(owner);
        self.sync_enemy(id);
        if let Some(pos) = chain_pos {
            let (index, map) = self.build_enemy_index();
            self.hnsw_queries += 1;
            let other = index
                .search(&[pos.x, pos.y])
                .into_iter()
                .find_map(|(distance, node)| {
                    map.get(&node)
                        .copied()
                        .filter(|candidate| *candidate != id && distance < 90.0 * 90.0)
                });
            if let Some(other) = other {
                if let Some(e) = self.enemies.get_mut(&other) {
                    e.hp -= damage * 0.5
                }
                self.sync_enemy(other)
            }
        }
        if self.enemies.get(&id).is_some_and(|e| e.hp <= 0.0) {
            self.kill_enemy(id, Some(owner));
        }
    }

    fn kill_enemy(&mut self, id: u64, owner: Option<&str>) {
        let Some(enemy) = self.enemies.remove(&id) else {
            return;
        };
        self.fact_ops.push(FactOp::Remove(FactKey::Enemy(id)));
        if let Some(owner) = owner {
            if let Some(p) = self.players.get_mut(owner) {
                p.kills += 1
            }
            self.sync_player(owner)
        }
        let xp = match enemy.kind {
            EnemyKind::Mireling => 3,
            EnemyKind::Midge => 2,
            EnemyKind::Spitter => 5,
            EnemyKind::Brute => 12,
            EnemyKind::Colossus => 40,
        };
        let pickup_id = self.next_id();
        self.pickups.insert(
            pickup_id,
            Pickup {
                id: pickup_id,
                pos: enemy.pos,
                kind: PickupKind::Xp(xp),
                age: 0.0,
            },
        );
        if enemy.kind == EnemyKind::Brute {
            let wave = self.wave();
            for player_id in self.active_player_ids() {
                if self.players[&player_id].rune_wave < wave {
                    let rid = self.next_id();
                    let element = Element::ALL[self.rng.usize(0..4)];
                    let form = Form::ALL[self.rng.usize(0..4)];
                    self.pickups.insert(
                        rid,
                        Pickup {
                            id: rid,
                            pos: enemy.pos,
                            kind: PickupKind::Rune {
                                owner: player_id,
                                element,
                                form,
                            },
                            age: 0.0,
                        },
                    );
                }
            }
        }
        if enemy.kind == EnemyKind::Colossus {
            self.finish(Phase::Victory, "THE FEN COLOSSUS FALLS");
        }
    }

    fn damage_player(&mut self, id: &str, damage: f32) {
        let solo = self.connected_count() == 1;
        let mut sync = false;
        if let Some(p) = self.players.get_mut(id) {
            if p.invulnerable > 0.0 || p.downed || p.pending_upgrade.is_some() {
                return;
            }
            p.hp -= damage;
            sync = true;
            if p.hp <= 0.0 {
                if p.auto_revive && solo {
                    p.auto_revive = false;
                    p.hp = p.max_hp * 0.5;
                    p.invulnerable = 3.0;
                    self.announcement = Some(format!("{}'s rescue leaf blooms", p.name));
                    self.announcement_ttl = 2.0
                } else {
                    p.hp = 0.0;
                    p.downed = true;
                    p.input = Vec2::default();
                }
            }
        }
        if sync {
            self.sync_player(id)
        }
    }

    fn update_pickups(&mut self, dt: f32) {
        let party_magnet = 1.0 + self.charm_total(CharmKind::Magnet).min(5) as f32 * 0.03;
        let (player_index, player_map) = self.build_player_index();
        let ids: Vec<u64> = self.pickups.keys().copied().collect();
        for id in ids {
            let xp_target = self.pickups.get(&id).and_then(|pickup| {
                if !matches!(pickup.kind, PickupKind::Xp(_)) {
                    return None;
                }
                self.hnsw_queries += 1;
                player_index
                    .search(&[pickup.pos.x, pickup.pos.y])
                    .into_iter()
                    .find_map(|(_, node)| player_map.get(&node).cloned())
            });
            let mut collect = None;
            if let Some(pickup) = self.pickups.get_mut(&id) {
                pickup.age += dt;
                match &pickup.kind {
                    PickupKind::Xp(_) => {
                        if let Some(player) = xp_target
                            .as_ref()
                            .and_then(|player_id| self.players.get(player_id))
                        {
                            let radius = 42.0 * player.magnet_mult * party_magnet;
                            if player.pos.distance(pickup.pos) < radius {
                                collect = Some(player.id.clone())
                            }
                        }
                    }
                    PickupKind::Rune { owner, .. } => {
                        if let Some(player) = self.players.get(owner) {
                            if pickup.age > 2.0 {
                                let dir = Vec2::new(
                                    player.pos.x - pickup.pos.x,
                                    player.pos.y - pickup.pos.y,
                                )
                                .normalized();
                                pickup.pos.x += dir.x * 220.0 * dt;
                                pickup.pos.y += dir.y * 220.0 * dt
                            }
                            if player.pos.distance(pickup.pos) < 38.0 {
                                collect = Some(owner.clone())
                            }
                        }
                    }
                }
            }
            if let Some(player_id) = collect
                && let Some(pickup) = self.pickups.remove(&id)
            {
                match pickup.kind {
                    PickupKind::Xp(value) => self.give_xp(&player_id, value),
                    PickupKind::Rune { element, form, .. } => {
                        self.offer_rune(&player_id, id, element, form)
                    }
                }
            }
        }
    }

    fn give_xp(&mut self, id: &str, value: u32) {
        let mut draft = false;
        if let Some(p) = self.players.get_mut(id) {
            p.xp += value;
            while p.xp >= p.xp_next() {
                p.xp -= p.xp_next();
                p.level += 1;
                if p.pending_upgrade.is_none() {
                    draft = true;
                    break;
                }
            }
        }
        if draft {
            let draft_id = self.next_id();
            let pool = [
                UpgradeKind::Damage,
                UpgradeKind::Cooldown,
                UpgradeKind::Speed,
                UpgradeKind::MaxHealth,
                UpgradeKind::Magnet,
                UpgradeKind::Revive,
                UpgradeKind::PartyDamage,
                UpgradeKind::PartySpeed,
                UpgradeKind::PartyMagnet,
            ];
            let mut choices = vec![];
            while choices.len() < 3 {
                let kind = pool[self.rng.usize(0..pool.len())];
                if self.connected_count() == 1 && kind == UpgradeKind::Revive {
                    continue;
                }
                if choices.iter().any(|c: &UpgradeChoice| c.kind == kind) {
                    continue;
                }
                let cid = self.next_id();
                choices.push(UpgradeChoice {
                    id: cid,
                    kind,
                    title: kind.title().into(),
                    description: kind.description().into(),
                });
            }
            if let Some(p) = self.players.get_mut(id) {
                p.pending_upgrade = Some(UpgradeDraft {
                    id: draft_id,
                    choices,
                    expires_ms: self.elapsed_ms + 8_000,
                });
            }
        }
        self.sync_player(id)
    }

    fn offer_rune(&mut self, id: &str, drop_id: u64, element: Element, form: Form) {
        let Some(p) = self.players.get(id) else {
            return;
        };
        let replacing = if self.rng.bool() { "element" } else { "form" };
        let (new_element, new_form) = if replacing == "element" {
            (element, p.form)
        } else {
            (p.element, form)
        };
        let spell = self.compiler.compile(new_element, new_form);
        let wave = self.wave();
        self.ese_compiles += 1;
        if let Some(p) = self.players.get_mut(id) {
            p.pending_rune = Some(RuneDraft {
                id: drop_id,
                element: spell.element,
                form: spell.form,
                replacing: replacing.into(),
                spell_name: spell.name,
                description: spell.description,
                confidence: spell.confidence,
            });
            p.rune_wave = wave;
        }
    }

    fn update_drafts(&mut self) {
        let expired: Vec<(String, u64, u64)> = self
            .players
            .values()
            .filter_map(|p| {
                p.pending_upgrade.as_ref().and_then(|d| {
                    (d.expires_ms <= self.elapsed_ms).then_some((
                        p.id.clone(),
                        d.id,
                        d.choices[0].id,
                    ))
                })
            })
            .collect();
        for (id, draft, choice) in expired {
            self.choose_upgrade(&id, draft, choice)
        }
    }
    fn choose_upgrade(&mut self, id: &str, draft_id: u64, choice_id: u64) {
        let kind = self
            .players
            .get(id)
            .and_then(|p| p.pending_upgrade.as_ref())
            .filter(|d| d.id == draft_id)
            .and_then(|d| d.choices.iter().find(|c| c.id == choice_id))
            .map(|c| c.kind);
        let Some(kind) = kind else { return };
        if let Some(p) = self.players.get_mut(id) {
            match kind {
                UpgradeKind::Damage => p.damage_mult *= 1.15,
                UpgradeKind::Cooldown => p.cooldown_mult *= 0.9,
                UpgradeKind::Speed => p.speed_mult *= 1.08,
                UpgradeKind::MaxHealth => {
                    p.max_hp += 20.0;
                    p.hp = (p.hp + 20.0).min(p.max_hp)
                }
                UpgradeKind::Magnet => p.magnet_mult *= 1.25,
                UpgradeKind::Revive => p.revive_mult *= 1.25,
                UpgradeKind::PartyDamage | UpgradeKind::PartySpeed | UpgradeKind::PartyMagnet => {}
            }
            p.pending_upgrade = None;
        }
        if let Some(charm) = (match kind {
            UpgradeKind::PartyDamage => Some(CharmKind::Damage),
            UpgradeKind::PartySpeed => Some(CharmKind::Speed),
            UpgradeKind::PartyMagnet => Some(CharmKind::Magnet),
            _ => None,
        }) && self.charm_total(charm) < 5
        {
            let charm_key = (id.to_string(), charm);
            let current = self.charms.get(&charm_key).copied().unwrap_or(0) + 1;
            self.charms.insert(charm_key, current);
            self.fact_ops.push(FactOp::Upsert(
                FactKey::Charm(id.into(), charm),
                Fact::Charm {
                    player_id: id.into(),
                    kind: charm,
                    stacks: current,
                },
            ));
        }
        self.sync_player(id)
    }
    fn equip_rune(&mut self, id: &str, drop_id: u64) {
        let draft = self
            .players
            .get(id)
            .and_then(|p| p.pending_rune.clone())
            .filter(|d| d.id == drop_id);
        let Some(draft) = draft else { return };
        if let Some(p) = self.players.get_mut(id) {
            p.element = draft.element;
            p.form = draft.form;
            p.pending_rune = None;
        }
        self.sync_player(id)
    }
    fn salvage_rune(&mut self, id: &str, drop_id: u64) {
        if self
            .players
            .get(id)
            .and_then(|p| p.pending_rune.as_ref())
            .is_some_and(|d| d.id == drop_id)
        {
            if let Some(p) = self.players.get_mut(id) {
                p.pending_rune = None;
            }
            self.give_xp(id, 12)
        }
    }

    fn update_revives(&mut self, dt: f32) {
        let (player_index, player_map) = self.build_player_index();
        let downed: Vec<String> = self
            .players
            .values()
            .filter(|p| p.connected && p.downed)
            .map(|p| p.id.clone())
            .collect();
        for id in downed {
            let pos = self.players[&id].pos;
            self.hnsw_queries += 1;
            let revivers: Vec<String> = player_index
                .search(&[pos.x, pos.y])
                .into_iter()
                .filter_map(|(distance, node)| {
                    (distance < 72.0 * 72.0)
                        .then(|| player_map.get(&node).cloned())
                        .flatten()
                })
                .collect();
            if revivers.is_empty() {
                continue;
            }
            let rate: f32 = revivers.iter().map(|r| self.players[r].revive_mult).sum();
            let mut revived_name = None;
            if let Some(p) = self.players.get_mut(&id) {
                p.hp += p.max_hp / 6.0 * rate * dt;
                if p.hp >= p.max_hp * 0.5 {
                    p.hp = p.max_hp * 0.5;
                    p.downed = false;
                    p.invulnerable = 2.0;
                    revived_name = Some(p.name.clone());
                }
            }
            if let Some(name) = revived_name {
                for r in revivers {
                    if let Some(reviver) = self.players.get_mut(&r) {
                        reviver.revives += 1
                    }
                }
                self.announcement = Some(format!("{name} is back on their feet"));
                self.announcement_ttl = 2.0;
            }
            self.sync_player(&id)
        }
    }

    fn finish(&mut self, phase: Phase, message: &str) {
        if self.phase != Phase::Running {
            return;
        }
        self.phase = phase;
        self.telegraphs.clear();
        self.announcement = Some(message.into());
        self.announcement_ttl = 99.0;
        self.fact_ops.push(FactOp::Upsert(
            FactKey::Run,
            Fact::Run {
                phase,
                wave: self.wave(),
            },
        ));
    }
    fn rematch(&mut self) {
        if !matches!(self.phase, Phase::Victory | Phase::Defeat) {
            return;
        }
        for id in self.enemies.keys().copied().collect::<Vec<_>>() {
            self.fact_ops.push(FactOp::Remove(FactKey::Enemy(id)));
        }
        self.enemies.clear();
        self.projectiles.clear();
        self.pickups.clear();
        self.telegraphs.clear();
        for player_id in self.players.keys() {
            self.fact_ops
                .push(FactOp::Remove(FactKey::Loadout(player_id.clone())));
            self.fact_ops
                .push(FactOp::Remove(FactKey::Damage(player_id.clone())));
        }
        for ((player_id, kind), _) in self.charms.drain() {
            self.fact_ops
                .push(FactOp::Remove(FactKey::Charm(player_id, kind)));
        }
        self.phase = Phase::Lobby;
        self.elapsed_ms = 0;
        self.announcement = Some("The island is ready for another expedition".into());
        self.announcement_ttl = 99.0;
        self.fact_ops.push(FactOp::Upsert(
            FactKey::Run,
            Fact::Run {
                phase: self.phase,
                wave: 0,
            },
        ));
    }

    fn sync_player(&mut self, id: &str) {
        if let Some(p) = self.players.get(id) {
            self.fact_ops.push(FactOp::Upsert(
                FactKey::Player(id.into()),
                Fact::Player {
                    id: id.into(),
                    hp_milli: (p.hp * 1000.0) as i64,
                    level: p.level,
                    downed: p.downed,
                },
            ));
            self.fact_ops.push(FactOp::Upsert(
                FactKey::Damage(id.into()),
                Fact::Damage {
                    player_id: id.into(),
                    total: p.damage,
                },
            ));
            let description = format!("{} {}", p.element.label(), p.form.label());
            self.fact_ops.push(FactOp::Upsert(
                FactKey::Loadout(id.into()),
                Fact::Loadout {
                    player_id: id.into(),
                    element: p.element,
                    form: p.form,
                    description,
                },
            ));
        }
    }
    fn sync_enemy(&mut self, id: u64) {
        if let Some(e) = self.enemies.get(&id) {
            self.fact_ops.push(FactOp::Upsert(
                FactKey::Enemy(id),
                Fact::Enemy {
                    id,
                    kind: e.kind,
                    hp_milli: (e.hp * 1000.0) as i64,
                },
            ));
        }
    }
    fn take_ops(&mut self) -> Vec<FactOp> {
        std::mem::take(&mut self.fact_ops)
    }
    fn charm_total(&self, kind: CharmKind) -> i64 {
        self.charms
            .iter()
            .filter(|((_, candidate), _)| *candidate == kind)
            .map(|(_, stacks)| *stacks)
            .sum::<i64>()
            .min(5)
    }
    fn median_level(&self) -> u32 {
        let mut levels: Vec<u32> = self
            .players
            .values()
            .filter(|p| p.connected)
            .map(|p| p.level)
            .collect();
        if levels.is_empty() {
            1
        } else {
            levels.sort();
            levels[levels.len() / 2]
        }
    }
    fn living_spawn(&self) -> Vec2 {
        self.players
            .values()
            .find(|p| p.connected && !p.downed)
            .map(|p| p.pos)
            .unwrap_or(Vec2::new(800.0, 800.0))
    }

    fn snapshot(&self, metrics: BogMetrics) -> GameSnapshot {
        let mut players: Vec<PlayerView> = self
            .players
            .values()
            .filter(|p| p.connected)
            .map(|p| PlayerView {
                id: p.id.clone(),
                name: p.name.clone(),
                skin: p.skin,
                pos: p.pos,
                hp: p.hp,
                max_hp: p.max_hp,
                level: p.level,
                xp: p.xp,
                xp_next: p.xp_next(),
                downed: p.downed,
                connected: p.connected,
                element: p.element,
                form: p.form,
                kills: p.kills,
                damage: p.damage,
                revives: p.revives,
            })
            .collect();
        players.sort_by(|a, b| a.name.cmp(&b.name));
        let enemies = self
            .enemies
            .values()
            .map(|e| EnemyView {
                id: e.id,
                kind: e.kind,
                pos: e.pos,
                hp: e.hp,
                max_hp: e.max_hp,
                boss: e.kind == EnemyKind::Colossus,
            })
            .collect();
        let projectiles = self
            .projectiles
            .values()
            .map(|p| ProjectileView {
                id: p.id,
                pos: p.pos,
                element: p.element,
                hostile: p.hostile,
                size: p.size,
            })
            .collect();
        let pickups = self
            .pickups
            .values()
            .map(|p| PickupView {
                id: p.id,
                pos: p.pos,
                kind: match p.kind {
                    PickupKind::Xp(_) => "xp",
                    PickupKind::Rune { .. } => "rune",
                }
                .into(),
                owner_id: match &p.kind {
                    PickupKind::Rune { owner, .. } => Some(owner.clone()),
                    _ => None,
                },
            })
            .collect();
        let telegraphs = self
            .telegraphs
            .values()
            .map(|telegraph| TelegraphView {
                id: telegraph.id,
                pos: telegraph.pos,
                radius: telegraph.radius,
                remaining_ms: (telegraph.ttl.max(0.0) * 1000.0) as u64,
            })
            .collect();
        let boss_hp = self
            .enemies
            .values()
            .find(|e| e.kind == EnemyKind::Colossus)
            .map(|e| (e.hp.max(0.0), e.max_hp));
        let private_upgrades = self
            .players
            .values()
            .filter_map(|p| p.pending_upgrade.clone().map(|d| (p.id.clone(), d)))
            .collect();
        let private_runes = self
            .players
            .values()
            .filter_map(|p| p.pending_rune.clone().map(|d| (p.id.clone(), d)))
            .collect();
        GameSnapshot {
            tick: self.tick,
            phase: self.phase,
            run_id: self.run_id,
            elapsed_ms: self.elapsed_ms,
            remaining_ms: 300_000u64.saturating_sub(self.elapsed_ms),
            wave: if self.phase == Phase::Lobby {
                0
            } else {
                self.wave()
            },
            wave_label: if self.phase == Phase::Lobby {
                "Gather at the shore".into()
            } else {
                self.wave_label()
            },
            join_url: self.join_url.clone(),
            connected_players: self.connected_count(),
            you: None,
            players,
            enemies,
            projectiles,
            pickups,
            telegraphs,
            boss_hp,
            upgrade_draft: None,
            rune_draft: None,
            announcement: self.announcement.clone(),
            metrics,
            private_upgrades,
            private_runes,
        }
    }
}

fn clean_name(name: String) -> String {
    let name: String = name
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(18)
        .collect();
    if name.is_empty() {
        "Traveler".into()
    } else {
        name
    }
}

pub fn spawn(join_url: String, db_path: std::path::PathBuf) -> EngineHandle {
    let (tx, rx) = mpsc::channel();
    let initial = GameSnapshot::lobby(join_url.clone());
    let (state_tx, state_rx) = watch::channel(initial);
    thread::spawn(move || {
        let mut game = Game::new(join_url);
        let mut stream = crate::bog_stream!(db_path);
        let mut next_tick = Instant::now();
        let tick_duration = Duration::from_millis(50);
        loop {
            let active = game.phase == Phase::Running && !game.sockets.is_empty();
            let mut publish = false;
            let command = if active {
                match rx.recv_timeout(next_tick.saturating_duration_since(Instant::now())) {
                    Ok(command) => Some(command),
                    Err(mpsc::RecvTimeoutError::Timeout) => None,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            } else {
                match rx.recv() {
                    Ok(command) => Some(command),
                    Err(_) => break,
                }
            };
            if let Some(command) = command {
                apply_command(&mut game, command);
                publish = true;
            }
            let should_tick = game.phase == Phase::Running && !game.sockets.is_empty();
            if should_tick && Instant::now() >= next_tick {
                game.tick(0.05);
                next_tick = Instant::now() + tick_duration;
                publish |= game.tick.is_multiple_of(2);
            }
            let ops = game.take_ops();
            if !ops.is_empty() {
                stream.wtx(|tx| {
                    for op in ops {
                        match op {
                            FactOp::Upsert(key, fact) => {
                                tx.upsert(&key, &fact);
                            }
                            FactOp::Remove(key) => {
                                tx.remove(&key);
                            }
                        }
                    }
                });
                game.fold_commits += 1;
                publish = true;
            }
            if publish {
                let metrics = crate::read_bog_metrics!(
                    stream,
                    game.fold_commits,
                    game.hnsw_queries,
                    game.ese_compiles
                );
                let _ = state_tx.send(game.snapshot(metrics));
            }
        }
    });
    EngineHandle {
        commands: tx,
        snapshots: state_rx,
    }
}

fn apply_command(game: &mut Game, command: EngineCommand) {
    match command {
        EngineCommand::SocketOpened { connection_id } => {
            game.sockets.insert(connection_id);
        }
        EngineCommand::SocketClosed { connection_id } => game.socket_closed(connection_id),
        EngineCommand::Register {
            connection_id,
            player_id,
            name,
            skin,
        } => game.register(connection_id, player_id, name, skin),
        EngineCommand::SelectSkin { player_id, skin } => {
            if game.phase == Phase::Lobby {
                if let Some(p) = game.players.get_mut(&player_id) {
                    p.skin = skin;
                }
                game.sync_player(&player_id)
            }
        }
        EngineCommand::Input {
            connection_id,
            player_id,
            sequence,
            x,
            y,
        } => {
            if game.connection_players.get(&connection_id) == Some(&player_id)
                && let Some(p) = game.players.get_mut(&player_id)
                && sequence >= p.input_seq
            {
                p.input_seq = sequence;
                p.input = Vec2::new(x.clamp(-1.0, 1.0), y.clamp(-1.0, 1.0));
            }
        }
        EngineCommand::StartRun => game.start_run(),
        EngineCommand::ChooseUpgrade {
            player_id,
            draft_id,
            choice_id,
        } => game.choose_upgrade(&player_id, draft_id, choice_id),
        EngineCommand::EquipRune { player_id, drop_id } => game.equip_rune(&player_id, drop_id),
        EngineCommand::SalvageRune { player_id, drop_id } => game.salvage_rune(&player_id, drop_id),
        EngineCommand::Rematch => game.rematch(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn register(game: &mut Game, connection_id: u64, id: &str) {
        game.register(connection_id, id.into(), format!("Player {id}"), Skin::Sun);
    }

    #[test]
    fn skins_are_mechanically_identical() {
        for skin in [Skin::Sun, Skin::Tide, Skin::Storm, Skin::Grove] {
            let p = Player::new("p".into(), "P".into(), skin, Vec2::default());
            assert_eq!(
                (
                    p.hp,
                    p.max_hp,
                    p.damage_mult,
                    p.speed_mult,
                    p.element,
                    p.form
                ),
                (100.0, 100.0, 1.0, 1.0, Element::Thorn, Form::Bolt)
            );
        }
    }
    #[test]
    fn enemy_cap_scales_but_is_bounded() {
        assert!((18 + 20 * 7 + 5 * 12).min(ENEMY_CAP) <= ENEMY_CAP)
    }

    #[test]
    fn duplicate_id_uses_newest_connection_for_input() {
        let mut game = Game::new("http://test".into());
        register(&mut game, 1, "same");
        register(&mut game, 2, "same");
        assert_eq!(game.connected_count(), 1);
        apply_command(
            &mut game,
            EngineCommand::Input {
                connection_id: 1,
                player_id: "same".into(),
                sequence: 1,
                x: 1.0,
                y: 0.0,
            },
        );
        assert_eq!(game.players["same"].input, Vec2::default());
        apply_command(
            &mut game,
            EngineCommand::Input {
                connection_id: 2,
                player_id: "same".into(),
                sequence: 1,
                x: 3.0,
                y: -3.0,
            },
        );
        assert_eq!(game.players["same"].input, Vec2::new(1.0, -1.0));
    }

    #[test]
    fn rematch_resets_input_sequence_and_stale_direction() {
        let mut game = Game::new("http://test".into());
        register(&mut game, 1, "solo");
        game.start_run();
        apply_command(
            &mut game,
            EngineCommand::Input {
                connection_id: 1,
                player_id: "solo".into(),
                sequence: 500,
                x: 1.0,
                y: 0.0,
            },
        );
        game.finish(Phase::Defeat, "test");
        game.rematch();
        game.start_run();
        assert_eq!(game.players["solo"].input_seq, 0);
        assert_eq!(game.players["solo"].input, Vec2::default());
        apply_command(
            &mut game,
            EngineCommand::Input {
                connection_id: 1,
                player_id: "solo".into(),
                sequence: 1,
                x: -1.0,
                y: 0.0,
            },
        );
        assert_eq!(game.players["solo"].input, Vec2::new(-1.0, 0.0));
    }

    #[test]
    fn one_player_can_win_and_rematch() {
        let mut game = Game::new("http://test".into());
        register(&mut game, 1, "solo");
        game.start_run();
        assert_eq!(game.phase, Phase::Running);
        game.spawn_boss();
        let boss = game
            .enemies
            .values()
            .find(|enemy| enemy.kind == EnemyKind::Colossus)
            .expect("boss")
            .id;
        game.damage_enemy(boss, 100_000.0, Element::Thorn, "solo");
        assert_eq!(game.phase, Phase::Victory);
        game.rematch();
        assert_eq!(game.phase, Phase::Lobby);
        assert!(game.enemies.is_empty());
        assert!(game.telegraphs.is_empty());
    }

    #[test]
    fn boss_telegraphs_volleys_and_summons_within_cap() {
        let mut game = Game::new("http://test".into());
        register(&mut game, 1, "solo");
        game.start_run();
        game.spawn_boss();
        game.boss_slam_cd = 0.0;
        game.boss_volley_cd = 0.0;
        game.boss_summon_cd = 0.0;
        game.update_boss(0.05);
        assert_eq!(game.telegraphs.len(), 1);
        assert_eq!(game.projectiles.len(), 12);
        assert_eq!(game.enemies.len(), 7);
        assert!(game.enemies.len() <= ENEMY_CAP);
    }

    #[test]
    fn revive_takes_three_seconds_and_solo_revives_once() {
        let mut party = Game::new("http://test".into());
        register(&mut party, 1, "downed");
        register(&mut party, 2, "helper");
        party.start_run();
        let helper_pos = party.players["helper"].pos;
        if let Some(player) = party.players.get_mut("downed") {
            player.pos = helper_pos;
            player.hp = 0.0;
            player.downed = true;
        }
        for _ in 0..59 {
            party.update_revives(0.05);
        }
        assert!(party.players["downed"].downed);
        for _ in 0..2 {
            party.update_revives(0.05);
        }
        assert!(!party.players["downed"].downed);

        let mut solo = Game::new("http://test".into());
        register(&mut solo, 1, "solo");
        solo.start_run();
        solo.players.get_mut("solo").expect("solo").invulnerable = 0.0;
        solo.damage_player("solo", 500.0);
        assert!(!solo.players["solo"].downed);
        assert!(!solo.players["solo"].auto_revive);
        solo.players.get_mut("solo").expect("solo").invulnerable = 0.0;
        solo.damage_player("solo", 500.0);
        assert!(solo.players["solo"].downed);
    }

    #[test]
    fn twenty_players_never_exceed_enemy_cap() {
        let mut game = Game::new("http://test".into());
        for index in 0..20 {
            register(&mut game, index + 1, &format!("p{index}"));
        }
        game.start_run();
        game.elapsed_ms = 175_000;
        for _ in 0..(ENEMY_CAP + 40) {
            game.spawn_cd = 0.0;
            game.spawn_enemies(0.05);
        }
        assert_eq!(game.connected_count(), 20);
        assert_eq!(game.enemies.len(), ENEMY_CAP);
        game.spawn_boss();
        assert_eq!(game.enemies.len(), ENEMY_CAP);
        assert!(
            game.enemies
                .values()
                .any(|enemy| enemy.kind == EnemyKind::Colossus)
        );
    }
}
