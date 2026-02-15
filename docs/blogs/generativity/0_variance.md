好，咱们先**把 Rust 全都忘掉**，连 `*mut &'id ()` 也先扔掉，
只回答两个问题：

1. *“covariant / contravariant / invariant 到底在说什么？”*
2. *“为什么这种分类是合理的，不是拍脑袋出来的？”*

等把这俩搞懂了，**再回去看 BrandedVec**，你会发现 `*mut &'id ()` 那种写法只是「往 invariant 一栏塞的一个技巧」，本质跟你已经理解的东西是同一个逻辑。

---

## 0. 一句话核心：variance = “自动转型能不能安全？”

先定一个游戏规则：

* 有两个类型 `Dog` 和 `Animal`，我们说：

  ```text
  Dog <: Animal
  ```

  意思是：**狗是一种动物**；
  程序里任何“要动物”的地方，你给它一只狗都行，是安全的。

这个叫「子类型（subtyping）」：**A <: B = 所有 A 的值在 B 的期待下都能安全使用**。

---

接下来我们玩一个问题：

> 已知 `Dog <: Animal`，那对于某个“泛型包装类型” `F<T>`，
> 什么时候可以自动把 `F<Dog>` 当成 `F<Animal>` 用？
> 什么时候可以把 `F<Animal>` 当成 `F<Dog>` 用？
> 什么时候两个方向都不行？

**variance** 就是拿这个问题，**把各种 `F` 分成三类**：

* covariant：可以 `F<Dog> -> F<Animal>`
* contravariant：可以 `F<Animal> -> F<Dog>`
* invariant：两个方向都不行，只能 `F<Dog> -> F<Dog>`

所有这些都是从「**能不能安全自动转型**」这个唯一原则推出来的，**不是拍脑袋的名词**。

---

## 1. covariant（协变）：最直觉的那种

先看你已经直觉能理解的 covariant，然后再在这个基础上看另外两种。

### 1.1 一个典型例子：只读盒子

想象一个类型：

```rust
struct ReadBox<T> {
    value: T,
}

impl<T> ReadBox<T> {
    fn get(&self) -> &T { &self.value }
}
```

它只能干一件事：**读出一个 `&T`，永远不会往里面写**。

问题：已知 `Dog <: Animal`，那 `ReadBox<Dog>` 和 `ReadBox<Animal>` 谁能自动变成谁？

---

### 1.2 场景分析：你“期待一个 `ReadBox<Animal>`”

假设有一个函数：

```rust
fn use_animals(box_animals: ReadBox<Animal>) {
    let a: &Animal = box_animals.get();
    // 只会把它当 Animal 用，比如打印种类
}
```

这个函数只做读操作，不往里面写。

现在别人手里只有一个 `ReadBox<Dog>`，想要把它传进来用：

```rust
let dogs: ReadBox<Dog> = ...;
use_animals(dogs); // <-- 这一步如果允许，就相当于：ReadBox<Dog> -> ReadBox<Animal>
```

**安全吗？**

* `use_animals` 只会做：`let a: &Animal = box_animals.get();`
* `dogs.get()` 返回 `&Dog`，可以自动当成 `&Animal` 用；
* 函数内部也不会试图往里面塞「不是狗的东西」。

→ 所以**完全安全**，可以自动把 `ReadBox<Dog>` 当成 `ReadBox<Animal>`。

> 于是说：**`ReadBox<T>` 对 `T` 是 covariant 的**：
> `Dog <: Animal` ⇒ `ReadBox<Dog> <: ReadBox<Animal>`

---

### 1.3 总结成一句话

> **covariant**：
> 如果 `S <: T`，那 `F<S> <: F<T>` 也成立。
> 这种 F 就是“方向不变”的（co = same），你已经有直觉了。

---

## 2. contravariant（逆变）：参数位置的那个反直觉玩意

这次我们看一个你很熟的例子：**函数的参数**。

还是那两个类型：

```text
Dog <: Animal
```

考虑一个函数类型：

```text
Handler<T> = fn(T) -> ()
```

问题：已知 `Dog <: Animal`，
那 `Handler<Dog>` 和 `Handler<Animal>` 谁可以自动当成谁？

---

### 2.1 场景：别人“期待一个能处理 Dog 的函数”

假设有这样一个函数：

```rust
fn call_with_dog(f: fn(Dog)) {
    let d = Dog;
    f(d);                  // 只会传“狗”进去
}
```

这里 `f` 的类型是 `fn(Dog)`，也就是 `Handler<Dog>`。

我们现在手里有一个函数：

```rust
fn handle_animal(a: Animal) {
    // 能处理任何 Animal，包括 Dog, Cat, Bird ...
}
```

它的类型是 `fn(Animal)`，也就是 `Handler<Animal>`。

问题：**能不能用 `handle_animal` 去填 `f: fn(Dog)` 这个位置？**

```rust
call_with_dog(handle_animal); // 也就是：把 Handler<Animal> 当成 Handler<Dog> 用
```

看一下实际发生的事：

