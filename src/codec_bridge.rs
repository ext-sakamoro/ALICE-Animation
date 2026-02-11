//! Bridge: ALICE-Animation → ALICE-Codec
//! Compresses ANIM binary episodes using ALICE-Codec (50KB → ~5KB).

use crate::episode::EpisodePackage;
// use alice_codec::{compress, decompress, CompressionConfig};

/// Compressed episode wrapper with codec metadata.
#[derive(Debug)]
pub struct CompressedEpisode {
    pub compressed_data: Vec<u8>,
    pub original_size: usize,
    pub compression_ratio: f32,
}

/// Compress a serialized ANIM episode using ALICE-Codec.
#[inline]
pub fn compress_episode(episode: &EpisodePackage) -> Result<CompressedEpisode, Box<dyn std::error::Error>> {
    let mut raw = Vec::new();
    let original_size = crate::episode::serialize_episode(episode, &mut raw)?;

    // TODO: Integrate with alice_codec once available
    // let config = CompressionConfig::default();
    // let compressed_data = compress(&raw, &config)?;

    // Placeholder: no compression yet
    let compressed_data = raw;
    let compression_ratio = original_size as f32 / compressed_data.len().max(1) as f32;

    Ok(CompressedEpisode {
        compressed_data,
        original_size,
        compression_ratio,
    })
}

/// Decompress back to EpisodePackage.
#[inline]
pub fn decompress_episode(compressed: &CompressedEpisode) -> Result<EpisodePackage, Box<dyn std::error::Error>> {
    // TODO: Integrate with alice_codec once available
    // let raw = decompress(&compressed.compressed_data)?;

    // Placeholder: assume no compression
    let raw = &compressed.compressed_data;
    let mut cursor = std::io::Cursor::new(raw);
    let episode = crate::episode::deserialize_episode(&mut cursor)?;
    Ok(episode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::director::{Cut, Director};
    use crate::npr::AnimeShading;
    use crate::scene::{Actor, SceneGraph};
    use crate::episode::EpisodeMetadata;
    use alice_sdf::SdfNode;

    #[test]
    fn test_compress_decompress_roundtrip() {
        let mut sg = SceneGraph::new();
        sg.add_actor(Actor::new("test", SdfNode::sphere(1.0)));
        let mut dir = Director::new("Test");
        dir.add_cut(Cut::new("c1", 0.0, 5.0));
        let meta = EpisodeMetadata::new("Test Episode", 1, 5.0);
        let episode = EpisodePackage::new(meta, sg, dir, AnimeShading::default());

        let compressed = compress_episode(&episode).unwrap();
        assert!(compressed.original_size > 0);
        assert!(compressed.compression_ratio > 0.0);

        let restored = decompress_episode(&compressed).unwrap();
        assert_eq!(restored.metadata.title, "Test Episode");
    }
}
