use crate::main_game::spawning_cfg::*;
use crate::prelude::*;

const DEPLOYER_TIME: f32 = 1.0;

pub fn tick(
    wave: &mut Wave,
    deployer_prefab: AssetKey,
    dt: f32,
    resources: &Resources,
    cmds: &mut CommandBuffer,
) {
    dump!("wave: {:#.2?}", wave);
    if let Some(prefab) = wave.tick_pickup_spawns(dt, &resources.world) {
        let pos = make_random_spawn_cell(resources.game_field_width, resources.game_field_height);
        spawn_prefab(cmds, resources, prefab, Transform::from_pos(pos));
    }
    if let Some(prefab) = wave.tick_enemy_spawns(dt, &resources.world) {
        let pos = make_random_spawn_pos(resources.game_field_width, resources.game_field_height);
        let deployer = spawn_prefab(cmds, resources, deployer_prefab, Transform::from_pos(pos));
        cmds.insert_one(deployer, Deployer { timer: DEPLOYER_TIME, prefab });
    }
    for (deployer_entity, (tf, deployer)) in
        &mut resources.world.query::<(&Transform, &mut Deployer)>()
    {
        deployer.timer -= dt;
        if deployer.timer <= 0.0 {
            cmds.despawn(deployer_entity);
            spawn_prefab(cmds, resources, deployer.prefab, *tf);
        }
    }
}

#[derive(Debug)]
pub struct Wave {
    wave_id: usize,
    pickups: Spawner<PICKUP_TYPE_COUNT>,
    enemies: Spawner<ENEMY_TYPE_COUNT>,
}

impl Wave {
    pub fn new(
        pickup_prefabs: [AssetKey; PICKUP_TYPE_COUNT],
        enemy_prefabs: [AssetKey; ENEMY_TYPE_COUNT],
    ) -> Wave {
        let mut res = Wave {
            wave_id: 0,
            pickups: Spawner::new(1.0, pickup_prefabs.map(SpawnEntry::new)),
            enemies: Spawner::new(1.0, enemy_prefabs.map(SpawnEntry::new)),
        };
        res.apply_cfg(WAVES[0]);
        res
    }

    fn tick_pickup_spawns(&mut self, dt: f32, world: &World) -> Option<AssetKey> {
        let pickup_count = world.query::<&AmmoPickup>().into_iter().count();
        if pickup_count < WAVES[self.wave_id].max_pickups_on_screen {
            self.pickups.tick(dt)
        } else {
            None
        }
    }

    fn tick_enemy_spawns(&mut self, dt: f32, world: &World) -> Option<AssetKey> {
        let enemy_count = world.query::<Or<&NpcAi, &Deployer>>().into_iter().count();
        if enemy_count < WAVES[self.wave_id].max_enemies_on_screen {
            self.enemies.tick(dt)
        } else {
            None
        }
    }

    pub fn is_complete(&self, world: &World) -> bool {
        if !self.are_spawners_empty() {
            return false;
        }

        if WAVES[self.wave_id].is_pickup_wave {
            world.query::<&AmmoPickup>().iter().next().is_none()
        } else {
            world
                .query::<Or<&NpcAi, &Deployer>>()
                .iter()
                .next()
                .is_none()
        }
    }

    pub fn next_wave(&mut self) -> bool {
        self.wave_id += 1;
        info!("next wave: {}", self.wave_id);
        if self.wave_id >= WAVES.len() {
            return false;
        }
        self.apply_cfg(WAVES[self.wave_id]);
        true
    }

    fn apply_cfg(&mut self, cfg: WaveCfg) {
        self.enemies.wait = cfg.enemies_wait;
        self.enemies
            .entries
            .iter_mut()
            .zip(cfg.enemies)
            .for_each(|(dst, src)| dst.apply_cfg(src));

        self.pickups.wait = cfg.pickup_wait;
        self.pickups
            .entries
            .iter_mut()
            .zip(cfg.pickups)
            .for_each(|(dst, src)| dst.apply_cfg(src));
    }

