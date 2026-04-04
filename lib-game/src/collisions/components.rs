use crate::prelude::*;

pub use lib_col::Shape;
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
    pub groups: Group,
    /// The group filter.
    /// The engine will pick all entities inside
    /// that group.
    #[serde(deserialize_with = "decode_collision_group_manifest")]
    pub filter: Group,
    #[serde(skip)]
    pub collision_slice: CollisionQuerySlice,
}

impl<const ID: usize> CollisionQuery<ID> {
    pub fn new(groups: Group, filter: Group) -> Self {
        Self { groups, filter, collision_slice: Default::default() }
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
    const VARIANT_NAMES: &[&str] = &["Level", "Characters", "Attacks", "Player"];
    const VARIANT_VALUES: &[Group] =
        &[col_group::LEVEL, col_group::CHARACTERS, col_group::ATTACKS, col_group::PLAYER];

    struct GroupVisitor(Group);

    impl<'de> serde::de::Visitor<'de> for GroupVisitor {
        type Value = Group;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "a sequence of strings")
        }

        fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let bail = <A::Error as serde::de::Error>::unknown_variant;
            while let Some(x) = seq.next_element::<&str>()? {
                let (_, x) = VARIANT_NAMES
                    .iter()
                    .copied()
                    .zip(VARIANT_VALUES.iter().copied())
                    .find(|(y, _)| *y == x)
                    .ok_or_else(|| bail(x, VARIANT_NAMES))?;
                self.0 = self.0.union(x);
            }
            Ok(self.0)
        }
    }

    debug_assert_eq!(VARIANT_NAMES.len(), VARIANT_VALUES.len());
    des.deserialize_seq(GroupVisitor(col_group::NONE))
}
