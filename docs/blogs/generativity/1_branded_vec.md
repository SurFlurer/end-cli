好，咱们就把自己当成一条什么都不懂的小蠢龙，**从零把 BrandedVec 这段代码捋一遍**：
先给你一个「**简化版库代码**」，再给「**调用处代码**」，然后**一行一行地推**「编译器在想啥」。

---

## 0. 先说一句：BrandedVec 的目标到底是啥

普通 `Vec`：

```rust
let mut v = vec![10, 11, 12];
let x = v[1];        // 这里有运行时 bounds check
let y = v.get(1);    // 这里也有运行时检查（返回 Option）
```

BrandedVec 想要做到：

> **我已经用某种方式确认 index 永远合法了，那后面的访问就别再做检查了，直接 `get_unchecked`，但是依然保证「永远不越界」。**

做法就是：
**给每个 Vec 和它的 index 都贴一个同样的「品牌」 `'id`，品牌不一样就不能混用。**

---

## 1. 简化版库代码长什么样（你可以当“差不多是这样”的伪代码）

### 1.1 品牌类型 InvariantLifetime 和两个新类型

```rust
use std::marker::PhantomData;

/// 一个“只在类型里存在”的生命周期参数，不占空间。
/// 真实论文里是 PhantomData<*mut &'id ()>，我这里简化一下。
struct InvariantLifetime<'id>(PhantomData<*mut &'id ()>);

/// 带品牌的 Vec：真正的数据在 `inner` 里，`'id` 只是贴在类型上的“牌子”。
pub struct BrandedVec<'id, T> {
    inner: Vec<T>,
    _marker: InvariantLifetime<'id>,
}

/// 带同一品牌的下标。
pub struct BrandedIndex<'id> {
    idx: usize,
    _marker: InvariantLifetime<'id>,
}
```

实际论文里就是这个结构：`BrandedVec<'id, T>` 和 `BrandedIndex<'id>` 只是普通 `Vec<T>` 和 `usize` 的新类型包装，外加一个 `InvariantLifetime<'id>` 来强制 `'id` 不可变（invariant），保证品牌不会被隐式转换掉。

### 1.2 关键方法签名

这部分和论文里的 API 是一致的：

```rust
impl<'id, T> BrandedVec<'id, T> {
    /// push 一个元素，返回“带同一品牌”的下标。
    pub fn push(&mut self, val: T) -> BrandedIndex<'id> {
        let idx = self.inner.len();
        self.inner.push(val);
        BrandedIndex { idx, _marker: InvariantLifetime(PhantomData) }
    }

    /// 做一次正常的 bounds check，把 usize 变成“checked index”。
    pub fn get_index(&self, index: usize) -> Option<BrandedIndex<'id>> {
        if index < self.inner.len() {
            Some(BrandedIndex { idx: index, _marker: InvariantLifetime(PhantomData) })
        } else {
            None
        }
    }

    /// 注意：这里不再做 bounds check，直接 unchecked。
    pub fn get(&self, index: BrandedIndex<'id>) -> &T {
        unsafe { self.inner.get_unchecked(index.idx) }
    }

    pub fn get_mut(&mut self, index: BrandedIndex<'id>) -> &mut T {
        unsafe { self.inner.get_unchecked_mut(index.idx) }
    }
}
```

**要点：**

* `get_index` 是**唯一一次真正做运行时检查的地方**；
* 一旦你有了 `BrandedIndex<'id>`，后面的 `get` / `get_mut` 全部 unchecked，**但类型保证你不会拿错 Vec**。

### 1.3 最诡异的：`new` 的签名

论文里给的 `new`：

```rust
impl<'id, T> BrandedVec<'id, T> {
    // 实际上这通常会是关联函数，不太在乎 impl 块怎么写，关键是签名
    pub fn new<R>(
        inner: Vec<T>,
        f: impl for<'a> FnOnce(BrandedVec<'a, T>) -> R,
    ) -> R {
        // 伪代码：真正实现是 unsafe + 一堆花活，这里只讲“抽象行为”：
        //
        // 1. 选择一个全新的品牌 'idX（你在任何地方都取不到它的名字）
        // 2. 构造 bvec: BrandedVec<'idX, T> { inner, _marker: ... }
        // 3. 调用 f(bvec)
        // 4. 把 f 的返回值 R 作为 new 的返回值
        todo!()
    }
}
```

关键是参数 `f` 的类型：

```rust
for<'a> FnOnce(BrandedVec<'a, T>) -> R
```

意思是：**不管给你一个什么生命周期 `'a`，你都能接受一个 `BrandedVec<'a, T>` 并返回一个 `R`**。
也就是说：

