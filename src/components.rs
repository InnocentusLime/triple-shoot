use std::path::Path;

use lib_game::{AssetKey, DeserializeWithManifestCtx, Resources, WeaponId};

use serde::Deserialize;

pub fn register_components(prefab_factory: &mut lib_game::PrefabFactory<Resources>) {
    prefab_factory.register_component_with_constructor_ctx::<PlayerData>("player_data");
    prefab_factory.register_component::<NpcAi>("npc");
    prefab_factory.register_component::<AmmoPickup>("ammo_pickup");
    prefab_factory.register_component::<Deployer>("deployer");
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct AmmoPickup {
    pub weapon: WeaponId,
    pub value: u32,
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
    pub shotgun: GunEntry,
    pub rifle: GunEntry,
}

impl PlayerData {
    pub fn get_gun(&self, gun_id: WeaponId) -> GunEntry {
        match gun_id {
            WeaponId::Shotgun => self.shotgun,
            WeaponId::Rifle => self.rifle,
        }
    }

    pub fn set_gun(&mut self, gun_id: WeaponId, new_gun: GunEntry) {
        match gun_id {
            WeaponId::Shotgun => self.shotgun = new_gun,
            WeaponId::Rifle => self.rifle = new_gun,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GunEntry {
    pub bullet_prefab: AssetKey,
    pub shoot_cooldown: f32,
    pub bullets_in_spread: u8,
    pub spread_angle: f32,
    pub max_ammo: u32,
    pub ammo: u32,
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
            shotgun: GunEntry {
                bullet_prefab: shotgun_bullet_prefab,
                shoot_cooldown: manifest.shotgun.shoot_cooldown,
                bullets_in_spread: manifest.shotgun.bullets_in_spread,
                spread_angle: manifest.shotgun.spread_angle,
                max_ammo: manifest.shotgun.max_ammo,
                ammo: 0,
            },
            rifle: GunEntry {
                bullet_prefab: rifle_bullet_prefab,
                shoot_cooldown: manifest.rifle.shoot_cooldown,
                bullets_in_spread: manifest.rifle.bullets_in_spread,
                spread_angle: manifest.rifle.spread_angle,
                max_ammo: manifest.rifle.max_ammo,
                ammo: 0,
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
    pub shotgun: GunEntryManifest<'a>,
    #[serde(borrow)]
    pub rifle: GunEntryManifest<'a>,
    pub speed: f32,
}

#[derive(Debug, Deserialize)]
pub struct GunEntryManifest<'a> {
    #[serde(borrow)]
    pub bullet_prefab: &'a Path,
    pub shoot_cooldown: f32,
    pub bullets_in_spread: u8,
    pub spread_angle: f32,
    pub max_ammo: u32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Deployer {
    #[serde(skip)]
    pub timer: f32,
    #[serde(skip)]
    pub prefab: AssetKey,
}

#[derive(Debug, Clone, Copy)]
pub struct GameOverCard;
