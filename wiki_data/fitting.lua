local data = {
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Ferrium", count = 1 } },
    products = { { item = "Ferrium Part", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Amethyst Fiber", count = 1 } },
    products = { { item = "Amethyst Part", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Steel", count = 1 } },
    products = { { item = "Steel Part", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Cryston Fiber", count = 1 } },
    products = { { item = "Cryston Part", count = 1 } },
},
{
    time = 2,
    mode = "Solid",
    ingredients = { { item = "Cuprium", count = 1 } },
    products = { { item = "Cuprium Part", count = 1 } },
},
}

for _, v in ipairs(data) do
    v.facility = "Fitting Unit"
end

return data