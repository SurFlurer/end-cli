pub(crate) fn facility_zh(name: &str) -> Option<&'static str> {
    match name {
        "Refining Unit" => Some("精炼炉"),
        "Shredding Unit" => Some("粉碎机"),
        "Fitting Unit" => Some("配件机"),
        "Moulding Unit" => Some("塑形机"),
        "Seed-Picking Unit" => Some("采种机"),
        "Planting Unit" => Some("种植机"),
        "Gearing Unit" => Some("装备原件机"),
        "Filling Unit" => Some("灌装机"),
        "Packaging Unit" => Some("封装机"),
        "Grinding Unit" => Some("研磨机"),
        "Reactor Crucible" => Some("反应池"),
        "Forge of the Sky" => Some("天有洪炉"),
        "Separating Unit" => Some("拆解机"),
        "Thermal Bank" => Some("热容池"),
        _ => None,
    }
}

pub(crate) fn item_zh(name: &str) -> Option<String> {
    let direct = match name {
        "Originium Ore" => Some("源石矿"),
        "Originium Powder" => Some("源石粉"),
        "Dense Originium Powder" => Some("浓缩源石粉"),
        "Origocrust" => Some("晶体外壳"),
        "Origocrust Powder" => Some("晶体外壳粉"),
        "Dense Origocrust Powder" => Some("浓缩晶体外壳粉"),
        "Packed Origocrust" => Some("封装晶体外壳"),
        "Ferrium Ore" => Some("铁矿"),
        "Ferrium" => Some("铁"),
        "Ferrium Part" => Some("铁制零件"),
        "Ferrium Powder" => Some("铁粉"),
        "Dense Ferrium Powder" => Some("浓缩铁粉"),
        "Ferrium Component" => Some("铁制部件"),
        "Ferrium Bottle" => Some("铁质瓶"),
        "Amethyst Ore" => Some("紫晶矿"),
        "Amethyst Fiber" => Some("紫晶纤维"),
        "Amethyst Part" => Some("紫晶零件"),
        "Amethyst Powder" => Some("紫晶粉"),
        "Amethyst Component" => Some("紫晶部件"),
        "Amethyst Bottle" => Some("紫晶质瓶"),
        "Cryston Part" => Some("晶通零件"),
        "Cryston Fiber" => Some("晶通纤维"),
        "Cryston Powder" => Some("晶通粉"),
        "Cryston Component" => Some("晶通部件"),
        "Cryston Bottle" => Some("晶通质瓶"),
        "Xiranite" => Some("希然石"),
        "Liquid Xiranite" => Some("液态希然石"),
        "Xiranite Component" => Some("希然石部件"),
        "Clean Water" => Some("清水"),
        "Jincao" => Some("金草"),
        "Jincao Powder" => Some("金草粉"),
        "Jincao Solution" => Some("金草溶液"),
        "Jincao Drink" => Some("金草饮料"),
        "Yazhen" => Some("雅针"),
        "Yazhen Powder" => Some("雅针粉"),
        "Yazhen Solution" => Some("雅针溶液"),
        "Yazhen Syringe (C)" => Some("雅针注射器(C)"),
        "Sandleaf" => Some("沙叶"),
        "Sandleaf Powder" => Some("沙叶粉"),
        "Buckflower" => Some("荞愈花"),
        "Buckflower Powder" => Some("荞愈花粉"),
        "Ground Buckflower Powder" => Some("研磨荞愈花粉"),
        "Citrome" => Some("柑实"),
        "Citrome Powder" => Some("柑实粉"),
        "Ground Citrome Powder" => Some("研磨柑实粉"),
        "Buck Capsule (A)" => Some("精选荞愈胶囊"),
        "Buck Capsule (B)" => Some("优质荞愈胶囊"),
        "Buck Capsule (C)" => Some("荞愈胶囊"),
        "Canned Citrome (A)" => Some("精选柑实罐头"),
        "Canned Citrome (B)" => Some("优质柑实罐头"),
        "Canned Citrome (C)" => Some("柑实罐头"),
        "LC Valley Battery" => Some("低容谷地电池"),
        "SC Valley Battery" => Some("中容谷地电池"),
        "HC Valley Battery" => Some("高容谷地电池"),
        "LC Wuling Battery" => Some("低容五陵电池"),
        "Carbon" => Some("碳"),
        "Carbon Powder" => Some("碳粉"),
        "Dense Carbon Powder" => Some("浓缩碳粉"),
        "Stabilized Carbon" => Some("稳定碳"),
        "Steel" => Some("钢"),
        "Steel Part" => Some("钢制零件"),
        "Steel Bottle" => Some("钢质瓶"),
        "Wood" => Some("木材"),
        "Power" => Some("电力"),
        "Industrial Explosive" => Some("工业炸药"),
        "Aketine" => Some("阿刻汀"),
        "Aketine Powder" => Some("阿刻汀粉"),
        "Aketine Seed" => Some("阿刻汀种子"),
        "Amber Rice" => Some("琥珀米"),
        "Amber Rice Seed" => Some("琥珀米种子"),
        "Redjade Ginseng" => Some("赤玉人参"),
        "Reed Rye" => Some("芦苇黑麦"),
        "Tartpepper" => Some("酸椒"),
        "Bumper-Rich" => Some("富缓冲剂"),
        "Burdo-Muck" => Some("伯尔多泥"),
        _ => None,
    };
    if let Some(v) = direct {
        return Some(v.to_string());
    }

    if let Some(base) = name.strip_suffix(" Seed") {
        return Some(format!("{}种子", item_zh(base)?));
    }

    if let Some((base, modifier)) = split_parenthesized(name) {
        let base_zh = item_zh(base)?;
        let modifier_zh = item_zh(modifier).unwrap_or_else(|| modifier.to_string());
        return Some(format!("{base_zh}（{modifier_zh}）"));
    }

    None
}

fn split_parenthesized(s: &str) -> Option<(&str, &str)> {
    let open = s.rfind(" (")?;
    if !s.ends_with(')') {
        return None;
    }
    let base = &s[..open];
    let inner = &s[(open + 2)..s.len() - 1];
    if base.is_empty() || inner.is_empty() {
        return None;
    }
    Some((base, inner))
}
