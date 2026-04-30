#![forbid(unsafe_code)]

pub const SNAPSHOT_FORMAT_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotManifest {
    pub format_version: u32,
    pub machine_profile: String,
    pub parent: Option<String>,
    pub created_by: String,
    pub chunks: Vec<ChunkRef>,
}

impl SnapshotManifest {
    pub fn new(machine_profile: impl Into<String>, created_by: impl Into<String>) -> Self {
        Self {
            format_version: SNAPSHOT_FORMAT_VERSION,
            machine_profile: machine_profile.into(),
            parent: None,
            created_by: created_by.into(),
            chunks: Vec::new(),
        }
    }

    pub fn with_parent(mut self, parent: impl Into<String>) -> Self {
        self.parent = Some(parent.into());
        self
    }

    pub fn add_chunk(&mut self, chunk: ChunkRef) {
        self.chunks.push(chunk);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChunkRef {
    pub kind: ChunkKind,
    pub hash: String,
    pub size_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChunkKind {
    Cpu,
    MemoryPage,
    Device,
    StorageOverlay,
    Scheduler,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_defaults_to_current_version() {
        let manifest = SnapshotManifest::new("bxr-minimal-x64-v1", "test");
        assert_eq!(manifest.format_version, SNAPSHOT_FORMAT_VERSION);
        assert_eq!(manifest.machine_profile, "bxr-minimal-x64-v1");
    }
}
