#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageIntel {
    pub estimated_kb: usize,
    pub category: &'static str,
    pub rationale: &'static str,
    pub recommendation: &'static str,
}

pub fn get_package_intel(_name: &str) -> Option<PackageIntel> {
    None
}
