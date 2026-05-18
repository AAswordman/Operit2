pub trait VersionSpec {}

pub struct PromptVersionManager<T: VersionSpec> {
    pub versionSpec: std::marker::PhantomData<T>,
}
