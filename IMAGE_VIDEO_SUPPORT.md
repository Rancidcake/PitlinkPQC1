# Image & Video Support

## ✅ Full Support for Images and Videos

The priority tagger and scheduler now have **comprehensive support** for image and video formats with format-specific optimizations.

## Image Format Support

### Supported Formats
- ✅ **JPEG/JPG** - Detected via magic number `FF D8 FF`
- ✅ **PNG** - Detected via magic number `89 50 4E 47`
- ✅ **GIF** - Detected via magic number `GIF89a` or `GIF87a`
- ✅ **WebP** - Detected via `RIFF...WEBP` header
- ✅ **BMP** - Detected via magic number `BM`
- ✅ **TIFF** - Detected via magic numbers `II*` or `MM*`

### Priority Logic for Images

| Size | Priority | Use Case |
|------|----------|----------|
| **< 100KB** | **High** | Thumbnails, previews, icons |
| **100KB - 10MB** | **Normal** | Standard images, photos |
| **10MB - 50MB** | **Low** | Large images, high-res photos |
| **> 50MB** | **Bulk** | Very large images, panoramas |

### Example

```rust
use trackshift::{PriorityTagger, ChunkPriority};

let tagger = PriorityTagger::new();

// Thumbnail image (50KB)
let thumbnail = load_image("thumb.jpg"); // < 100KB
let priority = tagger.tag_priority(&thumbnail, None);
// Result: ChunkPriority::High

// Standard photo (2MB)
let photo = load_image("photo.jpg"); // 1-10MB
let priority = tagger.tag_priority(&photo, None);
// Result: ChunkPriority::Normal

// Large panorama (20MB)
let panorama = load_image("panorama.jpg"); // > 10MB
let priority = tagger.tag_priority(&panorama, None);
// Result: ChunkPriority::Low
```

## Video Format Support

### Supported Formats
- ✅ **MP4** - Detected via `ftyp` box with `mp41`, `mp42`, `isom`, `avc1` brands
- ✅ **AVI** - Detected via `RIFF...AVI` header
- ✅ **MOV/QuickTime** - Detected via `ftyp` box with `qt` or `moov` brands
- ✅ **WebM** - Detected via magic number `1A 45 DF A3`
- ✅ **MKV** - Detected via magic number `1A 45 DF A3` (Matroska)

### Priority Logic for Videos

| Size | Priority | Use Case |
|------|----------|----------|
| **< 10MB** | **High** | Short clips, important content |
| **10MB - 100MB** | **Normal** | Standard videos, medium clips |
| **100MB - 500MB** | **Low** | Large videos, streaming-friendly |
| **> 500MB** | **Bulk** | Very large videos, movies |

### Example

```rust
use trackshift::{PriorityTagger, ChunkPriority};

let tagger = PriorityTagger::new();

// Short clip (5MB)
let clip = load_video("clip.mp4"); // < 10MB
let priority = tagger.tag_priority(&clip, None);
// Result: ChunkPriority::High

// Standard video (50MB)
let video = load_video("video.mp4"); // 10-100MB
let priority = tagger.tag_priority(&video, None);
// Result: ChunkPriority::Normal

// Large video (200MB)
let movie = load_video("movie.mp4"); // 100-500MB
let priority = tagger.tag_priority(&movie, None);
// Result: ChunkPriority::Low

// Very large video (1GB)
let large_movie = load_video("large_movie.mp4"); // > 500MB
let priority = tagger.tag_priority(&large_movie, None);
// Result: ChunkPriority::Bulk
```

## Audio Format Support

### Supported Formats
- ✅ **WAV** - Detected via `RIFF...WAVE` header
- ✅ **MP3** - Detected via magic number `FF FB` or `FF F3`
- ✅ **OGG** - Detected via magic number `OggS`
- ✅ **FLAC** - Detected via magic number `fLaC`

### Priority Logic for Audio

| Size | Priority | Use Case |
|------|----------|----------|
| **< 5MB** | **High** | Voice messages, alerts, notifications |
| **5MB - 50MB** | **Normal** | Standard audio files, songs |
| **> 50MB** | **Low** | Large audio files, albums |

## Integration with QUIC-FEC

Images and videos benefit from QUIC-FEC's Forward Error Correction:

### For Images
- **Small images** (< 100KB): High priority + FEC for reliability
- **Large images** (> 10MB): Low priority + FEC for patchy networks

### For Videos
- **Streaming**: Normal/Low priority + FEC for packet loss recovery
- **Small clips**: High priority + FEC for critical content
- **Large videos**: Bulk priority + FEC for reliable transfer

