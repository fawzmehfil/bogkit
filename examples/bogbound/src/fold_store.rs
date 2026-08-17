#[macro_export]
macro_rules! bog_stream {
    ($path:expr) => {{
        fold::stream::KeyedStream::new(
            $path,
            (
                fold::pipeline::terminal::Table::new("facts"),
                fold::pipeline::FilterMap::new(
                    |d: &fold::pipeline::Keyed<$crate::model::FactKey, $crate::model::Fact>| {
                        match &d.val {
                            $crate::model::Fact::Player { id, .. } => Some(id.clone()),
                            _ => None,
                        }
                    },
                    fold::pipeline::terminal::Count::new("players_count"),
                ),
                fold::pipeline::FilterMap::new(
                    |d: &fold::pipeline::Keyed<$crate::model::FactKey, $crate::model::Fact>| {
                        match &d.val {
                            $crate::model::Fact::Enemy { kind, .. } => {
                                Some(fold::pipeline::Keyed::new(*kind, 1_i64))
                            }
                            _ => None,
                        }
                    },
                    (
                        fold::pipeline::terminal::Count::new("enemies_count"),
                        fold::pipeline::Aggregate::new(
                            "enemy_counts_aggregate",
                            |acc: &mut i64, value: &i64, delta| *acc += value * delta as i64,
                            fold::pipeline::terminal::Table::new("enemy_counts"),
                        ),
                    ),
                ),
                fold::pipeline::FilterMap::new(
                    |d: &fold::pipeline::Keyed<$crate::model::FactKey, $crate::model::Fact>| {
                        match &d.val {
                            $crate::model::Fact::Damage { player_id, total } => {
                                Some(fold::pipeline::Scored::new(*total, player_id.clone()))
                            }
                            _ => None,
                        }
                    },
                    fold::pipeline::terminal::Ranked::new("damage_ranking"),
                ),
                fold::pipeline::FilterMap::new(
                    |d: &fold::pipeline::Keyed<$crate::model::FactKey, $crate::model::Fact>| {
                        match &d.val {
                            $crate::model::Fact::Charm { kind, stacks, .. } => {
                                Some(fold::pipeline::Keyed::new(*kind, *stacks))
                            }
                            _ => None,
                        }
                    },
                    fold::pipeline::Aggregate::new(
                        "charm_aggregate",
                        |acc: &mut i64, value: &i64, delta| *acc += value * delta as i64,
                        fold::pipeline::terminal::Table::new("charms"),
                    ),
                ),
                fold::pipeline::FilterMap::new(
                    |d: &fold::pipeline::Keyed<$crate::model::FactKey, $crate::model::Fact>| {
                        match &d.val {
                            $crate::model::Fact::Loadout {
                                player_id,
                                description,
                                ..
                            } => Some(fold::pipeline::Keyed::new(
                                player_id.clone(),
                                ese::encode_single(description),
                            )),
                            _ => None,
                        }
                    },
                    fold::pipeline::terminal::search::Hnsw::<
                        String,
                        f32,
                        anny::metric::Cosine,
                        { ese::DIMENSIONS },
                    >::new("loadout_hnsw", anny::metric::Cosine, 0xb06),
                ),
                (
                    fold::pipeline::FilterMap::new(
                        |d: &fold::pipeline::Keyed<$crate::model::FactKey, $crate::model::Fact>| {
                            match &d.val {
                                $crate::model::Fact::Player {
                                    hp_milli,
                                    max_hp_milli,
                                    downed,
                                    ..
                                } => Some(fold::pipeline::Keyed::new(
                                    0_u8,
                                    [*hp_milli, *max_hp_milli, i64::from(*downed)],
                                )),
                                _ => None,
                            }
                        },
                        fold::pipeline::Aggregate::new(
                            "director_party_aggregate",
                            |acc: &mut [i64; 3], value: &[i64; 3], delta| {
                                for index in 0..3 {
                                    acc[index] += value[index] * delta as i64;
                                }
                            },
                            fold::pipeline::terminal::Table::new("director_party"),
                        ),
                    ),
                    fold::pipeline::FilterMap::new(
                        |d: &fold::pipeline::Keyed<$crate::model::FactKey, $crate::model::Fact>| {
                            match &d.val {
                                $crate::model::Fact::Damage { total, .. } => {
                                    Some(fold::pipeline::Keyed::new(0_u8, *total))
                                }
                                _ => None,
                            }
                        },
                        fold::pipeline::Aggregate::new(
                            "director_damage_aggregate",
                            |acc: &mut i64, value: &u64, delta| {
                                *acc += *value as i64 * delta as i64
                            },
                            fold::pipeline::terminal::Table::new("director_damage"),
                        ),
                    ),
                ),
            ),
        )
    }};
}

#[macro_export]
macro_rules! read_bog_metrics {
    ($stream:expr, $commits:expr, $queries:expr, $compiles:expr) => {{
        $stream.rtx(
            |(
                _,
                players,
                (enemies, enemy_counts),
                damage,
                charms,
                loadouts,
                (party, total_damage),
            )| {
                let mut enemy_counts: Vec<($crate::model::EnemyKind, i64)> =
                    enemy_counts.iter().collect();
                enemy_counts.sort_by_key(|(kind, _)| match kind {
                    $crate::model::EnemyKind::Mireling => 0,
                    $crate::model::EnemyKind::Midge => 1,
                    $crate::model::EnemyKind::Spitter => 2,
                    $crate::model::EnemyKind::Brute => 3,
                    $crate::model::EnemyKind::Colossus => 4,
                });
                let mut charm_rows: Vec<($crate::model::CharmKind, i64)> = charms.iter().collect();
                charm_rows.sort_by_key(|(kind, _)| match kind {
                    $crate::model::CharmKind::Damage => 0,
                    $crate::model::CharmKind::Speed => 1,
                    $crate::model::CharmKind::Magnet => 2,
                });
                let top_damage = damage
                    .top(5)
                    .into_iter()
                    .map(|row| (row.val, row.score))
                    .collect();
                let _indexed_loadouts = loadouts.len();
                let party = party.get(&0_u8).unwrap_or([0, 0, 0]);
                $crate::model::BogMetrics {
                    fold_commits: $commits,
                    materialized_players: players.get(),
                    materialized_enemies: enemies.get(),
                    hnsw_queries: $queries,
                    ese_compiles: $compiles,
                    enemy_counts,
                    charms: charm_rows,
                    top_damage,
                    party_hp_milli: party[0],
                    party_max_hp_milli: party[1],
                    downed_players: party[2],
                    total_damage: total_damage.get(&0_u8).unwrap_or(0).max(0) as u64,
                }
            },
        )
    }};
}
