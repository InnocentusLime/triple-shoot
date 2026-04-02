use std::{marker::PhantomData, path::Path, rc::Rc};

use anyhow::Context;
use hashbrown::{HashMap, HashSet};
use hecs::{Component, DynamicBundleClone, EntityBuilderClone};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::value::RawValue as RawJsonValue;

#[derive(Deserialize)]
#[serde(transparent)]
pub struct PrePrefab<'a>(#[serde(borrow)] pub HashMap<&'a str, &'a RawJsonValue>);

pub trait DeserializeWithManifestCtx<T>: Sized + 'static {
    type Manifest<'a>: Deserialize<'a>;

    fn from_manifest(ctx: &mut T, manifest: Self::Manifest<'_>) -> anyhow::Result<Self>;
    fn deps(manifest: Self::Manifest<'_>) -> impl Iterator<Item = &'_ Path>;
}

pub struct PrefabFactory<T> {
    registry: HashMap<String, ComponentEntry<T>>,
    _phantom: PhantomData<fn(&mut T)>,
}

impl<T: 'static> PrefabFactory<T> {
    pub fn new() -> Self {
        PrefabFactory { registry: HashMap::new(), _phantom: PhantomData }
    }

    pub fn register_bundle<B: DeserializeOwned + Clone + DynamicBundleClone>(&mut self, key: &str) {
        self.register_no_dpeps::<B>(key, |_, builder, x| {
            for id in builder.component_types() {
                // NOTE: using implementation details. Not good.
                if x.with_ids(|types| types.contains(&id)) {
                    anyhow::bail!("component already inserted")
                }
            }

            builder.add_bundle(x);
            Ok(())
        });
    }

    pub fn register_component<C: DeserializeOwned + Clone + Component>(&mut self, key: &str) {
        self.register_no_dpeps::<C>(key, |_, builder, x| {
            if builder.has::<C>() {
                anyhow::bail!("component already inserted")
            } else {
                builder.add(x);
                Ok(())
            }
        });
    }

    pub fn register_component_with_constructor_ctx<C>(&mut self, key: &str)
    where
        C: Clone + Component + DeserializeWithManifestCtx<T>,
    {
        if self.registry.contains_key(key) {
            panic!("duplicate key {key:?}");
        }

        let entry = ComponentEntry {
            deps: Box::new(move |val, container| {
                let manifest = serde_json::from_str(val.get())?;
                container.extend(C::deps(manifest).map(Rc::from));
                Ok(())
            }),
            builder: Box::new(move |ctx, builder, val| {
                let manifest = serde_json::from_str(val.get())?;
                builder.add(C::from_manifest(ctx, manifest)?);
                Ok(())
            }),
        };

        self.registry.insert(key.to_string(), entry);
    }

    pub fn register_component_with_constructor<Seed: DeserializeOwned, C: Clone + Component>(
        &mut self,
        key: &str,
        constructor: impl Fn(Seed) -> C + 'static,
    ) {
        self.register_no_dpeps::<Seed>(key, move |_, builder, x| {
            if builder.has::<C>() {
                anyhow::bail!("component already inserted")
            } else {
                builder.add(constructor(x));
                Ok(())
            }
        });
    }

    pub fn register_no_dpeps<V: DeserializeOwned>(
        &mut self,
        key: &str,
        insert: impl Fn(&mut T, &mut EntityBuilderClone, V) -> anyhow::Result<()> + 'static,
    ) {
        if self.registry.contains_key(key) {
            panic!("duplicate key {key:?}");
        }

        let entry = ComponentEntry {
            deps: Box::new(|_, _| Ok(())),
            builder: Box::new(move |ctx, builder, val| {
                let val = serde_json::from_str::<V>(val.get())?;
                insert(ctx, builder, val)?;
                Ok(())
            }),
        };

        self.registry.insert(key.to_string(), entry);
    }

    pub fn list_deps(&self, pref: &PrePrefab) -> anyhow::Result<HashSet<Rc<Path>>> {
        let mut result = HashSet::new();
        for (name, value) in pref.0.iter() {
            let Some(entry) = self.registry.get(*name) else {
                anyhow::bail!("unknown component: {name:?}");
            };
            (entry.deps)(value, &mut result).with_context(|| format!("deps of {name:?}"))?;
        }

        Ok(result)
    }

    pub fn build(
        &self,
        ctx: &mut T,
        start: &mut EntityBuilderClone,
        pref: &PrePrefab,
    ) -> anyhow::Result<()> {
        for (name, value) in pref.0.iter() {
            let Some(entry) = self.registry.get(*name) else {
                anyhow::bail!("unknown component: {name:?}");
            };
            tracing::info!(entry = name, "build prefab component");
            (entry.builder)(ctx, start, value).with_context(|| format!("build {name:?}"))?;
        }
        Ok(())
    }
}

