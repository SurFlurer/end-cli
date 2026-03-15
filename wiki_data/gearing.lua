local data = {
{
    time = 10,
    mode = "Solid",
    ingredients = { { item = "Origocrust", count = 5 }, { item = "Amethyst Fiber", count = 5 } },
    products = { { item = "Amethyst Component", count = 1 } },
},
{
    time = 10,
    mode = "Solid",
    ingredients = { { item = "Origocrust", count = 10 }, { item = "Ferrium", count = 10 } },
    products = { { item = "Ferrium Component", count = 1 } },
},
{
    time = 10,
    mode = "Solid",
    ingredients = { { item = "Packed Origocrust", count = 10 }, { item = "Cryston Fiber", count = 10 } },
    products = { { item = "Cryston Component", count = 1 } },
},
{
    time = 10,
    mode = "Solid",
    ingredients = { { item = "Cuprium Part", count = 10 }, { item = "Xiranite", count = 10 } },
    products = { { item = "Cuprium Component", count = 1 } },
},
{
    time = 10,
    mode = "Solid",
    ingredients = { { item = "Packed Origocrust", count = 10 }, { item = "Xiranite", count = 10 } },
    products = { { item = "Xiranite Component", count = 1 } },
},
}

for _, v in ipairs(data) do
    v.facility = "Gearing Unit"
end

return data