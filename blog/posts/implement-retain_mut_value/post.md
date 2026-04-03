## 前言
上一次我们分析了 Vec 的 `retain_mut` 方法，这一方法可以让你安全地进行边遍历边删除的操作，并在这个过程中修改值，但是仍然不够灵活，毕竟 `remove` 方法都可以返回删除的值，凭什么 `retain` 方法就不能让我拿到被删除的值的所有权呢？这一次，我们来尝试一下实现 `retain_mut_value`，即可以边遍历边删除元素，同时还能得到被移出 Vec 的 value，从而避免 clone。
`retain_mut` 函数已经为我们搭好了一个安全、高性能的框架，并且具有 panic safety，所以以下的函数都会在它的基础上进行修改。
那让我们先从最简单的开始吧！
## retain_option
`retain_mut` 方法依靠返回的 `bool` 值判断是否需要删除，一个自然的想法是，我能不能不要返回 `bool` 值，而是返回 `Option` 呢？每次遍历的时候直接给闭包值的所有权，然后在闭包里可以选择把值取出来，并返回一个 `None`，或者不取出来，并返回一个 `Some`，里面包着原来的值。
那么写出来大概是这样：
```Rust
fn process_loop<F, T, const DELETED: bool>(
    original_len: usize,
    f: &mut F,
    g: &mut BackshiftOnDrop<'_, T>,
) where
    F: FnMut(T) -> Option<T>,
{
    while g.processed_len != original_len {
        // SAFETY: Unchecked element must be valid.
        let cur = unsafe { g.v.as_mut_ptr().add(g.processed_len) };
        // SAFETY: We never touch the content of `cur` again after reading,
        // because the content in `cur` is invalid now.
        // However, `cur` is still a valid pointer to a blank memory.
        let element = unsafe { ptr::read(cur) };
        // Advance early to avoid double drop if `f` panicked.
        g.processed_len += 1;
        g.deleted_cnt += 1;
        match f(element) {
            None => {
                // We already advanced the counter.
                if DELETED {
                    continue;
                } else {
                    break;
                }
            }
            Some(element) => {
                // This means we don't want to delete this element, so we should restore the counter.
                g.processed_len -= 1;
                g.deleted_cnt -= 1;
                if DELETED {
                    // SAFETY: `deleted_cnt` > 0, so the hole slot must not overlap with current element.
                    // We use write for move, and never touch this element again.
                    unsafe {
                        let hole_slot =
                            g.v.as_mut_ptr().add(g.processed_len - g.deleted_cnt);
                        ptr::write(hole_slot, element);
                    }
                } else {
                    // If there is not a hole slot, we move it back to the current position
                    // The element may be changed, so we should always write it back.
                    // SAFETY: `cur` is a valid pointer to a blank memory.
                    unsafe {
                        ptr::write(cur, element);
                    };
                }
                g.processed_len += 1;
            }
        }
    }
}
```
相比 `retain_mut`，有几个需要注意的问题：
1. 我们把具有所有权的值本身传给了闭包，闭包一旦 panic，这个值会被自动释放，此时 Vec 里面绝对不能留着这个值的空洞了。所以我们需要提前增加 `processed_len` 和 `deleted_cnt`（第 15～16 行），并在返回 `Some`的情况下把值减回去。你可能注意到这里先把 `processed_len` 减回去了，后面又再加上，很奇怪的操作对吗？注意按照原方法的逻辑，此时是还没有处理过这个元素的，此时使用`g.v.as_mut_ptr().add(g.processed_len - g.deleted_cnt)` 来计算当前的空洞元素的位置是正确的。如果我们不想减回去又再加回来，那就需要改成 `g.v.as_mut_ptr().add(g.processed_len - 1 - g.deleted_cnt)`，感觉代码可读性更糟糕了，没有必要。
2. （第 36 行）当移动元素去填补空洞的时候，需要用 `ptr::write()`，而不是 `ptr::copy` 或者直接解引用赋值 `*hole_slot = element`。`hole_slot` 的值已经被释放掉了，此时再解引用显然是未定义行为。而相比 `retain_mut`，我们这里是已经取得了元素的所有权，直接 copy 的话，后面还需要 `mem::forget()` 去避免离开作用域时元素被自动释放。这里用 write 就很合适，它的语义是写一个具有所有权的值到一个地址中，里面已经处理好了自动释放的问题。
3. `if DELETED`（第 30 行）在这里还有个 else 分支，这是为什么呢？因为我们已经把所有权交给闭包了，我们无法确定使用者是否会在里面修改元素的值：如果元素的值没有被修改，那当然可以不用处理，直接 forget 闭包返回的值就行，但是如果元素的值被修改了，我们需要重新把元素写回去。这就体现出这一实现的最大问题了，我们无法得知用户是否修改了元素的值，因此始终都需要把返回的 `Some` 里的元素重新写回 Vec，最坏情况下，用户一个元素都没改，却需要把整个 Vec的值都重写一遍，这显然是不合理的，我们需要一个性能更好的，不那么愚蠢的实现。

