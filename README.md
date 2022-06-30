# boil

boilerplate-free [newtype](https://doc.rust-lang.org/rust-by-example/generics/new_types.html) macro.  `boil` makes the newtype idiom ergonomic at scale.

Declare a newtype wrapping a single field:

```rust
use boil::boil;

#[boil]
struct Wrapped(u8);
```

The `boil` crate is a couple of macros with zero dependencies, and is free for noncommercial and "small commercial" use.

# Features

aka "why not declare my own newtype?"  See the motivation section, but a brief answer is:
* Auto implementations for `Deref`, `DerefMut`, `Borrow`, `BorrowMut`, `AsRef`, `AsMut`, `From`, `Into`
* projections to convert through wrapping types like `Box`, `Pin`, `Arc`, `Rc`, `Result`
* Memory-layout compatible with the field
* Field (in)visibility
* Supports all Rust language features, such as generics, `where` clauses, paths, etc.
* Supports wrapping unsized fields (see `boil_unsized`)
* Derive `Display` and `Error` from the field
* Extensive test coverage.

For more information on these items, see the extensive documentation and its examples.

# Motivation

Suppose you are writing cross-platform code for macos and windows:

```rust
#[cfg(target_os = "windows")]
mod imp {
    pub struct Widget;
    impl Widget {
        fn activate() {/* windows implementation here */}
        pub fn windows_only_fn() { }
    }
}
#[cfg(target_os = "macos")]
mod imp {
    pub struct Widget;
    impl Widget {
        fn activate() {/* macos implementation here */}
        pub fn macos_only_fn() { }

    }
}

use imp::Widget as Widget;
```

This is all fine and good, but:
1.  Where do you put the documentation for `activate`?
2.  How do you check some allegedly cross-platform function only uses `activate` and not `macos_only_fn`?

Okay, so you use traits:

```rust
trait Widget {
    fn activate(&self);
}

#[cfg(target_os = "windows")]
mod imp {
    pub struct Widget;
    impl Widget {
        pub fn windows_only_fn(&self) { }
    }
    impl super::Widget for Widget {
        fn activate(&self) { /*windows-specific implementation*/ }
    }
}
#[cfg(target_os = "macos")]
mod imp {
    pub struct Widget;
    impl Widget {
        pub fn mac_only_fn(&self) { }
    }
    impl super::Widget for Widget {
        fn activate(&self) { /* mac-specific implementation */ }
    }
}

fn function_with_platform_awareness(widget: &imp::Widget) {
    //call whatever we want as guarded by cfg
    #[cfg(target_os="windows")] widget.windows_only_fn();
    #[cfg(target_os="macos")] widget.mac_only_fn();
}
fn function_cross_platform<W: Widget>(w: &W) {
    //check at compile-time we only use cross-platform methods on Widget
    w.activate()
}
# fn main() { }
```

Sure, but:

1.  `W:Widget` is very silly; there's only one type of widget on our platform!
2.  But unless we take some complicated steps to close our trait, someone might write another implementation. Beyond making
    a nice API, this costs us optimization.
3.  What if we have APIs that can't appear in traits, like [`async`](https://rust-lang.github.io/async-book/07_workarounds/05_async_in_traits.html), [`const`](https://varkor.github.io/blog/2019/01/11/const-types-traits-and-implementations-in-Rust.html), or similar?

## Newtypes
For this we have the [newtype](https://doc.rust-lang.org/rust-by-example/generics/new_types.html) pattern.
We can create a concrete newtype to house our shared behavior:

```rust
#[cfg(target_os = "windows")]
mod imp {
    pub struct Widget;
    impl Widget {
        pub fn windows_only_fn(&self) { }
        fn activate(&self) { /*windows-specific implementation*/ }
    }
}
#[cfg(target_os = "macos")]
mod imp {
    pub struct Widget;
    impl Widget {
        pub fn mac_only_fn(&self) { }
        pub fn activate(&self) { /* mac-specific implementation */ }
    }
}
struct Widget(imp::Widget);
impl Widget {
    ///Cross-platform documentation for activate
    fn activate(&self) { self.0.activate() }
}

fn function_with_platform_awareness(w: &imp::Widget) {
    //call whatever we want as guarded by cfg
    #[cfg(target_os="windows")] w.windows_only_fn();
    #[cfg(target_os="macos")] w.mac_only_fn();
}
fn function_cross_platform(w: &Widget) {
    //check at compile-time we only use the cross-platform methods on Widget
    w.activate()
}
```

This is getting complicated fast.

At first it appears the annoying part is having to write thunks like `activate` an extra time on the newtype.  But if we want
documentation, and we want to transform the arguments (suppose they're *also* platform-wrapped types we want to 'lower' when
passing to platform-specfic `imp::Widget::activate()`), Rust is actually a fairly good language to express these problems.
(I might try to improve on this in the future.)

The more immediate problem is everything *else* that is hard about it.  For example:

1.  How can I cast easily between `Widget` and `imp::Widget` as I move between platform-agnostic and platform-specific code?
2.  How can I get `AsRef`, `std::borrow::Borrow`, `std::ops::Deref`,`From`,`Into`, etc?
3.  If I have something like `Box<imp::Widget>` how do I get `Box<Widget>`?  Do I need some
    runtime heap behavior to do the conversion?  What about `std::sync::Arc`, `std::rc::Rc`, `std::pin::Pin`, etc?
4.  If I have `imp::Widget: Display`, how do I derive `Display` on my `Widget`?
5.  What exactly is the memory layout of `Widget`?  Is it the same as `imp::Widget`?

Good Rust developers can quickly bang out the boilerplate to solve these problems.  In fact, this can be a useful exercise on a handful
of newtypes where `AsRef` vs `std::ops::Deref`, etc., can be carefully considered design decisions.  However, you will wind up with
pages and pages of subtly-different boilerplate for every newtype you declare.

`boil` is designed to solve this problem, giving you a concise, standardized way to declare newtypes consistently
and with broad feature support.