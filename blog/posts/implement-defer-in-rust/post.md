RAII 和 defer 是两种常见的资源管理方式，理论上来说，它们是完全可以互相替代的，但实际上它们更像是互补的，各有各的优势。当对象拥有资源时，比如一个 `File` 对象，这时候适合用 RAII，在离开作用域时自动关闭文件；而需要在作用域结束时执行特定的清理逻辑，这就更适合用 defer
比如在写包管理器的时候，使用数据库来管理安装的文件，数据库通过事务操作，可以确保数据具有原子性，要么提交，要么回滚，不存在中间状态。但是一般的文件系统不具备这样的功能，这时候可以用 defer 来实现，如果出错了，提前返回，通过 defer 自动回滚文件；如果成功了，就取消 defer，相当于是成功提交了事务。
下面这段代码就通过 defer 实现了这样的逻辑：

```rust
// runnable
use std::{
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
};

pub struct DropGuard<F: FnOnce(T), T> {
    f: ManuallyDrop<F>,
    inner: ManuallyDrop<T>,
}

impl<F: FnOnce(T), T> DropGuard<F, T> {
    pub fn new(inner: T, f: F) -> Self {
        Self {
            f: ManuallyDrop::new(f),
            inner: ManuallyDrop::new(inner),
        }
    }

    /// Consumes the `DropGuard` without invoking the drop function
    pub fn into_inner(self) -> T {
        let mut new_guard = ManuallyDrop::new(self);
        let value = unsafe { ManuallyDrop::take(&mut new_guard.inner) };
        unsafe { ManuallyDrop::drop(&mut new_guard.f) };
        value
    }
}

impl<F: FnOnce(T), T> Drop for DropGuard<F, T> {
    fn drop(&mut self) {
        let value = unsafe { ManuallyDrop::take(&mut self.inner) };
        let f = unsafe { ManuallyDrop::take(&mut self.f) };
        f(value);
    }
}

impl<F: FnOnce(T), T> Deref for DropGuard<F, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<F: FnOnce(T), T> DerefMut for DropGuard<F, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[macro_export]
macro_rules! defer {
    ($($move:ident)? |$($tokens:tt)*) => {
        $crate::defer_impl!(__guard $($move)? [] [] [] [,$($tokens)*]);
    };

    ($guard:ident, $($move:ident)? |$($tokens:tt)*) => {
        $crate::defer_impl!($guard $($move)? [] [] [] [,$($tokens)*] );
    };

    ($guard:ident, $body:expr) => {
        let $guard = DropGuard::new(
            (),
            |_| $body(),
        );
    };

    ($body:expr) => {
        let mut __defer_guard = DropGuard::new(
            (),
            |_| $body(),
        );
    };

}

#[macro_export]
macro_rules! defer_impl {
    ($guard:ident $($move:ident)? [$($caps:ident)*] [$($pats:tt)*]
     [$($comma:tt)?]
     [,mut $var:ident $($rest:tt)*]
    ) => {
        $crate::defer_impl!(
            $guard
            $($move)?
            [$($caps)* $var]
            [$($pats)* $($comma)? mut $var]
            [,]
            [$($rest)*]
        );
    };

    // ident, rest
    ($guard:ident $($move:ident)? [$($caps:ident)*] [$($pats:tt)*]
     [$($comma:tt)?]
     [, $bind:ident: &$var:ident $($rest:tt)*]
    ) => {
        let $bind = &mut $var;
        $crate::defer_impl!(
            $guard
            $($move)?
            [$($caps)* $bind]
            [$($pats)* $($comma)? $bind]
            [,]
            [$($rest)*]
        );
        let $bind = &mut **$bind;
    };

    ($guard:ident $($move:ident)? [$($caps:ident)*] [$($pats:tt)*]
     [$($comma:tt)?]
     [, &$var:ident $($rest:tt)*]
    ) => {
        let $var = &mut $var;
        $crate::defer_impl!(
            $guard
            $($move)?
            [$($caps)* $var]
            [$($pats)* $($comma)? $var]
            [,]
            [$($rest)*]
        );
        let $var = &mut **$var;
    };

    // ident, rest
    ($guard:ident $($move:ident)? [$($caps:ident)*] [$($pats:tt)*]
     [$($comma:tt)?]
     [, $var:ident $($rest:tt)*]
    ) => {
        $crate::defer_impl!(
            $guard
            $($move)?
            [$($caps)* $var]
            [$($pats)* $($comma)? $var]
            [,]
            [$($rest)*]
        );
    };

    // END
    ($guard:ident $($move:ident)? [$($caps:ident)*] [$($pats:tt)*] [,] [$(,)? | $body:block]) => {
        #[allow(unused_parens)]
        let mut $guard = DropGuard::new(
            ($($caps),*),
            $($move)? |($($pats)*)| $body,
        );

        #[allow(unused_parens)]
        let ($($caps),*) = &mut *$guard;
    };

}

// ANCHOR
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn copy_files() -> Result<(), io::Error> {
    let mut installed_files: Vec<PathBuf> = Vec::new();
    defer!(guard, |&installed_files| {
        for p in installed_files {
            let res = fs::remove_file(&p);
            println!("delete the file: {}, {res:?}", p.display());
        }
    });

    let files = fs::read_dir(".")?;
    for entry in files {
        let entry = entry?;
        let file = entry.path();
        let copy_to = Path::new("target").join(entry.file_name());
        installed_files.push(copy_to);
        println!(
            "copy file: {}, type: {:?}",
            file.display(),
            entry.file_type()?
        );
        fs::copy(file, installed_files.last().unwrap())?;
    }

    guard.into_inner();
    Ok(())
}

fn main() {
    let res = copy_files();
    println!("result: {res:?}");
}
// ANCHOR_END
```