* 是 `new` 来选 `'a`（起名 `'id1`、`'id2` 之类）；
* 你（闭包）**不能假设任何关于 `'a` 的额外信息**，只能把它当成“我有一个带某个品牌的 Vec”。

这就是所谓的 **rank-2 多态**：外层的函数 `new` 可以「随便选一个 `'_`」，而你的闭包必须能接受“任何 `'_`”。

---

## 2. 调用处代码（原文那段）

论文里给了这么一个例子：

```rust
let vec1: Vec<u8> = vec![10, 11];
let vec2: Vec<u8> = vec![20, 21];

BrandedVec::new(vec1, move |mut bvec1: BrandedVec<u8>| { // bvec1: BrandedVec<'id1, u8>
    bvec1.push(12);
    let i1 = bvec1.push(13); // i1: BrandedIndex<'id1>

    BrandedVec::new(vec2, move |mut bvec2: BrandedVec<u8>| { // bvec2: BrandedVec<'id2, u8>
        let i2 = bvec2.push(22); // i2: BrandedIndex<'id2>
        *bvec2.get_mut(i2) -= 1;  // 20,21,22 => 20,21,21
        println!("{:?}", bvec2.get(i2)); // 打印 21

        println!("{:?}", bvec1.get(i1)); // 打印 13

        // println!("{:?}", bvec2.get(i1)); // 这一行如果取消注释会编译错误
    }); // end of `bvec2` closure
}); // end of `bvec1` closure
```

这就是你想看的「**用户代码**」。

下面我们就从上到下一行一行**模拟编译器脑内发生了什么**。

---

## 3. 一行一行完整推一遍

### 3.1 创建两个普通 Vec

```rust
let vec1: Vec<u8> = vec![10, 11];
let vec2: Vec<u8> = vec![20, 21];
```

这里没有任何鬼东西，就是普通的 `Vec<u8>`，还没有品牌。

---

### 3.2 第一层 `BrandedVec::new` —— 生成 brand `'id1`

```rust
BrandedVec::new(vec1, move |mut bvec1: BrandedVec<u8>| {
    ...
});
```

**类型推导阶段发生的事：**

1. 编译器看到我们调用 `BrandedVec::new::<u8, _>(vec1, closure1)`。

2. `new` 的签名是：

   ```rust
   pub fn new<R>(inner: Vec<T>, f: impl for<'a> FnOnce(BrandedVec<'a, T>) -> R) -> R
   ```

   这里 `T = u8`，`inner = vec1`，`f = closure1`。

3. 因为 `f` 的类型是 `for<'a> FnOnce(BrandedVec<'a, u8>) -> R`，
   所以编译器会把你的闭包看成：

   ```rust
   |mut bvec1: BrandedVec<'id1, u8>| -> R1 { ... }
   ```

   这里的 `'id1` 是编译器给这一层 `new` **新造的一个品牌**。

   * 你在代码里写 `BrandedVec<u8>`，编译器实际上填的是 `BrandedVec<'id1, u8>`；
   * `'id1` 在整个程序里只有这一个地方出现（这段闭包内部）。

4. 在运行时，「概念上」发生的事情是：

   * `vec1` 被 `new` 吃掉；
   * `new` 把它包成一个 `BrandedVec<'id1, u8>`（`inner: vec1`）；
   * 然后调用你的闭包：`closure1(bvec1)`。

所以，你可以在脑内理解为：

```rust
// 想象 new 被展开成这样（伪代码）：
let bvec1: BrandedVec<'id1, u8> = BrandedVec::from_vec(vec1);
{
    // 闭包体
}
```

---

### 3.3 在第一层闭包里：两次 push，得到 `i1`

```rust
bvec1.push(12);
let i1 = bvec1.push(13);
```

利用我们之前的库定义：

```rust
impl<'id, T> BrandedVec<'id, T> {
    pub fn push(&mut self, val: T) -> BrandedIndex<'id> { ... }
}
```

推类型非常直接：

* 第一行：
  `bvec1` 的类型是 `BrandedVec<'id1, u8>`，
  所以 `bvec1.push(12)` 里 `self: &mut BrandedVec<'id1, u8>`，返回值类型是 `BrandedIndex<'id1>`，但我们没接住它，丢了。

* 第二行：
  `let i1 = bvec1.push(13);`
  这里 `i1: BrandedIndex<'id1>`，**带着同样的品牌 `'id1`**。

你可以把 `'id1` 理解成「贴在 bvec1 上的二维码」，
`push` 返回的 `i1` 也是同一个二维码，只能用来打开这条带 `'id1` 的 Vec。

到这里，第一层闭包环境中有：

* `bvec1: BrandedVec<'id1, u8>`（里面现在是 `[10, 11, 12, 13]`）
* `i1:     BrandedIndex<'id1>`（指的是 index 3）