    fn are_spawners_empty(&self) -> bool {
        if WAVES[self.wave_id].is_pickup_wave {
            self.pickups.entries.iter().all(|x| x.quota == 0)
        } else {
            self.enemies.entries.iter().all(|x| x.quota == 0)
        }
    }
}

#[derive(Debug)]
struct Spawner<const N: usize> {
    timer: f32,
    wait: f32,
    entries: [SpawnEntry; N],
}

impl<const N: usize> Spawner<N> {
    fn new(wait: f32, entries: [SpawnEntry; N]) -> Self {
        Spawner { timer: 0.0, wait, entries }
    }

    fn tick(&mut self, dt: f32) -> Option<AssetKey> {
        self.timer -= dt;
        for entry in &mut self.entries {
            if entry.timer >= 0.0 {
                entry.timer -= dt;
            }
        }
        if self.timer > 0.0 {
            return None;
        }
        self.timer = self.wait;
        let total_weight = self
            .entries
            .iter()
            .filter(|x| x.is_active())
            .map(|x| x.weight)
            .sum();
        if total_weight == 0 {
            return None;
        }

        let mut chosen_weight = fastrand::u32(0..total_weight);
        for entry in &mut self.entries {
            if !entry.is_active() {
                continue;
            }
            if chosen_weight < entry.weight {
                entry.quota -= 1;
                entry.timer = entry.wait;
                return Some(entry.prefab);
            }
            chosen_weight -= entry.weight;
        }

        None
    }
}

#[derive(Debug)]
struct SpawnEntry {
    timer: f32,
    wait: f32,
    weight: u32,
    quota: u32,
    prefab: AssetKey,
}

impl SpawnEntry {
    fn new(prefab: AssetKey) -> Self {
        SpawnEntry { timer: 0.0, wait: 1.0, weight: 1, quota: 0, prefab }
    }

    fn is_active(&self) -> bool {
        self.timer <= 0.0 && self.quota > 0
    }

    fn apply_cfg(&mut self, cfg: SpawnEntryCfg) {
        *self = SpawnEntry {
            timer: 0.0,
            wait: cfg.wait,
            weight: cfg.weight,
            quota: cfg.quota,
            prefab: self.prefab,
        };
    }
}

fn make_random_spawn_cell(game_field_width: f32, game_field_height: f32) -> Vec2 {
    const CELL_SIZE: f32 = 64.0;

    let center = vec2(SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32) / 2.0;
    let off_to_start = vec2(game_field_width, game_field_height) / 2.0;
    let start = center - off_to_start;
    let cells_horiz = (game_field_width / CELL_SIZE).floor() as u32;
    let cell_vert = (game_field_height / CELL_SIZE).floor() as u32;

    let cell_x = fastrand::u32(0..cells_horiz) as f32;
    let cell_y = fastrand::u32(0..cell_vert) as f32;
    start + vec2(cell_x, cell_y) * CELL_SIZE + Vec2::splat(CELL_SIZE / 2.0)
}

fn make_random_spawn_pos(game_field_width: f32, game_field_height: f32) -> Vec2 {
    const BUMP: f32 = 32.0;

    let center = vec2(SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32) / 2.0;
    let off = vec2(game_field_width, game_field_height) / 2.0 - Vec2::splat(BUMP);
    let noise_increment = fastrand::i32(-3..3);
    let spawnpoints = [
        center - Vec2::X * off + Vec2::Y * (32.0 * noise_increment as f32),
        center + Vec2::X * off + Vec2::Y * (32.0 * noise_increment as f32),
        center - Vec2::Y * off + Vec2::X * (32.0 * noise_increment as f32),
        center + Vec2::Y * off + Vec2::X * (32.0 * noise_increment as f32),
    ];
    fastrand::choice(spawnpoints).unwrap()
}
