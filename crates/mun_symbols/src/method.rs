use crate::prelude::*;

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

    pub fn load<'a>(&'a self, library: &'a Library) -> libloading::Result<Box<dyn Invokable + 'a>> {
        self.factory.of(library, self)
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
    fn invoke(&self, args: &[&dyn Reflectable]) -> Result<Box<dyn Reflectable>, String>;
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

pub struct MethodArg2RetFactory<A: Reflection + Clone, B: Reflection + Clone, Output: Reflection>(
    std::marker::PhantomData<A>,
    std::marker::PhantomData<B>,
    std::marker::PhantomData<Output>,
);

impl<A: Reflection + Clone, B: Reflection + Clone, Output: Reflection> Debug
    for MethodArg2RetFactory<A, B, Output>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MethodArg2RetFactory")
    }
}

unsafe impl<A: Reflection + Clone, B: Reflection + Clone, Output: Reflection> Send
    for MethodArg2RetFactory<A, B, Output>
{
}

unsafe impl<A: Reflection + Clone, B: Reflection + Clone, Output: Reflection> Sync
    for MethodArg2RetFactory<A, B, Output>
{
}

impl<A: Reflection + Clone, B: Reflection + Clone, Output: Reflection>
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

impl<A: Reflection + Clone, B: Reflection + Clone, Output: Reflection> MethodFactory
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
    fn invoke(&self, args: &[&dyn Reflectable]) -> Result<Box<dyn Reflectable>, String> {
        if 0 != args.len() {
            return Err(format!(
                "Invalid number of arguments. Expected: {}. Found: {}.",
                0,
                args.len()
            ));
        }

        if let Some(return_type) = self.info.returns {
            return Err(format!(
                "Invalid return type. Expected: None. Found: Some({})",
                return_type.name
            ));
        }

        (self.symbol)();

        Ok(Box::new(()))
    }
}

struct Method<'lib, A: Reflection + Clone, B: Reflection + Clone, Output: Reflection> {
    symbol: Symbol<'lib, fn(A, B) -> Output>,
    info: &'lib MethodInfo,
}

impl<'lib, A: Reflection + Clone, B: Reflection + Clone, Output: Reflection>
    Method<'lib, A, B, Output>
{
    pub fn new(symbol: Symbol<'lib, fn(A, B) -> Output>, info: &'lib MethodInfo) -> Self {
        Self { symbol, info }
    }
}

// macro this
impl<'lib, A: Reflection + Clone, B: Reflection + Clone, Output: Reflection> Invokable
    for Method<'lib, A, B, Output>
{
    fn invoke(&self, args: &[&dyn Reflectable]) -> Result<Box<dyn Reflectable>, String> {
        if 2 != args.len() {
            return Err(format!(
                "Invalid number of arguments. Expected: {}. Found: {}.",
                2,
                args.len()
            ));
        }

        let a: &A = args[0].downcast_ref().ok_or(format!(
            "Invalid argument type at index {}. Expected: {}. Found: {}.",
            0,
            self.info.args[0].name,
            args[0].reflect().name
        ))?;
        let b: &B = args[1].downcast_ref().ok_or(format!(
            "Invalid argument type at index {}. Expected: {}. Found: {}.",
            1,
            self.info.args[1].name,
            args[1].reflect().name
        ))?;

        let result = (self.symbol)(a.clone(), b.clone());

        if let Some(return_type) = self.info.returns {
            let result_type = result.reflect();
            if result_type.uuid != return_type.uuid {
                return Err(format!(
                    "Invalid return type. Expected: Some({}). Found: Some({})",
                    return_type.name, result_type.name,
                ));
            }
        }

        Ok(Box::new(result))
    }
}