---

### 3.4 第二层 `BrandedVec::new` —— 生成 brand `'id2`

```rust
BrandedVec::new(vec2, move |mut bvec2: BrandedVec<u8>| {
    ...
});
```

同样的推导流程再来一遍：

1. 这次 `new::<u8, _>(vec2, closure2)`。
2. `closure2` 被看成：`|mut bvec2: BrandedVec<'id2, u8>| -> R2 { ... }`

   * `'id2` 是这一层 `new` 新造出来的品牌；
   * 和 `'id1` 完全无关，是两个不同标签。
3. 运行时概念上变成：

   ```rust
   let bvec2: BrandedVec<'id2, u8> = BrandedVec::from_vec(vec2);
   {
       // 第二层闭包体
   }
   ```

此时第二层闭包环境里有：

* `bvec2: BrandedVec<'id2, u8>`（初始数据 `[20, 21]`）
* 外层闭包里的东西（`bvec1`, `i1`）也能被这个闭包捕获。

---

### 3.5 在第二层闭包里：push & 修改 & 打印

```rust
let i2 = bvec2.push(22);
*bvec2.get_mut(i2) -= 1;
println!("{:?}", bvec2.get(i2));
println!("{:?}", bvec1.get(i1));
// println!("{:?}", bvec2.get(i1)); // 这一行会编译错误
```

#### 3.5.1 `let i2 = bvec2.push(22);`

* `bvec2: BrandedVec<'id2, u8>`
  ⇒ `i2: BrandedIndex<'id2>`。
* 此时 `bvec2.inner` 从 `[20, 21]` 变成 `[20, 21, 22]`。

#### 3.5.2 `*bvec2.get_mut(i2) -= 1;`

看一下签名：

```rust
fn get_mut(&mut self, index: BrandedIndex<'id>) -> &mut T
```

这里：

* `self` 是 `&mut BrandedVec<'id2, u8>`;
* `index` 是 `BrandedIndex<'id2>`（参数 `i2`）；
* 所以类型统一得很好：`'id = 'id2`。

函数体（简化版）：

```rust
pub fn get_mut(&mut self, index: BrandedIndex<'id>) -> &mut T {
    unsafe { self.inner.get_unchecked_mut(index.idx) }
}
```

**没有 bounds check**，完全依靠“不可能构造出非法 `BrandedIndex<'id2>`”这一事实保证安全。

执行过程：

* `i2` 其实就是 `idx = 2`；
* 所以把 `bvec2.inner[2] = 22` 改成 `21`；
* 结果 `bvec2` 里实际变成 `[20, 21, 21]`。

#### 3.5.3 `println!("{:?}", bvec2.get(i2));`

同理：

```rust
fn get(&self, index: BrandedIndex<'id>) -> &T
```

* `self: &BrandedVec<'id2, u8>`;
* `index: BrandedIndex<'id2>`;
* 返回 `&u8` 指向 `inner[2]`（值 21），打印 21。

#### 3.5.4 `println!("{:?}", bvec1.get(i1));`

现在看起来好像有点 spooky：我们**在第二层闭包里面操作第一层的 Vec**：

* `bvec1: BrandedVec<'id1, u8>`；
* `i1:    BrandedIndex<'id1>`；
* 调用 `bvec1.get(i1)`，也没问题，因为类型完全对得上：

  ```rust
  &BrandedVec<'id1, u8> + BrandedIndex<'id1> -> &u8
  ```

运行时表现：

* `i1` 指的是 `bvec1` 里的 index 3，值是 13；
* 所以打印 13。

注意这个例子说明了一件事：

> **品牌 `'id` 不是“作用域”这么简单，而是「哪一个具体 Vec」的标签。**
> 多个 Vec 同时活着时，每个 Vec 有自己的 `'idX`，索引也跟着品牌走。

---

### 3.5.5 如果取消注释这一行会发生什么？

```rust
// println!("{:?}", bvec2.get(i1));
```

来推一下类型：

* `bvec2.get` 的签名：

  ```rust
  fn get<'id>(&self, index: BrandedIndex<'id>) -> &T
  // 更准确：impl<'id, T> BrandedVec<'id, T> { fn get(&self, BrandedIndex<'id>) -> &T }
  ```

* 对于这条调用来说：

  * `self` = `&BrandedVec<'id2, u8>`，所以这里 `'id` 必须替换成 `'id2`；
  * 也就是说它**期望的参数类型**是 `BrandedIndex<'id2>`；
  * 但我们实际传入的是 `i1: BrandedIndex<'id1>`。

编译器想 unify：

```text
BrandedIndex<'id2>  ==  BrandedIndex<'id1>
```

