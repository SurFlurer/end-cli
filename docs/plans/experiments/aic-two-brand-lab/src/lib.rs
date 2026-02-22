use generativity::{Guard, Id};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemId<'cid> {
    raw: u32,
    brand: Id<'cid>,
}

impl<'cid> ItemId<'cid> {
    pub fn as_u32(self) -> u32 {
        self.raw
    }

    fn from_index(index: usize, brand: Id<'cid>) -> Self {
        Self {
            raw: index as u32,
            brand,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutpostId<'sid> {
    raw: u32,
    brand: Id<'sid>,
}

impl<'sid> OutpostId<'sid> {
    pub fn as_u32(self) -> u32 {
        self.raw
    }

    fn from_index(index: usize, brand: Id<'sid>) -> Self {
        Self {
            raw: index as u32,
            brand,
        }
    }

    fn index(self) -> usize {
        self.raw as usize
    }
}

#[derive(Debug, Clone)]
pub struct Catalog<'cid> {
    brand: Id<'cid>,
    item_keys: Box<[&'static str]>,
}

impl<'cid> Catalog<'cid> {
    pub fn build(guard: Guard<'cid>) -> Self {
        Self {
            brand: guard.into(),
            item_keys: vec!["ore", "ingot"].into_boxed_slice(),
        }
    }

    pub fn item_id(&self, key: &str) -> Option<ItemId<'cid>> {
        self.item_keys
            .iter()
            .position(|candidate| *candidate == key)
            .map(|index| ItemId::from_index(index, self.brand))
    }

    pub fn item_key(&self, id: ItemId<'cid>) -> Option<&'static str> {
        self.item_keys.get(id.as_u32() as usize).copied()
    }
}

#[derive(Debug, Clone)]
pub struct OutpostInput<'cid> {
    pub sale_item: ItemId<'cid>,
}

#[derive(Debug, Clone)]
pub struct AicInputs<'cid, 'sid> {
    scenario_brand: Id<'sid>,
    outposts: Box<[OutpostInput<'cid>]>,
}

impl<'cid, 'sid> AicInputs<'cid, 'sid> {
    pub fn parse(
        catalog: &Catalog<'cid>,
        guard: Guard<'sid>,
        raw_outpost_sale_items: &[&str],
    ) -> Result<Self, ParseError> {
        let outposts = raw_outpost_sale_items
            .iter()
            .map(|key| {
                let sale_item = catalog
                    .item_id(key)
                    .ok_or_else(|| ParseError::UnknownItem((*key).to_string().into_boxed_str()))?;
                Ok(OutpostInput { sale_item })
            })
            .collect::<Result<Vec<_>, ParseError>>()?
            .into_boxed_slice();

        if outposts.is_empty() {
            return Err(ParseError::NoOutpost);
        }

        Ok(Self {
            scenario_brand: guard.into(),
            outposts,
        })
    }

    pub fn outpost(&self, id: OutpostId<'sid>) -> Option<&OutpostInput<'cid>> {
        self.outposts.get(id.index())
    }

    pub fn outposts_with_id(&self) -> impl Iterator<Item = (OutpostId<'sid>, &OutpostInput<'cid>)> + '_ {
        let brand = self.scenario_brand;
        self.outposts
            .iter()
            .enumerate()
            .map(move |(index, outpost)| (OutpostId::from_index(index, brand), outpost))
    }
}

#[derive(Debug, Clone)]
pub struct OutpostSaleQty<'cid, 'sid> {
    pub outpost_index: OutpostId<'sid>,
    pub item: ItemId<'cid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LogisticsNodeId<'rid> {
    raw: u32,
    brand: Id<'rid>,
}

impl<'rid> LogisticsNodeId<'rid> {
    pub fn as_u32(self) -> u32 {
        self.raw
    }

    fn from_index(index: usize, brand: Id<'rid>) -> Self {
        Self {
            raw: index as u32,
            brand,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StageSolution<'cid, 'sid> {
    pub outpost_sales_qty: Box<[OutpostSaleQty<'cid, 'sid>]>,
}

#[derive(Debug, Clone)]
pub struct OptimizationResult<'cid, 'sid, 'rid> {
    pub stage2: StageSolution<'cid, 'sid>,
    pub logistics_nodes: Box<[LogisticsNodeId<'rid>]>,
}

pub fn run_two_stage<'cid, 'sid, 'rid>(
    _catalog: &Catalog<'cid>,
    aic: &AicInputs<'cid, 'sid>,
    guard: Guard<'rid>,
) -> Result<OptimizationResult<'cid, 'sid, 'rid>, SolveError> {
    let rid = guard.into();
    let (outpost_index, outpost) = aic.outposts_with_id().next().ok_or(SolveError::NoOutpost)?;
    let line = OutpostSaleQty {
        outpost_index,
        item: outpost.sale_item,
    };

    Ok(OptimizationResult {
        stage2: StageSolution {
            outpost_sales_qty: vec![line].into_boxed_slice(),
        },
        logistics_nodes: vec![LogisticsNodeId::from_index(0, rid)].into_boxed_slice(),
    })
}

pub fn build_report<'cid, 'sid>(
    catalog: &Catalog<'cid>,
    aic: &AicInputs<'cid, 'sid>,
    result: &OptimizationResult<'cid, 'sid, '_>,
) -> Result<Box<str>, ReportError> {
    let first_sale = result
        .stage2
        .outpost_sales_qty
        .first()
        .ok_or(ReportError::NoSale)?;
    let outpost = aic
        .outpost(first_sale.outpost_index)
        .ok_or(ReportError::MissingOutpost(first_sale.outpost_index.as_u32()))?;
    let item_key = catalog
        .item_key(outpost.sale_item)
        .ok_or(ReportError::MissingItem(outpost.sale_item.as_u32()))?;

    Ok(format!("report item={item_key}").into_boxed_str())
}

pub fn use_result_node<'cid, 'sid, 'rid>(
    _aic: &AicInputs<'cid, 'sid>,
    _result: &OptimizationResult<'cid, 'sid, 'rid>,
    _node: LogisticsNodeId<'rid>,
) {
}

pub fn use_item_with_aic<'cid, 'sid>(
    _catalog: &Catalog<'cid>,
    _aic: &AicInputs<'cid, 'sid>,
    _item: ItemId<'cid>,
) {
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    UnknownItem(Box<str>),
    NoOutpost,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolveError {
    NoOutpost,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportError {
    NoSale,
    MissingOutpost(u32),
    MissingItem(u32),
}
