/*! boilerplate-free newtype macro.  boil lets you declare a newtype wrapping a single field:

```
use boil::boil;

#[boil]
struct Wrapped(u8);
```

boil is a single macro with zero dependencies.

# Motivation

Suppose you are writing cross-platform code for macos and windows:

```
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
#[cfg(target_os = "linux")]
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

```
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
#[cfg(target_os = "linux")]
mod imp {
    pub struct Widget;
    impl Widget {
        pub fn mac_only_fn(&self) { }
    }
    impl super::Widget for Widget {
        fn activate(&self) { /* linux-specific implementation */ }
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

# Newtypes
For this we have the [newtype](https://doc.rust-lang.org/rust-by-example/generics/new_types.html) pattern.
We can create a concrete newtype to house our shared behavior:

```
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
#[cfg(target_os = "linux")]
mod imp {
    pub struct Widget;
    impl Widget {
        pub fn mac_only_fn(&self) { }
        pub fn activate(&self) { /* linux-specific implementation */ }
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
2.  How can I get [AsRef], [std::borrow::Borrow], [std::ops::Deref],[From],[Into], etc?
3.  If I have something like `Box<imp::Widget>` how do I get `Box<Widget>`?  Do I need some
    runtime heap behavior to do the conversion?  What about [std::sync::Arc], [std::rc::Rc], [std::pin::Pin], etc?
4.  If I have `imp::Widget: Display`, how do I derive `Display` on my `Widget`?
5.  What exactly is the memory layout of `Widget`?  Is it the same as `imp::Widget`?

Good Rust developers can quickly bang out the boilerplate to solve these problems.  In fact, this can be a useful exercise on a handful
of newtypes where [AsRef] vs [std::ops::Deref], etc., can be carefully considered design decisions.  However, you will wind up with
pages and pages of subtly-different boilerplate for every newtype you declare.

[boil], in contrast, is designed for cases where we just want a newtype, thanks.  [boil] implements everything you might want into a
zero-cost abstraction so you can return to what you were doing instead of writing boilerplate.

*/

/**
Declares a wrapper type.

The payload must be sized.  For an unsized payload, see [boil_unsized].

```
use boil::boil;
#[boil]
struct Foo(u8);
```

# Access

## Field

You can expose the underlying field if desired.

```
# use boil::boil;
#[boil]
struct Foo(pub u8);
let f = Foo(2);
println!("{}",f.0);
```

**Note:** If the field is less accessible than the enclosing type, use the argument `scoped`:
```
# use boil::boil;
struct Private;
#[boil(scoped)]
pub struct Foo(Private);
```

Otherwise, you will get this error

```text
error[E0446]: private type `Private` in public interface
 --> src/lib.rs:180:1
  |
5 | struct Private;
  | --------------- `Private` declared as private
6 | #[boil]
  | ^^^^^^^ can't leak private type
  |
  = note: this error originates in the attribute macro `boil` (in Nightly builds, run with -Z macro-backtrace for more info)
```

Some behavior, such as [std::ops::Deref], cannot be implemented on types with fields less visible than their wrappers.  `scoped` disables these behaviors.


## Deref, AsRef, ec.

Types declared with [boil] implement [std::ops::Deref] and [std::ops::DerefMut] (*when non-scoped*), [std::borrow::Borrow], [std::borrow::BorrowMut], [AsRef], and [AsMut].

```
# use boil::boil;
# #[boil] struct Foo(u8);
let f = Foo(2);
let g: &u8 = &f;
```

## From, Into

Boil types implement [From] and [Into].

```
# use boil::boil;
# #[boil] struct Foo(u8);
let f: Foo = 2.into();
let g: u8 = f.into();
```

## Projection

Boil types support projection via From/Into for [Box] and [std::pin::Pin].

```
# use boil::boil;
# #[boil] struct Foo(u8);
let i: Box<u8> = Box::new(Foo(1)).into();
let o: Box<Foo> = Box::new(2).into();
```

You can also project through [std::sync::Arc] and [std::rc::Rc].  Due to the orphan rule, [From]/[Into] is not supported here
but there are functions on the boiled type itself:
```
# use boil::boil;
# #[boil] struct Foo(u8);
use std::sync::Arc;
let i: Arc<u8> = Arc::new(Foo(1)).to_arc();
let o: Arc<Foo> = Foo::from_arc(Arc::new(2));
```

**Warning**: These conversions have the same visibility as the inner field, which is private by default.

## Result

In Rust, `Result` implements `From/Into` automatically with its `Error` variant, and so operators like `?` can convert
between boiled and unboiled errors.

```
# use boil::{Display,Error,boil};
use std::sync::mpsc::RecvTimeoutError;
#[boil]
#[derive(Debug, Display,Error)]
struct MyError(RecvTimeoutError);

fn return_underlying() -> Result<(), RecvTimeoutError> {
    Err(MyError(RecvTimeoutError::Disconnected))?
}
fn return_wrapped() -> Result<(), MyError> {
    Err(RecvTimeoutError::Disconnected)?
}
```

However, what about converting the `Ok` variant?  Unfortunately, the equivalent code doesn't work due to limitations in Rust.

Instead, we provide the `from_result` and `to_result` functions.

```
# use boil::boil;
use std::sync::mpsc::RecvTimeoutError;

#[boil]
struct MyOk(u8);

fn return_underlying(src: Result<MyOk,RecvTimeoutError>) -> Result<u8, RecvTimeoutError> {
    MyOk::into_result(src)
}
fn return_wrapped(src: Result<u8,RecvTimeoutError>) -> Result<MyOk, RecvTimeoutError> {
    MyOk::from_result(src)
}
```

**Warning**: These conversions have the same visibility as the inner field, which is private by default.

## Memory layout

Boil wrappers have the same memory layout as their underlying types.
```
# use boil::boil;
#[boil] struct Foo(u8);
let v: u8 = 5;
//safe because `Foo` and `u8` have the same memory layout
let u: *const Foo = unsafe { std::mem::transmute(&v) };
```

# Features

`boil` should support all standard Rust syntax to declare types, including generics, associated types, where clauses, paths, and more.
```
use boil::boil;
mod imp {
    use std::marker::PhantomData;
    pub trait AssociatedType { type A; }
    pub struct Example<'a,'b,C,D>(&'a PhantomData<C>, &'b PhantomData<D>);
}

#[boil]
struct Example<'a, 'b, C, D: std::fmt::Debug> (imp::Example<'a, 'b, C, D>)  where 'b: 'a, C: imp::AssociatedType + Sync, D: ;
```
*/
pub use procmacro::boil;

/**
Declares a wrapper type for an unsized payload.  Wrapping unsized types have unique constraints.

As a detail, the implementation currently assumes your particular unsized payload is some `dyn Trait`.  This limitation
may be lifted in the future.

# Motivation

Suppose we have some platform-specific generic type, and we want to erase the generic.

```
use boil::boil;
mod imp {
    //Erase `Erase` from our platform-specific type.
    struct Generic<Erase> { field: std::marker::PhantomData<Erase> }
    //declare a trait
    //the trait object can be our 'erased type'.
    pub trait Erased { }
    impl<Erase> Erased for Generic<Erase> {}
}
//Our wrapper can then hold a pointer to the trait object
#[boil]
struct Wrap<'a>(&'a dyn imp::Erased);
```

Fair enough.  But what if we have `Arc<Generic<Erased>>` and want to convert it to some `Arc<Wrap<'_>>`? For that our Arc needs to contain
an owned value, and one compatible with `Generic<Erased>`.  So `Wrap` must contain, not a pointer to `Generic`, as in this listing.
But to *be* `Generic` *itself*.

