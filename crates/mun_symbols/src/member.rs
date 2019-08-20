use crate::prelude::*;

pub trait MemberInfo {
    fn name(&self) -> &str;

    fn privacy(&self) -> Privacy;
    fn is_public(&self) -> bool;
    fn is_private(&self) -> bool;
}
