use std::path::Path;

use lib_game::{AssetKey, DeserializeWithManifestCtx, Resources, WeaponId};

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
    pub speed: f32,
    pub next_shoot: f32,
    pub current_weapon: WeaponId,
    pub shotgun: ShotgunEntry,
    pub rifle: RifleEntry,
}

#[derive(Debug, Clone, Copy)]
pub struct ShotgunEntry {
    pub bullet_prefab: AssetKey,
    pub shoot_cooldown: f32,
    pub bullets_in_spread: u8,
    pub spread_angle: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct RifleEntry {
    pub bullet_prefab: AssetKey,
    pub shoot_cooldown: f32,
}

impl DeserializeWithManifestCtx<Resources> for PlayerData {
    type Manifest<'a> = PlayerDataManifest<'a>;

    fn from_manifest(
        resources: &mut Resources,
        manifest: Self::Manifest<'_>,
    ) -> anyhow::Result<Self> {
        let Some(shotgun_bullet_prefab) = resources.prefabs.resolve(manifest.shotgun.bullet_prefab)
        else {
            anyhow::bail!("No such prefab: {:?}", manifest.shotgun.bullet_prefab);
        };
        let Some(rifle_bullet_prefab) = resources.prefabs.resolve(manifest.rifle.bullet_prefab)
        else {
            anyhow::bail!("No such prefab: {:?}", manifest.rifle.bullet_prefab);
        };
        Ok(PlayerData {
            speed: manifest.speed,
            next_shoot: 0.0,
            current_weapon: WeaponId::Shotgun,
            shotgun: ShotgunEntry {
                bullet_prefab: shotgun_bullet_prefab,
                shoot_cooldown: manifest.shotgun.shoot_cooldown,
                bullets_in_spread: manifest.shotgun.bullets_in_spread,
                spread_angle: manifest.shotgun.spread_angle,
            },
            rifle: RifleEntry {
                bullet_prefab: rifle_bullet_prefab,
                shoot_cooldown: manifest.rifle.shoot_cooldown,
            },
        })
    }

    fn deps(manifest: Self::Manifest<'_>) -> impl Iterator<Item = &'_ Path> {
        [manifest.shotgun.bullet_prefab, manifest.rifle.bullet_prefab].into_iter()
    }
}

#[derive(Debug, Deserialize)]
pub struct PlayerDataManifest<'a> {
    #[serde(borrow)]
    pub shotgun: ShotgunEntryManifest<'a>,
    #[serde(borrow)]
    pub rifle: RifleEntryManifest<'a>,
    pub speed: f32,
}

#[derive(Debug, Deserialize)]
pub struct ShotgunEntryManifest<'a> {
    #[serde(borrow)]
    pub bullet_prefab: &'a Path,
    pub shoot_cooldown: f32,
    pub bullets_in_spread: u8,
    pub spread_angle: f32,
}

#[derive(Debug, Deserialize)]
pub struct RifleEntryManifest<'a> {
    #[serde(borrow)]
    pub bullet_prefab: &'a Path,
    pub shoot_cooldown: f32,
}
