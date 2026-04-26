mod ai;
mod player;
mod spawning;
mod spawning_cfg;

use std::path::PathBuf;

use crate::prelude::*;

const PLAYER: &str = "prefab/player.json";
const WALL_HORIZ: &str = "prefab/wall_horiz.json";
const WALL_VERT: &str = "prefab/wall_vert.json";
const FOLLOWER: &str = "prefab/enemy/follower.json";
const SHOTGUN_PICKUP: &str = "prefab/shotgun_pickup.json";
const DEPLOYER: &str = "prefab/deployer.json";

pub struct MainGame {
    reset_confirmed: bool,

    /* Wave info */
    wave: spawning::Wave,
    deployer_prefab: AssetKey,

    /* Debug flags */
    do_ai: bool,
    do_player_controls: bool,
}

impl MainGame {
    pub fn make_state_request() -> StateRequest {
        let dependencies = [
            PLAYER,
            WALL_HORIZ,
            WALL_VERT,
            FOLLOWER,
            SHOTGUN_PICKUP,
            DEPLOYER,
            "atlas/grad.png",
            "atlas/ui.png",
            "atlas/game_over.png",
        ];

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

        let wave = spawning::Wave::new(
            [resources.prefabs.resolve(SHOTGUN_PICKUP).unwrap()],
            [
                resources.prefabs.resolve(FOLLOWER).unwrap(),
                resources.prefabs.resolve(FOLLOWER).unwrap(),
            ],
        );
        let deployer_prefab = resources.prefabs.resolve(DEPLOYER).unwrap();

        Box::new(MainGame {
            reset_confirmed: false,
            wave,
            deployer_prefab,
            do_player_controls: true,
            do_ai: true,
        })
    }

    fn decide_reset(&self, world: &World) -> Option<StateRequest> {
        if world_has_player(world) || !self.reset_confirmed {
            return None;
        }
        Some(MainGame::make_state_request())
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

        if self.wave.is_complete(&resources.world) && !self.wave.next_wave() {
            return Some(crate::win::Win::make_state_request());
        }

        if should_spawn_game_over(&resources.world) {
            let over_card = resources.textures.resolve("atlas/game_over.png").unwrap();
            cmds.spawn((
                Transform::from_xy(SCREEN_WIDTH as f32 / 2.0, SCREEN_HEIGHT as f32 / 2.0),
                Sprite {
                    layer: 0,
                    texture: over_card,
                    tex_rect_pos: uvec2(0, 0),
                    tex_rect_size: uvec2(256, 256),
                    color: Color::from_hex(0xffffffff),
                    sort_offset: 0.0,
                    local_offset: Vec2::ZERO,
                },
                GameOverCard,
            ));
        }

        ai::sync_gfx(resources);
        self.decide_reset(&resources.world)
    }

    fn input(
        &mut self,
        dt: f32,
        input_model: &InputModel,
        resources: &mut Resources,
        cmds: &mut CommandBuffer,
    ) {
        player::input(dt, input_model, resources, cmds);
        ai::think(dt, resources);
        ai::boid_steering(resources);
        spawning::tick(&mut self.wave, self.deployer_prefab, dt, resources, cmds);
        self.reset_confirmed = player_wants_restart(&resources.world, input_model);
    }

    fn ui(&mut self, resources: &mut Resources, out: &mut Vec<UiElement>) {
        let Some((_, (hp, data))) = resources
            .world
            .query_mut::<(&Hp, &PlayerData)>()
            .into_iter()
            .next()
        else {
            return;
        };
        let center = vec2(SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32) / 2.0;
        let cooling_down = data.next_shoot > 0.0;
        let ammo_tint = if cooling_down {
            Color::from_vec4(Vec3::splat(0.6).extend(1.0))
        } else {
            Color::from_vec4(vec4(1.0, 1.0, 1.0, 1.0))
        };
        let ammo_pos = center + vec2(resources.game_field_width, resources.game_field_height) / 2.0;
        let hp_pos = center + vec2(-resources.game_field_width, -resources.game_field_height) / 2.0;

        out.extend([
            UiElement {
                tint: ammo_tint,
                ty: UiElementType::StackCounter {
                    val: data.shotgun.ammo,
                    max_val: data.shotgun.max_ammo,
                    tex_rect_pos: uvec2(9, 3),
                    tex_rect_size: uvec2(72, 32),
                    direction: StackDirection::Up,
                    spacing: -6.0,
                },
                anchoring: Anchoring::LeftBot,
                pos: ammo_pos,
            },
            UiElement {
                tint: Color::from_vec4(vec4(1.0, 1.0, 1.0, 1.0)),
                ty: UiElementType::StackCounter {
                    val: hp.hp.max(0) as u32,
                    max_val: 3,
                    tex_rect_pos: uvec2(22, 45),
                    tex_rect_size: uvec2(19, 17),
                    direction: StackDirection::Left,
                    spacing: 4.0,
                },
                anchoring: Anchoring::Right,
                pos: hp_pos,
            },
        ]);

        if cooling_down {
            let progress = 1.0 - data.next_shoot / data.shotgun.shoot_cooldown;
            out.push(UiElement {
                tint: Color::from_vec4(vec4(1.0, 1.0, 1.0, 1.0)),
                ty: UiElementType::CircleFill { progress },
                anchoring: Anchoring::Bot,
                pos: ammo_pos + vec2(36.0, 0.0),
            });
        }
    }
}

fn should_spawn_game_over(world: &World) -> bool {
    let has_gameover = world.query::<&GameOverCard>().into_iter().next().is_some();
    !has_gameover && !world_has_player(world)
}

fn player_wants_restart(world: &World, input_model: &InputModel) -> bool {
    !world_has_player(world) && input_model.shoot_down
}

fn world_has_player(world: &World) -> bool {
    world.query::<&PlayerTag>().into_iter().next().is_some()
}
