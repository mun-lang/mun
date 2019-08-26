use crate::prelude::*;

use std::any::Any;
use std::fmt::{self, Debug};

use libloading::{Library, Symbol};

#[derive(Debug)]
pub struct MethodInfo {
    name: String,
    privacy: Privacy,
    pub args: &'static [&'static TypeInfo],
    pub returns: Option<&'static TypeInfo>,
    factory: &'static dyn MethodFactory,
}

impl MethodInfo {
    pub fn new(
        name: &str,
        privacy: Privacy,
        args: &'static [&'static TypeInfo],
        returns: Option<&'static TypeInfo>,
        factory: &'static dyn MethodFactory,
    ) -> MethodInfo {
        MethodInfo {
            name: name.to_string(),
            privacy,
            args,
            returns,
            factory,
        }
    }

    pub fn factory(&self) -> &dyn MethodFactory {
        self.factory
    }
}

impl MemberInfo for MethodInfo {
    fn name(&self) -> &str {
        &self.name
    }

    fn privacy(&self) -> Privacy {
        self.privacy
    }

    fn is_private(&self) -> bool {
        self.privacy == Privacy::Private
    }

    fn is_public(&self) -> bool {
        self.privacy == Privacy::Public
    }
}

pub trait Invokable {
    fn invoke(&self, args: &[&dyn Any]) -> Result<Box<dyn Any>, &'static str>;
}

pub trait MethodFactory: Debug + Sync + Send {
    fn of<'a>(
        &self,
        library: &'a Library,
        info: &'a MethodInfo,
    ) -> libloading::Result<Box<dyn Invokable + 'a>>;
}

#[derive(Debug)]
pub struct EmptyMethodFactory;

impl MethodFactory for EmptyMethodFactory {
    fn of<'a>(
        &self,
        library: &'a Library,
        info: &'a MethodInfo,
    ) -> libloading::Result<Box<dyn Invokable + 'a>> {
        let symbol = unsafe { library.get(info.name().as_ref()) }?;

        Ok(Box::new(EmptyMethod::new(symbol, info)))
    }
}

pub struct MethodArg2RetFactory<A: Reflectable + Clone, B: Reflectable + Clone, Output: Reflectable>(
    std::marker::PhantomData<A>,
    std::marker::PhantomData<B>,
    std::marker::PhantomData<Output>,
);

impl<A: Reflectable + Clone, B: Reflectable + Clone, Output: Reflectable> Debug
    for MethodArg2RetFactory<A, B, Output>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MethodArg2RetFactory")
    }
}

unsafe impl<A: Reflectable + Clone, B: Reflectable + Clone, Output: Reflectable> Send
    for MethodArg2RetFactory<A, B, Output>
{
}

unsafe impl<A: Reflectable + Clone, B: Reflectable + Clone, Output: Reflectable> Sync
    for MethodArg2RetFactory<A, B, Output>
{
}

impl<A: Reflectable + Clone, B: Reflectable + Clone, Output: Reflectable>
    MethodArg2RetFactory<A, B, Output>
{
    pub fn new() -> Self {
        Self(
            std::marker::PhantomData,
            std::marker::PhantomData,
            std::marker::PhantomData,
        )
    }
}

impl<A: Reflectable + Clone, B: Reflectable + Clone, Output: Reflectable> MethodFactory
    for MethodArg2RetFactory<A, B, Output>
{
    fn of<'a>(
        &self,
        library: &'a Library,
        info: &'a MethodInfo,
    ) -> libloading::Result<Box<dyn Invokable + 'a>> {
        let symbol = unsafe { library.get(info.name().as_ref()) }?;

        Ok(Box::new(Method::<A, B, Output>::new(symbol, info)))
    }
}

struct EmptyMethod<'lib> {
    symbol: Symbol<'lib, fn()>,
    info: &'lib MethodInfo,
}

impl<'lib> EmptyMethod<'lib> {
    pub fn new(symbol: Symbol<'lib, fn()>, info: &'lib MethodInfo) -> Self {
        Self { symbol, info }
    }
}

impl<'lib> Invokable for EmptyMethod<'lib> {
    fn invoke(&self, args: &[&dyn Any]) -> Result<Box<dyn Any>, &'static str> {
        if self.info.args.len() != args.len() {
            return Err("Invalid number of arguments.");
        }

        if self.info.returns.is_some() {
            return Err("Invalid return type");
        }

        (self.symbol)();

        Ok(Box::new(()))
    }
}

struct Method<'lib, A: Reflectable + Clone, B: Reflectable + Clone, Output: Reflectable> {
    symbol: Symbol<'lib, fn(A, B) -> Output>,
    info: &'lib MethodInfo,
}

impl<'lib, A: Reflectable + Clone, B: Reflectable + Clone, Output: Reflectable>
    Method<'lib, A, B, Output>
{
    pub fn new(symbol: Symbol<'lib, fn(A, B) -> Output>, info: &'lib MethodInfo) -> Self {
        Self { symbol, info }
    }
}

// macro this
impl<'lib, A: Reflectable + Clone, B: Reflectable + Clone, Output: Reflectable> Invokable
    for Method<'lib, A, B, Output>
{
    fn invoke(&self, args: &[&dyn Any]) -> Result<Box<dyn Any>, &'static str> {
        if self.info.args.len() != args.len() {
            return Err("Invalid number of arguments.");
        }

        let a: &A = args[0].downcast_ref().ok_or("Invalid A argument type.")?;
        let b: &B = args[1].downcast_ref().ok_or("Invalid B argument type.")?;

        let result = (self.symbol)(a.clone(), b.clone());

        if let Some(return_type) = self.info.returns {
            if result.reflect().uuid != return_type.uuid {
                return Err("Invalid return type");
            }
        }

        Ok(Box::new(result))
    }
}