## retain_mut_leak
从上面的经验可以得知，把所有权交给闭包相当麻烦，并不合适，我们或许该走引用这条道。
在介绍下一个方法前，我们来回顾一下 Rust 的安全保证，safe Rust 需要保证内存安全，但是需要保证内存不泄漏吗？
并不需要，Rust 没有打算保证内存不泄漏，`Box::leak()` 被标记为安全方法。原因有两点：
1. 内存泄漏相较于其他的内存安全问题（解引用空指针、野指针等），影响较小，不会立刻导致程序崩溃等问题。并且，事实上并不存在真正的内存泄漏，程序退出时一定会释放掉申请的所有内存，所谓的内存泄漏，只是相当于把释放内存的代码放到了最后，当程序结束时才执行。
2. Rust 有引用计数指针，很容易就可以写出循环引用的代码，互相引用的内存永远不会被释放，而 Rust 没有 gc (垃圾回收器)，无法自动运行检测循环引用的算法并清理循环引用。这就意味着，除非把引用计数指针也标记为 unsafe，safe Rust 中依旧存在内存泄漏。

综上所述，我们可以实现一个会泄漏内存的但是仍然符合 Rust 安全模型的 `retain_mut_leak` 方法，在闭包里提供对元素的可变引用，如果需要保留元素，那就不用动它，如果不需要保留元素，那就把它从 Vec 里移出来，但是不释放它的内存。
在调用这个方法时，如果不需要移除元素，返回 `true` 即可，如果需要移除元素，就需要使用 unsafe 方法 `ptr::read()` 读出元素，并返回 `false` 。
这个方法的实现很简单，只需要去掉标准库版本的 `unsafe { ptr::drop_in_place(cur) };` 就可以了。
但是对于 panic safety 来说，还有点问题。当调用者在闭包中读出了元素，此后又 panic 了，元素会被自动 drop，从而在 Vec 里留下空洞。虽然说，这是由于调用 unsafe 方法导致的，我们可以不对此负责，但是最好还是像上个实现那样，在调用闭包前就增加 `processed_len` 和 `deleted_cnt`，并在返回 `true` 的时候减回去。这样，当调用闭包时发生了 panic，如果在读出元素之前，那就是内存泄漏，如果在读出元素之后，那就没有任何问题，无论怎样都不会在 Vec 里留下空洞。

这一方法完美吗？似乎还是不好，如果想获取所有权，必须使用 unsafe 操作，但其实这都还在其次，最大的问题是很容易发生内存泄漏，如果用户返回了 `false`，但是忘记了读出元素，内存会泄漏，如果读出元素前就 panic 了，还是会内存泄漏。
究其原因，是由于返回 `false`( 表示保留元素与否 )，和读出元素是两个独立的操作，很容易就因为疏忽或 panic 而导致漏掉一个操作。
## retain_mut_value
有什么办法能把两个操作封装在一起，并且保证它们之间不会发生 panic 呢？
也许，就不应该直接给闭包提供元素本身的引用，我们把元素封装起来，就像标准库把指针用 Box 封装起来那样，再定义一些方法和实现 Deref trait，似乎也足够灵活，并且能把多个操作封装成一个方法调用。

