Rust 使用生命周期来追踪借用与所有权之间的关系。但是，实现生命周期检查并不简单，如果实现得不好，要么就限制太严，要么就允许了未定义行为。

为了能够灵活地使用生命周期，同时避免误用，Rust 使用了 **subtyping（子类型）** 和 **variance（变型）**。

让我们以这个例子开始。
```Rust
// Note: debug expects two parameters with the *same* lifetime
fn debug<'a>(a: &'a str, b: &'a str) {
    println!("a = {a:?} b = {b:?}");
}

fn main() {
    let hello: &'static str = "hello";
    {
        let world = String::from("world");
        let world = &world; // 'world has a shorter lifetime than 'static
        debug(hello, world);
    }
}
```
如果以一种保守的方式实现生命周期检查，那么，因为 `hello` 和 `world` 拥有不同的生命周期，我们可能看到下面的错误：
```
error[E0308]: mismatched types
 --> src/main.rs:10:16
   |
10 |         debug(hello, world);
   |                      ^
   |                      |
   |                      expected `&'static str`, found struct `&'world str`
```
这相当令人遗憾。在这种情况下，我们希望的是：接受任何跟 `'world` 至少活得一样长的类型。下面，让我们试试在生命周期中使用 subtyping。

## Subtyping
subtyping 的概念是：某个类型可以替代另一种类型来使用。
我们设想 `Sub` 是 `Super` 的子类型（在本文中会使用这样的标记 `Sub <: Super` )。
这意味着，`Sub` 满足了成为 `Super` 类型的所有条件，并且 `Sub` 还有可能满足了其他的额外条件。
那么，为了在生命周期中使用 subtyping，我们需要明确生命周期应当满足什么条件。
> `'a` 定义了一片代码区

之后我们可以定义它们之间是如何互相关联的
> `'long <: 'short` : 当且仅当 `'long` 定义了一片**完全包含** `'short` 的代码区

`'long` 可能定义了一块大于 `'short` 的区域，但这仍然符合我们的定义。
> 在之后的内容里，我们将发现 subtyping 其实远比这复杂和巧妙得多，但这条简单的规则已经覆盖了 99% 的情况，并且相当符合直觉。除非你在写 unsafe 代码，否则编译器会自动帮你处理所有的例外情况。
> 但是这是 Rustonomicon，我们正在写 unsafe 代码，所以我们需要理解这东西到底是如何工作的，以及什么情况下我们会搞坏它。

回到我们上面的例子，我们可以说 `'static <: 'world`。目前，让我们先接受一个这样的概念：生命周期的子类型可以通过引用传递（关于这一点稍后会在 Variance 节中讨论）。比如：`&'static str` 是 `&'world str` 的子类型，我们可以把`&'static str` 降级为 `&'world str` ，就像把子类降级为基类一样。这样一来，上面的例子就可以编译了。
```Rust
fn debug<'a>(a: &'a str, b: &'a str) {
    println!("a = {a:?} b = {b:?}");
}

fn main() {
    let hello: &'static str = "hello";
    {
        let world = String::from("world");
        let world = &world; // 'world has a shorter lifetime than 'static
        debug(hello, world); // hello silently downgrades from `&'static str` into `&'world str`
    }
}
```

## Variance
在上面一节中，我们略过了这样一个条件：`'static <: 'b` 意味着 `&'static T <: &'b T`。要使这样的条件成立，需要使用一个叫做 Variance 的属性。但它并不总是像这个例子那么简单。为了理解它，让我们稍稍扩展一下这个例子：
```Rust
fn assign<T>(input: &mut T, val: T) {
    *input = val;
}

fn main() {
    let mut hello: &'static str = "hello";
    {
        let world = String::from("world");
        assign(&mut hello, &world);
    }
    println!("{hello}"); // use after free 😿
}
```
在 `assign` 函数中，我们让 `hello` 指向了 `world`，但是在 `hello` 被 println使用之前， `world` 就离开了作用域。
这是一个经典的 use-after-free 问题！
我们的第一反应可能是把问题归咎于 `assign` 的实现，但是它确实没有任何问题。我们想把 `T` 赋值给另一个 `T`，这并不奇怪。
问题在于，当我们把 `hello` 作为 `input` 传入时，因为 `T` 对于两个参数都是相同的，对于 `val` ，`T` 被推断为 `&'b str` ；对于 `input` ，`T` 也需要是 `&'b str` ，实际上是把 `hello` 的类型 `& mut &‘static str` 转换为 `&mut &'b str` 传入了 `input` 。但是，我们不能假定 `&mut &'static str` 和 `&mut &'b str` 是兼容的。这意味着，`&mut &'static str` 不能成为 `&mut &'b str` 的子类型，尽管 `'static` 是 `'b` 的子类型。

