
## String 与 str
### String
这是最常见的字符串类型，内部是合法的 UTF-8，从其他字符串转换时，需要经过检查，以保证合法性。
与 C 语言中的字符串不同，它不以 '\0' 结尾，而是在里面记录了字符串的长度信息，以较小的空间占用换来了更高效的读取方式，也使得获取字符串的长度这一操作几乎零成本，不需要遍历整个字符串。同时也更灵活了，可以在字符串里存储 '\0'（好奇怪的需求）
这是标准库里的定义：
```Rust
#[derive(PartialEq, PartialOrd, Eq, Ord)]
#[stable(feature = "rust1", since = "1.0.0")]
#[lang = "String"]
pub struct String {
    vec: Vec<u8>,
}
```
可以看出来，没有任何神秘的操作，里面就是个存储着 u8 的 vector，构造字符串、转换字符串时的安全性，完全由它的方法来保证。

### str
它是 Rust 的 primitive type (原始类型)，就像 i32，u8，bool 一样。&str 表示的是字符串切片，它指向一块存储着合法的 UTF-8 数组的内存。str 不能直接存储或者用于函数参数 / 返回值等，因为它表示的是一块动态大小的内存，编译时无法知道它的大小，所以只能以引用/指针的形式来使用它，比如：&str, \*const str, Box\<str> 等等。
&str 最常用的地方有两个，string literals (字符串字面量)和表示字符串切片。
string literals 就是代码里的常量字符串，比如写 `let text = "abc";` ， 其中 "abc" 就是常量字符串，它直接被编码进二进制文件中，在整个程序运行期间都有效。
字符串切片通常是对 String 的切片，从 String 产生一个 &str 是几乎零成本的，当然，其他类型也可以通过一些转换，表示为字符串切片，此时 &str 指向的内存就不属于 String 了，但是仍然是合法的 UTF-8 数组。
&str 和 String 非常像，它是一个胖指针，由两个部分组成：指向 \[u8] 的指针，和表示字符串长度的 usize 。
#### len 方法
String 和 str 都有 len 方法，但是它们都只表示内部存储的 u8 数组的长度，并不一定是人类可读的字符串长度，因为 UTF-8 是可变长度编码，当一个 u8 存不下字符时，会使用多个 u8。比如
```Rust
let len = "foo".len();
assert_eq!(3, len);

assert_eq!("ƒoo".len(), 4); // fancy f!
assert_eq!("ƒoo".chars().count(), 3);
```
当然，实际上字符串的处理更加复杂[^1]。

#### 小歪个题
Rust 和 C 在处理数组/切片这方面的设计有很大的区别，C 里面倾向于更小的内存占用，比如字符串使用 '\0' 结尾，数组长度只能使用 `sizeof` 获取，`sizeof` 会在编译时就进行计算，无法动态获取数组长度，所以数组长度和数组完全是分开管理的，函数传参也需要传数组本身和长度两个参数。
而 Rust 更倾向于安全可控，数组/切片/字符串都会存储自己的长度，如果是数组本身，长度就存储在类型里面，如果是数组指针/引用/切片，则是一个胖指针，里面存储着数据和长度两个信息。
一个有趣的设计是 unsized type，即编译时未知大小的类型，str 就是一种 unsized type，只能使用胖指针来进行存储。
主要问题在于，要访问一个内存中的数据，只有单个指针是不够的，指针只指向数据起始处，数据长度或终止处是未知的。如果指针指向的是一种 sized type，那编译时就能知道它的大小，可以在编译时就自动把相关操作都加上长度，这样就能读取到完整的数据，从而进行解引用。如果编译时大小未知，那就要靠运行时动态存储大小了，比如在结构体里单独用一个字段存储，或者用胖指针。