* `call_with_dog` 保证只会传「狗」进去；
* `handle_animal` 能处理任何动物，当然也包括狗。

所以这整件事**是安全的**。

> 换句话说：**我们可以把 `Handler<Animal>` 当成 `Handler<Dog>` 用。**

写成 variance 格式就是：

> 如果 `Dog <: Animal`，那 `Handler<Animal> <: Handler<Dog>`
> 类型参数的方向「反过来了」，所以叫 **contra-variant**。

---

### 2.2 反过来呢？把 Handler<Dog> 当 Handler<Animal>？

现在反过来玩一下：假设我们有

```rust
fn call_with_animal(f: fn(Animal)) {
    let a = Cat;   // 注意，这里可能是 Cat 而不是 Dog
    f(a);
}
```

这里 `f: fn(Animal) = Handler<Animal>`。

你手里只有一个“只会处理狗”的函数：

```rust
fn handle_dog(d: Dog) {
    // 只会处理狗
}
```

你想干这种事：

```rust
call_with_animal(handle_dog); // 想把 Handler<Dog> 当成 Handler<Animal> 用
```

那就会炸：

* `call_with_animal` 可能传一只猫进来；
* `handle_dog` 接受参数类型是 `Dog`，拿到一只猫就不行了；
* 这就是**不安全的自动转换**。

所以 `Handler<Dog> -> Handler<Animal>` **不允许**。

---

### 2.3 总结一下 contravariant

> **contravariant**：
> 如果 `S <: T`，那 **`F<T> <: F<S>`**。
> 函数参数就是典型例子：`fn(T)` 对 T 是逆变的。

它完全不是拍脑袋定义的，而是从「**能不能安全地把一个东西当成另一个东西用**」推出来的。

---

## 3. invariant（不变）：两个方向都不安全，所以谁都不让转

现在来看第三种情况：**有些 F<T> 既不能 covariant，也不能 contravariant**，
那只能「**完全不变**」，也就是 **invariant**。

最经典的例子：一个既能读又能写 T 的容器。

比如这样一个东西（伪代码）：

```rust
struct Cell<T> {
    value: T,
}

impl<T> Cell<T> {
    fn get(&self) -> &T { &self.value }       // 读
    fn set(&mut self, v: T) { self.value = v } // 写
}
```

问题照旧：`Dog <: Animal`，
`Cell<Dog>` 与 `Cell<Animal>` 有啥自动转换关系？

---

### 3.1 假设它是 covariant：Cell<Dog> -> Cell<Animal>

假设有个函数：

```rust
fn use_cell_of_animal(c: &mut Cell<Animal>) {
    c.set(Cat);           // 往里面塞一只猫
}
```

我们手里有一个 `Cell<Dog>`：

```rust
let mut cdog: Cell<Dog> = Cell { value: Dog };
```

如果允许自动转换 `Cell<Dog> -> Cell<Animal>`，那就可以：

```rust
use_cell_of_animal(&mut cdog);
```

然后 `use_cell_of_animal` 往里面塞了一只 `Cat`：

* 从函数视角看，是塞到了 `Cell<Animal>` 里，没毛病；
* 但实际上 `cdog` 的真实类型是 `Cell<Dog>`，
  也就是说 **你现在有一个 `Cell<Dog>`，里面装了一只猫** → 类型不一致。

这就炸了。

所以 **不能 covariant**。

---

### 3.2 假设它是 contravariant：Cell<Animal> -> Cell<Dog>

再假设你有：

```rust
fn use_cell_of_dog(c: &mut Cell<Dog>) {
    let d: &Dog = c.get();   // 期望里面一定是 Dog
    // do something that only works if it's actually a Dog
}
```

手里有一个 `Cell<Animal>`：

```rust
let mut canimal: Cell<Animal> = Cell { value: Cat };
```

如果允许 `Cell<Animal> -> Cell<Dog>`，你就可以写：

```rust
use_cell_of_dog(&mut canimal);
```

函数体里 `c.get()` 拿到的是 `&Animal`，

* 但是 `use_cell_of_dog` 把它当成 `&Dog` 来用；
* 实际上可能是 `Cat`，又炸。

所以 **不能 contravariant**。

---

### 3.3 那还能怎么办？ → invariant

我们发现：

* 当你既能 **读** T 又能 **写** T 的时候：

  * 「把 `Cell<Dog>` 当成 `Cell<Animal>`」会炸；
  * 「把 `Cell<Animal>` 当成 `Cell<Dog>`」也会炸。

唯一安全的自动转换规则是：

> 只有在 **T 完全一样** 的时候才能互相当成对方。
> 即：`Cell<Dog>` 只能当 `Cell<Dog>` 用。

这就是「**invariant**」：

> 如果 `S <: T`，那既不有 `F<S> <: F<T>`，也不有 `F<T> <: F<S>`，
> 只在 `S = T` 时才有 `F<S> = F<T>` 的关系。

也就是说：

* **covariant**：可以“往上转”（子 → 父）
* **contravariant**：可以“往下转”（父 → 子）
* **invariant**：谁也不能转

