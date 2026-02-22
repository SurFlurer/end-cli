use aic_two_brand_lab::{AicInputs, Catalog, ItemId, use_item_with_aic};

fn cross_mix_item<'cid1, 'cid2, 'sid>(
    catalog_2: &Catalog<'cid2>,
    aic_2: &AicInputs<'cid2, 'sid>,
    item_1: ItemId<'cid1>,
) {
    // Intentional misuse:
    // item_1 is from catalog 'cid1, but consumed with catalog_2/aic_2 ('cid2).
    use_item_with_aic(catalog_2, aic_2, item_1);
}

fn main() {
    let _ = cross_mix_item;
}
