use std::path::{Path, PathBuf};

use crate::components::*;
use crate::prelude::*;

use serde::Deserialize;

pub fn spawn_prefab(
    cmds: &mut CommandBuffer,
    resources: &Resources,
    prefab: AssetKey,
    tf: Transform,
) {
    let prefab = resources.prefabs.get(prefab).expect("Dangling prefab key");
    let ent = resources.world.reserve_entity();
    cmds.insert(ent, prefab);
    if prefab.has::<Transform>() {
        cmds.insert_one(ent, tf);
    }
}

pub fn register_libgame_components(prefab_factory: &mut PrefabFactory<Resources>) {
    prefab_factory.register_component_with_constructor("transform", Transform::from_pos);
    prefab_factory.register_component_with_constructor("hp", Hp::new);
    prefab_factory.register_component::<BodyTag>("body");
    prefab_factory.register_component::<KinematicControl>("kinematic");
    prefab_factory.register_component::<PlayerTag>("player");
    prefab_factory.register_component::<col_query::Level>("level_query");
    prefab_factory.register_component::<col_query::Damage>("damage_query");
    prefab_factory.register_component::<col_query::Pickup>("pickup_query");
    prefab_factory.register_component::<col_query::Interaction>("interaction_query");
    prefab_factory.register_component::<col_query::Grazing>("grazing_query");
    prefab_factory.register_component::<ProjectileTag>("projectile");
    prefab_factory.register_component::<Team>("team");

    prefab_factory.register_component_with_constructor_ctx(
        "sprite",
        SpriteManifest::into_sprite,
        SpriteManifest::dependencies,
    );
}

#[derive(Debug, Deserialize)]
pub struct SpriteManifest {
    pub layer: u32,
    pub texture: PathBuf,
    pub tex_rect_pos: UVec2,
    pub tex_rect_size: UVec2,
    pub sort_offset: f32,
    pub local_offset: Vec2,
}

impl SpriteManifest {
    pub fn into_sprite(self, resources: &mut Resources) -> anyhow::Result<Sprite> {
        let Some(texture) = resources.textures.resolve(&self.texture) else {
            anyhow::bail!("No such texture: {:?}", self.texture);
        };
        Ok(Sprite {
            layer: self.layer,
            texture,
            tex_rect_pos: self.tex_rect_pos,
            tex_rect_size: self.tex_rect_size,
            color: mimiq::WHITE,
            sort_offset: self.sort_offset,
            local_offset: self.local_offset,
        })
    }

    pub fn dependencies(data: &serde_json::value::RawValue) -> anyhow::Result<Vec<PathBuf>> {
        #[derive(Deserialize)]
        pub struct Deps<'a> {
            #[serde(borrow)]
            pub texture: &'a Path,
        }
        let deps = serde_json::from_str::<Deps>(data.get())?;
        Ok(vec![deps.texture.into()])
    }
}
