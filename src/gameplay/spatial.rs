//! Game-owned spatial indexing primitives shared by gameplay query implementations.

mod dynamic_aabb_tree;

pub(crate) use dynamic_aabb_tree::DynamicAabbTree;
