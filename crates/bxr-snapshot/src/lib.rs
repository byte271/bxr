#![forbid(unsafe_code)]

pub const SNAPSHOT_FORMAT_VERSION: u32 = 1;

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

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

impl ChunkRef {
    pub fn from_bytes(kind: ChunkKind, bytes: &[u8]) -> Self {
        Self {
            kind,
            hash: content_hash_hex(bytes),
            size_bytes: bytes.len() as u64,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChunkKind {
    Cpu,
    MemoryPage,
    Device,
    StorageOverlay,
    Scheduler,
}

pub fn content_hash_hex(bytes: &[u8]) -> String {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("fnv1a64:{hash:016x}")
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

    #[test]
    fn chunk_hashes_are_content_addressed() {
        let first = ChunkRef::from_bytes(ChunkKind::Cpu, b"cpu-state");
        let second = ChunkRef::from_bytes(ChunkKind::Cpu, b"cpu-state");
        let changed = ChunkRef::from_bytes(ChunkKind::Cpu, b"cpu-state!");

        assert_eq!(first.hash, second.hash);
        assert_ne!(first.hash, changed.hash);
        assert_eq!(first.size_bytes, 9);
        assert!(first.hash.starts_with("fnv1a64:"));
    }
}