Variance 是 Rust 引入的一个概念，用于通过泛型参数描述子类型之间的关系。

> NOTE: 我们定义泛型 `F<T>` ，以便讨论 `T` 。

`F` 的 variance 类型取决于它的输入如何影响输出，设有两个类型 `Sub` 和 `Super`，`Sub` 是 `Super` 的子类型，则：
- `F` 是协变 ( covariant )的——如果 `F<Sub>` 是 `F<Super>` 的子类型（子类型的关系可以传递）
- `F` 是逆变 ( contravariant ) 的——如果`F<Super>` 是 `F<Sub>` 的子类型（子类型的关系反转了）
- 否则 `F` 是不变 ( invariant ) 的，即子类型的关系不复存在

> NOTE: 与其他有继承功能的语言不同，比如 `class Cat` 协变为 `class Animal` ，之后就真的只能当 `Animal` 来用了（虽然可能指向的还是同一个对象，但原来的类型已经被完全隐藏了）。生命周期作为泛型参数传入，这里的 variance 只是用于约束传入的生命周期，并不会影响实际使用的生命周期。比如函数参数需要 `'a` 生命周期，返回值也是 `'a` 生命周期，但实际上用了一个 `'static`，那么返回的也是 `'static` ，而不会被降级。

如果我们还记得上面的例子，就可以知道如果 `'a <: 'b` ，那么`&'a T` 是 `&'b T` 的子类型，因此我们可以说 `&'a T` 在 `'a` 上是协变的。
同时，我们已经发现不能认为 `&mut &'a T` 是 `&mut &'b T` 的子类型，因此我们可以说 `&mut T` 在 `T` 上是不变的。
这里列出了常见类型的 variances:

|                 | 'a        | T                 | U         |
| --------------- | --------- | ----------------- | --------- |
| `&'a T `        | covariant | covariant         |           |
| `&'a mut T`     | covariant | invariant         |           |
| `Box<T>`        |           | covariant         |           |
| `Vec<T>`        |           | covariant         |           |
| `UnsafeCell<T>` |           | invariant         |           |
| `Cell<T>`       |           | invariant         |           |
| `fn(T) -> U`    |           | **contra**variant | covariant |
| `*const T`      |           | covariant         |           |
| `*mut T`        |           | invariant         |           |

注意这里的 `&'a mut T` 对 `'a` 是协变的，因为协变只是缩短生命周期，这是完全没问题的。
但对于 `fn(T) -> U`，这里函数参数是逆变的，所以对于 `fn(&'a mut T) -> U` 来说，它对 `'a` 也是逆变的，但对 `T` 仍然是不变的。

回到刚才的例子，因为`&mut T` 在 `T` 上是不变的，所以如果我们去掉那对大括号，仍然不能编译。
```Rust
fn assign<T>(input: &mut T, val: T) {
    *input = val;
}

fn main() {
    let mut hello: &'static str = "hello";
    let world = String::from("world");
    assign(&mut hello, &world);
    println!("{hello}");
}
```
```
error[E0597]: `world` does not live long enough
  --> src/main.rs:8:24
   |
 6 |     let mut hello: &'static str = "hello";
   |                    ------------ type annotation requires that `world` is borrowed for `'static`
 7 |     let world = String::from("world");
   |         ----- binding `world` declared here
 8 |     assign(&mut hello, &world);
   |                        ^^^^^^ borrowed value does not live long enough
 9 |     println!("{hello}");
10 | }
   | - `world` dropped here while still borrowed
```
因为 `hello` 的类型被我们声明为 `&'static str` ，不能改变，而因为 `&mut T` 的不变性，在传入 `assign` 函数时，`T` 只能被推导为 `&'static str` ，所以会要求 `val` 的类型也是`&'static str` 。
解决方法也很简单，去掉对 `hello` 的类型声明，让编译器自动推导，这样 `hello` 就会被推导为 `&'b str` ，和 `&world` 的类型一样，代码能够正常编译运行了。

