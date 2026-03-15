local data = {
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Buckflower Seed", count = 1 } },
    products = { { item = "Buckflower", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Citrome Seed", count = 1 } },
    products = { { item = "Citrome", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Sandleaf Seed", count = 1 } },
    products = { { item = "Sandleaf", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Aketine Seed", count = 1 } },
    products = { { item = "Aketine", count = 1 } },
},
{
    time = 2,
    mode = "Fluid",
    ingredients = { { item = "Jincao Seed", count = 1 }, { item = "Clean Water", count = 1 } },
    products = { { item = "Jincao", count = 2 } },
},
{
    time = 2,
    mode = "Fluid",
    ingredients = { { item = "Yazhen Seed", count = 1 }, { item = "Clean Water", count = 1 } },
    products = { { item = "Yazhen", count = 2 } },
},
}

for _, v in ipairs(data) do
    v.facility = "Planting Unit"
end

return data