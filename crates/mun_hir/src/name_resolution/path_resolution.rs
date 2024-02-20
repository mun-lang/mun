use std::iter::successors;

use crate::{
    ids::{ItemDefinitionId, ModuleId},
    item_scope::BUILTIN_SCOPE,
    module_tree::LocalModuleId,
    package_defs::PackageDefs,
    DefDatabase, Name, PackageId, Path, PathKind, PerNs, Visibility,
};

/// Indicates whether or not any newly resolved import statements will actually
/// change the outcome of an operation. This is useful to know if more
/// iterations of an algorithm might be required, or whether its hopeless.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReachedFixedPoint {
    Yes,
    No,
}

/// Contains the result of resolving a path. It contains how far the path was
/// able to be resolved as well as the resolved values or types so far.
#[derive(Debug, Clone)]
pub(crate) struct ResolvePathResult {
    pub(crate) resolved_def: PerNs<(ItemDefinitionId, Visibility)>,
    pub(crate) segment_index: Option<usize>,
    pub(crate) reached_fixedpoint: ReachedFixedPoint,
    pub(crate) package: Option<PackageId>,
}

impl ResolvePathResult {
    /// Constructs an empty `ResolvePathResult`
    fn empty(reached_fixedpoint: ReachedFixedPoint) -> ResolvePathResult {
        ResolvePathResult::with(PerNs::none(), reached_fixedpoint, None, None)
    }

    /// Constructs a new instance of `ResolvePathResult`
    fn with(
        resolved_def: PerNs<(ItemDefinitionId, Visibility)>,
        reached_fixedpoint: ReachedFixedPoint,
        segment_index: Option<usize>,
        package: Option<PackageId>,
    ) -> ResolvePathResult {
        ResolvePathResult {
            resolved_def,
            segment_index,
            reached_fixedpoint,
            package,
        }
    }
}

impl PackageDefs {
    /// Resolves the specified `path` from within the specified `module`. Also
    /// optionally returns which part of the path was resolved, if this is
    /// not `None` it means the path didn't resolve completely yet.
    pub(crate) fn resolve_path_in_module(
        &self,
        db: &dyn DefDatabase,
        module: LocalModuleId,
        path: &Path,
    ) -> (PerNs<(ItemDefinitionId, Visibility)>, Option<usize>) {
        let res = self.resolve_path_with_fixedpoint(db, module, path);
        (res.resolved_def, res.segment_index)
    }

    /// Resolves the specified `name` from within the specified `module`
    fn resolve_name_in_module(
        &self,
        _db: &dyn DefDatabase,
        module: LocalModuleId,
        name: &Name,
    ) -> PerNs<(ItemDefinitionId, Visibility)> {
        self[module]
            .get(name)
            .or(BUILTIN_SCOPE.get(name).copied().unwrap_or_else(PerNs::none))
    }

    /// Resolves the specified `path` from within the specified `module`. Also
    /// returns whether or not additions to the `PackageDef` would change
    /// the result or whether a fixed point has been reached. This is useful
    /// when resolving all imports.
    pub(crate) fn resolve_path_with_fixedpoint(
        &self,
        db: &dyn DefDatabase,
        original_module: LocalModuleId,
        path: &Path,
    ) -> ResolvePathResult {
        let mut segments = path.segments.iter().enumerate();
        let mut curr_per_ns: PerNs<(ItemDefinitionId, Visibility)> = match path.kind {
            PathKind::Plain => {
                let (_, segment) = match segments.next() {
                    Some((idx, segment)) => (idx, segment),
                    None => return ResolvePathResult::empty(ReachedFixedPoint::Yes),
                };
                self.resolve_name_in_module(db, original_module, segment)
            }
            PathKind::Super(lvl) => {
                let m = successors(Some(original_module), |m| self.module_tree[*m].parent)
                    .nth(lvl as usize);
                if let Some(local_id) = m {
                    PerNs::types((
                        ModuleId {
                            package: self.module_tree.package,
                            local_id,
                        }
                        .into(),
                        Visibility::Public,
                    ))
                } else {
                    return ResolvePathResult::empty(ReachedFixedPoint::Yes);
                }
            }
            PathKind::Package => PerNs::types((
                ModuleId {
                    package: self.module_tree.package,
                    local_id: self.module_tree.root,
                }
                .into(),
                Visibility::Public,
            )),
        };

        for (i, segment) in segments {
            let (curr, vis) = match curr_per_ns.take_types() {
                Some(r) => r,
                None => {
                    return ResolvePathResult::empty(ReachedFixedPoint::No);
                }
            };

            curr_per_ns = match curr {
                ItemDefinitionId::ModuleId(module) => self[module.local_id].get(segment),
                // TODO: Enum variants
                s => {
                    return ResolvePathResult::with(
                        PerNs::types((s, vis)),
                        ReachedFixedPoint::Yes,
                        Some(i),
                        Some(self.module_tree.package),
                    );
                }
            };
        }

        ResolvePathResult::with(
            curr_per_ns,
            ReachedFixedPoint::Yes,
            None,
            Some(self.module_tree.package),
        )
    }
}
