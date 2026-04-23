use std::path::Path;

pub use crate::collisions::components::*;
pub use crate::render::components::*;

use crate::prelude::*;

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct PlayerTag;

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Damage {
    #[serde(default)]
    pub heavy: i32,
    #[serde(default)]
    pub light: i32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Defence {
    #[serde(default)]
    pub heavy: i32,
    #[serde(default)]
    pub light: i32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct KnockbackTag;

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ProjectileTag {
    pub speed: f32,
    #[serde(default)]
    pub pierce_count: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Lifetime {
    pub time_left: f32,
}

impl Lifetime {
    pub fn from_time(time: f32) -> Self {
        Lifetime { time_left: time }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Team {
    Player,
    Enemy,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct KnockbackState {
    pub knockback_length: f32,
    pub knockback_speed: f32,
    #[serde(skip)]
    pub knockback_direction: Vec2,
    #[serde(skip)]
    pub knockback_left: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Hp {
    pub cooldown_length: f32,
    pub hp: i32,
    #[serde(skip)]
    pub cooldown: f32,
}

impl Hp {
    pub fn cooling_down(&self) -> bool {
        self.cooldown > 0.0
    }

    pub fn damage(&mut self, delta: i32) {
        if delta <= 0 {
            return;
        }
        if self.cooling_down() {
            return;
        }
        self.hp -= delta;
        self.cooldown = self.cooldown_length;
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct SpawnAtEdgesDirector {
    pub spawn_time: f32,
    #[serde(skip)]
    pub next_spawn: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct SpawnerOf {
    #[serde(skip, default = "entity_dangling")]
    pub director: Entity,
}

#[derive(Debug, Clone, Copy)]
pub struct Spawner {
    pub quota: u32,
    pub cooldown_length: f32,
    pub cooldown: f32,
    pub weight: u32,
    pub prefab: AssetKey,
}

impl Spawner {
    pub fn can_spawn(&self) -> bool {
        self.cooldown <= 0.0 && self.quota > 0
    }
}

impl DeserializeWithManifestCtx<Resources> for Spawner {
    type Manifest<'a> = MobSpawnerManifest<'a>;

    fn from_manifest(
        resources: &mut Resources,
        manifest: Self::Manifest<'_>,
    ) -> anyhow::Result<Self> {
        let Some(prefab) = resources.prefabs.resolve(manifest.prefab) else {
            anyhow::bail!("No such prefab: {:?}", manifest.prefab);
        };
        Ok(Spawner {
            quota: manifest.quota,
            cooldown_length: manifest.cooldown_length,
            cooldown: 0.0,
            weight: manifest.weight,
            prefab,
        })
    }

    fn deps(manifest: Self::Manifest<'_>) -> impl Iterator<Item = &'_ Path> {
        [manifest.prefab].into_iter()
    }
}

#[derive(Debug, Deserialize)]
pub struct MobSpawnerManifest<'a> {
    pub quota: u32,
    pub cooldown_length: f32,
    pub weight: u32,
    #[serde(borrow)]
    pub prefab: &'a Path,
}

#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub pos: Vec2,
    pub angle: f32,
}

impl Transform {
    pub const IDENTITY: Self = Self { pos: Vec2::ZERO, angle: 0.0 };

    pub fn from_unit(_: ()) -> Self {
        Self::IDENTITY
    }

    pub fn from_pos(pos: Vec2) -> Self {
        Self { pos, angle: 0.0 }
    }

    pub fn from_xy(x: f32, y: f32) -> Self {
        Self::from_pos(vec2(x, y))
    }
}