### 实现 TakeCell
首先可以肯定的是，我们需要能原地修改元素，这就意味着，结构体里面存储的必须是引用或者指针。
还有一点可以肯定的是，我们需要获取元素的所有权，所以这个结构体里面，必须能表达空或者非空，此处可以用 `Option` 或者指针来表示。
这样一来我们就遇到了一个问题，结构体里面可能为空，难不成我们每次修改元素的时候，都要用 `unwrap` 或者直接解引用指针？还是说，每次修改元素都要做非空判断？虽然说对性能的影响很小，但是这也太难受了吧。
让我们再次回顾一下 Rust 的安全模型，safe Rust 里面无论怎么操作，都不会出现 UB (未定义行为)，但是一旦涉及了 unsafe，即使是 safe Rust 代码，也可能出现 UB。比方说标准库的 `retain_mut`，如果只使用 safe 代码，确实是不会出现 UB 的，但是如果我们用了 unsafe 代码，比方说，我用 `ptr::read()` 把可变引用里的值读出来，并把值 drop 掉，再返回一个 `false`，表示这个值需要删除。那么当执行到 `unsafe { ptr::drop_in_place(cur) };` 就会发生二次释放，在标准库的 safe 方法里产生了 UB 😱

所以，我们完全可以采取不同的思路，Rust 的设计是，产生指针、获取指针是安全的，只有解引用指针是不安全的。我们可以反过来，只有创建结构体或者把值从结构体里面取出来的时候是 unsafe，其他时候都是 safe。这样一来，我们确保了只有 safe 方法不会导致 UB，而涉及到 unsafe 的时候，就不需要我们负责了。
使用指针的实现如下：
```Rust
pub struct TakeCell<T> {
    value: *mut T,
}

impl<T> TakeCell<T> {
    /// # Safety
    /// The caller must ensure one of the following rules:
    /// - the `value` is valid and will not be used after calling `take_inner`.
    /// - the `value` is null, and will be used.
    unsafe fn new(value: *mut T) -> Self {
        TakeCell { value }
    }
    /// # Safety
    /// - The caller must ensure that `TakeCell` is not taken before calling this method.
    /// - The caller must ensure that the `TakeCell` is not used after taking the inner value.
    pub unsafe fn take_inner(&mut self) -> T {
        debug_assert!(self.is_valid(), "TakeCell is already taken");
        // SAFETY: We ensure that `value` is valid and not null.
        let inner = unsafe { ptr::read(self.value) };
        self.value = ptr::null_mut(); // Mark as taken
        inner
    }
    pub fn is_valid(&self) -> bool {
        !self.value.is_null()
    }
}

impl<T> Deref for TakeCell<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        debug_assert!(self.is_valid(), "TakeCell is already taken");
        // SAFETY: We ensure that `value` is valid and not null.
        unsafe { &*self.value }
    }
}

impl<T> DerefMut for TakeCell<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        debug_assert!(self.is_valid(), "TakeCell is already taken");
        // SAFETY: We ensure that `value` is valid and not null.
        unsafe { &mut *self.value }
    }
}
```
实现相当简单，我就不过多赘述了。当然，操作指针可能还显得有点危险，我们完全可以实现一个使用 `Option` 与引用的版本。
### 实现 TakeRef
```Rust
pub struct TakeRef<'a, T> {
    value: Option<&'a mut T>,
}

impl<'a, T> TakeRef<'a, T> {
    fn new(value: &'a mut T) -> Self {
        TakeRef { value: Some(value) }
    }

    /// # Safety
    /// The caller must ensure the `TakeRef` will never be used.
    unsafe fn new_none() -> Self {
        TakeRef { value: None }
    }

    /// # Safety
    /// - The caller must ensure that `TakeRef` is not taken before calling this method.
    /// - The caller must ensure that the `TakeRef` is not used after taking the inner value.
    pub unsafe fn take_inner(&mut self) -> T {
        debug_assert!(self.is_valid(), "TakeRef is already taken");
        // SAFETY: We ensure that `value` is some.
        let inner = unsafe { ptr::read(self.value.take().unwrap_unchecked()) };
        inner
    }

    pub fn is_valid(&self) -> bool {
        self.value.is_some()
    }
}

impl<'a, T> Deref for TakeRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        debug_assert!(self.is_valid(), "TakeRef is already taken");
        // SAFETY: We ensure that `value` is some.
        unsafe { self.value.as_deref().unwrap_unchecked() }
    }
}

impl<'a, T> DerefMut for TakeRef<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        debug_assert!(self.is_valid(), "TakeRef is already taken");
        // SAFETY: We ensure that `value` is some.
        unsafe { self.value.as_deref_mut().unwrap_unchecked() }
    }
}
```
这里的 new 可以被标记为 safe 方法，因为 Rust 的引用总是有效的，存入一个有效的引用，后续使用的时候总是安全的。
在初始化 DropGuard 的时候，我们需要一个占位的 TakeRef，它永远不会被使用，所以我在这里还写了一个 unsafe 的 `new_none` 方法。
在两种实现里我都加入了许多 `debug_assert`，用于保证这个结构体的实现没有出错。其实，如果你想要一个完全安全的版本，可以把所有的 `debug_assert` 都改为 `assert`，再把所有的 unsafe 标记移除掉就行了。但我觉得这样性能太差了，适当地用一点 unsafe，可以在符合 Rust 的安全模型的情况下，移除掉这些 `assert`。

