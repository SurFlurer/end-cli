先简单把这篇《GhostCell: Separating Permissions from Data in Rust》捋一遍，方便后面一起「细啃」。

> 我会按：问题背景 → BrandedVec 热身 → GhostCell API → 双向链表例子 → 图里的细节 来说。需要的话我们之后可以一段段地精读代码 / proof。

---

## 1. 这篇论文到底在解决什么问题？

Rust 的核心原则是 **AXM：aliasing XOR mutability**

> 要么「很多别名 + 只读」，要么「唯一别名 + 可变」，不能既有很多别名又能写。

对树这种「没有共享指针」的数据结构，AXM 特别好用：

* 有个 `&mut Root`，整棵树都能随便改；
* 有一堆 `&Root`，大家可以只读访问，线程安全。

**但对于有内部共享的数据结构（graphs / doubly-linked list / cyclic 结构），AXM 很难受：**

* 你天然会有「一个节点有多个前驱」这种 alias；
* 按照 Rust base 规则，这些被 alias 的 node 是不能通过安全 API 写的。

现实里的两种办法：

1. **直接 `unsafe`：**像 `std::collections::LinkedList` 一样，用裸指针自己保证不要踩雷。
2. **每个节点套一个「cell 型同步原语」：**`RwLock<Node>` / `Mutex<Node>` / `RefCell<Node>`，让你拿 `&NodeRef` 也能间接改。
   代价是：

   * 要么有运行时借用检查（`RefCell`）；
   * 要么每个节点都有锁（`RwLock` / `Mutex`），遍历时疯狂加解锁，性能炸裂。

GhostCell 的目标可以一句话概括：

> **不牺牲安全性，也不牺牲性能，用类型系统静态管理「共享可变」的数据结构。**

核心思路：**把「权限」从「数据本身」上拆出来，用 branded types（幽灵生命周期）把它们重新连起来。**

---

## 2. 热身：BrandedVec（用「品牌化下标」干掉 bounds check）

论文第二节先讲一个经典 warm-up：**BrandedVec + BrandedIndex**。

### 2.1 普通 Vec 的问题

* 访问 `vec[i]` 会做 **运行时边界检查**；越界就 panic。
* 有些场景中你在逻辑上已经能证明 `i` 永远合法，但编译器不懂，只能每次检查。

BrandedVec 的想法：

* 把 `Vec<T>` 包一层，变成 `BrandedVec<'id, T>`；
* 把合法的下标也包一层，变成 `BrandedIndex<'id>`；
* 这个 `'id` 是一个「**品牌生命周期**」，**唯一标识这条具体的 vector**。

于是类型系统保证：

> 只有「带同一个 brand 的 index」才能访问这个 vector，并且这些 index 一旦被构造出来，一直合法（因为这个 Vec 之后只会增长不会缩短）。

### 2.2 API 形状

（第 7 页左右的伪代码）

```rust
struct BrandedVec<'id, T> { ... }
struct BrandedIndex<'id> { ... }

impl<'id, T> BrandedVec<'id, T> {
    pub fn new<R>(
        inner: Vec<T>,
        f: impl for<'a> FnOnce(BrandedVec<'a, T>) -> R
    ) -> R { ... }

    pub fn push(&mut self, val: T) -> BrandedIndex<'id> { ... }

    pub fn get_index(&self, index: usize) -> Option<BrandedIndex<'id>> { ... }

    pub fn get(&self, index: BrandedIndex<'id>) -> &T { ... }
    pub fn get_mut(&mut self, index: BrandedIndex<'id>) -> &mut T { ... }
}
```

关键几个点：

1. `new` 的类型有个 **rank-2 多态**：`for<'a> FnOnce(BrandedVec<'a, T>) -> R`

   * 意味着：`new` **自己选一个新生命周期 `'id`**，然后把 `BrandedVec<'id, T>` 交给你的闭包；
   * 闭包不能假设任何关于 `'id` 的额外性质，只知道「有这么个 brand」。
2. `push` 返回 `BrandedIndex<'id>`：
   下标上也绑定了同一个 `'id`。
