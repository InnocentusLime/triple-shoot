use crate::prelude::*;

pub const MAX_COLLISION_QUERIES: usize = 8;

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct CollisionQuery<const ID: usize> {
    /// The group membership.
    /// The engine will pick all entities with
    /// their group intersecting with this field.
    ///
    /// Setting it to an empty group will make the
    /// collision engine skip this query.
    #[serde(deserialize_with = "decode_collision_group_manifest")]
    pub group: Group,
    /// The group filter.
    /// The engine will pick all entities inside
    /// that group.
    #[serde(deserialize_with = "decode_collision_group_manifest")]
    pub filter: Group,
    /// The collider to use for the check.
    pub collider: Shape,
    #[serde(skip)]
    pub collision_slice: CollisionQuerySlice,
}

impl<const ID: usize> CollisionQuery<ID> {
    pub fn new(collider: Shape, group: Group, filter: Group) -> Self {
        Self { collider, group, filter, collision_slice: Default::default() }
    }

    pub fn has_collided(&self) -> bool {
        self.collision_slice.len > 0
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct CollisionQuerySlice {
    pub off: usize,
    pub len: usize,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct KinematicControl {
    #[serde(skip)]
    pub dr: Vec2,
    #[serde(deserialize_with = "decode_collision_group_manifest")]
    pub collision: Group,
    pub slide: bool,
    #[serde(skip)]
    pub collided: bool,
}

impl KinematicControl {
    pub fn new(collision: Group, slide: bool) -> Self {
        KinematicControl { dr: Vec2::ZERO, collision, slide, collided: false }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct BodyTag {
    #[serde(deserialize_with = "decode_collision_group_manifest")]
    pub groups: Group,
    pub shape: Shape,
}

pub mod col_group {
    use lib_col::Group;

    pub const NONE: Group = Group::empty();
    pub const LEVEL: Group = Group::from_id(0);
    pub const CHARACTERS: Group = Group::from_id(1);
    pub const PLAYER: Group = Group::from_id(2);
    pub const ATTACKS: Group = Group::from_id(3);
}

pub mod col_query {
    pub const LEVEL: usize = 0;
    pub const DAMAGE: usize = 1;
    pub const PICKUP: usize = 2;
    pub const INTERACTION: usize = 3;
    pub const GRAZING: usize = 4;

    pub type Level = super::CollisionQuery<LEVEL>;
    pub type Damage = super::CollisionQuery<DAMAGE>;
    pub type Pickup = super::CollisionQuery<PICKUP>;
    pub type Interaction = super::CollisionQuery<INTERACTION>;
    pub type Grazing = super::CollisionQuery<GRAZING>;
}

fn decode_collision_group_manifest<'de, D>(des: D) -> Result<Group, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let groups = <_>::deserialize(des)?;
    Ok(CollisionGroupManifest::fold_groups(groups))
}

#[derive(Debug, Deserialize)]
enum CollisionGroupManifest {
    Level,
    Characters,
    Player,
    Attacks,
}

impl CollisionGroupManifest {
    fn fold_groups(groups: Vec<CollisionGroupManifest>) -> Group {
        groups
            .into_iter()
            .map(CollisionGroupManifest::into_group)
            .fold(col_group::NONE, Group::union)
    }

    fn into_group(self) -> Group {
        match self {
            CollisionGroupManifest::Level => col_group::LEVEL,
            CollisionGroupManifest::Characters => col_group::CHARACTERS,
            CollisionGroupManifest::Player => col_group::PLAYER,
            CollisionGroupManifest::Attacks => col_group::ATTACKS,
        }
    }
}
