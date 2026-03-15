local data = {
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Ferrium", count = 2 } },
    products = { { item = "Ferrium Bottle", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Amethyst Fiber", count = 2 } },
    products = { { item = "Amethyst Bottle", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Steel", count = 2 } },
    products = { { item = "Steel Bottle", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Cryston Fiber", count = 2 } },
    products = { { item = "Cryston Bottle", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Cuprium", count = 2 } },
    products = { { item = "Cuprium Bottle", count = 1 } },
},
}

for _, v in ipairs(data) do
    v.facility = "Moulding Unit"
end

return data