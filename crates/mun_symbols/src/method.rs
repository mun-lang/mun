use crate::prelude::*;

#[derive(Debug)]
pub struct MethodInfo {
    name: String,
    privacy: Privacy,
    args: &'static [&'static TypeInfo],
    returns: Option<&'static TypeInfo>,
}

impl MethodInfo {
    pub fn new(
        name: &str,
        privacy: Privacy,
        args: &'static [&'static TypeInfo],
        returns: Option<&'static TypeInfo>,
    ) -> MethodInfo {
        MethodInfo {
            name: name.to_string(),
            privacy,
            args,
            returns,
        }
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
