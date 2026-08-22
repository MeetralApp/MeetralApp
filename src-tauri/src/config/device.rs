use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRef {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
}

impl DeviceRef {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.id.trim().is_empty() && self.name.trim().is_empty()
    }

    pub fn is_configured(&self) -> bool {
        !self.is_empty()
    }
}
