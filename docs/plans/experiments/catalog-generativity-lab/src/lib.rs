//! Minimal experiment for branded `Catalog`/`ItemId` with `generativity`.
//!
//! This crate demonstrates three points:
//!
//! 1. `ItemId<'id>` is tied to one specific catalog brand.
//! 2. `AicInputs<'id>` is parsed from `Catalog<'id>`, so solver inputs are same-brand by type.
//! 3. Solver code can use a branded item vector and drop runtime cross-catalog validation.
//!
//! ```compile_fail
//! use catalog_generativity_lab::{Catalog, CatalogBuilder, ItemId};
//! use generativity::make_guard;
//!
//! fn item_name<'id>(catalog: &Catalog<'id>, item: ItemId<'id>) -> &str {
//!     catalog.item_key(item)
//! }
//!
//! make_guard!(g1);
//! let mut b1 = CatalogBuilder::new(g1);
//! let ore_1 = b1.add_item("ore").unwrap();
//! let c1 = b1.build();
//!
//! make_guard!(g2);
//! let mut b2 = CatalogBuilder::new(g2);
//! b2.add_item("ore").unwrap();
//! let c2 = b2.build();
//!
//! let _ = c1;
//! let _ = item_name(&c2, ore_1);
//! //                   ^^^^^^ type mismatch: `ore_1` does not belong to `c2` brand.
//! ```

use generativity::{Guard, Id};
use std::collections::{hash_map::Entry, HashMap};
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemId<'id> {
    index: usize,
    brand: Id<'id>,
}

impl<'id> ItemId<'id> {
    fn new(index: usize, brand: Id<'id>) -> Self {
        Self { index, brand }
    }

    pub fn index(self) -> usize {
        self.index
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    DuplicateItemKey(String),
}

#[derive(Debug)]
pub struct CatalogBuilder<'id> {
    brand: Id<'id>,
    items: Vec<String>,
    item_index: HashMap<String, ItemId<'id>>,
}

impl<'id> CatalogBuilder<'id> {
    pub fn new(guard: Guard<'id>) -> Self {
        Self {
            brand: guard.into(),
            items: Vec::new(),
            item_index: HashMap::new(),
        }
    }

    pub fn add_item(&mut self, key: impl Into<String>) -> Result<ItemId<'id>, BuildError> {
        let key = key.into();
        match self.item_index.entry(key.clone()) {
            Entry::Occupied(_) => Err(BuildError::DuplicateItemKey(key)),
            Entry::Vacant(slot) => {
                let id = ItemId::new(self.items.len(), self.brand);
                self.items.push(key);
                slot.insert(id);
                Ok(id)
            }
        }
    }

    pub fn build(self) -> Catalog<'id> {
        Catalog {
            items: self.items.into_boxed_slice(),
            item_index: self.item_index,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Catalog<'id> {
    items: Box<[String]>,
    item_index: HashMap<String, ItemId<'id>>,
}

impl<'id> Catalog<'id> {
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    pub fn item_id(&self, key: &str) -> Option<ItemId<'id>> {
        self.item_index.get(key).copied()
    }

    pub fn item_key(&self, item: ItemId<'id>) -> &str {
        // SAFETY: `ItemId<'id>` is minted only by `CatalogBuilder<'id>`, and this catalog was
        // built from the same builder/brand. So `item.index` must be in-bounds for `items`.
        unsafe { self.items.get_unchecked(item.index()) }
    }
}

#[derive(Debug, Clone)]
pub struct AicInputs<'id> {
    supply_per_min: Vec<(ItemId<'id>, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AicParseError {
    UnknownItemKey(String),
}

impl<'id> AicInputs<'id> {
    pub fn parse<'a>(
        catalog: &Catalog<'id>,
        raw_supply: impl IntoIterator<Item = (&'a str, u32)>,
    ) -> Result<Self, AicParseError> {
        let mut supply_per_min = Vec::new();
        for (item_key, qty_per_min) in raw_supply {
            let Some(item) = catalog.item_id(item_key) else {
                return Err(AicParseError::UnknownItemKey(item_key.to_string()));
            };
            supply_per_min.push((item, qty_per_min));
        }
        Ok(Self { supply_per_min })
    }

    pub fn supply_per_min(&self) -> &[(ItemId<'id>, u32)] {
        &self.supply_per_min
    }
}

#[derive(Debug, Clone)]
pub struct ItemVec<'id, T> {
    values: Box<[T]>,
    _brand: PhantomData<fn(ItemId<'id>) -> ItemId<'id>>,
}

impl<'id, T: Clone> ItemVec<'id, T> {
    pub fn filled(catalog: &Catalog<'id>, value: T) -> Self {
        Self {
            values: vec![value; catalog.item_count()].into_boxed_slice(),
            _brand: PhantomData,
        }
    }
}

impl<'id, T> ItemVec<'id, T> {
    pub fn get(&self, item: ItemId<'id>) -> &T {
        // SAFETY: same-brand `ItemId<'id>` cannot exist outside the source catalog domain.
        unsafe { self.values.get_unchecked(item.index()) }
    }

    pub fn get_mut(&mut self, item: ItemId<'id>) -> &mut T {
        // SAFETY: same justification as `get`.
        unsafe { self.values.get_unchecked_mut(item.index()) }
    }

    pub fn as_slice(&self) -> &[T] {
        &*self.values
    }
}

impl<'id, T> Index<ItemId<'id>> for ItemVec<'id, T> {
    type Output = T;

    fn index(&self, index: ItemId<'id>) -> &Self::Output {
        self.get(index)
    }
}

impl<'id, T> IndexMut<ItemId<'id>> for ItemVec<'id, T> {
    fn index_mut(&mut self, index: ItemId<'id>) -> &mut Self::Output {
        self.get_mut(index)
    }
}

pub fn run_solver<'id>(catalog: &Catalog<'id>, aic: &AicInputs<'id>) -> ItemVec<'id, i64> {
    let mut item_balance = ItemVec::filled(catalog, 0_i64);
    for (item, qty_per_min) in aic.supply_per_min() {
        item_balance[*item] += *qty_per_min as i64;
    }
    item_balance
}

#[cfg(test)]
mod tests {
    use super::{AicInputs, AicParseError, CatalogBuilder, run_solver};
    use generativity::make_guard;

    #[test]
    fn parse_and_solve_same_brand_inputs() {
        make_guard!(guard);
        let mut b = CatalogBuilder::new(guard);
        b.add_item("ore").expect("ore inserted");
        b.add_item("ingot").expect("ingot inserted");
        let catalog = b.build();

        let aic = AicInputs::parse(&catalog, [("ore", 12), ("ingot", 3)]).expect("aic parsed");
        let balance = run_solver(&catalog, &aic);

        assert_eq!(balance.as_slice(), &[12, 3]);
    }

    #[test]
    fn parse_rejects_unknown_key() {
        make_guard!(guard);
        let mut b = CatalogBuilder::new(guard);
        b.add_item("ore").expect("ore inserted");
        let catalog = b.build();

        let err = AicInputs::parse(&catalog, [("missing", 1)]).expect_err("must fail");
        assert_eq!(err, AicParseError::UnknownItemKey("missing".to_string()));
    }
}
