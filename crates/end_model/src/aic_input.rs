use std::collections::HashSet;
use std::iter::FromIterator;
use std::num::NonZeroU32;
use vector_map::VecMap;

use crate::{DisplayName, ItemId, Key};
use generativity::{Guard, Id};
use thiserror::Error;

/// Scenario region used for facility availability constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScenarioRegion {
    FourthValley,
    Wuling,
}

impl ScenarioRegion {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::FourthValley => "fourth_valley",
            Self::Wuling => "wuling",
        }
    }
}

/// Stable identifier for an outpost in [`AicInputs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OutpostId<'sid> {
    raw: u32,
    brand: Id<'sid>,
}

impl<'sid> OutpostId<'sid> {
    /// Returns the underlying numeric representation.
    pub fn as_u32(self) -> u32 {
        self.raw
    }

    fn from_index(index: usize, brand: Id<'sid>) -> Self {
        Self {
            raw: index as u32,
            brand,
        }
    }

    pub fn index(self) -> usize {
        self.raw as usize
    }
}

/// Sparse map keyed by [`ItemId`] with unique keys guaranteed by representation.
///
/// This uses a vector-backed map (`VecMap`) and is intended for small collections
/// such as outpost price tables.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemU32Map<'id>(VecMap<ItemId<'id>, u32>);

impl<'id> ItemU32Map<'id> {
    pub fn new() -> Self {
        Self(VecMap::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(VecMap::with_capacity(capacity))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn insert(&mut self, item: ItemId<'id>, value: u32) -> Option<u32> {
        self.0.insert(item, value)
    }

    pub fn get(&self, item: ItemId<'id>) -> Option<&u32> {
        self.0.get(&item)
    }

    pub fn iter(&self) -> impl Iterator<Item = (ItemId<'id>, u32)> + '_ {
        self.0.iter().map(|(item, value)| (*item, *value))
    }
}

impl<'id> Extend<(ItemId<'id>, u32)> for ItemU32Map<'id> {
    fn extend<T: IntoIterator<Item = (ItemId<'id>, u32)>>(&mut self, iter: T) {
        for (item, value) in iter {
            self.insert(item, value);
        }
    }
}

impl<'id> FromIterator<(ItemId<'id>, u32)> for ItemU32Map<'id> {
    fn from_iter<T: IntoIterator<Item = (ItemId<'id>, u32)>>(iter: T) -> Self {
        let mut map = Self::new();
        map.extend(iter);
        map
    }
}

impl<'id, const N: usize> From<[(ItemId<'id>, u32); N]> for ItemU32Map<'id> {
    fn from(value: [(ItemId<'id>, u32); N]) -> Self {
        value.into_iter().collect()
    }
}

impl<'id> From<Vec<(ItemId<'id>, u32)>> for ItemU32Map<'id> {
    fn from(value: Vec<(ItemId<'id>, u32)>) -> Self {
        value.into_iter().collect()
    }
}

impl<'id> IntoIterator for ItemU32Map<'id> {
    type Item = (ItemId<'id>, u32);
    type IntoIter = vector_map::IntoIter<ItemId<'id>, u32>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Sparse map keyed by [`ItemId`] with non-zero values and unique keys guaranteed by representation.
///
/// This currently uses a vector-backed map (`VecMap`) and is intended for small collections
/// such as scenario external supply/consumption tables.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemNonZeroU32Map<'id>(VecMap<ItemId<'id>, NonZeroU32>);

impl<'id> ItemNonZeroU32Map<'id> {
    pub fn new() -> Self {
        Self(VecMap::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(VecMap::with_capacity(capacity))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn insert(&mut self, item: ItemId<'id>, value: NonZeroU32) -> Option<NonZeroU32> {
        self.0.insert(item, value)
    }

    pub fn get(&self, item: ItemId<'id>) -> Option<&NonZeroU32> {
        self.0.get(&item)
    }

    pub fn iter(&self) -> impl Iterator<Item = (ItemId<'id>, NonZeroU32)> + '_ {
        self.0.iter().map(|(item, value)| (*item, *value))
    }
}

impl<'id> Extend<(ItemId<'id>, NonZeroU32)> for ItemNonZeroU32Map<'id> {
    fn extend<T: IntoIterator<Item = (ItemId<'id>, NonZeroU32)>>(&mut self, iter: T) {
        for (item, value) in iter {
            self.insert(item, value);
        }
    }
}

impl<'id> FromIterator<(ItemId<'id>, NonZeroU32)> for ItemNonZeroU32Map<'id> {
    fn from_iter<T: IntoIterator<Item = (ItemId<'id>, NonZeroU32)>>(iter: T) -> Self {
        let mut map = Self::new();
        map.extend(iter);
        map
    }
}

impl<'id, const N: usize> From<[(ItemId<'id>, NonZeroU32); N]> for ItemNonZeroU32Map<'id> {
    fn from(value: [(ItemId<'id>, NonZeroU32); N]) -> Self {
        value.into_iter().collect()
    }
}

impl<'id> From<Vec<(ItemId<'id>, NonZeroU32)>> for ItemNonZeroU32Map<'id> {
    fn from(value: Vec<(ItemId<'id>, NonZeroU32)>) -> Self {
        value.into_iter().collect()
    }
}

