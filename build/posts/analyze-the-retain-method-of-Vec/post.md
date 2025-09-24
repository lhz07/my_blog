```Rust
/// Retains only the elements specified by the predicate, passing a mutable reference to it.
///
/// In other words, remove all elements `e` such that `f(&mut e)` returns `false`.
/// This method operates in place, visiting each element exactly once in the
/// original order, and preserves the order of the retained elements.
///
/// # Examples
///
/// ```
/// let mut vec = vec![1, 2, 3, 4];
/// vec.retain_mut(|x| if *x <= 3 {
///     *x += 1;
///     true
/// } else {
///     false
/// });
/// assert_eq!(vec, [2, 3, 4]);
/// ```
#[stable(feature = "vec_retain_mut", since = "1.61.0")]
pub fn retain_mut<F>(&mut self, mut f: F)
where
    F: FnMut(&mut T) -> bool,
{
    let original_len = self.len();

    if original_len == 0 {
        // Empty case: explicit return allows better optimization, vs letting compiler infer it
        return;
    }

    // Avoid double drop if the drop guard is not executed,
    // since we may make some holes during the process.
    unsafe { self.set_len(0) };

    // Vec: [Kept, Kept, Hole, Hole, Hole, Hole, Unchecked, Unchecked]
    //      |<-              processed len   ->| ^- next to check
    //                  |<-  deleted cnt     ->|
    //      |<-              original_len                          ->|
    // Kept: Elements which predicate returns true on.
    // Hole: Moved or dropped element slot.
    // Unchecked: Unchecked valid elements.
    //
    // This drop guard will be invoked when predicate or `drop` of element panicked.
    // It shifts unchecked elements to cover holes and `set_len` to the correct length.
    // In cases when predicate and `drop` never panick, it will be optimized out.
    struct BackshiftOnDrop<'a, T, A: Allocator> {
        v: &'a mut Vec<T, A>,
        processed_len: usize,
        deleted_cnt: usize,
        original_len: usize,
    }

    impl<T, A: Allocator> Drop for BackshiftOnDrop<'_, T, A> {
        fn drop(&mut self) {
            if self.deleted_cnt > 0 {
                // SAFETY: Trailing unchecked items must be valid since we never touch them.
                unsafe {
                    ptr::copy(
                        self.v.as_ptr().add(self.processed_len),
                        self.v
                            .as_mut_ptr()
                            .add(self.processed_len - self.deleted_cnt),
                        self.original_len - self.processed_len,
                    );
                }
            }
            // SAFETY: After filling holes, all items are in contiguous memory.
            unsafe {
                self.v.set_len(self.original_len - self.deleted_cnt);
            }
        }
    }

    let mut g = BackshiftOnDrop {
        v: self,
        processed_len: 0,
        deleted_cnt: 0,
        original_len,
    };

    fn process_loop<F, T, A: Allocator, const DELETED: bool>(
        original_len: usize,
        f: &mut F,
        g: &mut BackshiftOnDrop<'_, T, A>,
    ) where
        F: FnMut(&mut T) -> bool,
    {
        while g.processed_len != original_len {
            // SAFETY: Unchecked element must be valid.
            let cur = unsafe { &mut *g.v.as_mut_ptr().add(g.processed_len) };
            if !f(cur) {
                // Advance early to avoid double drop if `drop_in_place` panicked.
                g.processed_len += 1;
                g.deleted_cnt += 1;
                // SAFETY: We never touch this element again after dropped.
                unsafe { ptr::drop_in_place(cur) };
                // We already advanced the counter.
                if DELETED {
                    continue;
                } else {
                    break;
                }
            }
            if DELETED {
                // SAFETY: `deleted_cnt` > 0, so the hole slot must not overlap with current element.
                // We use copy for move, and never touch this element again.
                unsafe {
                    let hole_slot = g.v.as_mut_ptr().add(g.processed_len - g.deleted_cnt);
                    ptr::copy_nonoverlapping(cur, hole_slot, 1);
                }
            }
            g.processed_len += 1;
        }
    }

    // Stage 1: Nothing was deleted.
    process_loop::<F, T, A, false>(original_len, &mut f, &mut g);

    // Stage 2: Some elements were deleted.
    process_loop::<F, T, A, true>(original_len, &mut f, &mut g);

    // All item are processed. This can be optimized to `set_len` by LLVM.
    drop(g);
}

