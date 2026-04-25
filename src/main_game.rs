use std::path::PathBuf;

use crate::prelude::*;

const ENEMY_TYPE_COUNT: usize = 1;
const PICKUP_TYPE_COUNT: usize = 1;

const PLAYER: &str = "prefab/player.json";
const WALL_HORIZ: &str = "prefab/wall_horiz.json";
const WALL_VERT: &str = "prefab/wall_vert.json";
const LIGHT: &str = "prefab/light.json";
const SHOTGUN_PICKUP: &str = "prefab/shotgun_pickup.json";

pub struct MainGame {
    /* Wave info */
    wave: Wave,

    /* Debug flags */
    do_ai: bool,
    do_player_controls: bool,
}

impl MainGame {
    pub fn make_state_request() -> StateRequest {
        let dependencies = [PLAYER, WALL_HORIZ, WALL_VERT, LIGHT, SHOTGUN_PICKUP];

        StateRequest {
            name: "main game",
            constructor: Box::new(Self::new_state),
            dependencies: dependencies.into_iter().map(PathBuf::from).collect(),
        }
    }

    pub fn new_state(resources: &mut Resources, cmds: &mut CommandBuffer) -> Box<dyn State> {
        let wall_horiz = resources.prefabs.resolve(WALL_HORIZ).unwrap();
        let wall_vert = resources.prefabs.resolve(WALL_VERT).unwrap();
        let center = vec2(SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32) / 2.0;
        let play_off = vec2(resources.game_field_width, resources.game_field_height) * 0.5;

        spawn_prefab(
            cmds,
            resources,
            wall_horiz,
            Transform::from_pos(center - play_off * Vec2::Y),
        );
        spawn_prefab(
            cmds,
            resources,
            wall_horiz,
            Transform::from_pos(center + play_off * Vec2::Y),
        );
        spawn_prefab(
            cmds,
            resources,
            wall_vert,
            Transform::from_pos(center - play_off * Vec2::X),
        );
        spawn_prefab(
            cmds,
            resources,
            wall_vert,
            Transform::from_pos(center + play_off * Vec2::X),
        );

        let player_prefab = resources.prefabs.resolve(PLAYER).unwrap();
        spawn_prefab(cmds, resources, player_prefab, Transform::from_pos(center));

        let wave = Wave::new(
            [resources.prefabs.resolve(SHOTGUN_PICKUP).unwrap()],
            [resources.prefabs.resolve(LIGHT).unwrap()],
        );

        Box::new(MainGame { wave, do_player_controls: true, do_ai: true })
    }
}

impl State for MainGame {
    fn handle_command(&mut self, _resources: &mut Resources, cmd: &DebugCommand) -> bool {
        match &*cmd.command {
            "nopl" => self.do_player_controls = false,
            "pl" => self.do_player_controls = true,
            "noai" => self.do_ai = false,
            "ai" => self.do_ai = true,
            _ => return false,
        }
        true
    }

    fn plan_collision_queries(
        &mut self,
        _dt: f32,
        _resources: &mut Resources,
        _cmds: &mut CommandBuffer,
    ) {
    }

    fn update(
        &mut self,
        _dt: f32,
        resources: &mut Resources,
        _collisions: &CollisionSolver,
        cmds: &mut CommandBuffer,
    ) -> Option<StateRequest> {
        for (ent, (col_q, ammo)) in &mut resources
            .world
            .query::<(&col_query::Interaction, &AmmoPickup)>()
        {
            if !col_q.has_collided() {
                continue;
            }
            cmds.despawn(ent);
            for (_, data) in &mut resources.world.query::<&mut PlayerData>() {
                let gun = data.get_gun(ammo.weapon);
                let new_ammo = gun.max_ammo.min(gun.ammo + ammo.value);
                data.set_gun(ammo.weapon, GunEntry { ammo: new_ammo, ..gun });
            }
        }

        if self.wave.is_complete(&resources.world) {
            self.wave.next_wave();
        }

        None
    }

    fn input(
        &mut self,
        dt: f32,
        input_model: &InputModel,
        resources: &mut Resources,
        cmds: &mut CommandBuffer,
    ) {
        let mut player_pos = Vec2::ZERO;
        let mut query = resources
            .world
            .query::<(&mut Transform, &mut KinematicControl, &mut PlayerData)>();
        for (_, (tf, kin, data)) in &mut query {
            kin.dr = data.speed * dt * input_model.player_move_direction;
            let spawn_pos = tf.pos + 8.0 * input_model.player_aim_direction;
            player_pos = tf.pos;

            dump!("Player weapn: {:?}", data.current_weapon);
            dump!("Shotgun: {} / {}", data.shotgun.ammo, data.shotgun.max_ammo);
            dump!("Rifle: {} / {}", data.rifle.ammo, data.rifle.max_ammo);
            dump!(
                "Cooldown: {:.2}. Can shoot: {}",
                data.next_shoot,
                data.next_shoot <= 0.0
            );

            if let Some(weapon_request) = input_model.player_weapon_request
                && data.next_shoot <= 0.0
            {
                data.current_weapon = weapon_request
            }

            if input_model.shoot_down && data.next_shoot <= 0.0 {
                shoot(
                    resources,
                    data,
                    cmds,
                    input_model.player_aim_direction,
                    spawn_pos,
                );
            }

            if data.next_shoot > 0.0 {
                data.next_shoot -= dt;
            }
        }
        std::mem::drop(query);

        let mut query = resources
            .world
            .query::<(&Transform, &mut KinematicControl, &NpcAi)>();
        for (this, (tf, kin, ai)) in &mut query {
            match ai {
                NpcAi::JustFollowPlayer { speed } => {
                    let walk_dir = (player_pos - tf.pos).normalize_or_zero();
                    let steer_dir = steer_dir(&resources.world, this, tf.pos);
                    let move_dir = (0.4 * walk_dir + 0.8 * steer_dir).normalize_or_zero();

                    kin.dr = (*speed * dt) * move_dir;
                }
            }
        }

        dump!("wave: {:#.2?}", self.wave);
        if let Some(prefab) = self.wave.pickups.tick(dt) {
            let pos =
                make_random_spawn_cell(resources.game_field_width, resources.game_field_height);
            spawn_prefab(cmds, resources, prefab, Transform::from_pos(pos));
        }
        if let Some(prefab) = self.wave.enemies.tick(dt) {
            let pos =
                make_random_spawn_pos(resources.game_field_width, resources.game_field_height);
            spawn_prefab(cmds, resources, prefab, Transform::from_pos(pos));
        }
    }
}