而这三种是**从「哪里自动转型会炸」推出来的，不是随意起的名字**。

---

## 4. 现在回头看：`'id` 作为“品牌”，我们想要的是哪种？

BrandedVec 里的 `'id` 是想当「**品牌标签**」，不是普通意义上的“生命周期长短”。

我们想要的语义是：

* `BrandedVec<'id1, T>` 和 `BrandedVec<'id2, T>` 之间，**只要 `'id1 != 'id2` 就绝对不能互转**；
* 同理，`BrandedIndex<'id1>` 永远不能当作 `BrandedIndex<'id2>` 用。

这正是「**invariant** 的行为」：
必须 `'id1 == 'id2` 才能用，哪怕 `'id1: 'id2` 或 `'id2: 'id1` 也不行。

如果 `'id` 不 invariant 会怎样？

* 假设 `'long` 活得比 `'short` 久，Rust 里通常有 `'long: 'short`；

* 如果 `BrandedIndex<'id>` 在 `'id` 上是协变的，编译器可能会帮你做类似这种：

  ```rust
  let idx_long: BrandedIndex<'long> = ...;
  let idx_short: BrandedIndex<'short> = idx_long; // 自动往“短”转
  ```

* 一旦有这种转换，就有机会把一个本来属于 Vec1 的 index 混到 Vec2 里去（因为 Vec 的 `'id` 可能也被转换了）；

* 这会直接毁掉 BrandedVec “永不越界”的核心保证。

> 所以我们要从根本上禁止任何基于 `'id` 的自动转换：
> `'id` 在 `BrandedVec` / `BrandedIndex` 中**必须是 invariant**。

而 Rust 的做法是：**通过看类型结构自动算 variance**，比如：

* `&'a T` 对 `'a` 是 covariant；
* `*mut T` 对 `T` 是 invariant；
* `PhantomData<U>` 获得 U 的 variance。

那么：

```rust
struct InvariantLifetime<'id>(PhantomData<*mut &'id ()>);
```

一步步展开：

1. `&'id ()` 对 `'id` 是 **协变**（类似 `&T`）；
2. `*mut X` 对 `X` 是 **不变**（因为原始指针可以被写和读，类似上面 Cell 的例子）；
3. 组合起来：`*mut &'id ()` 对 `'id` 的效果是 **不变**；
4. 再包一层 `PhantomData`：沿袭 `*mut &'id ()` 的 variance，所以 `InvariantLifetime<'id>` 对 `'id` 是 **不变**；
5. 把这个东西塞进 `BrandedVec` / `BrandedIndex`，它们的 `'id` 就跟着变成 invariant。

所以 `*mut &'id ()` 只是一个「**我知道这个结构会让 `'id` 变成 invariant** 的模板」，
从「函数参数是 contravariant，读写 cell 是 invariant」这一类**非常原始的 substitutability 规则**推出来，完全不是拍脑袋的。

---

## 5. 回答你的那句话：“我们知道为了 BrandedVec 需要 invariant，但一开始为什么这玩意合理？”

总结一下逻辑链：

1. **起点**：我们希望类型系统能帮我们做一些「自动转型」，比如把狗当动物用。
2. **要求**：这些自动转型必须满足「**任何地方期待 B 时，你给它一个 A 都不会炸**」。
3. 对每个泛型构造器 `F<T>`，我们问：“已知 `S <: T`，哪些方向的 `F<S>` / `F<T>` 自动转型是安全的？”

   * 只 `F<S> -> F<T>` 安全 ⇒ F covariant；
   * 只 `F<T> -> F<S>` 安全 ⇒ F contravariant；
   * 两个方向都不安全 ⇒ F invariant。
4. 函数参数自然落在 contravariant，那是因为「参数是在负位」；
   读写 Cell 自然落在 invariant，那是因为两边都用到了 T 的能力。
5. BrandedVec 想要一个“品牌标签 `'id`”，语义是「**除了完全相同的 `'id`，任何自动转型都不允许**」，
   这恰好就是 invariant 的模式。
6. Rust 提供了根据类型结构自动推 variance 的机制；我们利用 `*mut &'id ()` 这个结构，
   把 `'id` 放在一个“会被算成 invariant”的位置上，借力打力。

所以：

* variance 这三个词不是随便造的；
* 它们是回答同一个非常具体的问题：「**什么样的自动转型是永远不出事的？**」；
* BrandedVec 只是利用了**第三种情况（invariant）**，把 `'id` 从“生命周期长短”转成了“纯身份标签”。

---

如果你愿意，下一步我们可以写一个**极小的 Rust demo**：

* 先写一个 `struct Cov<T>(PhantomData<&'static T>);`
* 再写一个 `struct Invar<T>(PhantomData<*mut T>);`
* 然后用几行代码让编译器报出“cannot convert Cov<'a> to Cov<'b> / can convert”之类的错误，用**编译器报错**来“实测” variance 的行为，比嘴上讲可能更直观。