impl<'id> IntoIterator for ItemNonZeroU32Map<'id> {
    type Item = (ItemId<'id>, NonZeroU32);
    type IntoIter = vector_map::IntoIter<ItemId<'id>, NonZeroU32>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// One outpost configuration, includes price table and money cap.
#[derive(Debug, Clone)]
pub struct OutpostInput<'id> {
    pub key: Key,
    pub en: Option<DisplayName>,
    pub zh: Option<DisplayName>,
    pub money_cap_per_hour: u32,
    pub prices: ItemU32Map<'id>,
}

/// Full scenario inputs consumed by optimization.
#[derive(Debug, Clone)]
pub struct AicInputs<'cid, 'sid> {
    region: ScenarioRegion,
    supply_per_min: ItemNonZeroU32Map<'cid>,
    external_consumption_per_min: ItemNonZeroU32Map<'cid>,
    outposts: Box<[OutpostInput<'cid>]>,
    external_power_consumption_w: u32,
    scenario_brand: Id<'sid>,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum AicBuildError {
    #[error("Duplicate outpost key: {key}")]
    DuplicateOutpostKey { key: Key },
}

impl<'cid, 'sid> AicInputs<'cid, 'sid> {
    pub fn parse(
        guard: Guard<'sid>,
        external_power_consumption_w: u32,
        supply_per_min: ItemNonZeroU32Map<'cid>,
        external_consumption_per_min: ItemNonZeroU32Map<'cid>,
        outposts: Vec<OutpostInput<'cid>>,
    ) -> Result<Self, AicBuildError> {
        Self::parse_with_region(
            guard,
            ScenarioRegion::Wuling,
            external_power_consumption_w,
            supply_per_min,
            external_consumption_per_min,
            outposts,
        )
    }

    pub fn parse_with_region(
        guard: Guard<'sid>,
        region: ScenarioRegion,
        external_power_consumption_w: u32,
        supply_per_min: ItemNonZeroU32Map<'cid>,
        external_consumption_per_min: ItemNonZeroU32Map<'cid>,
        outposts: Vec<OutpostInput<'cid>>,
    ) -> Result<Self, AicBuildError> {
        let mut seen = HashSet::with_capacity(outposts.len());
        for outpost in &outposts {
            if !seen.insert(outpost.key.as_str()) {
                return Err(AicBuildError::DuplicateOutpostKey {
                    key: outpost.key.clone(),
                });
            }
        }

        Ok(Self {
            region,
            supply_per_min,
            external_consumption_per_min,
            outposts: outposts.into_boxed_slice(),
            external_power_consumption_w,
            scenario_brand: guard.into(),
        })
    }

    pub fn external_power_consumption_w(&self) -> u32 {
        self.external_power_consumption_w
    }

    pub fn region(&self) -> ScenarioRegion {
        self.region
    }

    pub fn supply_per_min(&self) -> &ItemNonZeroU32Map<'cid> {
        &self.supply_per_min
    }

    pub fn external_consumption_per_min(&self) -> &ItemNonZeroU32Map<'cid> {
        &self.external_consumption_per_min
    }

    pub fn outposts(&self) -> &[OutpostInput<'cid>] {
        &self.outposts
    }