So, our payload would have to be `dyn imp::Erased` (that is, without the `&`).  But `dyn` is
[unsized](https://web.mit.edu/rust-lang_v1.25/arch/amd64_ubuntu1404/share/doc/rust/html/book/first-edition/unsized-types.html),
which has many restrictions in Rust.  Sometimes the restrictions of unsized types are undesirable, but here they're inherent to
what we want to do, which is to erase `Generic` (and, consequently, its size).

Therefore, we must embrace the unsized restrictions.  `boil_unsized` implements a wrapper suitable for unsized types.  Due to the
rules of Rust, this works differently than for sized types.  Sometimes the details of these differences will be brought to your attention.

However, `boil_unsized` is at a *high level* the same thing as `boil`, that is, it wraps a payload.  So you might be satisfied with
just using it the same way until you encounter a problem, and that philosophy will get you pretty far.

# Like `boil`

The following traits are implemented like [boil]
* [std::ops::Deref], [std::ops::DerefMut],
* [std::borrow::Borrow], [std::borrow::BorrowMut],
* [AsRef], [AsMut]
* [Box] projections
* [std::pin::Pin] projections
* [std::sync::Arc]/[std::rc::Rc] projections

## Trait assumptions

As a detail, in cases where we are converting to the payload type, `boil_unsized`
assumes the payload is some `dyn Trait`.  For example, [AsRef] could not be implemented as

```compile_fail
impl AsRef <dyn Erased> for Wrap {
    fn as_ref(&self) -> &dyn Erased { todo!() }  //error
}
```

due to

```text
error: `impl` item signature doesn't match `trait` item signature
   --> src/main.rs:19:3
    |
19  | { fn as_ref(&self) -> &dyn Erased { todo!() } }
    |   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ found `fn(&'1 Wrap) -> &'1 (dyn Erased + '1)`
    |
    = note: expected `fn(&'1 Wrap) -> &'1 (dyn Erased + 'static)`
               found `fn(&'1 Wrap) -> &'1 (dyn Erased + '1)`
    = help: the lifetime requirements from the `impl` do not correspond to the requirements in the `trait`
    = help: verify the lifetime relationships in the `trait` and `impl` between the `self` argument, the other inputs and its output
```

The issue here is an incongruity between the lifetime elided for the function (which is anonymous) vs the lifetime
elided for `AsRef` (which is `'static`).  We must instead return `&(dyn Erased + 'static)`, explicitly which we append
to the return type.  See
<https://stackoverflow.com/questions/70717413/why-is-static-lifetime-required-for-references-returned-from-methods-in-an-impl>
for more details.

`boil_unsized` inserts these lifetimes explicitly into appropriate implementations.


# From / Into

Sized types must generally appear behind some kind of reference or pointer.  As a consequence, we can't implement
`From<Payload>` or `To<Payload>` because they involve some concrete `<Payload>`.

Instead, `boil_unsized` generates `From<&Payload>` and `From<&mut Payload>` (and vice versa).

```
use boil::boil_unsized;
struct Imp;
trait Erased {}
impl Erased for Imp {}

#[boil_unsized]
struct Wrap(dyn Erased);
# fn main() {
let i = Imp;
let e: &dyn Erased = &i;
let w: &Wrap = e.into();
# }
```

# Result conversions

Like [boil], `boil_unsized` generates `from_result` and `into_result` functions.  Like [From], these cannot be implemented
on owned types due to the size restrictions, so they are implemented on `&Wrapped` and `&Payload` respectively.

In addition, `from_result_mut` and `into_result_mut` are implemented for the respective conversions of `&mut Wrapped` and `&mut Payload`.






*/
pub use procmacro::boil_unsized;

/**
Derives [Display] for a type declared with [boil].

The payload must implement `Display`.

```
use boil::boil;
#[boil]
#[derive(boil::Display)]
struct Display(u8);
*/
pub use procmacro::Display;

/**
Derives [Error] for a type declared with [boil].

The payload must implement [Error].

use boil::boil;
#[boil]
#[derive(boil::Display,boil::Error)]
struct Display(std::convert::Infallible);
*/
pub use procmacro::Error;

///This example [boil]ed type shows the traits and functions that are implemented by calling [boil].
///
///This struct is not real API, but appears in the documentation as an example.
#[cfg(doc)]
#[boil]
pub struct Example(pub u8);

///This example [boil_unsized] type shows the traits and functions that are implemented by calling [boil_unsized].
///
///This struct is not real API, but appears in the documentation as an example.
#[cfg(doc)]
#[boil_unsized]
pub struct ExampleUnsized(pub dyn Sync);

