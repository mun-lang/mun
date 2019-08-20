use crate::prelude::*;

#[derive(Debug)]
pub struct MethodInfo {
    name: String,
    privacy: Privacy,
    types: Vec<&'static TypeInfo>,
    returns: bool,
}

impl MethodInfo {
    pub fn new(
        name: &str,
        privacy: Privacy,
        mut args: Vec<&'static TypeInfo>,
        result: Option<&'static TypeInfo>,
    ) -> MethodInfo {
        let returns = if let Some(result) = result {
            args.push(result);
            true
        } else {
            false
        };

        MethodInfo {
            name: name.to_string(),
            privacy,
            types: args,
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