    /// Returns an outpost by id.
    pub fn outpost(&self, id: OutpostId<'sid>) -> Option<&OutpostInput<'cid>> {
        self.outposts.get(id.index())
    }

    /// Returns all outposts paired with their stable ids.
    pub fn outposts_with_id(
        &self,
    ) -> impl Iterator<Item = (OutpostId<'sid>, &OutpostInput<'cid>)> + '_ {
        let brand = self.scenario_brand;
        self.outposts
            .iter()
            .enumerate()
            .map(move |(index, outpost)| (OutpostId::from_index(index, brand), outpost))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use generativity::make_guard;

    use crate::{
        AicBuildError, Catalog, DisplayName, ItemDef, ItemNonZeroU32Map, ItemU32Map, Key,
        OutpostInput, ThermalBankDef,
    };
    use std::num::NonZeroU32;

    fn key(value: &str) -> Key {
        value.try_into().expect("valid key")
    }

    fn name(value: &str) -> DisplayName {
        value.try_into().expect("valid display name")
    }

    fn sample_catalog<'id>(
        guard: generativity::Guard<'id>,
    ) -> (Catalog<'id>, crate::ItemId<'id>, crate::ItemId<'id>) {
        let mut builder = Catalog::builder(guard);
        let a = builder
            .add_item(ItemDef {
                key: key("a"),
                en: name("A"),
                zh: name("A"),
            })
            .expect("item a should be insertable");
        let b = builder
            .add_item(ItemDef {
                key: key("b"),
                en: name("B"),
                zh: name("B"),
            })
            .expect("item b should be insertable");
        let builder = builder
            .add_thermal_bank(ThermalBankDef {
                key: key("thermal-bank"),
                en: name("Thermal Bank"),
                zh: name("Thermal Bank"),
            })
            .expect("thermal bank should be insertable");
        let catalog = builder.build();
        (catalog, a, b)
    }

    #[test]
    fn item_u32_map_keeps_unique_keys() {
        make_guard!(guard);
        let (_, item, _) = sample_catalog(guard);
        let mut map = ItemU32Map::new();

        assert_eq!(map.insert(item, 10), None);
        assert_eq!(map.insert(item, 20), Some(10));
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(item), Some(&20));
    }

    #[test]
    fn item_u32_map_from_vec_uses_last_value_for_duplicates() {
        make_guard!(guard);
        let (_, a, b) = sample_catalog(guard);
        let map: ItemU32Map = vec![(a, 1), (b, 3), (a, 2)].into();

        assert_eq!(map.len(), 2);
        assert_eq!(map.get(a), Some(&2));
        assert_eq!(map.get(b), Some(&3));
    }

    #[test]
    fn item_non_zero_u32_map_from_vec_uses_last_value_for_duplicates() {
        make_guard!(guard);
        let (_, a, b) = sample_catalog(guard);
        let map: ItemNonZeroU32Map = vec![
            (a, NonZeroU32::new(1).expect("non-zero")),
            (b, NonZeroU32::new(3).expect("non-zero")),
            (a, NonZeroU32::new(2).expect("non-zero")),
        ]
        .into();

        assert_eq!(map.len(), 2);
        assert_eq!(map.get(a), Some(&NonZeroU32::new(2).expect("non-zero")));
        assert_eq!(map.get(b), Some(&NonZeroU32::new(3).expect("non-zero")));
    }

    #[test]
    fn aic_parse_rejects_duplicate_outpost_keys() {
        make_guard!(catalog_guard);
        let (_, _, b) = sample_catalog(catalog_guard);
        make_guard!(aic_guard);
        let camp = key("Camp");
        let err = crate::AicInputs::parse(
            aic_guard,
            0,
            vec![(b, NonZeroU32::new(1).expect("non-zero"))].into(),
            Default::default(),
            vec![
                OutpostInput {
                    key: camp.clone(),
                    en: Some(name("Camp")),
                    zh: Some(name("Camp_zh")),
                    money_cap_per_hour: 1,
                    prices: vec![(b, 1)].into(),
                },
                OutpostInput {
                    key: camp.clone(),
                    en: Some(name("Camp2")),
                    zh: Some(name("Camp2_zh")),
                    money_cap_per_hour: 2,
                    prices: vec![(b, 2)].into(),
                },
            ],
        )
        .expect_err("duplicate outpost key should fail");

        assert!(
            matches!(
                err,
                AicBuildError::DuplicateOutpostKey { ref key } if key == &camp
            ),
            "unexpected error: {err:?}"
        );
    }
}