3. `get_index`：用普通 usize 试图变成 branded index，如果越界返回 `None`，如果成功，你之后就可以用 `get` / `get_mut` **无检查访问**。
4. 注意没有 `pop`：
   这是为了保证「**长度单调不减**」，否则旧 index 就可能变成悬空。

### 2.3 实现里的 `InvariantLifetime<'id>`

类型定义大概是：

```rust
struct InvariantLifetime<'id>(PhantomData<*mut &'id ()>);

struct BrandedVec<'id, T> {
    inner: Vec<T>,
    _marker: InvariantLifetime<'id>,
}

struct BrandedIndex<'id> {
    idx: usize,
    _marker: InvariantLifetime<'id>,
}
```

* `PhantomData` 确保这个东西在类型系统里「**拥有** `'id`」，但运行时占用 **0 字节**。
* 用 `*mut &'id ()` 这种写法让编译器认为 `'id` 是 **invariant** 的，防止通过子类型把 `'id1` 偷换成 `'id2`。

unsafe 出现在哪？

* 在 `get` / `get_mut` 里，用 `get_unchecked` 或直接做 unchecked indexing；
* 安全性保证完全靠上面这套「品牌 + 单调长度 + 没有 pop」的约束；
* 这也是为什么需要 RustBelt 那套语义证明来证明「真的不会越界」。

---

## 3. GhostCell 的 API：把「权限」抽出来

热身完，就进正菜：GhostCell。核心是两个类型：

```rust
struct GhostToken<'id> {
    _marker: InvariantLifetime<'id>
}

struct GhostCell<'id, T: ?Sized> {
    _marker: InvariantLifetime<'id>,
    value: UnsafeCell<T>,
}
```

含义：

* `GhostCell<'id, T>`：带 brand `'id` 的数据单元，「**数据** 在这里」。
* `GhostToken<'id>`：对所有 brand 为 `'id` 的 GhostCell 的「**总权限**」。

API 的关键方法：

```rust
impl<'id> GhostToken<'id> {
    fn new<R>(f: impl for<'new_id> FnOnce(GhostToken<'new_id>) -> R) -> R { ... }
}

impl<'id, T> GhostCell<'id, T> {
    fn new(value: T) -> Self { ... }
    fn into_inner(self) -> T { ... }
    fn get_mut(&mut self) -> &mut T { ... }
    fn from_mut(t: &mut T) -> &mut Self { ... }

    fn borrow<'a>(&'a self, token: &'a GhostToken<'id>) -> &'a T {
        unsafe { &*self.value.get() }
    }

    fn borrow_mut<'a>(&'a self, token: &'a mut GhostToken<'id>) -> &'a mut T {
        unsafe { &mut *self.value.get() }
    }
}
```

注意几点：

1. `GhostToken::new` 和刚才 BrandedVec 的 `new` 一样，用 rank-2 多态生成一个全新的 `'id`。
2. **只要你有 `&GhostToken<'id>`，就可以对任何 `&GhostCell<'id, T>` 做只读 borrow；**
3. **只有你有 `&mut GhostToken<'id>`，才能对任何 `&GhostCell<'id, T>` 做可变 borrow；**
4. `borrow` / `borrow_mut` 里面就是 `UnsafeCell` 解引用，没有 runtime check，是「零开销」。

也就是说，**AXM 不再绑在 `&T` / `&mut T` 上，而是绑在 `&GhostToken` / `&mut GhostToken` 上**：

* 「谁拿着 token，谁就可以控制这批 GhostCell 的读写模式」；
* GhostCell 本身可以到处 alias，**但离开 token 你啥也干不了**。

线程安全：

* Rust 的借用检查保证：

  * 同一时刻要么有很多 `&GhostToken<'id>`（大家都只能读）、
  * 要么有唯一一个 `&mut GhostToken<'id>`（这个人可以写任意 GhostCell）。
* 于是你可以：

  * 把 `GhostToken<'id>` 放进 `RwLock`、`Mutex`、`channel` 等等；
  * 通过「谁拿到 token」来同步对整个数据结构的访问模式。

这就是论文标题里的：**Separating Permissions from Data**。

---

## 4. 双向链表例子：Node 结构 + API

### 4.1 Node 结构

链表节点定义（第 11 页左右）：

