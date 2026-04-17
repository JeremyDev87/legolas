use crate::models::{
    DuplicatePackage, HeavyDependency, Impact, LazyLoadCandidate, TreeShakingWarning,
};

pub fn estimate_impact(
    _heavy_dependencies: &[HeavyDependency],
    _duplicate_packages: &[DuplicatePackage],
    _lazy_load_candidates: &[LazyLoadCandidate],
    _tree_shaking_warnings: &[TreeShakingWarning],
) -> Impact {
    Impact::default()
}