```

### 为什么需要两个`process_loop`？
在不删除元素的情况下，只需要遍历一遍所有元素，无需多余操作。
但是一旦需要删除元素，即使只删除 1 个元素，也需要把后面的元素挨个向左移动一次。
所以第一个 `process_loop` 就是处理没有删除元素的情况，此时：
1. 如果遍历完整个 `Vec` 也没有遇到需要删除的元素，第二个 `process_loop` 也不会运行。
2. 如果遇到需要删除的元素，证明后面的元素都要进行一次移动了，所以运行第二个 `process_loop`。在这个循环里面，将从之前未处理过的元素继续遍历，每次遍历的时候，都将后面的元素经过一次移动直接到位——从当前位置往回移动 x 位（ x 即为已经删除掉的元素数量）。因为此时至少有一个元素被删除了，所以绝对需要复制到前面的位置，而不是原地复制，这里使用 `copy_nonoverlapping` 可以进行更激进的优化
#### 模版优化
这里还有一个十分有用的优化，`fn process_loop<F, T, A: Allocator, const DELETED: bool>` 这个函数定义中的 `const DELETED: bool` ，表明 `DELETED` 是一个常量，编译的时候，会根据模版生成对应的两套代码，并且在这两套代码中，由于 `DELETED` 是常量，两个 `if DELETED` 分支都可以直接被优化掉！
事实上，只用一个 `process_loop` ，再加上一个 bool 变量用于判断，也是可以实现的，但是每次循环都需要多加一个 if 判断，而这个实现可以把 if 判断给优化掉
### 时间复杂度
虽然进行了两次 `process_loop` ，但第二次是接着第一次继续运行的，没有重复处理元素，时间复杂度为 O(n)，n 为原 `Vec` 的长度，非常高效。
### 空间复杂度
只使用了数个 `usize` 变量和引用/指针，复杂度为 O(1)，非常高效。
### `panic` 时的安全处理
传入 `retain` 的用于判断的闭包，和 `Vec` 中元素的 `Drop` 方法，都是有可能 `panic` 的，所以还需要在发生 `panic` 的时候进行安全处理，不能把已经产生的空洞留在 `Vec` 里面，也就是说，我们要保证即使发生了 `panic` ，这个 `Vec` 里的元素依旧全部都是有效的，没有空洞（已经释放掉的元素）。
在这里使用了常见的 drop guard 方式，为`struct BackshiftOnDrop<'a, T, A: Allocator>` 实现了 `Drop` trait，这样它会在被释放的时候自动调用我们给它实现的 `drop` 方法。在我们实现的 `drop` 方法里面，需要做两件事：
1. 把后面未处理的元素全部搬到前面来，补上之前由于删除造成的空洞
2. 重新设置 `Vec` 的长度，把已经删除的元素数量从中减去
#### 如果我们没有发生 panic 呢？
1. 当 drop 发生时，所有元素都被处理完了，这段代码
```Rust
ptr::copy(self.v.as_ptr().add(self.processed_len),self.v.as_mut_ptr().add(self.processed_len - self.deleted_cnt),self.original_len - self.processed_len,);
```
实际上是 `ptr::copy(x, y, 0);` ，需要复制的长度为 0 ，编译器会把它优化掉。
2. 仍然会重新设置 `Vec` 的长度，把已经删除的元素数量从中减去
#### drop guard 方法的有效性
当程序 `panic` 的时候，会自动调用当前使用的所有变量的 `drop` 方法，在同作用域内按照声明的顺序释放，先释放当前作用域，再从里到外挨个释放外部的作用域。但是，有两种情况会使 drop guard 失效：
1. 在 `panic` 之后，调用 `drop` 方法的时候又发生了 `panic` ，此时 Rust 为了避免无限 `panic` ，会直接 `abort` ，即中止程序，剩下变量的 `drop` 方法都不会被运行，
2. 有时为了缩小程序的体积，可能会在 `Cargo.toml` 中把 `panic` 都改为 `abort` (来源：[min-sized-rust](https://github.com/johnthagen/min-sized-rust?tab=readme-ov-file#abort-on-panic))，这样程序一旦出错会直接中止，不会打印 `panic` 的 traceback 信息，这时也会导致 drop guard 失效。