```rust
struct Node<'arena, 'id, T> {
    data: T,
    prev: Option<NodeRef<'arena, 'id, T>>,
    next: Option<NodeRef<'arena, 'id, T>>,
}

type NodeRef<'arena, 'id, T> =
    &'arena GhostCell<'id, Node<'arena, 'id, T>>;
```

这里有两个 lifetime：

* `'arena`：**真正的 lifetime**，表示这个节点存活在 arena 里；
* `'id`：**品牌 lifetime**，用来把这些 Node「绑到同一个权限域」上，和 GhostToken 对齐。

内存管理用的是 `TypedArena`，相当于 region-based memory：

* 所有 Node 放在同一个 arena；
* arena drop 时统一回收；
* 没有 per-node `Rc` 开销。

> 论文后面也有用 `Arc` 的版本，说明 GhostCell 本身不依赖具体分配方式。

### 4.2 公共 API：new / iterate / iter_mut / insert_next / remove

看一个关键方法签名（第 12–13 页）：

```rust
impl<'arena, 'id, T> Node<'arena, 'id, T> {
    pub fn new(
        data: T,
        arena: &'arena TypedArena<Node<'arena, 'id, T>>
    ) -> NodeRef<'arena, 'id, T> {
        GhostCell::from_mut(
            arena.alloc(Self { data, prev: None, next: None })
        )
    }

    pub fn iterate(
        node: NodeRef<'arena, 'id, T>,
        token: &GhostToken<'id>,
        f: impl Fn(&T),
    ) { ... }

    pub fn iter_mut(
        node: NodeRef<'arena, 'id, T>,
        token: &mut GhostToken<'id>,
        f: impl FnMut(&mut T),
    ) { ... }

    pub fn insert_next(
        node1: NodeRef<'arena, 'id, T>,
        node2: NodeRef<'arena, 'id, T>,
        token: &mut GhostToken<'id>,
    ) { ... }

    pub fn remove(
        node: NodeRef<'arena, 'id, T>,
        token: &mut GhostToken<'id>,
    ) { ... }
}
```

* `iterate`：只需要 `&GhostToken<'id>`，可以在多个线程里同时跑，因为只读。
* `iter_mut` / `insert_next` / `remove`：需要 `&mut GhostToken<'id>`，即「独占写权限」。

### 4.3 `insert_next` 的步骤 & 图 1（第 13 页）

源码（已简化过）：

```rust
pub fn insert_next(
    node1: NodeRef<'arena, 'id, T>,
    node2: NodeRef<'arena, 'id, T>,
    token: &mut GhostToken<'id>,
) {
    // Step 1: 把 node2 从它原来的位置上拆下来
    Self::remove(node2, token);

    // Step 2: 让 node1 和 node1_old_next 指向 node2
    let node1_old_next: Option<NodeRef<_>> = node1.borrow(token).next;
    if let Some(node1_old_next) = node1_old_next {
        node1_old_next.borrow_mut(token).prev = Some(node2);
    }
    node1.borrow_mut(token).next = Some(node2);

    // Step 3: 更新 node2 的 prev/next
    let node2: &mut Node<_> = node2.borrow_mut(token);
    node2.prev = Some(node1);
    node2.next = node1_old_next;
}
```

图 1（第 13 页的图）具体画了一个例子：把节点 `2` 插到 `(0,1,3)` 中 `1` 的后面，链上还有别的节点 4、5。步骤是：

1. **remove(node2)**：

   * 删掉 `4 -> 2 -> 5` 这段；
   * 把 `4.next` 改成 `5`，`5.prev` 改成 `4`；
2. **重连 node1 两侧的指针**：

   * 记录 `old_next = node1.next`（图中为 `3`）；
   * `old_next.prev = Some(node2)`；
   * `node1.next = Some(node2)`；
3. **更新 node2 本身的 prev/next**：

   * `node2.prev = Some(node1)`；
   * `node2.next = old_next`。

注意，每次修改某个节点时，都是：

* 先用 `borrow_mut(token)` 把 `&GhostCell` 变成 `&mut Node`；
* 改完立即结束这个 `&mut Node` 的生命周期，token 又可以再被借给下一次 `borrow_mut`。

