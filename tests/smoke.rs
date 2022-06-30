use boil::boil;
use std::fmt::Debug;
use crate::imp::AssociatedType;

mod imp {
    use std::marker::PhantomData;

    #[derive(Debug,Clone)]
    pub struct Foo;

    pub struct FooG<G>(PhantomData<G>);

    pub struct FooLifetime<'a>(&'a PhantomData<u8>);
    pub struct FooComplex<'a, 'b, C, D: ?Sized>(&'a PhantomData<C>, &'b PhantomData<D>);

    pub trait AssociatedType {
        type A;
    }
}

#[derive(Debug)]
#[boil]
#[derive(Clone)]
struct Foo(imp::Foo);

#[boil]
struct FooG<G>(imp::FooG<G>);

#[boil]
struct FooLifetime<'a>(imp::FooLifetime<'a>);

#[boil]
struct FooCondition<G: Sync>(imp::FooG<G>);

#[boil]
struct FooTwoCondition<G: Sync + Send>(imp::FooG<G>);

#[boil]
struct FooPath<G: std::fmt::Debug>(imp::FooG<G>);

trait Demo {}
#[boil]
struct FooComplex<'a, 'b: 'a, C: std::fmt::Debug + Sync, D: Demo>(imp::FooComplex<'a, 'b, C, D>);
#[allow(unused)]
#[boil]
struct FooAT<'a, 'b: 'a, C: imp::AssociatedType<A=D>,D: imp::AssociatedType<A=C>>(imp::FooComplex<'a, 'b, C, D>);

#[allow(unused)]
#[boil]
struct FooNestAT<'a, 'b: 'a, C: imp::AssociatedType<A=dyn imp::AssociatedType<A=D>>,D: imp::AssociatedType<A=C>>(imp::FooComplex<'a, 'b, C, D>);

#[boil]
struct FooDyn<'a, 'b: 'a, C>(imp::FooComplex<'a, 'b, C, dyn AssociatedType<A=C>>);

#[boil]
struct FooWhere<'a, 'b, C, D> (imp::FooComplex<'a, 'b, C, D>)  where 'b: 'a, C: AssociatedType + Sync, D: 'static;

#[boil]
struct FooOrphanRule(std::sync::Mutex<u8>);

#[derive(boil::Display)]
#[boil] struct Display(u8);

#[allow(unused)]
fn deref() {
    let foo = Foo(imp::Foo);
    let f: &imp::Foo = &foo;
}
#[allow(unused)]
fn display() {
    let d = Display(0);
    println!("{}",d);
}
#[allow(unused)]
fn ex() {
    let _a: &dyn AsRef<imp::Foo> = &Foo(imp::Foo);
}

#[allow(unused)]
fn result() {
    #[derive(Clone)]
    #[boil] struct U8(u8);
    #[boil]
    #[derive(boil::Error,Debug,boil::Display,Clone)]
    struct MError(std::convert::Infallible);
    let source_result: Result<u8,std::convert::Infallible> = Ok(2);
    let _r: Result<U8,std::convert::Infallible> = U8::from_result(source_result);
    let _r: Result<U8, MError> = U8::from_result(source_result);

    //reverse project

    let dest_result: Result<U8, MError> = Ok(2.into());
    let _r: Result<u8,std::convert::Infallible> = U8::into_result(dest_result.clone());
    let _r: Result<u8,MError> = U8::into_result(dest_result);

    let dest_result_2: Result<U8,std::convert::Infallible> = Ok(2.into());
    let _r: Result<u8,std::convert::Infallible> = U8::into_result(dest_result_2.clone());
    let _r: Result<u8,MError> = U8::into_result(dest_result_2);
}

#[boil]
pub struct DynField<'a>(pub(crate) &'a dyn imp::AssociatedType<A=u8>);

// #[boil]
pub struct Unsized(pub(crate) dyn imp::AssociatedType<A=u8>);

pub(crate) trait CratePrivateTrait {}
pub(crate) struct CratePrivateStruct;
#[boil(scoped)]
pub struct CratePrivateWrap(pub(crate) CratePrivateStruct);
#[boil::boil_unsized(scoped)]
pub struct TraitPrivatewrap(pub(crate) dyn CratePrivateTrait);