这里先用 defer 定义了出错时需要执行哪些逻辑，再开始复制文件，出错时会通过问号提前返回，defer 在退出作用域时自动执行，整个流程非常自然，不妨点击右上角的运行按钮看看结果如何。
关于 Rust 中的 defer，社区中也有不少讨论[^1]，可见需求不小，但官方依旧不太想做这个功能，社区里比较有名的实现是 [scopeguard](https://crates.io/crates/scopeguard) ，但是它提供的 defer 宏并不好用，由于借用检查器的限制，完全无法实现上面那样的代码，所以我尝试自己实现了一个比较好用的 defer 宏，效果如上，用起来很灵活，不会因为借用检查器而受到很大的限制。下面介绍一下如何实现这个宏。

## 实现 DropGuard

### 尝试简单实现

defer 宏的作用是让代码写起来更简单，但要实现 defer，首先需要实现一个 DropGuard，以上面的例子来说，除去取消功能，它还至少需要两个功能：

1. 可以存储一个实现了 FnOnce 的闭包[^2]，并在 drop 时调用。
2. 可以单独存储数据，虽然闭包也能捕获数据，但这样只能在 drop 的时候访问，而我们上面的代码，是需要在定义 defer 后，往`installed_files` 里添加文件路径的，所以数据必须单独存储，便于外部访问。

那么最简单的实现大概长这样

```Rust
// runnable
// ANCHOR
struct DropGuard<T, F: FnOnce(T)> {
    inner: T,
    f: F,
}

impl<T, F: FnOnce(T)> Drop for DropGuard<T, F> {
    fn drop(&mut self) {
        (self.f)(self.inner);
    }
}
// ANCHOR_END

fn main() {}
```

但很可惜，这段代码没法编译，原因是 FnOnce 被调用时会被消耗，存储的值 T 也需要传给 FnOnce 使用，但 drop 方法的参数是 `&mut self`，这意味着只能更改 Self 里的值，但不能消耗掉。

#### 为什么这么设计？

乍一看很奇怪，结构体被 drop 之后就不能再使用了，为什么不能消耗所有权？
一方面，drop 的本质是在作用域结束时，会自动帮你调用 drop 方法，像这样：

```Rust
fn main() {
    {
        let s1 = "hello".to_string();
        let s2 = "hello".to_string();
        // 作用域结束，自动调用 drop 方法
        s2.drop();
        s1.drop();
    }
}
```

而对于方法，如果方法参数是 `self`，自然也需要调用 drop 方法。那对于 drop 方法本身来说就会发生无限的递归了。

```Rust
fn drop(self) {
    self.drop();
}
```

想解决这个问题，必须要对 drop 方法的实现做一些特殊的调整，但这样就会使语言的行为更加不一致了。

另一方面，对于 unsized type，它们可能也需要 drop 方法，如果方法参数是引用，引用的大小是可以在编译时确定的，但如果是 `self` ，就没法确定大小了，无法编译。

```Rust
// runnable
// ANCHOR
struct UnsizedType<T: ?Sized>(T);

impl<T: ?Sized> UnsizedType<T> {
    fn drop(self) {}
}
// ANCHOR_END

fn main() {}
```

### 进一步探索实现

既然 drop 方法的签名没法改，就只能想办法从可变引用中取出值了，最简单的想法是用 `Option`，可以取出里面的值，留下 `None` 在里面，但是它有两个问题：

1. 每次都要判断里面是 `Some` 还是 `None` ，虽然开销很小，但没法忽略
2. `Option` 的本质是 tagged union，需要额外占用 1 字节的空间存储当前的状态，而且由于结构体需要内存对齐，为了存储状态而多占用的空间可能不止 1 字节。

<!-- end list -->

```Rust
// runnable
struct DropGuard<T, F: FnOnce(T)> {
    inner: Option<T>,
    f: Option<F>,
}

impl<T, F: FnOnce(T)> Drop for DropGuard<T, F> {
    fn drop(&mut self) {
        unsafe {
            let f = self.f.take().unwrap();
            let inner = self.inner.take().unwrap();
            (f)(inner);
        }
    }
}

fn main() {
    println!("u64 size: {}", size_of::<u64>());
    println!("u64 with Option size: {}", size_of::<Option<u64>>());
}
```

其实我们根本不需要那个多余的 tag，直接用 union 不就行了，像这样

```Rust
// runnable
// ANCHOR
use std::ptr;

union Opt<T> {
    val: T,
    _nothing: (),
}

struct DropGuard<T, F: FnOnce(T)> {
    inner: Opt<T>,
    f: Opt<F>,
}

impl<T, F: FnOnce(T)> Drop for DropGuard<T, F> {
    fn drop(&mut self) {
        unsafe {
            let f = ptr::read(&self.f.val);
            let inner = ptr::read(&self.inner.val);
            (f)(inner);
        }
    }
}
// ANCHOR_END

fn main() {}
```

Oops，被编译器阻止了，为什么之前用 `Option` 没问题，但现在用 `union` 就不行了呢？
主要问题是，对于 `Option`，它被 drop 的时候，因为有 tag 记录了当前状态，可以判断当前是否存储了值，如果有值就 drop 掉，如果没有值就什么都不做。
但是对于 `union`，无法得知它的状态，保守的做法应该是什么也不干，但是这样就内存泄漏了，所以这里编译器要求 `union` 里存储的类型要么是 `Copy` 的，无需 drop，要么是 `ManuallyDrop<T>`，也不需要 drop
顾名思义，被 `ManuallyDrop` 包裹的值，将不会被 drop，需要手动进行，这似乎也正好是我们需要的。

我们刚刚还没有尝试，如果直接读出值会怎么样，请运行下面的例子看看：

```Rust
// runnable
use std::ptr;

struct DropGuard<T, F: FnOnce(T)> {
    inner: T,
    f: F,
}

impl<T, F: FnOnce(T)> Drop for DropGuard<T, F> {
    fn drop(&mut self) {
        unsafe {
            let f = ptr::read(&self.f);
            let inner = ptr::read(&self.inner);
            (f)(inner);
        }
    }
}

fn main() {
    let a = Box::new(3);
    let _guard = DropGuard {
        inner: a,
        f: |_| {
            // do nothing
        },
    };
}
```

我们成功触发了 double free，这是因为 `drop` 方法只用来处理当前结构体的析构过程，而结构体内部的各个字段，则由它们自己的 `drop` 方法来处理。
在上面的代码中，当 `DropGuard` 离开作用域时，先运行了它自己的 `drop` 方法，读出了 `a`，并传入闭包，闭包结束时，`a` 自然被 drop，`Box`[^3] 的 `drop` 方法里运行了一次 `free`，之后则开始析构各个字段，析构 `inner` 字段的时候，又 drop 了一次 `a`，`Box` 的 `drop` 方法里又运行了一次 `free`，于是就触发 double free 了。

显而易见，`ManuallyDrop` 在这里非常有用，它可以阻止第二次 drop，从而让这段代码正常运行。

### 最终实现

```rust
// runnable
use std::mem::ManuallyDrop;

pub struct DropGuard<F: FnOnce(T), T> {
    f: ManuallyDrop<F>,
    inner: ManuallyDrop<T>,
}

impl<F: FnOnce(T), T> DropGuard<F, T> {
    pub fn new(inner: T, f: F) -> Self {
        Self {
            f: ManuallyDrop::new(f),
            inner: ManuallyDrop::new(inner),
        }
    }
}

impl<F: FnOnce(T), T> Drop for DropGuard<F, T> {
    fn drop(&mut self) {
        let value = unsafe { ManuallyDrop::take(&mut self.inner) };
        let f = unsafe { ManuallyDrop::take(&mut self.f) };
        f(value);
    }
}

fn main() {
    {
        let a = Box::new(3);
        let _guard = DropGuard::new(a, |_| {
            println!("drop `a`");
        });
    }
    println!("dropped `a` safely");
}
```

这个实现就比较完善了，也没有内存错误，但作为 `DropGuard`，还需要实现两个功能

#### Deref Trait

当把值交给 `DropGuard` 后，可能还需要修改值，每次都调用 `guard.inner` 非常麻烦，可以通过实现 Deref trait 来改善这一点。
顾名思义，`Deref` trait 就是定义当解引用这个结构体的引用时，应该返回什么，因为 Rust 区分可变性，所以 `Deref` 和 `DerefMut` 分别对应着不可变引用和可变引用两种情况。

```rust
// runnable
use std::mem::ManuallyDrop;

pub struct DropGuard<F: FnOnce(T), T> {
    f: ManuallyDrop<F>,
    inner: ManuallyDrop<T>,
}

impl<F: FnOnce(T), T> DropGuard<F, T> {
    pub fn new(inner: T, f: F) -> Self {
        Self {
            f: ManuallyDrop::new(f),
            inner: ManuallyDrop::new(inner),
        }
    }
}

impl<F: FnOnce(T), T> Drop for DropGuard<F, T> {
    fn drop(&mut self) {
        let value = unsafe { ManuallyDrop::take(&mut self.inner) };
        let f = unsafe { ManuallyDrop::take(&mut self.f) };
        f(value);
    }
}

// ANCHOR
use std::ops::{Deref, DerefMut};

impl<F: FnOnce(T), T> Deref for DropGuard<F, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<F: FnOnce(T), T> DerefMut for DropGuard<F, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

fn main() {
    let a = vec![1, 2];
    let mut guarded_a = DropGuard::new(a, |a| {
        println!("drop `a`");
        println!("{:?}", a);
    });
    println!("modify a");
    // 这里调用 `push` 方法时，因为 `DropGuard` 没有这个方法，
    // 编译器会加上足够多的 `*` 号进行解引用，直到找到对应的方法为止。
    guarded_a.push(3);
    // 显式地写出来就是这样
    let v: &mut Vec<i32> = &mut *guarded_a;
    v.push(4);
}
// ANCHOR_END
```

#### 取消 guard

文章开头，我举了一个包管理器的例子，安装文件时，如果出错了，就通过 `DropGuard` 自动回滚，如果成功了，则需要取消它，如何实现呢？
显然，我们没法直接取消 `drop` 方法，但我们可以把 `DropGuard` 本身也塞进 `ManuallyDrop` 里，这样就可以避免它的 `drop` 方法被执行了。

```rust
// runnable
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

impl<F: FnOnce(T), T> Drop for DropGuard<F, T> {
    fn drop(&mut self) {
        let value = unsafe { ManuallyDrop::take(&mut self.inner) };
        let f = unsafe { ManuallyDrop::take(&mut self.f) };
        f(value);
    }
}

impl<F: FnOnce(T), T> Deref for DropGuard<F, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<F: FnOnce(T), T> DerefMut for DropGuard<F, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

// ANCHOR
pub struct DropGuard<F: FnOnce(T), T> {
    f: ManuallyDrop<F>,
    inner: ManuallyDrop<T>,
}

impl<F: FnOnce(T), T> DropGuard<F, T> {
    // ANCHOR_END
    pub fn new(inner: T, f: F) -> Self {
        Self {
            f: ManuallyDrop::new(f),
            inner: ManuallyDrop::new(inner),
        }
    }
    // ANCHOR
    pub fn into_inner(self) -> T {
        // 塞进 ManuallyDrop
        let mut new_guard = ManuallyDrop::new(self);
        // 取出存储的值
        let value = unsafe { ManuallyDrop::take(&mut new_guard.inner) };
        // 不需要运行闭包了，直接 drop 掉
        unsafe { ManuallyDrop::drop(&mut new_guard.f) };
        // 返回值
        value
    }
}

fn main() {
    let a = vec![1, 2];
    let mut guarded_a = DropGuard::new(a, |a| {
        println!("drop `a`");
        println!("{:?}", a);
    });
    println!("modify a");
    guarded_a.push(3);
    // 取消 guard
    let a = guarded_a.into_inner();
}
// ANCHOR_END
```

### 灵活使用

这个 `DropGuard` 看起来简单，但是因为使用了泛型，用起来十分灵活。
比如同时处理多个值，可以把值打包成元组塞进去，重新借用后再解构元组
也可以只传引用进去，不把整个值 move 进去，这样之后还能继续用。

```Rust
// runnable
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

impl<F: FnOnce(T), T> Drop for DropGuard<F, T> {
    fn drop(&mut self) {
        let value = unsafe { ManuallyDrop::take(&mut self.inner) };
        let f = unsafe { ManuallyDrop::take(&mut self.f) };
        f(value);
    }
}

impl<F: FnOnce(T), T> Deref for DropGuard<F, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<F: FnOnce(T), T> DerefMut for DropGuard<F, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub struct DropGuard<F: FnOnce(T), T> {
    f: ManuallyDrop<F>,
    inner: ManuallyDrop<T>,
}

impl<F: FnOnce(T), T> DropGuard<F, T> {
    pub fn new(inner: T, f: F) -> Self {
        Self {
            f: ManuallyDrop::new(f),
            inner: ManuallyDrop::new(inner),
        }
    }

    pub fn into_inner(self) -> T {
        // 塞进 ManuallyDrop
        let mut new_guard = ManuallyDrop::new(self);
        // 取出存储的值
        let value = unsafe { ManuallyDrop::take(&mut new_guard.inner) };
        // 不需要运行闭包了，直接 drop 掉
        unsafe { ManuallyDrop::drop(&mut new_guard.f) };
        // 返回值
        value
    }
}

// ANCHOR
fn handle_multiple_values() {
    let a = vec![1, 2];
    let b = "hello".to_string();
    // 把 a, b 打包成元组放进去，创建 guard 之后，使用模式匹配解构元组
    let (a, b) = &mut *DropGuard::new((a, b), |(a, b)| {
        println!("drop `a`");
        println!("{:?}", a);
        println!("drop b: {b}");
    });
    println!("modify a");
    a.push(3);
    println!("print b: {b}");
}

fn return_value() -> Vec<i32> {
    let mut a = vec![1, 2];
    let b = "hello".to_string();
    // 为 a 创建可变引用，之后只把引用放进 guard，外面的 a 不受影响
    let a_ref = &mut a;
    let mut guard = DropGuard::new((a_ref, b), |(a_ref, b)| {
        a_ref.push(4);
        println!("drop b: {b}");
    });
    let (a_ref, b) = &mut *guard;
    println!("modify a");
    a_ref.push(3);
    println!("print b: {b}");
    // guard 持有对 a 的引用，必须先 drop 掉 guard，才能返回 a
    drop(guard);
    a
}

fn main() {
    println!("----------------- handle multiple values -----------------");
    handle_multiple_values();
    println!("----------------- return value -----------------");
    let a = return_value();
    println!("returned a: {a:?}");
}
// ANCHOR_END
```

其实就功能性来说，目前这个 `DropGuard` 已经完全可以实现开头提到的包管理器的例子了，但显然的问题是，写起来太繁琐了，要处理 a, b 两个变量，需要写整整 3 次 `(a, b)`，如果变量名再长一点，写起来更麻烦；如果希望只放引用进去，还要再多写一行 `let a_ref = &mut a;`，实在是繁琐得不能再繁琐了。对于这种重复的工作，在 Rust 中可以用宏来解决。

## 宏与 `macro_rules!`

Rust 中有两种宏：声明式宏和过程宏，就像 C 里的 `define` 一样，可以在编译之前，先处理一遍代码。
其中过程宏最强大，它几乎可以处理任意输入，感觉上像是编译器的插件，你可以自由地定义一套语法，只需要在过程宏里写 Rust 代码解析，并输出为合法的语法树就行，比如你可以在里面写 html，然后在过程宏里解析并生成合法的 Rust 语法树。而本质上它是个单独的程序，运行 `cargo build` 时，会先编译它，之后 `rustc` 会在需要的地方运行它来处理代码，再进入正常的编译流程。显然，这意味着过程宏会大幅拖慢编译速度，不仅需要多编译一个程序，语法树需要在编译器和过程宏的动态库之间传递，而且这一操作是阻塞的，后续的类型检查、借用检查等都要等过程宏的代码生成结束才能运行。

而声明式宏则是内置在 `rustc` 里的一套语法规则，和普通代码的不同之处是，它处理的是输入的 tokens，并输出为合法的代码。它的规则相对有限，且只能通过模式匹配和递归来处理，无法像过程宏那样拥有判断和循环功能。但它最大的优点就是快，因为内置在编译器里，对编译速度的影响很小。

### 简单的 `macro_rules`

声明式宏本质上就是按照分支顺序挨个进行模式匹配，比如一个简单的宏：

```Rust
// runnable
macro_rules! string {
    ($s:literal) => {{ String::from($s) }};
}

fn main() {
    let s = string!("hello");
    println!("type of s: {}", std::any::type_name_of_val(&s));
}
```

这里使用美元符号来声明参数，紧跟着的是参数名，冒号后面是参数类型
本文用到的参数类型[^4]有这几种：

- `literal`: 表示字面量
- `block`: 表示代码块，也就是用大括号包裹的一系列语句，值得注意的是，`block` 本身就是一个表达式
- `expr`: 表示表达式，它也包括 `block`
- `ident`: 表示 identifier，基本上就是匹配一个单词，但是要遵循变量的命名规则，不能以数字开头，中间的符号也有限制，不过比变量的限制更少，这里可以匹配保留的关键字
- `ty`: 表示类型，可以匹配 `String`, `&mut Vec<i32>` 之类的各种类型
- `tt`: 最通用的匹配，表示一个 token，而 Rust 代码就是由一堆 tokens 组成的

`=>` 符号后面的内容会直接在原地展开，如果多加一个大括号，则展开的内容都被大括号包裹起来了，整个分支的内容作为表达式返回；接单大括号，则直接展开这个分支的所有内容。

宏也可以直接匹配字面 token，比如这样：

```Rust
// runnable
macro_rules! string {
    // 这个分支返回表达式
    (@expression $s:literal) => {{ String::from($s) }};
    // 这个分支是语句
    (@statement $var:ident, $s:literal) => {
        // 就像 shell 语法一样，使用美元符号来引用匹配到的 token
        let $var = String::from($s);
    };
}

fn main() {
    let s1 = string!(@expression "hello");
    println!("type of s1: {}", std::any::type_name_of_val(&s1));
    string!(@statement s2, "hello");
    println!("type of s2: {}", std::any::type_name_of_val(&s2));
}
```

原地展开的好处是，可以传入变量名，并在宏里面定义变量，以及宏里面定义的变量和外部是同一作用域
比如这个例子，可以给外部的变量绑定宏内部的变量引用。

```Rust
// runnable
macro_rules! string {
    ($var:ident, $s:literal) => {
        let temp = String::from($s);
        let $var = &temp;
    };
}

fn main() {
    string!(s, "hello");
    println!("type of s: {}", std::any::type_name_of_val(&s));
}
```

使用表达式就做不到这点，因为宏内部是单独的块作用域，返回的时候内部的变量就被销毁了。

```Rust
// runnable
macro_rules! string {
    ($s:literal) => {{
        let temp = String::from($s);
        &temp
    }};
}

fn main() {
    let s = string!("hello");
    println!("type of s: {}", std::any::type_name_of_val(&s));
}
```

Rust 的宏具有卫生性[^5] (hygiene)，所以即使是原地展开，和外部共享作用域，也不会污染命名空间。

```Rust
// runnable
macro_rules! string {
    ($var:ident, $s:literal) => {
        let temp = String::from($s);
        let $var = &temp;
    };
}

fn main() {
    string!(s, "hello");
    println!("type of s: {}", std::any::type_name_of_val(&s));
    println!("{}", temp);
}
```

### 重复匹配

可以重复匹配 token，比如我想要打印东西，可以写这样的宏

```Rust
// runnable
macro_rules! myprint {
    ($s:literal, $($args:expr),*) => {
        print!($s, $($args),*);
    };
}

fn main() {
    let s1 = "hello";
    let s2 = "world".to_string();
    myprint!("{} {}\n", s1, s2);
    myprint!("\n",);
}
```

`$($args:expr),*` 可以重复匹配一堆表达式，它们以逗号分隔

- 小括号里是要匹配的模式，可以是任何合法的其他模式，比如字面量 tokens，使用美元符号捕获参数，甚至是另一个重复匹配。在这里是匹配一个表达式
- 小括号后面紧跟的是分隔符，可以省略，表示没有分隔符，也就是默认的按空格分隔
- 最后面的 `*` 表示匹配 0 次或多次，换成 `+` 则表示匹配 1 次或多次，换成 `?` 则表示匹配 0 次或 1 次

匹配之后使用相同的语法来展开，但不需要写类型
可以写成 `$($args),*`

- 小括号里是要重复输出的内容，此处可以输出字面量，也可以使用小括号内部匹配到的变量，也可以使用外部匹配的变量，但不能使用其他小括号匹配到的变量
- 小括号后面紧跟的是分隔符，可以省略，表示没有分隔符，也就是默认的按空格分隔，这里的分隔符不必跟匹配时用的一样
- 最后面的 `*` 表示重复次数，可以跟之前匹配时使用的不相同，但仍然遵循相同的规则，也就是说，如果前面用 `*` 匹配到了 1 次，后面用 `+` 来重复是没问题的；但如果前面用 `*` 匹配到了 0 次，那就会在后面用 `+` 的时候报错。

我们刚刚的 `myprint!` 宏有点小问题，就是在没有参数时，没法省略逗号，有多个参数时，尾部也不能多一个逗号。

```Rust
// runnable
macro_rules! myprint {
    ($s:literal , $($args:expr),*) => {
        print!($s, $($args),*);
    };
}

fn main() {
    let s1 = "hello";
    let s2 = "world".to_string();
    myprint!("{} {}\n", s1, s2,);
    myprint!("hi\n");
}
```

使用问号来改进一下

```Rust
// runnable
macro_rules! myprint {
    ($s:literal $(,$args:expr)* $(,)?) => {
        print!($s, $($args),*);
    };
}

fn main() {
    let s1 = "hello";
    let s2 = "world".to_string();
    myprint!("{} {}\n", s1, s2);
    myprint!("{} {}\n", s1, s2,);
    myprint!("hi\n");
    myprint!("hi\n",);
}
```

我们刚刚提到了 `tt`，重复匹配 `tt`，则相当于是匹配任意的 Rust token tree，这是个非常强力的匹配模式
所以可以把所有传进来的 tokens 直接传给其他的宏

```Rust
// runnable
macro_rules! myprint {
    ($($token:tt)*) => {
        print!($($token)*);
    };
}

fn main() {
    let s1 = "hello";
    let s2 = "world".to_string();
    myprint!("{} {}\n", s1, s2);
    myprint!("{} {}\n", s1, s2,);
    myprint!("hi\n");
    myprint!("hi\n",);
}
```

利用展开时的语法，还可以做些有趣的事，比如重复匹配了一堆参数，不知道有多少个，可以将每个参数展开为 `+1`，然后在最前面补个 0，就组合成了一个表达式

```Rust
// runnable
// 对于多个分支的宏，需要注意顺序，将条件更精确的写在前面
macro_rules! count {
    (@consume $token:tt) => {
        1
    };
    ($($token:tt)*) => {
        0 $(+ count!(@consume $token))*
    };
}

macro_rules! myprint {
    ($($token:tt)*) => {
        let count = count!($($token)*);
        println!("tokens count: {count}");
        print!($($token)*);
    };
}

fn main() {
    let s1 = "hello";
    let s2 = "world".to_string();
    myprint!("{} {}\n", s1, s2);
}
```

使用 `cargo expand`[^6] 可以看到宏被展开成了这样：

```Rust
fn main() {
    let s1 = "hello";
    let s2 = "world".to_string();
    let count = 0 + 1 + 1 + 1 + 1 + 1;
    {
        ::std::io::_print(format_args!("tokens count: {0}\n", count));
    };
    {
        ::std::io::_print(format_args!("{0} {1}\n", s1, s2));
    };
}
```

当然也有其他写法，比如把每个 token 展开为 `()`，即 unit type，然后放到数组里，并统计这个数组的长度。
unit type 是 ZST (zero-sized type) 类型，不占用任何空间，所以这里无论有多少参数都不会导致栈溢出，这一写法和刚刚的加法表达式在编译后的机器码应该是等价的。

```Rust
// runnable
macro_rules! count {
    (@consume $token:tt) => {
        ()
    };
    ($($token:tt)*) => {
        [$(count!(@consume $token)),*].len()
    };
}

macro_rules! myprint {
    ($($token:tt)*) => {
        let count = count!($($token)*);
        println!("tokens count: {count}");
        print!($($token)*);
    };
}

fn main() {
    let s1 = "hello";
    let s2 = "world".to_string();
    myprint!("{} {}\n", s1, s2);
}
```

展开后是这样：

```Rust
fn main() {
    let s1 = "hello";
    let s2 = "world".to_string();
    let count = [(), (), (), (), ()].len();
    {
        ::std::io::_print(format_args!("tokens count: {0}\n", count));
    };
    {
        ::std::io::_print(format_args!("{0} {1}\n", s1, s2));
    };
}
```

### Debug

我们的宏越来越复杂了，很多时候没法一次写对，好在编译器提供了一些实用的调试工具

- `trace_macros!`
  在启用后，会输出宏每次的展开过程
  需要启用 `trace_macros` feature，并使用 nightly 版本的 Rust 编译
  这里的在线运行功能好像在编译成功时就不会输出宏的展开过程了，所以我仍然选择使用 stable 版本进行编译

<!-- end list -->

```Rust
// runnable
#![feature(trace_macros)]

macro_rules! count {
    (@consume $token:tt) => {
        ()
    };
    ($($token:tt)*) => {
        [$(count!(@consume $token)),*].len()
    };
}

macro_rules! myprint {
    ($($token:tt)*) => {
        let count = count!($($token)*);
        println!("tokens count: {count}");
        print!($($token)*);
    };
}

fn main() {
    let s1 = "hello";
    let s2 = "world".to_string();
    trace_macros!(true);
    myprint!("{} {}\n", s1, s2);
    trace_macros!(false);
}
```

- `log_syntax!($tt)`
  需要启用 `log_syntax` feature，并使用 nightly 版本的 Rust 编译
  它会把输入的 tokens 全部打印出来，比如把 hello world 程序传进去，它会输出所有 tokens

<!-- end list -->

```Rust
// runnable nightly
#![feature(log_syntax)]

fn main() {
    log_syntax!(
        fn main() {
            println!("hello world");
        }
    );
}
```

### 递归

结合递归与重复匹配，可以写出来的东西就更有趣了，比如著名的 tt muncher（Token 吞噬者），即递归下降解析 token tree

```Rust
// runnable nightly
#![feature(log_syntax)]

macro_rules! tt_muncher{
    // 没有 token 的时候终止
    () => {};
    // 每次匹配一个 token，剩下的继续传递
    ($token:tt $($rest:tt)*) => {
        log_syntax!($token);
        tt_muncher!($($rest)*);
    }
}

fn main() {
    tt_muncher!(
        fn main() {
            println!("hello world");
        }
    );
}
```

tt 会将大/中/小括号作为一个整体，所以上面的代码会把 `main` 函数的函数体作为整个 token 打印出来
我们可以让它匹配得更细一些

```Rust
// runnable
macro_rules! tt_muncher{
    // 终止分支
    () => {};
    // 匹配大括号里面的内容
    ({$($inner:tt)*} $($rest:tt)*) => {
        println!("{{");
        tt_muncher!($($inner)*);
        println!("}}");
        tt_muncher!($($rest)*);
    };
    // 匹配小括号里面的内容
    (($($inner:tt)*) $($rest:tt)*) => {
        println!("(");
        tt_muncher!($($inner)*);
        println!(")");
        tt_muncher!($($rest)*);
    };
    // 匹配单个 token
    ($token:tt $($rest:tt)*) => {
        // stringify 可以将输入的 token 转换为 &'static str
        println!("{}", stringify!($token));
        tt_muncher!($($rest)*);
    };
}

fn main() {
    tt_muncher!(
        fn main() {
            println!("hello world");
        }
    );
}
```

tt muncher 每次吞噬一个 token，并传递剩下的 token，到最后一个分支的时候就没有任何 token 了
如果希望能保留一些 token 呢，注意我们处理的是 tokens，并且是在编译时最早期的阶段处理，所以没法用 Vec 之类的存起来，而 `macro_rules` 也不支持把 tokens 作为返回值传回去，所以如果希望保留一些中间值，则需要作为参数继续传递下去，每次调用都带上之前的参数，并加上新的内容，我们称之为 Push-down Accumulator （下推累加器）
我们之前提到过，只有大/中/小括号在解析时，会连带内部的内容视为一整个 token，这其实意味着，只有它们可以终止 $($token:tt)\* 的匹配，而不会出现歧义。所以在使用累加器的时候，如果搭配 tt 来存储内容，则一般都会用括号把中间的内容包起来

举个解析函数签名的例子：
乍一看像是一堆乱码，其实只是在 tt muncher 的基础上加了累加器

```Rust
// runnable
macro_rules! parse_fn{
    // 入口分支，提取出函数名及小括号内部的参数列表
    (fn $name:ident ($($inner:tt)*);) => {
        // 把函数名继续传下去，`[]` 是存放解析结果的，目前为空，最后把参数列表整个传进去
        // 注意这里在最后加了一个逗号，这是为了方便处理逗号匹配，保证最后一个参数后面也有逗号
        // 这样每次都匹配参数和逗号就行了
        // 如果参数列表尾部已经有逗号了，那多余的逗号会在结果分支被处理
        parse_fn!($name [] $($inner)*,);
    };
    // 传递函数名和解析结果的列表，这里不关心列表内部的内容，直接用 tt 匹配
    // 在列表后面，匹配形如“参数名: 参数类型,” 这样的参数，注意这里会匹配一个逗号
    // 最后传递剩下的 tokens
    ($name:ident [$($tokens:tt)*] $var:ident : $type:ty, $($rest:tt)*) => {
        // 原样展开匹配到的结果列表，并把参数存储为 (, 名字, 类型) 的形式
        parse_fn!($name [$($tokens)* (, $var, $type)] $($rest)*);
    };
    // 这里负责匹配参数名前面带 mut 的情况
    ($name:ident [$($tokens:tt)*] mut $var:ident : $type:ty, $($rest:tt)*) => {
        // 把参数存储为 (mut, 名字, 类型) 的形式
        parse_fn!($name [$($tokens)* (mut, $var, $type)] $($rest)*);
    };
    // 结果分支
    // 接收函数名和结果列表
    // 我们之前在列表里存的要么是(, name, type)，要么是(mut, name, type)
    // 所以这里可选匹配第一个逗号前面的内容
    // 最后可选匹配一个逗号，处理参数列表尾部有逗号的情况
    ($name:ident [$(($($mut:ident)?, $v:ident, $t:ty))*] $(,)?) => {
        println!("function name: {}", stringify!($name));
        println!("parameters:");
        $(
            print!("\tname: {}, type: {}", stringify!($v), stringify!($t));
            let mutable = stringify!($($mut)?);
            if !mutable.is_empty(){
                println!(", is_mutable: {}", mutable);
            }else{
                println!();
            }
        )*
    };
}

fn main() {
    parse_fn!(
        fn run(title: String, mut tags: Vec<String>);
    );
}
```

使用 `trace_macros!` 可以看到这样的展开过程：

```Rust
// DONT_FORMAT
= note: expanding `parse_fn! { fn run(title: String, mut tags: Vec<String>); }`
= note: to `parse_fn! (run [] title: String, mut tags: Vec<String>,);`
= note: expanding `parse_fn! { run [] title: String, mut tags: Vec<String>, }`
= note: to `parse_fn! (run [(, title, String)] mut tags: Vec<String>,);`
= note: expanding `parse_fn! { run [(, title, String)] mut tags: Vec<String>, }`
= note: to `parse_fn! (run [(, title, String)(mut, tags, Vec<String>)]);`
```

通过递归下降和累加器来计算，这让人想起了 Haskell 和函数式编程，值得一提的是，`macro_rules!` 是图灵完备的，玩法很多，比如已经有人用它写出了 [brainfxxk 的编译器](https://gist.github.com/judofyr/7ed4b52af2107119d4cc1f989ca63201)。

当然也可以用它来计算斐波那契数列，不过算不了多少项，n 大了之后会很慢
毕竟 `macro_rules` 是用来处理 tokens 的，数字也只能表示成 tokens 的形式，编译期计算还是应该用 `const fn`

```Rust
// runnable
macro_rules! fib {
    () => {
        0
    };
    (a) => {
        1
    };
    (a a $($n:ident)*) => {
        fib!(a $($n)*) + fib!($($n)*)
    };
}

fn main() {
    println!("fib 18: {}", fib!(a a a a a a a a a a a a a a a a a a));
}
```

## 实现 `defer!` 宏

刚刚介绍了一大堆 `macro_rules!` 的内容，让我们回忆一下 `defer!` 宏要简化的东西：

```Rust
// DONT_FORMAT
let (a, b) = &mut *DropGuard::new((a, b), |(a, b)| {
    println!("drop `a`");
    println!("{:?}", a);
    println!("drop b: {b}");
});
```

显然，我们希望通过宏实现少写几次 `(a, b)`

### 简单实现

```Rust
// runnable
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

impl<F: FnOnce(T), T> Drop for DropGuard<F, T> {
    fn drop(&mut self) {
        let value = unsafe { ManuallyDrop::take(&mut self.inner) };
        let f = unsafe { ManuallyDrop::take(&mut self.f) };
        f(value);
    }
}

impl<F: FnOnce(T), T> Deref for DropGuard<F, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<F: FnOnce(T), T> DerefMut for DropGuard<F, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub struct DropGuard<F: FnOnce(T), T> {
    f: ManuallyDrop<F>,
    inner: ManuallyDrop<T>,
}

impl<F: FnOnce(T), T> DropGuard<F, T> {
    pub fn new(inner: T, f: F) -> Self {
        Self {
            f: ManuallyDrop::new(f),
            inner: ManuallyDrop::new(inner),
        }
    }
    pub fn into_inner(self) -> T {
        let mut new_guard = ManuallyDrop::new(self);
        let value = unsafe { ManuallyDrop::take(&mut new_guard.inner) };
        unsafe { ManuallyDrop::drop(&mut new_guard.f) };
        value
    }
}

// ANCHOR
macro_rules! defer{
    ([$($vars:tt)*] $body:block) => {
        #[allow(unused_parens)]
        let mut __guard = $crate::DropGuard::new(
            ($($vars)*),
            |($($vars)*)| $body,
        );

        #[allow(unused_parens)]
        let ($($vars)*) = &mut *__guard;
    };
}

fn main() {
    let a = vec![1, 2];
    let b = "hello".to_string();
    defer!([a, b] {
        println!("drop `a`");
        println!("{:?}", a);
        println!("drop b: {b}");
    });
    println!("modify a");
    a.push(3);
    println!("print b: {b}");
}
// ANCHOR_END
```

看起来不错，但是有个问题，我们设计成了类似 C++ 的 lambda 表达式的语法，而 rustfmt 根本不认，它会直接放弃格式化宏内部的所有内容。
想让 rustfmt 能够格式化，我们必须得写成合法的 Rust 语法，这里最适合的是伪装成闭包

### 伪装成闭包的宏

```Rust
// runnable
macro_rules! defer{
    (|$($vars:tt)*| $body:block) => {
        #[allow(unused_parens)]
        let mut __guard = $crate::DropGuard::new(
            ($($vars)*),
            |($($vars)*)| $body,
        );

        #[allow(unused_parens)]
        let ($($vars)*) = &mut *__guard;
    };
}

fn main() {
    let a = vec![1, 2];
    let b = "hello".to_string();
    defer!(|a, b| {
        println!("drop `a`");
        println!("{:?}", a);
        println!("drop b: {b}");
    });
    println!("modify a");
    a.push(3);
    println!("print b: {b}");
}
// ANCHOR_END
```

竟然没法编译！原因是这里产生了歧义，之前我们说过，`$($vars:tt)*` 是一个非常强力的匹配模式，它会吞噬掉后面的一切 token，直到遇到括号，这也是为什么之前可以编译，因为之前用的是中括号。现在的竖线则会被吞噬进去，所以编译器无法决定这个竖线应该算成重复匹配 tt，还是匹配字面上的竖线。

这就使我们的宏稍微复杂了一些，需要先匹配左竖线，再匹配右竖线。
不妨分成 `defer` 和 `defer_impl` 两个宏

```Rust
// runnable
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

impl<F: FnOnce(T), T> Drop for DropGuard<F, T> {
    fn drop(&mut self) {
        let value = unsafe { ManuallyDrop::take(&mut self.inner) };
        let f = unsafe { ManuallyDrop::take(&mut self.f) };
        f(value);
    }
}

impl<F: FnOnce(T), T> Deref for DropGuard<F, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<F: FnOnce(T), T> DerefMut for DropGuard<F, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub struct DropGuard<F: FnOnce(T), T> {
    f: ManuallyDrop<F>,
    inner: ManuallyDrop<T>,
}

impl<F: FnOnce(T), T> DropGuard<F, T> {
    pub fn new(inner: T, f: F) -> Self {
        Self {
            f: ManuallyDrop::new(f),
            inner: ManuallyDrop::new(inner),
        }
    }
    pub fn into_inner(self) -> T {
        let mut new_guard = ManuallyDrop::new(self);
        let value = unsafe { ManuallyDrop::take(&mut new_guard.inner) };
        unsafe { ManuallyDrop::drop(&mut new_guard.f) };
        value
    }
}

// ANCHOR
macro_rules! defer_impl{
    ([$($vars:tt)*] $(,)? | $body:block) => {
        #[allow(unused_parens)]
        let mut __guard = $crate::DropGuard::new(
            ($($vars)*),
            |($($vars)*)| $body,
        );

        #[allow(unused_parens)]
        let ($($vars)*) = &mut *__guard;
    };
    ([$($vars:tt)*] ,$v:tt $($rest:tt)*) => {
        defer_impl!([$($vars)* $v,] $($rest)*);
    };
}

macro_rules! defer{
    (|$($vars:tt)*) => {
        defer_impl!([] ,$($vars)*);
    };
}

fn main() {
    let a = vec![1, 2];
    let b = "hello".to_string();
    defer!(|a, b| {
        println!("drop `a`");
        println!("{:?}", a);
        println!("drop b: {b}");
    });
    println!("modify a");
    a.push(3);
    println!("print b: {b}");
}
// ANCHOR_END
```

值得一提的是这里匹配逗号的方式，每次匹配形如 `, tt` 这样的内容，这样只需要一开始在最前面加一个逗号，匹配完所有输入后，要么剩下的是一个逗号，要么什么都不剩，结果分支那里不需要额外调整。

如果每次匹配 `tt ,` 这样的内容，为了处理末尾没有逗号的情况，要么在输入内容的最后面加一个逗号，但这里做不到，一开始输入进来的内容包括了右竖线和 block；要么在结果分支处理最后一次匹配，会更麻烦。

### 支持取消功能

需要给 `defer!` 宏增加一个分支，并给 `defer_impl!` 宏的每个分支加上 `guard` ，以便传递到结果分支

```Rust
// runnable
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

impl<F: FnOnce(T), T> Drop for DropGuard<F, T> {
    fn drop(&mut self) {
        let value = unsafe { ManuallyDrop::take(&mut self.inner) };
        let f = unsafe { ManuallyDrop::take(&mut self.f) };
        f(value);
    }
}

impl<F: FnOnce(T), T> Deref for DropGuard<F, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<F: FnOnce(T), T> DerefMut for DropGuard<F, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub struct DropGuard<F: FnOnce(T), T> {
    f: ManuallyDrop<F>,
    inner: ManuallyDrop<T>,
}

impl<F: FnOnce(T), T> DropGuard<F, T> {
    pub fn new(inner: T, f: F) -> Self {
        Self {
            f: ManuallyDrop::new(f),
            inner: ManuallyDrop::new(inner),
        }
    }
    pub fn into_inner(self) -> T {
        let mut new_guard = ManuallyDrop::new(self);
        let value = unsafe { ManuallyDrop::take(&mut new_guard.inner) };
        unsafe { ManuallyDrop::drop(&mut new_guard.f) };
        value
    }
}

// ANCHOR
macro_rules! defer_impl{
    ($guard:ident [$($vars:tt)*] $(,)? | $body:block) => {
        #[allow(unused_parens)]
        let mut $guard = $crate::DropGuard::new(
            ($($vars)*),
            |($($vars)*)| $body,
        );

        #[allow(unused_parens)]
        let ($($vars)*) = &mut *$guard;
    };
    ($guard:ident [$($vars:tt)*] ,$v:tt $($rest:tt)*) => {
        defer_impl!($guard [$($vars)* $v,] $($rest)*);
    };
}

macro_rules! defer{
    (|$($vars:tt)*) => {
        defer_impl!(__guard [] ,$($vars)*);
    };
    ($guard:ident, |$($vars:tt)*) => {
        defer_impl!($guard [] ,$($vars)*);
    };
}

fn main() {
    let a = vec![1, 2];
    defer!(guard, |a| {
        println!("drop `a`");
        println!("{:?}", a);
    });
    println!("modify a");
    a.push(3);
    guard.into_inner();
}
// ANCHOR_END
```

### 支持修饰符

对于上面的例子，无法实现在 `defer!` 的闭包里修改 `a`，虽然大部分情况用不上，而且通过重新绑定也能实现，但是既然都伪装成闭包的形状了，干脆支持 `mut` 关键字吧。
这里宏又变得更复杂了，之前我们只是提取竖线内部的内容，并按原样使用，但现在不一样了，声明闭包的时候需要 `mut` 关键字，但构造元组和解构元组的时候并不需要，所以我们得存储更多的信息。
这里使用了我们之前解析函数签名时的技巧，把 `mut` 存成可选的，并用逗号分隔。

顺带一提，之前的实现有个缺点，会把单变量存储为 `(var,)`（一个单变量元组）， 而不是 `(var)`（单独的变量，带有多余的括号），虽然对 `defer!` 宏没什么影响，但是调用 guard 的 `into_inner` 方法时就很反直觉了，明明只存了一个变量，怎么变成了一个单变量元组呢？
而这个缺点在下面的实现里被解决了，因为逗号不再被存储到列表里，而是在展开的时候作为分隔符加入。

这是我写到这里才想到的，之前我设计成了存储成两个列表，一个用于元组，另一个用于闭包，但处理逗号时很麻烦。如果对那个方案感兴趣，可以看看本文最开始的代码块，点击右上角的第一个按钮就会显示出完整的代码。

```Rust
// runnable
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

impl<F: FnOnce(T), T> Drop for DropGuard<F, T> {
    fn drop(&mut self) {
        let value = unsafe { ManuallyDrop::take(&mut self.inner) };
        let f = unsafe { ManuallyDrop::take(&mut self.f) };
        f(value);
    }
}

impl<F: FnOnce(T), T> Deref for DropGuard<F, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<F: FnOnce(T), T> DerefMut for DropGuard<F, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub struct DropGuard<F: FnOnce(T), T> {
    f: ManuallyDrop<F>,
    inner: ManuallyDrop<T>,
}

impl<F: FnOnce(T), T> DropGuard<F, T> {
    pub fn new(inner: T, f: F) -> Self {
        Self {
            f: ManuallyDrop::new(f),
            inner: ManuallyDrop::new(inner),
        }
    }
    pub fn into_inner(self) -> T {
        let mut new_guard = ManuallyDrop::new(self);
        let value = unsafe { ManuallyDrop::take(&mut new_guard.inner) };
        unsafe { ManuallyDrop::drop(&mut new_guard.f) };
        value
    }
}

// ANCHOR
macro_rules! defer_impl{
    ($guard:ident [$($($mut:ident)?, $vars:ident)*] $(,)? | $body:block) => {
        // 在这里展开时，逗号作为分隔符被加入
        // 所以对于单个变量，会变成 `(var)`，这里的括号是多余的
        // 可以使用下面这个属性宏来避免警告
        #[allow(unused_parens)]
        let mut $guard = $crate::DropGuard::new(
            ($($vars),*),
            |($($($mut)? $vars),*)| $body,
        );

        #[allow(unused_parens)]
        let ($($vars),*) = &mut *$guard;
    };
    // 增加一个匹配 mut 的分支，因为逗号都在这些中间的分支被处理了，所以结果完全一样，
    // 仍然只需要一个结果分支
    ($guard:ident [$($($mut:ident)?, $vars:ident)*] , mut $v:ident $($rest:tt)*) => {
        defer_impl!($guard [$($vars)* mut, $v] $($rest)*);
    };
    ($guard:ident [$($($mut:ident)?, $vars:ident)*] ,$v:ident $($rest:tt)*) => {
        defer_impl!($guard [$($vars)* , $v] $($rest)*);
    };
}

macro_rules! defer{
    (|$($vars:tt)*) => {
        defer_impl!(__guard [] ,$($vars)*);
    };
    ($guard:ident, |$($vars:tt)*) => {
        defer_impl!($guard [] ,$($vars)*);
    };
}

fn main() {
    let a = vec![1, 2];
    defer!(|mut a| {
        a.push(4);
        println!("drop `a`");
        println!("{:?}", a);
    });
    println!("modify a");
    a.push(3);
}
// ANCHOR_END
```

### 支持捕获引用

之前的实现都是捕获变量本身，把所有权捕获到 `DropGuard` 结构体里。但有时可能不希望移动所有权，只想捕获引用。
目前想要捕获引用，只能自己手动创建

```Rust
// runnable
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

impl<F: FnOnce(T), T> Drop for DropGuard<F, T> {
    fn drop(&mut self) {
        let value = unsafe { ManuallyDrop::take(&mut self.inner) };
        let f = unsafe { ManuallyDrop::take(&mut self.f) };
        f(value);
    }
}

impl<F: FnOnce(T), T> Deref for DropGuard<F, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<F: FnOnce(T), T> DerefMut for DropGuard<F, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub struct DropGuard<F: FnOnce(T), T> {
    f: ManuallyDrop<F>,
    inner: ManuallyDrop<T>,
}

impl<F: FnOnce(T), T> DropGuard<F, T> {
    pub fn new(inner: T, f: F) -> Self {
        Self {
            f: ManuallyDrop::new(f),
            inner: ManuallyDrop::new(inner),
        }
    }
    pub fn into_inner(self) -> T {
        let mut new_guard = ManuallyDrop::new(self);
        let value = unsafe { ManuallyDrop::take(&mut new_guard.inner) };
        unsafe { ManuallyDrop::drop(&mut new_guard.f) };
        value
    }
}

macro_rules! defer_impl{
    ($guard:ident [$($($mut:ident)?, $vars:ident)*] $(,)? | $body:block) => {
        #[allow(unused_parens)]
        let mut $guard = $crate::DropGuard::new(
            ($($vars),*),
            |($($($mut)? $vars),*)| $body,
        );

        #[allow(unused_parens)]
        let ($($vars),*) = &mut *$guard;
    };
    ($guard:ident [$($($mut:ident)?, $vars:ident)*] , mut $v:ident $($rest:tt)*) => {
        defer_impl!($guard [$($vars)* mut, $v] $($rest)*);
    };
    ($guard:ident [$($($mut:ident)?, $vars:ident)*] ,$v:ident $($rest:tt)*) => {
        defer_impl!($guard [$($vars)* , $v] $($rest)*);
    };
}

macro_rules! defer{
    (|$($vars:tt)*) => {
        defer_impl!(__guard [] ,$($vars)*);
    };
    ($guard:ident, |$($vars:tt)*) => {
        defer_impl!($guard [] ,$($vars)*);
    };
}

// ANCHOR
fn main() {
    let mut a = vec![1, 2];
    let a = &mut a;
    defer!(|a| {
        a.push(4);
        println!("drop `a`");
        println!("{:?}", a);
    });
    println!("modify a");
    a.push(3);
    println!("{}", std::any::type_name_of_val(&a));
}
// ANCHOR_END
```

这个写法有点麻烦，而且内部重新借用后的 `a` 的类型变成了 `&mut &mut Vec<i32>`，这个双重引用完全没必要。
模仿 C++ 的语法，可以给宏加入捕获引用的功能

```Rust
// runnable
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

impl<F: FnOnce(T), T> Drop for DropGuard<F, T> {
    fn drop(&mut self) {
        let value = unsafe { ManuallyDrop::take(&mut self.inner) };
        let f = unsafe { ManuallyDrop::take(&mut self.f) };
        f(value);
    }
}

impl<F: FnOnce(T), T> Deref for DropGuard<F, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<F: FnOnce(T), T> DerefMut for DropGuard<F, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub struct DropGuard<F: FnOnce(T), T> {
    f: ManuallyDrop<F>,
    inner: ManuallyDrop<T>,
}

impl<F: FnOnce(T), T> DropGuard<F, T> {
    pub fn new(inner: T, f: F) -> Self {
        Self {
            f: ManuallyDrop::new(f),
            inner: ManuallyDrop::new(inner),
        }
    }
    pub fn into_inner(self) -> T {
        let mut new_guard = ManuallyDrop::new(self);
        let value = unsafe { ManuallyDrop::take(&mut new_guard.inner) };
        unsafe { ManuallyDrop::drop(&mut new_guard.f) };
        value
    }
}

// ANCHOR
macro_rules! defer_impl{
    ($guard:ident [$($($mut:ident)?, $vars:ident)*] $(,)? | $body:block) => {
        #[allow(unused_parens)]
        let mut $guard = $crate::DropGuard::new(
            ($($vars),*),
            |($($($mut)? $vars),*)| $body,
        );

        #[allow(unused_parens)]
        let ($($vars),*) = &mut *$guard;
    };
    ($guard:ident [$($($mut:ident)?, $vars:ident)*] , mut $v:ident $($rest:tt)*) => {
        defer_impl!($guard [$($vars)* mut, $v] $($rest)*);
    };
    // 只需要多加一个分支，匹配带 `&` 符号的变量
    ($guard:ident [$($($mut:ident)?, $vars:ident)*] , &$v:ident $($rest:tt)*) => {
        // 先创建可变引用
        let $v = &mut $v;
        // 把可变引用的所有权交给 guard
        defer_impl!($guard [$($vars)* , $v] $($rest)*);
        // 之后重新借用回来，并且多解引用一次，避免双重引用
        let $v = &mut **$v;
    };
    ($guard:ident [$($($mut:ident)?, $vars:ident)*] ,$v:ident $($rest:tt)*) => {
        defer_impl!($guard [$($vars)* , $v] $($rest)*);
    };
}

macro_rules! defer{
    (|$($vars:tt)*) => {
        defer_impl!(__guard [] ,$($vars)*);
    };
    ($guard:ident, |$($vars:tt)*) => {
        defer_impl!($guard [] ,$($vars)*);
    };
}

fn main() {
    let mut a = vec![1, 2];
    defer!(|&a| {
        a.push(4);
        println!("drop `a`");
        println!("{:?}", a);
    });
    println!("modify a");
    a.push(3);
    println!("{}", std::any::type_name_of_val(&a));
}
// ANCHOR_END
```

但是这样一来少了给引用命名的功能，多数情况下，捕获引用而非所有权，是为了后续能在 drop 之后返回变量
所以再加一个分支

```Rust
// runnable
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

impl<F: FnOnce(T), T> Drop for DropGuard<F, T> {
    fn drop(&mut self) {
        let value = unsafe { ManuallyDrop::take(&mut self.inner) };
        let f = unsafe { ManuallyDrop::take(&mut self.f) };
        f(value);
    }
}

impl<F: FnOnce(T), T> Deref for DropGuard<F, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<F: FnOnce(T), T> DerefMut for DropGuard<F, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub struct DropGuard<F: FnOnce(T), T> {
    f: ManuallyDrop<F>,
    inner: ManuallyDrop<T>,
}

impl<F: FnOnce(T), T> DropGuard<F, T> {
    pub fn new(inner: T, f: F) -> Self {
        Self {
            f: ManuallyDrop::new(f),
            inner: ManuallyDrop::new(inner),
        }
    }
    pub fn into_inner(self) -> T {
        let mut new_guard = ManuallyDrop::new(self);
        let value = unsafe { ManuallyDrop::take(&mut new_guard.inner) };
        unsafe { ManuallyDrop::drop(&mut new_guard.f) };
        value
    }
}

// ANCHOR
macro_rules! defer_impl{
    ($guard:ident [$($($mut:ident)?, $vars:ident)*] $(,)? | $body:block) => {
        #[allow(unused_parens)]
        let mut $guard = $crate::DropGuard::new(
            ($($vars),*),
            |($($($mut)? $vars),*)| $body,
        );

        #[allow(unused_parens)]
        let ($($vars),*) = &mut *$guard;
    };
    ($guard:ident [$($($mut:ident)?, $vars:ident)*] , mut $v:ident $($rest:tt)*) => {
        defer_impl!($guard [$($vars)* mut, $v] $($rest)*);
    };
    // 使用 `bind: &var`，类似构建结构体时的语法
    ($guard:ident [$($($mut:ident)?, $vars:ident)*] , $bind:ident: &$var:ident $($rest:tt)*) => {
        let $bind = &mut $var;
        defer_impl!($guard [$($vars)* , $bind] $($rest)*);
        let $bind = &mut **$bind;
    };
    ($guard:ident [$($($mut:ident)?, $vars:ident)*] , &$v:ident $($rest:tt)*) => {
        let $v = &mut $v;
        defer_impl!($guard [$($vars)* , $v] $($rest)*);
        let $v = &mut **$v;
    };
    ($guard:ident [$($($mut:ident)?, $vars:ident)*] ,$v:ident $($rest:tt)*) => {
        defer_impl!($guard [$($vars)* , $v] $($rest)*);
    };
}

macro_rules! defer{
    (|$($vars:tt)*) => {
        defer_impl!(__guard [] ,$($vars)*);
    };
    ($guard:ident, |$($vars:tt)*) => {
        defer_impl!($guard [] ,$($vars)*);
    };
}

fn return_owned() -> Vec<i32> {
    let mut a = vec![1, 2];
    defer!(guard, |a_ref: &a| {
        a_ref.push(4);
        println!("drop `a_ref`");
        println!("{:?}", a_ref);
    });
    println!("modify a");
    a_ref.push(3);
    println!("a_ref: {}", std::any::type_name_of_val(&a_ref));
    drop(guard);
    a
}

fn main() {
    let a = return_owned();
    println!("return a: {}", std::any::type_name_of_val(&a));
}
// ANCHOR_END
```

### 再加点小功能

闭包支持使用 `move` 来表示内部使用的变量都需要被移动进去，虽然我觉得在这里不太用得到，但是这个功能很好加。
另外就是加上不指定变量的两种情况。

```Rust
// runnable
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

impl<F: FnOnce(T), T> Drop for DropGuard<F, T> {
    fn drop(&mut self) {
        let value = unsafe { ManuallyDrop::take(&mut self.inner) };
        let f = unsafe { ManuallyDrop::take(&mut self.f) };
        f(value);
    }
}

impl<F: FnOnce(T), T> Deref for DropGuard<F, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<F: FnOnce(T), T> DerefMut for DropGuard<F, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub struct DropGuard<F: FnOnce(T), T> {
    f: ManuallyDrop<F>,
    inner: ManuallyDrop<T>,
}

impl<F: FnOnce(T), T> DropGuard<F, T> {
    pub fn new(inner: T, f: F) -> Self {
        Self {
            f: ManuallyDrop::new(f),
            inner: ManuallyDrop::new(inner),
        }
    }
    pub fn into_inner(self) -> T {
        let mut new_guard = ManuallyDrop::new(self);
        let value = unsafe { ManuallyDrop::take(&mut new_guard.inner) };
        unsafe { ManuallyDrop::drop(&mut new_guard.f) };
        value
    }
}

// ANCHOR
macro_rules! defer_impl{
    // 只需将 move 作为可选参数捕获并传递即可，最后在闭包前面展开
    ($guard:ident $($move:ident)? [$($($mut:ident)?, $vars:ident)*] $(,)? | $body:block) => {
        #[allow(unused_parens)]
        let mut $guard = $crate::DropGuard::new(
            ($($vars),*),
            $($move)? |($($($mut)? $vars),*)| $body,
        );

        #[allow(unused_parens)]
        let ($($vars),*) = &mut *$guard;
    };
    ($guard:ident $($move:ident)? [$($($mut:ident)?, $vars:ident)*] , mut $v:ident $($rest:tt)*) => {
        defer_impl!($guard [$($vars)* mut, $v] $($rest)*);
    };
    ($guard:ident $($move:ident)? [$($($mut:ident)?, $vars:ident)*] , $bind:ident: &$var:ident $($rest:tt)*) => {
        let $bind = &mut $var;
        defer_impl!($guard [$($vars)* , $bind] $($rest)*);
        let $bind = &mut **$bind;
    };
    ($guard:ident $($move:ident)? [$($($mut:ident)?, $vars:ident)*] , &$v:ident $($rest:tt)*) => {
        let $v = &mut $v;
        defer_impl!($guard [$($vars)* , $v] $($rest)*);
        let $v = &mut **$v;
    };
    ($guard:ident $($move:ident)? [$($($mut:ident)?, $vars:ident)*] ,$v:ident $($rest:tt)*) => {
        defer_impl!($guard [$($vars)* , $v] $($rest)*);
    };
}

macro_rules! defer{
    // 指定捕获变量，使用内部 guard，不可取消
    (|$($vars:tt)*) => {
        defer_impl!(__guard [] ,$($vars)*);
    };
    // 指定捕获变量，使用指定的 guard，可取消
    ($guard:ident, $($move:ident)? |$($vars:tt)*) => {
        defer_impl!($guard $($move:ident)? [] ,$($vars)*);
    };
    // 不指定捕获变量，使用指定的 guard，可取消
    // 这里直接捕获表达式，可以接收闭包，move 等功能都由闭包原生支持
    ($guard:ident, $body:expr) => {
        let $guard = $crate::DropGuard::new(
            (),
            |_| $body(),
        );
    };
    // 不指定捕获变量，使用内部 guard，不可取消
    // scopeguard 的 defer 宏就只支持这一种情况
    ($body:expr) => {
        let __guard = $crate::DropGuard::new(
            (),
            |_| $body(),
        );
    };
}

fn main() {
    let mut a = vec![1, 2];
    defer!(guard, move || {
        a.push(3);
        println!("drop `a`");
        println!("{:?}", a);
        println!("type of a: {}", std::any::type_name_of_val(&a));
    });
}
// ANCHOR_END
```

### 关于可变性的设计

Rust 里采取的是默认不可变，需要时使用 `mut` 声明可变这一策略，这个宏也在尽量遵循这个设计。

对于可变性的需求，可以分为在宏内部和在宏之后
这里的可变性，是指对被捕获值的可变性，而不是变量本身的可变性，如捕获 `String` 类型为可变引用，宏内部使用的是 `s: &mut String` ，这里可以通过 `s` 去修改字符串，就称为可变，但 `s` 这个变量本身不可变（而且似乎也没有可变的需求）

| 方式 | 在宏内部 | 在宏之后 |
| --- | --- | --- |
| 捕获为可变引用 | 可变 | 可变 |
| 捕获时移动变量所有权 | 不可变（默认行为） | 可变 |
| 捕获时移动变量所有权 | 可变（通过 `mut` 来声明） | 可变 |
| 使用闭包自带的捕获功能捕获为不可变引用 | 不可变 | 不可变 |
| 不支持 | 可变 | 不可变 |

对于捕获引用的情况来说，宏内部和宏之后使用的变量名和类型是一样的，可变性最好保持一样；
对于捕获所有权的情况来说，捕获所有权之后，内部当然可以选择是否需要可变性，但是宏之后的变量是重新绑定的，两者的类型不一致，所以可变性不必相同
理论上来说可以支持最后一种情况，但是相当奇怪，且语法不好设计，就没实现。如果确实有需求，完全可以通过手动的 `let` 重绑定来实现。

## 局限性

我们在 Rust 中实现了 `defer!` 宏，简化了语法，覆盖了基础功能，但使用 `macro_rules!` 能做到的功能有限，比如它需要手动指定需要的变量，不能完全自动捕获；它不像 Go 的 defer，可以直接 return 函数返回值；也不像 Zig 中的 defer，可以在根据出错与否自动取消或执行。使用过程宏或许能实现更多功能，但更复杂的功能终究还是需要编译器来实现，那样会更简单也更高效，希望 defer 能早日进入 Rust 的 RFC

### defer 中的陷阱

defer 与 RAII 的不同之处在于，它更灵活自由，但仍然遵循着先定义的后释放的原则，所以使用时需要特别小心释放顺序
如果是手动管理 C 里的资源，最好像 RAII 一样，获取后立即声明 defer

```Rust
// runnable
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

impl<F: FnOnce(T), T> Drop for DropGuard<F, T> {
    fn drop(&mut self) {
        let value = unsafe { ManuallyDrop::take(&mut self.inner) };
        let f = unsafe { ManuallyDrop::take(&mut self.f) };
        f(value);
    }
}

impl<F: FnOnce(T), T> Deref for DropGuard<F, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<F: FnOnce(T), T> DerefMut for DropGuard<F, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub struct DropGuard<F: FnOnce(T), T> {
    f: ManuallyDrop<F>,
    inner: ManuallyDrop<T>,
}

impl<F: FnOnce(T), T> DropGuard<F, T> {
    pub fn new(inner: T, f: F) -> Self {
        Self {
            f: ManuallyDrop::new(f),
            inner: ManuallyDrop::new(inner),
        }
    }
    pub fn into_inner(self) -> T {
        let mut new_guard = ManuallyDrop::new(self);
        let value = unsafe { ManuallyDrop::take(&mut new_guard.inner) };
        unsafe { ManuallyDrop::drop(&mut new_guard.f) };
        value
    }
}

macro_rules! defer_impl{
    ($guard:ident $($move:ident)? [$($($mut:ident)?, $vars:ident)*] $(,)? | $body:block) => {
        #[allow(unused_parens)]
        let mut $guard = $crate::DropGuard::new(
            ($($vars),*),
            $($move)? |($($($mut)? $vars),*)| $body,
        );

        #[allow(unused_parens)]
        let ($($vars),*) = &mut *$guard;
    };
    ($guard:ident $($move:ident)? [$($($mut:ident)?, $vars:ident)*] , mut $v:ident $($rest:tt)*) => {
        defer_impl!($guard [$($vars)* mut, $v] $($rest)*);
    };
    ($guard:ident $($move:ident)? [$($($mut:ident)?, $vars:ident)*] , $bind:ident: &$var:ident $($rest:tt)*) => {
        let $bind = &mut $var;
        defer_impl!($guard [$($vars)* , $bind] $($rest)*);
        let $bind = &mut **$bind;
    };
    ($guard:ident $($move:ident)? [$($($mut:ident)?, $vars:ident)*] , &$v:ident $($rest:tt)*) => {
        let $v = &mut $v;
        defer_impl!($guard [$($vars)* , $v] $($rest)*);
        let $v = &mut **$v;
    };
    ($guard:ident $($move:ident)? [$($($mut:ident)?, $vars:ident)*] ,$v:ident $($rest:tt)*) => {
        defer_impl!($guard [$($vars)* , $v] $($rest)*);
    };
}

macro_rules! defer{
    (|$($vars:tt)*) => {
        defer_impl!(__guard [] ,$($vars)*);
    };
    ($guard:ident, $($move:ident)? |$($vars:tt)*) => {
        defer_impl!($guard $($move:ident)? [] ,$($vars)*);
    };
    ($guard:ident, $body:expr) => {
        let $guard = $crate::DropGuard::new(
            (),
            |_| $body(),
        );
    };
    ($body:expr) => {
        let __guard = $crate::DropGuard::new(
            (),
            |_| $body(),
        );
    };
}

// ANCHOR
fn correct() {
    let a: *mut i32 = Box::leak(Box::new(1));
    defer!(|| {
        unsafe {
            println!("drop a: {}", *a);
            drop(Box::from_raw(a));
        }
    });
    let b = Box::leak(Box::new(a));
    defer!(|| {
        unsafe {
            println!("drop b: {}", **b);
            drop(Box::from_raw(b));
        }
    });
}

fn wrong() {
    let a: *mut i32 = Box::leak(Box::new(1));
    let b = Box::leak(Box::new(a));
    defer!(|| {
        unsafe {
            println!("drop b: {}", **b);
            drop(Box::from_raw(b));
        }
    });
    defer!(|| {
        unsafe {
            println!("drop a: {}", *a);
            drop(Box::from_raw(a));
        }
    });
}

fn main() {
    println!("correct:");
    correct();
    println!("wrong:");
    wrong();
    println!("finish");
}
// ANCHOR_END
```

运行程序会发现 `wrong` 函数里 drop b 的时候发生了 use after free 的经典问题，原因就是先释放了 a，后释放了 b

## 最后

本来还想再多说说各种语言里的 defer，但就这样吧，这篇文章已经很长很长了……

[^1]:
    [Perhaps Rust needs "defer"](https://gaultier.github.io/blog/perhaps_rust_needs_defer.html)
    [Pre-RFC: defer statement](https://internals.rust-lang.org/t/pre-rfc-defer-statement/16644)
    [A defer discussion](https://internals.rust-lang.org/t/a-defer-discussion/20387)
    [Pre-pre-pre-RFC: implicit code control and defer statements](https://internals.rust-lang.org/t/pre-pre-pre-rfc-implicit-code-control-and-defer-statements/20071)

[^2]:
    Rust 的闭包有 3 种类型，`Fn`, `FnMut`，`FnOnce`，这里使用 `FnOnce`，即只能执行一次，会消耗所有权的闭包，另外两种闭包都可以协变为 `FnOnce`，所以用在这里通用性最好。

[^3]:
    `Box` 是 Rust 中的智能指针，相当于 C++ 里的 `unique_ptr`

[^4]:
    更多类型请参考[Captures](https://danielkeep.github.io/tlborm/book/mbe-macro-rules.html#captures)

[^5]:
    可以参考[Hygiene](https://danielkeep.github.io/tlborm/book/mbe-min-hygiene.html)这一章中的解释

[^6]:
    可以通过 crates.io 安装 [cargo-expand](https://crates.io/crates/cargo-expand) 包