**类型系统强迫你「一次只 mut 一个 Node」，不会在逻辑上同时拿到两个 `&mut Node` 指向同一块内存。**

### 4.4 限制：为什么做不了「所有节点的同时 `&mut` 迭代器」？

论文在这里直接说了一个硬限制：

* 标准库的 `LinkedList`（用 `unsafe` 实现）有一个「mutable iterator」，可以一次性给你「所有节点的 `&mut T`」；
* 这个 API 只有在**链表 acyclic** 的前提下才 sound；
* 但 GhostCell 版本允许你构造 **有环** 的列表（通过 `insert_next` 适当玩一玩）；
* 在有环的场景下，「拿一串 `&mut T`」很容易导致别名。

而 GhostCell + Rust 类型系统目前没法表达「**链表 acyclic**」这种全局性质，所以：

* **只读迭代**：可以一次给你一堆 `&T`（因为 alias 只读没问题）；
* **可变迭代**：只能「一个一个给 `&mut T`」。

---

## 5. Client 侧：把 GhostToken 放进锁里 / 线程里

论文后面给了一个小程序，展示「如何同时提供无锁只读 + 上锁修改」。

关键 pattern：

1. 用 `GhostToken::new(|mut token| { ... })` 拿到这个 brand 的 token。
2. 构造一个 list。
3. 用 `rayon::join` 开两个线程：

   * 每个线程只拿 `&GhostToken<'id>`；
   * 都调用 `Node::iterate`，无锁并发打印内容 —— 读多写少场景下非常自然。
4. 然后把 `token` 放进一个 `RwLock<GhostToken<'id>>`：

   * 拿 **read lock** 的线程获得 `&GhostToken<'id>`，可以只读遍历；
   * 拿 **write lock** 的线程获得 `&mut GhostToken<'id>`，可以 `iter_mut` 把所有节点加 100。

> 这段代码说明：我们真正「锁住」的是 **权限（GhostToken）**，而不是数据（nodes）。但通过 `'id` 这条幽灵连接，等价于整个 list 都在锁里。

你也可以不用锁，而是用 channel：

* 把 `GhostToken` 在线程之间「消息传递」，谁拿到了谁就有 mut 全权限；
* 由于 `GhostToken` 是 zero-sized type，传输成本理论上是 0（只要编译器能优化掉）。

---

## 6. Graph + DFS 例子（快速扫一眼）

结构基本类似：

```rust
struct Node<'arena, 'id, T> {
    data: T,
    uid: u32,
    edges: Vec<NodeRef<'arena, 'id, T>>,
}

struct DFSVisitor<'arena, 'id, T> {
    stack: Vec<NodeRef<'arena, 'id, T>>,
    mark: FixedBitSet, // 用 uid 做 visited map
}
```

DFS 的 `iter_mut`：

1. `stack.pop()` 拿到一个 `NodeRef`；
2. `borrow_mut(token)` 拿 `&mut Node` 改 `data`，并记录 `uid`；
3. `借用结束后`，再用 `borrow(token)`（只读）去访问 `edges` 并把未访问节点压栈。

这里也用到了「non-lexical lifetimes」：编译器可以看到 `node_mut` 只用到某一行，之后 `token` 又可以被认为是重新可借用的。否则这些写法会被 borrow checker 拒掉。

---

## 7. 我们可以怎么继续「一起看」

如果你愿意下一步深入，我可以按你感兴趣的方向选一块细抠，比如：

* **型系统 / Verus 视角**：详细展开 GhostCell 背后的 Branded types 语义、variance、和 RustBelt 里的 lifetime logic（`&κ_full P` / `[κ]` 那些规则）；
* **实战 API 设计**：对比 GhostCell 和典型的 `Rc<RefCell<T>>` / `Arc<RwLock<T>>`，分析什么时候值得引入 GhostCell 这种「权限抽象层」；
* **把这个 pattern 映射到你自己的项目**：比如你现在的 graph / catalog / IdVec 里，有没有可以用「brand + token」方式重构的地方。

你可以直接说「我们从第 X 节开始细读」，或者丢你现在卡住的那一段代码 / 公式出来，我就按那块往下拆。
