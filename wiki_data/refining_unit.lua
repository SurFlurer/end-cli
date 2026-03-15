local data = {
{
    time = 2,
    mode = "Fluid",
    ingredients = { { item = "Cuprium Ore", count = 1 }, { item = "Clean Water", count = 1 } },
    products = { { item = "Cuprium", count = 1 }, { item = "Sewage", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Ferrium Ore", count = 1 } },
    products = { { item = "Ferrium", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Ferrium Powder", count = 1 } },
    products = { { item = "Ferrium", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Amethyst Ore", count = 1 } },
    products = { { item = "Amethyst Fiber", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Amethyst Powder", count = 1 } },
    products = { { item = "Amethyst Fiber", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Originium Ore", count = 1 } },
    products = { { item = "Origocrust", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Origocrust Powder", count = 1 } },
    products = { { item = "Origocrust", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Dense Origocrust Powder", count = 1 } },
    products = { { item = "Packed Origocrust", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Dense Ferrium Powder", count = 1 } },
    products = { { item = "Steel", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Cryston Powder", count = 1 } },
    products = { { item = "Cryston Fiber", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Dense Carbon Powder", count = 1 } },
    products = { { item = "Stabilized Carbon", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Dense Originium Powder", count = 1 } },
    products = { { item = "Dense Origocrust Powder", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Originium Powder", count = 1 } },
    products = { { item = "Origocrust Powder", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Buckflower", count = 1 } },
    products = { { item = "Carbon", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Citrome", count = 1 } },
    products = { { item = "Carbon", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Sandleaf", count = 1 } },
    products = { { item = "Carbon", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Wood", count = 1 } },
    products = { { item = "Carbon", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Jincao", count = 1 } },
    products = { { item = "Carbon", count = 2 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Yazhen", count = 1 } },
    products = { { item = "Carbon", count = 2 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Buckflower Powder", count = 1 } },
    products = { { item = "Carbon Powder", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Citrome Powder", count = 1 } },
    products = { { item = "Carbon Powder", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Sandleaf Powder", count = 3 } },
    products = { { item = "Carbon Powder", count = 2 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Jincao Powder", count = 1 } },
    products = { { item = "Carbon Powder", count = 2 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Yazhen Powder", count = 1 } },
    products = { { item = "Carbon Powder", count = 2 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Ground Buckflower Powder", count = 1 } },
    products = { { item = "Dense Carbon Powder", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Ground Citrome Powder", count = 1 } },
    products = { { item = "Dense Carbon Powder", count = 1 } },
},
}

for _, v in ipairs(data) do
    v.facility = "Refining Unit"
end

return data