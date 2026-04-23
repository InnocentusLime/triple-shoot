use std::path::Path;

use crate::components::*;
use crate::prelude::*;

use serde::Deserialize;

pub fn spawn_prefab(
    cmds: &mut CommandBuffer,
    resources: &Resources,
    prefab: AssetKey,
    tf: Transform,
) -> Entity {
    let prefab = resources.prefabs.get(prefab).expect("Dangling prefab key");
    let ent = resources.world.reserve_entity();
    cmds.insert(ent, prefab);
    if prefab.has::<Transform>() {
        cmds.insert_one(ent, tf);
    }
    ent
}

pub fn register_libgame_components(prefab_factory: &mut PrefabFactory<Resources>) {
    prefab_factory.register_component_with_constructor("transform", Transform::from_unit);
    prefab_factory.register_component_with_constructor("lifetime", Lifetime::from_time);
    prefab_factory.register_component::<Shape>("shape");
    prefab_factory.register_component::<Hp>("hp");
    prefab_factory.register_component::<BodyTag>("body");
    prefab_factory.register_component::<KinematicControl>("kinematic");
    prefab_factory.register_component::<PlayerTag>("player");
    prefab_factory.register_component::<KnockbackState>("knockback");
    prefab_factory.register_component::<col_query::Level>("level_query");
    prefab_factory.register_component::<col_query::Damage>("damage_query");
    prefab_factory.register_component::<col_query::Pickup>("pickup_query");
    prefab_factory.register_component::<col_query::Interaction>("interaction_query");
    prefab_factory.register_component::<col_query::Grazing>("grazing_query");
    prefab_factory.register_component::<ProjectileTag>("projectile");
    prefab_factory.register_component::<Team>("team");
    prefab_factory.register_component::<KnockbackTag>("knockback_effect");
    prefab_factory.register_component::<Damage>("damage");
    prefab_factory.register_component::<Defence>("defence");
    prefab_factory.register_component::<SpawnAtEdgesDirector>("spawn_at_edges_director");
    prefab_factory.register_component::<SpawnAtCellsDirector>("spawn_at_cells_director");
    prefab_factory.register_component::<SpawnerOf>("spawner_of");

    prefab_factory.register_component_with_constructor_ctx::<Sprite>("sprite");
    prefab_factory.register_component_with_constructor_ctx::<Spawner>("spawner");
}

impl DeserializeWithManifestCtx<Resources> for Sprite {
    type Manifest<'a> = SpriteManifest<'a>;

    fn from_manifest(
        resources: &mut Resources,
        manifest: Self::Manifest<'_>,
    ) -> anyhow::Result<Self> {
        let Some(texture) = resources.textures.resolve(manifest.texture) else {
            anyhow::bail!("No such texture: {:?}", manifest.texture);
        };
        Ok(Sprite {
            layer: manifest.layer,
            texture,
            tex_rect_pos: manifest.tex_rect_pos,
            tex_rect_size: manifest.tex_rect_size,
            color: mimiq::WHITE,
            sort_offset: manifest.sort_offset,
            local_offset: manifest.local_offset,
        })
    }

    fn deps(manifest: Self::Manifest<'_>) -> impl Iterator<Item = &'_ Path> {
        [manifest.texture].into_iter()
    }
}

#[derive(Debug, Deserialize)]
pub struct SpriteManifest<'a> {
    pub layer: u32,
    #[serde(borrow)]
    pub texture: &'a Path,
    pub tex_rect_pos: UVec2,
    pub tex_rect_size: UVec2,
    pub sort_offset: f32,
    pub local_offset: Vec2,
}