impl<T: 'static> Default for PrefabFactory<T> {
    fn default() -> Self {
        Self::new()
    }
}

struct ComponentEntry<T> {
    deps: ComponentDependencies,
    builder: ComponentBuilder<T>,
}

type ComponentDependencies =
    Box<dyn Fn(&RawJsonValue, &mut HashSet<Rc<Path>>) -> anyhow::Result<()>>;

type ComponentBuilder<T> =
    Box<dyn Fn(&mut T, &mut EntityBuilderClone, &RawJsonValue) -> anyhow::Result<()>>;

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use hashbrown::HashSet;
    use hecs::*;

    use crate::PrefabFactory;

    #[test]
    fn empty() {
        let prefab = build_from_json(r#"{}"#);
        assert_eq!(prefab.component_types().count(), 0);
    }

    #[test]
    fn single() {
        let prefab = build_from_json(
            r#"{
  "a": null
}"#,
        );
        assert_eq!(
            prefab.component_types().collect::<Vec<_>>(),
            &[TypeId::of::<A>()]
        );

        let prefab = build_from_json(
            r#"{
  "b": 3
}"#,
        );
        assert_eq!(
            prefab.component_types().collect::<Vec<_>>(),
            &[TypeId::of::<B>()]
        );
        assert_eq!(prefab.get::<&B>().expect("No component").0, 3);

        let prefab = build_from_json(
            r#"{
  "c": [2, 3]
}"#,
        );
        assert_eq!(
            prefab.component_types().collect::<Vec<_>>(),
            &[TypeId::of::<C>()]
        );
        assert_eq!(prefab.get::<&C>().expect("No component").0, 2);
        assert_eq!(prefab.get::<&C>().expect("No component").1, 3);
    }

    #[test]
    fn bundle() {
        let prefab = build_from_json(
            r#"{
  "ac": {
    "a": null,
    "c": [1,2]
  }
}"#,
        );
        assert_eq!(
            prefab.component_types().collect::<HashSet<_>>(),
            HashSet::from_iter([TypeId::of::<A>(), TypeId::of::<C>(),])
        );
    }

    #[test]
    fn multi() {
        let prefab = build_from_json(
            r#"{
  "ac": {
    "a": null,
    "c": [1,2]
  },
  "b": 3
}"#,
        );
        assert_eq!(
            prefab.component_types().collect::<HashSet<_>>(),
            HashSet::from_iter([TypeId::of::<A>(), TypeId::of::<B>(), TypeId::of::<C>(),])
        );
        assert_eq!(prefab.get::<&B>().expect("No component").0, 3);
        assert_eq!(prefab.get::<&C>().expect("No component").0, 1);
        assert_eq!(prefab.get::<&C>().expect("No component").1, 2);
    }

    #[test]
    fn duplicate() {
        build_from_json_failing(
            r#"{
  "ac": {
    "a": null,
    "c": [1,2]
  },
  "a": null
}"#,
        );
    }

    fn build_from_json_failing(s: &str) {
        let fac = make_factory();
        let mut prefab = EntityBuilderClone::new();
        let preprefab = serde_json::from_str(s).expect("parse");
        fac.build(&mut (), &mut prefab, &preprefab)
            .expect_err("build");
    }

    fn build_from_json(s: &str) -> EntityBuilderClone {
        let fac = make_factory();
        let mut prefab = EntityBuilderClone::new();
        let preprefab = serde_json::from_str(s).expect("parse");
        fac.build(&mut (), &mut prefab, &preprefab).expect("build");

        prefab
    }

    fn make_factory() -> PrefabFactory<()> {
        let mut res = PrefabFactory::new();
        res.register_component::<A>("a");
        res.register_component::<C>("c");
        res.register_component_with_constructor("b", B);
        res.register_bundle::<BundleAC>("ac");
        res
    }

    #[derive(Clone, Copy, serde::Deserialize)]
    struct A;

    #[derive(Clone, Copy)]
    struct B(i32);

    #[derive(Clone, Copy, serde::Deserialize)]
    struct C(i32, i32);

    #[derive(Clone, Copy, Bundle, DynamicBundleClone, serde::Deserialize)]
    struct BundleAC {
        a: A,
        c: C,
    }
}
