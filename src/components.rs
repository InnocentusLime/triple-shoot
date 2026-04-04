use std::path::Path;

use lib_game::{AssetKey, DeserializeWithManifestCtx, Resources};

use serde::Deserialize;

pub fn register_components(prefab_factory: &mut lib_game::PrefabFactory<Resources>) {
    prefab_factory.register_component_with_constructor_ctx::<PlayerData>("player_data");
    prefab_factory.register_component_with_constructor_ctx::<EnemySpawner>("enemy_spawner");
    prefab_factory.register_component::<NpcAi>("npc");
}

#[derive(Debug, Clone, Copy)]
pub struct EnemySpawner {
    pub spawn_time: f32,
    pub next_spawn: f32,
    pub enemy_prefab: AssetKey,
}

impl DeserializeWithManifestCtx<Resources> for EnemySpawner {
    type Manifest<'a> = EnemySpawnerManifest<'a>;

    fn from_manifest(
        resources: &mut Resources,
        manifest: Self::Manifest<'_>,
    ) -> anyhow::Result<Self> {
        let Some(enemy_prefab) = resources.prefabs.resolve(manifest.enemy_prefab) else {
            anyhow::bail!("No such prefab: {:?}", manifest.enemy_prefab);
        };
        Ok(EnemySpawner {
            spawn_time: manifest.spawn_time,
            next_spawn: manifest.spawn_time,
            enemy_prefab,
        })
    }

    fn deps(manifest: Self::Manifest<'_>) -> impl Iterator<Item = &'_ Path> {
        [manifest.enemy_prefab].into_iter()
    }
}

#[derive(Debug, Deserialize)]
pub struct EnemySpawnerManifest<'a> {
    pub spawn_time: f32,
    #[serde(borrow)]
    pub enemy_prefab: &'a Path,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(tag = "ai")]
pub enum NpcAi {
    JustFollowPlayer { speed: f32 },
}

#[derive(Debug, Clone, Copy)]
pub struct PlayerData {
    pub bullet_prefab: AssetKey,
    pub speed: f32,
    pub shoot_cooldown: f32,
    pub next_shoot: f32,
}

impl DeserializeWithManifestCtx<Resources> for PlayerData {
    type Manifest<'a> = PlayerDataManifest<'a>;

    fn from_manifest(
        resources: &mut Resources,
        manifest: Self::Manifest<'_>,
    ) -> anyhow::Result<Self> {
        let Some(bullet_prefab) = resources.prefabs.resolve(manifest.bullet_prefab) else {
            anyhow::bail!("No such prefab: {:?}", manifest.bullet_prefab);
        };
        Ok(PlayerData {
            bullet_prefab,
            speed: manifest.speed,
            shoot_cooldown: manifest.shoot_cooldown,
            next_shoot: 0.0,
        })
    }

    fn deps(manifest: Self::Manifest<'_>) -> impl Iterator<Item = &'_ Path> {
        [manifest.bullet_prefab].into_iter()
    }
}

#[derive(Debug, Deserialize)]
pub struct PlayerDataManifest<'a> {
    #[serde(borrow)]
    pub bullet_prefab: &'a Path,
    pub speed: f32,
    pub shoot_cooldown: f32,
}
