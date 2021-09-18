//! This module is responsible for parsing collecting all the packages in a project

use std::collections::{HashSet, VecDeque};

use cargo_metadata::{
    DependencyKind, Metadata, MetadataCommand, NodeDep, Package, PackageId, Resolve,
};
use thiserror::Error;

// TODOs:
// - passthrough the frozen etc options
// - passthrough build / dev dep options

#[derive(Error, Debug)]
pub enum PackageLoaderError {
    #[error(transparent)]
    CargoMetadata(#[from] cargo_metadata::Error),
    #[error("Unable to resolve dependencies")]
    DependencyResolution,
    #[error("{0} package not found")]
    PackageNotFound(PackageId),
}

pub struct PackageLoader {
    metadata: Metadata,
}

impl PackageLoader {
    /// Create a new package loader that loads teh cargo metadata
    pub fn new() -> Result<Self, PackageLoaderError> {
        let metadata = MetadataCommand::new().exec()?;
        Ok(Self { metadata })
    }

    /// Get the top level packages for this project
    pub fn get_package_roots(&self) -> Result<Vec<&Package>, PackageLoaderError> {
        let resolve = self
            .metadata
            .resolve
            .as_ref()
            .ok_or(PackageLoaderError::DependencyResolution)?;

        if let Some(root) = &resolve.root {
            Ok(vec![self.metadata.packages.by_id(root)?])
        } else {
            self.metadata
                .workspace_members
                .iter()
                .map(|member| self.metadata.packages.by_id(member))
                .collect()
        }
    }

    /// Collect all packages that are dependencies of the root packages
    pub fn get_root_dependencies(
        &self,
        roots: &[&Package],
    ) -> Result<Vec<&Package>, PackageLoaderError> {
        let mut result = vec![];
        let mut added = HashSet::new();

        let mut to_check: VecDeque<_> = roots.iter().map(|p| &p.id).collect();
        let packages = &self.metadata.packages;
        let resolve = &self
            .metadata
            .resolve
            .as_ref()
            .ok_or(PackageLoaderError::DependencyResolution)?;

        while let Some(id) = to_check.pop_back() {
            if added.insert(id) {
                let package = packages.by_id(id)?;
                result.push(package);
                for dep in resolve.by_id(id)? {
                    if dep
                        .dep_kinds
                        .iter()
                        .any(|info| info.kind == DependencyKind::Normal)
                    {
                        to_check.push_back(&dep.pkg);
                    }
                }
            }
        }
        Ok(result)
    }
}

trait PackagesExt {
    fn by_id(&self, id: &PackageId) -> Result<&Package, PackageLoaderError>;
}

impl PackagesExt for Vec<Package> {
    fn by_id(&self, id: &PackageId) -> Result<&Package, PackageLoaderError> {
        self.iter()
            .find(|package| &package.id == id)
            .ok_or_else(|| PackageLoaderError::PackageNotFound(id.clone()))
    }
}

trait ResolveExt {
    fn by_id(&self, id: &PackageId) -> Result<&[NodeDep], PackageLoaderError>;
}

impl ResolveExt for Resolve {
    fn by_id(&self, id: &PackageId) -> Result<&[NodeDep], PackageLoaderError> {
        self.nodes
            .iter()
            .find(|node| &node.id == id)
            .map(|node| node.deps.as_ref())
            .ok_or_else(|| PackageLoaderError::PackageNotFound(id.clone()))
    }
}