### 实现 retain_mut_value
在实现了 TakeRef / TakeCell 之后，我们就可以着手实现方法本身了，使用两种结构体的实现差不多，在此仅介绍使用 TakeRef 的实现。
```Rust
fn retain_mut_value_another<F>(&mut self, mut f: F)
where
	F: FnMut(&mut TakeRef<T>),
{
	struct BackshiftOnDrop<'a, T> {
		v: &'a mut Vec<T>,
		processed_len: usize,
		deleted_cnt: usize,
		original_len: usize,
		take_ref: TakeRef<'a, T>,
		processed: bool,
	}

	impl<T> Drop for BackshiftOnDrop<'_, T> {
		fn drop(&mut self) {
			if !self.processed && !self.take_ref.is_valid() {
				self.deleted_cnt += 1;
				self.processed_len += 1;
			}
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

	let original_len = self.len();

	if original_len == 0 {
		// Empty case: explicit return allows better optimization, vs letting compiler infer it
		return;
	}

	// Avoid double drop if the drop guard is not executed,
	// since we may make some holes during the process.
	unsafe { self.set_len(0) };

	let mut g = BackshiftOnDrop {
		v: self,
		processed_len: 0,
		deleted_cnt: 0,
		original_len,
		// SAFETY: `original_len` is always greater than 0, so the loop will always run at least once.
		// Therefore, this `none` will definitely be replaced.
		take_ref: unsafe { TakeRef::new_none() },
		processed: false,
	};

	fn process_loop<F, T, const DELETED: bool>(
		original_len: usize,
		f: &mut F,
		g: &mut BackshiftOnDrop<'_, T>,
	) where
		F: FnMut(&mut TakeRef<T>),
	{
		while g.processed_len != original_len {
			// SAFETY: Unchecked element must be valid.
			let cur = unsafe { &mut *g.v.as_mut_ptr().add(g.processed_len) };
			let take_ref = TakeRef::new(cur);
			// Reset processed flag for each element
			g.processed = false;
			// Update the TakeRef
			g.take_ref = take_ref;
			f(&mut g.take_ref);
			if !g.take_ref.is_valid() {
				// advance the counter
				g.processed_len += 1;
				g.deleted_cnt += 1;
				// Mark as processed
				g.processed = true;
				if DELETED {
					continue;
				} else {
					break;
				}
			}
			if DELETED {
				// SAFETY: `deleted_cnt` > 0, so the hole slot must not overlap with current element.
				// We use write for move, and never touch this element again.
				unsafe {
					let hole_slot = g.v.as_mut_ptr().add(g.processed_len - g.deleted_cnt);
					ptr::copy_nonoverlapping(
						g.take_ref.value.as_deref().unwrap_unchecked(),
						hole_slot,
						1,
					);
				}
			}
			// Advance the counter and mark as processed
			g.processed_len += 1;
			g.processed = true;
		}
	}

	// Stage 1: Nothing was deleted.
	process_loop::<F, T, false>(original_len, &mut f, &mut g);

	// Stage 2: Some elements were deleted.
	process_loop::<F, T, true>(original_len, &mut f, &mut g);

	// All item are processed. This can be optimized to `set_len` by LLVM.
	drop(g);
}
```
相比标准库，我们这里要改动的东西不少：
1. 如果闭包在执行的时候 panic 了，那么我们需要移动元素，填补 Vec 里的空洞，这里的情况有些不同，我们无法直接确定要填补的空洞，因为 TakeRef 此时可能被 `take_inner` 了，也可能没有。如果里面的值已经被取走了，那此时 Vec 里出现了一个空洞，应该把它填上；如果里面的值还没有被取走，也就是说，可能维持原样，或者被修改了，此时就不能动 Vec 里的值。因此，我们需要在 drop guard 里存储当前的 TakeRef，用于在 `drop` 方法中进行判断。我们还需要在 drop guard 里存储一个布尔值，在这里是 `processed`，用于判断当前的 TakeRef 是否被妥善处理了。只有当 `processed` 为 `false`，且 TakeRef 里的值已经被取走了，才需要在 drop 方法里额外填补一个空洞，否则无需处理。
2. 由于无论是否 panic，TakeRef 都会记录下来当前的值是否已经被取走，所以我们无需提前增加 `processed_len` 和 `deleted_cnt`，维持标准库的写法即可。
3. 你可能会注意到，在第 50 行，初始化 `g` 的时候，使用了一个 `new_none`，这个 TakeRef 有没有可能被实际使用呢？绝无可能。当 Vec 的长度为 0 时，我们早就在第 43 行显式返回了。所以如果能继续往下走，意味着至少需要进行一次 `process_loop`，只要进入了循环，这个存储着 `None` 的 TakeRef 立刻会被换成一个存储着有效引用的 TakeRef。