但是 BrandedIndex 里的 `'id` 是 **invariant** 的（因为用了那个 `InvariantLifetime<'id>` 的新类型），
所以 `'id1` 和 `'id2` 必须「完全相等」才行。偏偏这两个是两个调用 `new` 时各自产生的**不同 brand**。

因此编译器直接报错，大意是：

```text
expected struct `BrandedIndex<'id2>`,
   found struct `BrandedIndex<'id1>`
```

这就防止了一个严重 bug：
如果真的执行了 `bvec2.get(i1)`，`i1` 的数值是 3，而 `bvec2` 的长度只有 3（index 0,1,2），这就越界了。

---

## 4. 把“库 + 用户”连在一起的终极口胡版

你可以这样脑补「一次调用 `BrandedVec::new` 全流程」：

1. 你写：

   ```rust
   let vec = vec![10, 11];
   BrandedVec::new(vec, |mut bvec| {
       let i = bvec.push(99);
       println!("{}", bvec.get(i));
   });
   ```

2. 编译器把它想象成：

   ```rust
   // Step 1：给这一层 new 生成一个独一无二的标签 'idX
   // Step 2：把普通 Vec 包装成 BrandedVec<'idX, _>
   let mut bvec: BrandedVec<'idX, u8> = BrandedVec::from_vec(vec![10, 11]);

   // Step 3：执行你的闭包（其中所有对 bvec 的操作都是带品牌的）
   let i: BrandedIndex<'idX> = bvec.push(99);   // idx = 2
   let x: &u8 = bvec.get(i);                    // unsafe 下标访问，但永不越界
   println!("{}", x);                           // 打印 99
   ```

3. 你**永远没办法拿到 `'idX` 这个名字**，也没办法造出其它 `BrandedVec<'idX, T>` 或 `BrandedIndex<'idX>` 除了这条 `Vec` 提供的。

4. 但是在闭包内部，你就把 `BrandedVec<'idX, T>` 当成普通 `Vec<T>` 用就行，只不过：

   * 需要运行时 bounds check 的地方用 `get_index`；
   * 之后所有访问都用 `get` / `get_mut`，不再检查。

> 口胡：**「`new` 是一个可以给 Vec 贴一次性标签并在一个闭包里让你用这条贴好标签的 Vec 的函数；你在闭包里拿到的 index 也带着同一个标签，从而确保你永远不会用错 Vec。」**

---

## 5. 如果你想照着抄一个极简 demo

一个能在自己项目里调试感受的极简 demo（省略 unsafe 和 variance 的严谨性，只保留核心结构）大概是：

```rust
use std::marker::PhantomData;

struct Brand<'id>(PhantomData<*mut &'id ()>);

pub struct BrandedVec<'id, T> {
    inner: Vec<T>,
    _marker: Brand<'id>,
}

pub struct BrandedIndex<'id> {
    idx: usize,
    _marker: Brand<'id>,
}

// 这个在真实 Rust 里写不出完整实现（需要 unsafe 花活），
// 但可以这样想象：new 用于“开一个 brand 作用域”。
impl<T> BrandedVec<'static, T> {
    pub fn new<R>(
        inner: Vec<T>,
        f: impl for<'id> FnOnce(BrandedVec<'id, T>) -> R,
    ) -> R {
        // 真实现要通过 unsafe/transmute 之类构造出那个 'id，
        // 我们这里只管抽象行为，不实现。
        todo!()
    }
}

impl<'id, T> BrandedVec<'id, T> {
    pub fn push(&mut self, value: T) -> BrandedIndex<'id> {
        let idx = self.inner.len();
        self.inner.push(value);
        BrandedIndex { idx, _marker: Brand(PhantomData) }
    }

    pub fn get_index(&self, index: usize) -> Option<BrandedIndex<'id>> {
        if index < self.inner.len() {
            Some(BrandedIndex { idx: index, _marker: Brand(PhantomData) })
        } else {
            None
        }
    }

    pub fn get(&self, index: BrandedIndex<'id>) -> &T {
        &self.inner[index.idx] // 这里先别管 unchecked，写个安全版本感受一下
    }

    pub fn get_mut(&mut self, index: BrandedIndex<'id>) -> &mut T {
        &mut self.inner[index.idx]
    }
}
```

然后在 main 里抄论文例子试着写一遍，编译器就会帮你体验一把「品牌不匹配」是什么感觉了。

---

如果你愿意，下一步我们可以：

* 把**真实可编译**的一个“小号 BrandedVec crate”写出来（用 `unsafe` 填掉 `todo!()`，看看 Clippy / Miri 有什么意见）；
* 或者进一步拆：**把 `for<'a>` 这层 HRTB 用更接地气的“手摇泛型”方式解释一遍**（比如类比 Haskell `runST`）。