### 更精确的生命周期
我们下面来看一个更复杂的例子
```Rust
struct Interface<'a> {
    manager: &'a mut Manager<'a>,
}

impl<'a> Interface<'a> {
    pub fn noop(self) {
        println!("interface consumed");
    }
}

struct Manager<'a> {
    text: &'a str,
}

struct List<'a> {
    manager: Manager<'a>,
}

impl<'a> List<'a> {
    pub fn get_interface(&'a mut self) -> Interface<'a> {
        Interface {
            manager: &mut self.manager,
        }
    }
}

fn main() {
	// 1 start
	let text = "hello";
    let mut list = List {
        manager: Manager { text },
    };
    // 1 end
	// 2 start
    list.get_interface().noop();
    // 2 end
    // 3 start

    println!("Interface should be dropped here and the borrow released");

    // this fails because inmutable/mutable borrow
    // but Interface should be already dropped here and the borrow released
    use_list(&list);
    // 3 end
}

fn use_list(list: &List) {
    println!("{}", list.manager.text);
}
```
```
error[E0502]: cannot borrow `list` as immutable because it is also borrowed as mutable
  --> src/main.rs:38:14
   |
32 |     list.get_interface().noop(); // 2
   |     ---- mutable borrow occurs here
...
38 |     use_list(&list); // 4
   |              ^^^^^
   |              |
   |              immutable borrow occurs here
   |              mutable borrow later used here
```
这段代码的表现非常奇怪，对 `list` 的可变引用应该在使用完 `noop` 方法后就销毁了，但实际上却没有，导致编译器抱怨我们在持有可变引用的同时尝试借用新的不可变引用。
我们来分析一下这段代码：
1. `text` 的生命周期是 `'static` 
2. `list` 的生命周期设为 `'l` ，作用域为 1-3，由于 `'static` 是 `'a` 的子类型，可以协变，所以 `list` 现在持有生命周期为 `'static` 的引用
3. 然后 `get_interface` 方法创建了一个 `list` 的可变引用 `&'a mut list` ，并返回了一个生命周期为 `'a` 的 `Interface<'a>`，`'a` 的作用域暂时未知
4. `noop` 方法销毁了 `Interface` ，但是引用只会失效（由编译器自动推导何时失效），不能被主动销毁，所以对 `&'a mut list` 没有影响
5. `use_list` 函数尝试对 `list` 创建新的不可变引用 `&'b list` ，作用域为 3。显然 `list` 此时仍然需要有效，所以 `'a` 的作用域是 1-3，则 `&'a mut list` 的作用域也是 1-3，所以无法再创建重叠的不可变引用 `&'b list`

我们可以简化一下这个例子：
```Rust
struct List<'a> {
    text: &'a str,
}

impl<'a> List<'a> {
    pub fn get_str(&'a mut self) -> &'a mut &str {
        &mut self.text
    }
}

fn main() {
    let mut list = List {
        text: "hello"
    };

    let _ = list.get_str();

    println!("Text should be dropped here and the borrow released");

    // this fails because inmutable/mutable borrow
    // but text should be already dropped here and the borrow released
    use_list(&list);
}

fn use_list(list: &List) {
    println!("{}", list.text);
}
```
这样问题就很明显了，并且也很好修复，只需要改一下 `get_str` 方法：
```Rust
// 改成这样
impl<'a> List<'a> {
    pub fn get_str<'b>(&'b mut self) -> &'b mut &'a str {
        &mut self.text
    }
}
// 或者
// 因为只需要增加 'b 这一个生命周期，可以省略
impl<'a> List<'a> {
    pub fn get_str(&mut self) -> &mut &'a str {
        &mut self.text
    }
}
```
更精确地表示生命周期，字符串本身的生命周期和 `List` 对象一致，但是对字符串的可变引用的生命周期可以更短。

### 逆变
前面我们已经详细讨论过协变与不变了，下面我们来看看逆变。
在 Rust 中，只有一种类型是逆变的，就是函数指针的参数。
我们先来解释为什么 `fn(T) -> U` 对 `U` 是协变的。
考虑这样的函数签名：
```Rust
fn get_str() -> &'a str;
```
也就是说，调用方期望，调用这个函数之后，获得一个生命周期为 `'a` 的引用
如果我实际传入这样的函数：
```Rust
fn get_static() -> &'static str;
```
显然是没问题的，调用方期望能得到一个生命周期为 `'a` 的引用，就是说引用至少能活得跟 `'a` 一样长，但实际上得到了一个生命周期为 `'static` 的引用，活得比 `'a` 更长。

