use std::io::{Read, Write};

use serde::{Deserialize, Serialize};

use crate::director::Director;
use crate::npr::AnimeShading;
use crate::scene::SceneGraph;

/// Binary format magic bytes.
const EPISODE_MAGIC: [u8; 4] = *b"ANIM";
/// Format version.
const EPISODE_VERSION: u16 = 1;

/// Episode metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeMetadata {
    pub title: String,
    pub episode_number: u32,
    pub duration_seconds: f32,
    pub resolution: (u32, u32),
}

impl EpisodeMetadata {
    pub fn new(title: impl Into<String>, episode_number: u32, duration: f32) -> Self {
        Self {
            title: title.into(),
            episode_number,
            duration_seconds: duration,
            resolution: (1920, 1080),
        }
    }
}

/// Complete episode package: all data needed to render an episode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodePackage {
    pub metadata: EpisodeMetadata,
    pub scene_graph: SceneGraph,
    pub director: Director,
    pub shading: AnimeShading,
}

impl EpisodePackage {
    pub fn new(
        metadata: EpisodeMetadata,
        scene_graph: SceneGraph,
        director: Director,
        shading: AnimeShading,
    ) -> Self {
        Self {
            metadata,
            scene_graph,
            director,
            shading,
        }
    }

    /// Estimate serialized size in bytes (rough).
    pub fn estimate_size(&self) -> usize {
        // Rough estimate: metadata + scene + director + shading
        let actors = self.scene_graph.actor_count();
        let cuts = self.director.cut_count();
        256 + actors * 512 + cuts * 256
    }
}

/// Serialize an episode package to a writer.
///
/// Binary format:
/// `[Magic "ANIM" 4B][Version 2B][Flags 2B][Size 4B][CRC32 4B][Bincode Body]`
pub fn serialize_episode<W: Write>(episode: &EpisodePackage, writer: &mut W) -> std::io::Result<usize> {
    // Serialize body first to get size and CRC
    let body = bincode::serialize(episode)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let crc = crc32fast::hash(&body);
    let size = body.len() as u32;
    let flags: u16 = 0;

    // Write header
    writer.write_all(&EPISODE_MAGIC)?;
    writer.write_all(&EPISODE_VERSION.to_le_bytes())?;
    writer.write_all(&flags.to_le_bytes())?;
    writer.write_all(&size.to_le_bytes())?;
    writer.write_all(&crc.to_le_bytes())?;

    // Write body
    writer.write_all(&body)?;

    Ok(16 + body.len())
}

/// Deserialize an episode package from a reader.
pub fn deserialize_episode<R: Read>(reader: &mut R) -> std::io::Result<EpisodePackage> {
    // Read header (16 bytes)
    let mut header = [0u8; 16];
    reader.read_exact(&mut header)?;

    // Validate magic
    if &header[0..4] != &EPISODE_MAGIC {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid magic bytes: expected ANIM",
        ));
    }

    // Parse header fields
    let version = u16::from_le_bytes([header[4], header[5]]);
    if version != EPISODE_VERSION {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Unsupported version: {}", version),
        ));
    }

    let _flags = u16::from_le_bytes([header[6], header[7]]);
    let size = u32::from_le_bytes([header[8], header[9], header[10], header[11]]) as usize;
    let expected_crc = u32::from_le_bytes([header[12], header[13], header[14], header[15]]);

    // Read body
    let mut body = vec![0u8; size];
    reader.read_exact(&mut body)?;

    // Validate CRC
    let actual_crc = crc32fast::hash(&body);
    if actual_crc != expected_crc {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "CRC mismatch: expected {:#010x}, got {:#010x}",
                expected_crc, actual_crc
            ),
        ));
    }

    // Deserialize
    bincode::deserialize(&body)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::director::{Cut, Director};
    use crate::scene::{Actor, SceneGraph};
    use alice_sdf::SdfNode;

    fn make_test_episode() -> EpisodePackage {
        let mut sg = SceneGraph::new();
        let id_a = sg.add_actor(Actor::new("hero", SdfNode::sphere(1.0)));
        let id_b = sg.add_actor(Actor::new("villain", SdfNode::box3d(1.0, 1.0, 1.0)));

        let mut dir = Director::new("Test Episode");
        let c1 = dir.add_cut(Cut::new("intro", 0.0, 3.0).with_actors(vec![id_a]));
        let c2 = dir.add_cut(Cut::new("battle", 3.0, 8.0).with_actors(vec![id_a, id_b]));

        let meta = EpisodeMetadata::new("Test", 1, 8.0);
        EpisodePackage::new(meta, sg, dir, AnimeShading::default())
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let episode = make_test_episode();
        let mut buf = Vec::new();
        let written = serialize_episode(&episode, &mut buf).unwrap();
        assert!(written > 16);
        assert!(written < 10_000); // Should be compact

        let mut cursor = std::io::Cursor::new(&buf);
        let restored = deserialize_episode(&mut cursor).unwrap();
        assert_eq!(restored.metadata.title, "Test");
        assert_eq!(restored.metadata.episode_number, 1);
        assert_eq!(restored.scene_graph.actor_count(), 2);
        assert_eq!(restored.director.cut_count(), 2);
    }

    #[test]
    fn test_invalid_magic() {
        let buf = b"BADMxxxxxxxxxxxxbody";
        let mut cursor = std::io::Cursor::new(&buf[..]);
        assert!(deserialize_episode(&mut cursor).is_err());
    }

    #[test]
    fn test_estimate_size() {
        let episode = make_test_episode();
        let est = episode.estimate_size();
        assert!(est > 0);
    }
}
