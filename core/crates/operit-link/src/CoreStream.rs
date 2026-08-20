use crate::CoreValue;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Describes one stream property that the generic Link bridge can subscribe to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreStreamDescriptor {
    /// Identifies one logical stream independently from its current source.
    pub streamId: String,
    pub targetPath: String,
    pub propertyName: String,
    pub args: CoreValue,
}
/// Carries a Link-owned stream source without exposing transport state to models.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound(serialize = "", deserialize = ""))]
pub struct CoreStream<T> {
    #[serde(rename = "$coreStream")]
    pub descriptor: CoreStreamDescriptor,
    #[serde(skip)]
    marker: PhantomData<T>,
}

impl<T, U> PartialEq<CoreStream<U>> for CoreStream<T> {
    /// Compares embedded stream sources without comparing their item marker types.
    fn eq(&self, other: &CoreStream<U>) -> bool {
        self.descriptor == other.descriptor
    }
}

impl<T> CoreStream<T> {
    /// Creates a stream value backed by one explicit Core property source.
    pub fn new_at(
        streamId: impl Into<String>,
        targetPath: impl Into<String>,
        propertyName: impl Into<String>,
        args: CoreValue,
    ) -> Self {
        Self {
            descriptor: CoreStreamDescriptor {
                streamId: streamId.into(),
                targetPath: targetPath.into(),
                propertyName: propertyName.into(),
                args,
            },
            marker: PhantomData,
        }
    }
}