## CString 与 CStr
### CString
与 String 类似，也是具有所有权的字符串类型，但它是 C-compatible 的。它以 '\0' 结尾，并且中间不含任何 '\0'，且内部可以是任意编码。对于 CString，Rust 也尝试让它变得更加安全，就是在构造的时候检查中间是否有 '\0'，从而保证 CString 内部一定是有效的 C 字符串。
这是标准库里对它的定义：
```Rust
#[derive(PartialEq, PartialOrd, Eq, Ord, Hash, Clone)]
#[rustc_diagnostic_item = "cstring_type"]
#[stable(feature = "alloc_c_string", since = "1.64.0")]
pub struct CString {
    // Invariant 1: the slice ends with a zero byte and has a length of at least one.
    // Invariant 2: the slice contains only one zero byte.
    // Improper usage of unsafe function can break Invariant 2, but not Invariant 1.
    inner: Box<[u8]>,
}
```
可以看出，内部是一个长度不可变的动态分配的 u8 数组，与 C 中的字符串很相似。并且它始终保证只在结尾有一个 '\0'，当需要传给 C 的函数时，直接传这个数组的指针即可。当然，这里的 Box 也是一个胖指针，记录了长度数据，所以可以很方便地获取字符串长度，并且不会在计算长度的时候发生越界错误。

标准库还为它实现了额外的 Drop 方法
```Rust
// Turns this `CString` into an empty string to prevent
// memory-unsafe code from working by accident. Inline
// to prevent LLVM from optimizing it away in debug builds.
#[stable(feature = "cstring_drop", since = "1.13.0")]
#[rustc_insignificant_dtor]
impl Drop for CString {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            *self.inner.get_unchecked_mut(0) = 0;
        }
    }
}
```
这里非常有意思，首先说说这个方法本身的作用，它是帮助进行 use after free 检测的，当 CString 离开作用域，被自动销毁的时候，会调用这个 Drop 方法，自动将数组的首位设为 '\0'，这样即使还持有它的指针，并且指针指向的内存还没有被销毁，尝试使用的时候，也会发现字符串为空，可以更早地发现错误。因为 CString 的实现保证了 "the slice ends with a zero byte and has a length of at least one"，所以这里访问首位永远都不会越界。
再说说优化，这里使用了 `#[rustc_insignificant_dtor]`, "A type marked with the attribute `rustc_insignificant_dtor` is considered to not be significant. A drop is significant if it is implemented by the user or does anything that will have any observable behavior (other than freeing up memory)". 也就是说，虽然手动为它实现了 drop 方法，但这个 drop 方法不重要，可以在必要的时候优化掉，避免在 release 模式下影响性能。
这个标注是给 rustc 的，rustc 会生成 LLVM IR，然后由 LLVM 后端进行编译，LLVM 的优化十分优秀，它会认为 drop 方法尝试写入了一块即将释放的内存，并且在释放之前也没被再次读取，就把 drop 方法给优化掉了。
所以在这里还标注了 `#[inline]`，把代码内联进去，这样在 debug build 下，LLVM 不会优化掉 drop 方法，但是在 release build 下，LLVM 经过分析后仍然能够优化掉 drop 方法。

### CStr
这是一种 unsized type，它的大小在编译时未知，就像 str 一样，常常使用 `&CStr` 来表示。
它是对 CString 的借用，一般来说也就是对 CString 的切片，这是标准库对它的定义
```Rust
#[repr(transparent)]
pub struct CStr {
    inner: [c_char],
}
```
`#[repr(transparent)]` 表示内存布局和 inner 完全一致，struct 是完全透明的，不存在额外的对齐。在传递字符串作为 C 函数的参数时，可以直接传递 CStr 的指针。

## Cow
顺便来说一下 Rust 里的 Cow 字符串。Cow，即 copy on write，写时复制，即在复制的时候，并不真正深拷贝，而是只复制指向原来字符串的指针/引用，在需要修改字符串的时候，才实际进行深拷贝，复制整个字符串。
典型的 Cow 实现，比如 Qt 的 QString，更类似于 Rust 里的 Arc\<String>，每次复制的时候只增加引用计数，需要修改的时候才调用 Arc 的 `make_mut()` 方法，该方法会检查当前的引用计数是否为1，如果为1，则直接返回 String 的可变引用，否则就深拷贝整个 String，并放到新的 Arc 里，再返回对它的可变引用。

