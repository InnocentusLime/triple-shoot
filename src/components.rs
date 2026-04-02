use std::path::{Path, PathBuf};

use lib_game::{AssetKey, Resources};

use serde::Deserialize;

pub fn register_components(prefab_factory: &mut lib_game::PrefabFactory<Resources>) {
    prefab_factory.register_component_with_constructor_ctx(
        "player_arsenal",
        PlayerArsenalManifest::into_tag,
        PlayerArsenalManifest::dependencies,
    );
    prefab_factory.register_component_with_constructor_ctx(
        "enemy_spawner",
        EnemySpawnerManifest::into_tag,
        EnemySpawnerManifest::dependencies,
    );
    prefab_factory.register_component::<NpcAi>("npc");
}

#[derive(Debug, Clone, Copy)]
pub struct EnemySpawner {
    pub spawn_time: f32,
    pub next_spawn: f32,
    pub enemy_prefab: AssetKey,
}

#[derive(Debug, Deserialize)]
pub struct EnemySpawnerManifest {
    pub spawn_time: f32,
    pub enemy_prefab: PathBuf,
}

impl EnemySpawnerManifest {
    pub fn into_tag(self, resources: &mut Resources) -> anyhow::Result<EnemySpawner> {
        let Some(enemy_prefab) = resources.prefabs.resolve(&self.enemy_prefab) else {
            anyhow::bail!("No such prefab: {:?}", self.enemy_prefab);
        };
        Ok(EnemySpawner { spawn_time: self.spawn_time, next_spawn: self.spawn_time, enemy_prefab })
    }

    pub fn dependencies(data: &serde_json::value::RawValue) -> anyhow::Result<Vec<PathBuf>> {
        #[derive(Deserialize)]
        pub struct Deps<'a> {
            #[serde(borrow)]
            pub enemy_prefab: &'a Path,
        }
        let deps = serde_json::from_str::<Deps>(data.get())?;
        Ok(vec![deps.enemy_prefab.into()])
    }
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub enum NpcAi {
    JustFollowPlayer,
}

#[derive(Debug, Clone, Copy)]
pub struct PlayerArsenal {
    pub bullet_prefab: AssetKey,
}

#[derive(Debug, Deserialize)]
pub struct PlayerArsenalManifest {
    pub bullet_prefab: PathBuf,
}

impl PlayerArsenalManifest {
    pub fn into_tag(self, resources: &mut Resources) -> anyhow::Result<PlayerArsenal> {
        let Some(bullet_prefab) = resources.prefabs.resolve(&self.bullet_prefab) else {
            anyhow::bail!("No such prefab: {:?}", self.bullet_prefab);
        };
        Ok(PlayerArsenal { bullet_prefab })
    }

    pub fn dependencies(data: &serde_json::value::RawValue) -> anyhow::Result<Vec<PathBuf>> {
        #[derive(Deserialize)]
        pub struct Deps<'a> {
            #[serde(borrow)]
            pub bullet_prefab: &'a Path,
        }
        let deps = serde_json::from_str::<Deps>(data.get())?;
        Ok(vec![deps.bullet_prefab.into()])
    }
}