## 结束了
大概就是这些内容，全套源码和测试在我的 [GitHub 仓库](https://github.com/lhz07/my_rust) 中。使用 `cargo +nightly miri test` 进行测试，避免出现 UB 或者其他的小问题。

## 其他
### 关于 panic safety
Rust 的 panic safety 一般对应于 C++ 的 Basic exception safety，也有的会对应于 Strong exception safety，比如 Vec 的 `insert` 方法。但不同的是，C++ 很依赖异常来处理错误，除了 Google 的代码规范不使用异常以外，大部分 C++ 代码都会使用异常。而 Rust 的 panic 主要用于不可恢复的错误，大部分错误处理其实是由 Result 和 Option 来完成的。
那为什么还要实现 panic safety？
一方面是为了保证 safe Rust 的安全性，另一方面则是，你不知道别人会怎么处理错误，有的人觉得 panic 虽然慢一点，但是也能接受，标准库需要照顾到各种需求，所以我们也顺便实现一下。
举一个使用 panic 处理错误的例子：很多时候会用到线程来并发地执行任务，如果不需要频繁地、快速地从线程中恢复错误，其实 panic 是一个可接受的选项，此时 panic safety 就很重要了，如果保证了 panic safety，那即使线程 panic 了，共享的数据结构也还能用，否则一旦线程 panic，整个数据结构就像被污染了一样，完全不能用了。
还有一点则是，有的库内部认为这个错误不可恢复，所以使用了 panic，但是可能在实际使用中，需要恢复这个错误，这个时候会用 `catch_unwind` 来处理，此时 panic safety 也很重要。