fn shoot(
    resources: &Resources,
    data: &mut PlayerData,
    cmds: &mut CommandBuffer,
    aim_dir: Vec2,
    spawn_pos: Vec2,
) {
    let gun = data.get_gun(data.current_weapon);
    if gun.ammo == 0 {
        return;
    }

    let spread_angle = gun.spread_angle.to_radians();
    let base = -spread_angle / 2.0;
    let delta = spread_angle / (gun.bullets_in_spread as f32);
    for offset_idx in 0..gun.bullets_in_spread {
        let offset = base + (offset_idx as f32) * delta;
        spawn_prefab(
            cmds,
            resources,
            gun.bullet_prefab,
            Transform { pos: spawn_pos, angle: aim_dir.to_angle() + offset },
        );
    }

    data.next_shoot = gun.shoot_cooldown;
    data.set_gun(data.current_weapon, GunEntry { ammo: gun.ammo - 1, ..gun });
}

fn steer_dir(world: &World, this: Entity, pos: Vec2) -> Vec2 {
    const SEPARATION_RADIUS: f32 = 20.0;

    let mut result = Vec2::ZERO;
    for (other, (tf, team)) in &mut world.query::<(&Transform, &Team)>() {
        if *team != Team::Enemy {
            continue;
        }
        if other == this {
            continue;
        }

        let dr = pos - tf.pos;
        let dist = dr.length();
        if dist < SEPARATION_RADIUS {
            result += dr.normalize_or_zero();
        }
    }

    result.normalize_or_zero()
}

static WAVES: [WaveCfg; 2] = [
    WaveCfg {
        is_pickup_wave: true,
        pickup_wait: 1.0,
        pickups: [SpawnEntryCfg { wait: 0.8, weight: 1, quota: 3 }],
        enemies_wait: 1.0,
        enemies: [SpawnEntryCfg::disabled()],
    },
    WaveCfg {
        is_pickup_wave: false,
        pickup_wait: 1.0,
        pickups: [SpawnEntryCfg { wait: 1.0, weight: 1, quota: 10 }],
        enemies_wait: 1.0,
        enemies: [SpawnEntryCfg { wait: 1.0, weight: 1, quota: 20 }],
    },
];

#[derive(Debug, Clone, Copy)]
struct WaveCfg {
    is_pickup_wave: bool,
    pickup_wait: f32,
    pickups: [SpawnEntryCfg; PICKUP_TYPE_COUNT],
    enemies_wait: f32,
    enemies: [SpawnEntryCfg; ENEMY_TYPE_COUNT],
}

#[derive(Debug, Clone, Copy)]
struct SpawnEntryCfg {
    wait: f32,
    weight: u32,
    quota: u32,
}

impl SpawnEntryCfg {
    const fn disabled() -> SpawnEntryCfg {
        SpawnEntryCfg { wait: 1.0, weight: 1, quota: 0 }
    }
}

#[derive(Debug)]
struct Wave {
    wave_id: usize,
    pickups: Spawner<PICKUP_TYPE_COUNT>,
    enemies: Spawner<ENEMY_TYPE_COUNT>,
}

impl Wave {
    fn new(
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

    fn is_complete(&self, world: &World) -> bool {
        if !self.are_spawners_empty() {
            return false;
        }

        if WAVES[self.wave_id].is_pickup_wave {
            world
                .query::<&AmmoPickup>()
                .iter()
                .next()
                .is_none()
        } else {
            world.query::<()>().with::<&NpcAi>().iter().next().is_none()
        }
    }

    fn next_wave(&mut self) {
        self.wave_id += 1;
        info!("next wave: {}", self.wave_id);
        if self.wave_id >= WAVES.len() {
            self.wave_id = WAVES.len() - 1
        }
        self.apply_cfg(WAVES[self.wave_id]);
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
        if self.wave_id == 0 {
            return true;
        }
        if self.wave_id == 1 {
            return self.pickups.entries.iter().all(|x| x.quota == 0);
        }
        self.enemies.entries.iter().all(|x| x.quota == 0)
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