但是对于参数来说就不一样了。
考虑这样的函数签名：
```Rust
fn store_ref(&'a str);
```
意味着调用方实际上会往函数里传一个生命周期为 `'a` 的引用，所以这个函数需要能处理任何活得至少跟 `'a` 一样久的引用。
如果我实际传入这样的函数：
```Rust
fn store_static(&'static str);
```
这个函数只能处理活得至少跟 `'static` 一样久的引用，但调用方传的是 `'a` ，根据协变规则，`'static` 可以协变为 `'a` ，但是不能反过来，所以传入这样的函数是不可行的。
来看一个具体的例子
```Rust
thread_local! {
    pub static StaticVecs: RefCell<Vec<&'static str>> = RefCell::new(Vec::new());
}

/// saves the input given into a thread local `Vec<&'static str>`
fn store(input: &'static str) {
    StaticVecs.with_borrow_mut(|v| v.push(input));
}

/// Calls the function with it's input (must have the same lifetime!)
fn demo<'a>(input: &'a str, f: fn(&'a str)) {
    f(input);
}

fn main() {
    demo("hello", store); // "hello" is 'static. Can call `store` fine

    {
        let smuggle = String::from("smuggle");

	    // 设这里 smuggle 是 'b，此处自动推导会得出 demo 要求 <'a> 为 'static, 但 'b 无法协变为 'static，因此无法编译
        demo(&smuggle, store);
    }

    // 如果允许编译，那这里就会出现 use after free 😿
    StaticVecs.with_borrow(|v| println!("{v:?}"));
}
```

但是，如果我们把这两个函数签名对调，就会发现引用可以自动协变了！
如果需要接收 `'static` 的函数，但实际上传入了接收 `'a` 的函数。当调用方传参的时候，传的是 `'static` ，可以自动协变为 `'a` ，完全没问题。
因此我们可以得出，函数指针对于参数是逆变的，对于返回值是协变的。

### variance in struct
简单来说，struct 继承了它的字段的 variance
比如一个 struct `MyType` ，它有一个泛型参数 `T` ，字段 `p` 的类型是 `T` ，那么 Mytype 对于 `T` 的 variance 就是 `p` 对于 `T` 的 variance
如果 `T` 被用在了多个字段呢？
有这样的规则：
- 如果所有使用了 `T` 的字段都是协变的，那么 `MyType` 对 `T` 是协变的
- 如果所有使用了 `T` 的字段都是逆变的，那么 `MyType` 对 `T` 是逆变的
- 否则，`MyType` 对 `T` 是不变的

```Rust
use std::cell::Cell;

struct MyType<'a, 'b, A: 'a, B: 'b, C, D, E, F, G, H, In, Out, Mixed> {
    a: &'a A,     // covariant over 'a and A
    b: &'b mut B, // covariant over 'b and invariant over B

    c: *const C,  // covariant over C
    d: *mut D,    // invariant over D

    e: E,         // covariant over E
    f: Vec<F>,    // covariant over F
    g: Cell<G>,   // invariant over G

    h1: H,        // would also be covariant over H except...
    h2: Cell<H>,  // invariant over H, because invariance wins all conflicts

    i: fn(In) -> Out,       // contravariant over In, covariant over Out

    k1: fn(Mixed) -> usize, // would be contravariant over Mixed except..
    k2: Mixed,              // invariant over Mixed, because invariance wins all conflicts
}
```

## 其他
本来是想翻译《The Rustonomicon》的这篇文章：[Subtyping and Variance](https://doc.rust-lang.org/nomicon/subtyping.html#subtyping-and-variance) ，但是翻译了一半之后就有点懒了🫠，所以前半部分基本是直译，加了一点自己的东西，后半部分就只是参考这篇文章写的相关内容。
还参考了这篇博客：[Variance - best perspective of understanding lifetime in Rust](https://dev.to/arichy/variance-best-perspective-of-understanding-lifetime-in-rust-m84) ，虽然感觉里面有挺多错误的，但是总体来说讲得很好，用 class 作为例子很清晰地说明了 variance 的各种内容。我估计之后会参考这篇博客再写一篇，用 class 的例子讲解 variance
这篇博客的作者是个前端程序员，然后从 JS/TS 转到 Rust 了，我看了一下他的其他文章，挺多都写得很不错，作为 Rust 的入门或进阶理论都很合适，比如这篇 [Pin in Rust: The Why and How of Immovable Memory](https://dev.to/arichy/pin-in-rust-the-why-and-how-of-immovable-memory-481b) 和这篇 [A Journey From JS To Rust](https://dev.to/arichy/a-journey-from-js-to-rust-3oa1) ，里面基本没有错误，讲解得也比较清楚。
