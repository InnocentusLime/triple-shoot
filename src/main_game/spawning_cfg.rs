use crate::prelude::*;

pub const ENEMY_TYPE_COUNT: usize = 2;
pub const PICKUP_TYPE_COUNT: usize = 1;

pub static WAVES: [WaveCfg; 3] = [
    WaveCfg {
        is_pickup_wave: true,
        pickup_wait: 0.2,
        pickups: [SpawnEntryCfg { wait: 0.1, weight: 1, quota: 3 }],
        max_pickups_on_screen: 1,
        enemies_wait: 1.0,
        enemies: [disabled(), disabled()],
        max_enemies_on_screen: 0,
    },
    WaveCfg {
        is_pickup_wave: false,
        pickup_wait: 0.4,
        pickups: [SpawnEntryCfg { wait: 0.8, weight: 1, quota: 2 }],
        max_pickups_on_screen: 1,
        enemies_wait: 0.5,
        enemies: [SpawnEntryCfg { wait: 0.5, weight: 1, quota: 4 }, disabled()],
        max_enemies_on_screen: 2,
    },
    WaveCfg {
        is_pickup_wave: false,
        pickup_wait: 1.0,
        pickups: [SpawnEntryCfg { wait: 0.8, weight: 1, quota: 6 }],
        max_pickups_on_screen: 2,
        enemies_wait: 0.5,
        enemies: [
            SpawnEntryCfg { wait: 0.5, weight: 1, quota: 18 },
            SpawnEntryCfg { wait: 3.0, weight: 3, quota: 7 },
        ],
        max_enemies_on_screen: 20,
    },
];

#[derive(Debug, Clone, Copy)]
pub struct WaveCfg {
    pub is_pickup_wave: bool,

    pub pickup_wait: f32,
    pub pickups: [SpawnEntryCfg; PICKUP_TYPE_COUNT],
    pub max_pickups_on_screen: usize,

    pub enemies_wait: f32,
    pub enemies: [SpawnEntryCfg; ENEMY_TYPE_COUNT],
    pub max_enemies_on_screen: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct SpawnEntryCfg {
    pub wait: f32,
    pub weight: u32,
    pub quota: u32,
}

const fn disabled() -> SpawnEntryCfg {
    SpawnEntryCfg { wait: 1.0, weight: 1, quota: 0 }
}