### Example Integration

```rust
use trackshift::*;
use quic_fec::connection::QuicFecConnection;
use quic_fec::fec::FecConfig;

// Initialize components
let tagger = PriorityTagger::new();
let scheduler = PriorityScheduler::new();
let quic_conn = QuicFecConnection::connect(/* ... */).await?;

// Process video file
let video_data = load_video("video.mp4");
let priority = tagger.tag_priority(&video_data, None);

// Schedule with priority
scheduler.schedule(video_data.clone(), priority, None)?;

// Send via QUIC-FEC (with FEC for reliability)
let fec_config = if priority == ChunkPriority::Bulk {
    FecConfig::for_file_transfer()  // High redundancy for large files
} else {
    FecConfig::for_telemetry()  // Standard FEC
};

quic_conn.send_with_fec(video_data, fec_config).await?;
```

## Integration with FileType::Media

The system recognizes media files via `FileType::Media`:

```rust
use trackshift::telemetry_ai::FileType;

// Media files are automatically classified
let file_type = FileType::Media;  // Video, audio files

// AI system uses this for optimization:
// - Streaming-friendly decisions
// - Chunk size recommendations
// - Integrity check methods
```

## Streaming Optimizations

### Video Streaming
- **Chunking**: Large videos are automatically chunked for streaming
- **Priority**: Normal/Low priority allows background transfer
- **FEC**: Forward Error Correction recovers lost packets
- **Handover**: Seamless network handover during streaming

### Image Streaming
- **Progressive**: Large images can be sent progressively
- **Thumbnails First**: Thumbnails get high priority
- **Full Image**: Full resolution sent at lower priority

## Compression for Media

### Images
- **Already Compressed**: JPEG, PNG, WebP are already compressed
- **Additional Compression**: Usually not beneficial
- **Recommendation**: Send as-is (no additional compression)

### Videos
- **Already Compressed**: MP4, WebM are already compressed
- **Additional Compression**: Usually not beneficial
- **Recommendation**: Send as-is (no additional compression)

### Audio
- **Already Compressed**: MP3, OGG, FLAC are already compressed
- **Additional Compression**: Usually not beneficial
- **Recommendation**: Send as-is (no additional compression)

## Priority Scheduling for Media

The scheduler handles media files with appropriate priority:

```rust
use trackshift::*;

let scheduler = PriorityScheduler::new();

// Thumbnail (high priority)
scheduler.schedule(thumbnail_data, ChunkPriority::High, None)?;

// Video clip (high priority)
scheduler.schedule(clip_data, ChunkPriority::High, None)?;

// Large video (low priority)
scheduler.schedule(large_video_data, ChunkPriority::Low, None)?;

// Retrieve in priority order
while let Some(chunk) = scheduler.get_next() {
    send_media_chunk(&chunk.data)?;
}
```

## Network-Aware Media Transfer

### Good Network
- **High Priority Media**: Sent immediately
- **Low Priority Media**: Sent with normal bandwidth allocation
- **Bulk Media**: Sent with reduced bandwidth

### Patchy Network
- **High Priority Media**: Sent with FEC protection
- **Low Priority Media**: Buffered or delayed
- **Bulk Media**: Paused until network improves

### Example

```rust
let decision = ai_system.process_file_transfer(
    video_data,
    network_metrics,
    file_size,
    FileType::Media,  // Media file
    FilePriority::Normal,
    bytes_transferred,
    true,  // Requires integrity check
    0,     // No failures
    false, // Not a resume
)?;

// Decision includes:
// - Recommended chunk size (for streaming)
// - Enable parallel transfer (for large files)
// - Integrity check method (CRC32 for large files)
// - Retry strategy (based on network quality)
```

## Summary

| Format | Detection | Priority Logic | FEC Support | Streaming |
|--------|-----------|----------------|-------------|-----------|
| **Images** | ✅ Magic numbers | Size-based | ✅ Yes | ✅ Progressive |
| **Videos** | ✅ Magic numbers | Size-based | ✅ Yes | ✅ Chunked |
| **Audio** | ✅ Magic numbers | Size-based | ✅ Yes | ✅ Streaming |

## Benefits

1. **Automatic Detection**: No manual format specification needed
2. **Smart Priority**: Size-based priority assignment
3. **FEC Protection**: QUIC-FEC for reliable transfer
4. **Streaming-Friendly**: Chunking and priority for streaming
5. **Network-Aware**: Adapts to network conditions

The system now provides **complete support** for images and videos with intelligent priority tagging and scheduling!