而 Rust 的 Cow 更加简单，它本质上是个枚举，定义如下：
```Rust
#[stable(feature = "rust1", since = "1.0.0")]
#[rustc_diagnostic_item = "Cow"]
pub enum Cow<'a, B: ?Sized + 'a>
where
    B: ToOwned,
{
    /// Borrowed data.
    #[stable(feature = "rust1", since = "1.0.0")]
    Borrowed(#[stable(feature = "rust1", since = "1.0.0")] &'a B),

    /// Owned data.
    #[stable(feature = "rust1", since = "1.0.0")]
    Owned(#[stable(feature = "rust1", since = "1.0.0")] <B as ToOwned>::Owned),
}
```

它可能为 owned，也就是具有所有权；或者 borrowed，也就是借用状态。它的好处是，灵活地把借用状态和 owned 状态整合成了一种类型。
比如说，我有一个操作是对字符串进行替换，比如把字符串里的 '\\' 都替换成 '\\\\'，如果字符串里真的有 '\\'，那替换之后肯定需要重新分配内存，返回的是一个具有所有权的 String；但如果没有需要替换的字符，显然没必要重新分配，应该返回一个对原字符串的借用。但很显然，方法的返回值必须是固定的一种类型，这个时候就可以用 Cow 把返回值包起来，不管是 String 还是借用的 &str，都可以表示为 Cow。
在实际使用的时候呢，如果是只读操作，可以把 Cow 当作 &str 使用，如果是写操作，就判断是否为 Owned，如果为 Owned 就把 String 取出来，否则就调用 `to_owned()` 方法得到 Owned 的值。

那复制 Cow 的时候会发生什么呢？这是 `clone()` 方法的实现：
```Rust
#[stable(feature = "rust1", since = "1.0.0")]
impl<B: ?Sized + ToOwned> Clone for Cow<'_, B> {
    fn clone(&self) -> Self {
        match *self {
            Borrowed(b) => Borrowed(b),
            Owned(ref o) => {
                let b: &B = o.borrow();
                Owned(b.to_owned())
            }
        }
    }
}
```
可见，当 b 是借用状态时，则再次产生一个借用状态的 Cow；当 b 是 owned 状态时，就对 b 调用 `to_owned()` 方法（大概率进行了深拷贝），产生一个 owned 的 Cow。
得益于 Rust 较为智能的生命周期推导，大部分情况下，这个 Cow 实现已经足够灵活了，并且没有引用计数的开销。

当然，生命周期本身是个非常复杂的东西，我实际使用的时候，会稍微简化一下，像这样，用于错误处理。
这是我定义的自己的错误类型
```Rust
enum CatError{
	Custom(Cow<'static, str>),
}
impl CatError {
    pub fn custom<S: Into<Cow<'static, str>>>(s: S) -> Self {
        CatError::Custom(s.into())
    }
}
```
将 Cow 的借用限制为静态生命周期，也就是字符串常量。
配合 `custom` 方法，在构造错误时，如果需要携带额外信息，就使用 `CatError::custom(format!("error: {e}"))`—— 构造一个字符串，再用 Cow 包起来；如果不需要额外信息，就可以 `CatError::custom("an error occured")` ，直接把字符串常量包起来，没有额外分配。


[^1]: [索引字符串](https://kaisery.github.io/trpl-zh-cn/ch08-02-strings.html#%E7%B4%A2%E5%BC%95%E5%AD%97%E7%AC%A6%E4%B8%B2), [indexing-into-strings](https://doc.rust-lang.org/stable/book/ch08-02-strings.html#indexing-into-strings)
