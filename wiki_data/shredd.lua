local data = {
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Cuprium", count = 1 } },
    products = { { item = "Cuprium Powder", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Ferrium", count = 1 } },
    products = { { item = "Ferrium Powder", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Amethyst Fiber", count = 1 } },
    products = { { item = "Amethyst Powder", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Originium Ore", count = 1 } },
    products = { { item = "Originium Powder", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Carbon", count = 1 } },
    products = { { item = "Carbon Powder", count = 2 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Origocrust", count = 1 } },
    products = { { item = "Origocrust Powder", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Buckflower", count = 1 } },
    products = { { item = "Buckflower Powder", count = 2 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Citrome", count = 1 } },
    products = { { item = "Citrome Powder", count = 2 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Sandleaf", count = 1 } },
    products = { { item = "Sandleaf Powder", count = 3 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Aketine", count = 1 } },
    products = { { item = "Aketine Powder", count = 2 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Jincao", count = 1 } },
    products = { { item = "Jincao Powder", count = 2 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Yazhen", count = 1 } },
    products = { { item = "Yazhen Powder", count = 2 } },
},
}

for _, v in ipairs(data) do
    v.facility = "Shredding Unit"
end

return data