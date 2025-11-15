# Data Format Support - Priority Tagger & Scheduler

## ✅ Universal Format Support

**YES, the priority tagger and scheduler work with ALL data formats!**

Both components are **format-agnostic** and handle:
- ✅ **Text** (logs, plain text)
- ✅ **JSON** (structured data)
- ✅ **XML** (structured markup)
- ✅ **Binary** (any binary data)
- ✅ **Structured Binary** (protobuf, msgpack, etc.)
- ✅ **Compressed** (ZIP, GZIP, etc.)
- ✅ **Encrypted** (any encrypted data)
- ✅ **Any format** (format-agnostic design)

## How It Works

### Priority Tagger - Format-Aware Detection

The priority tagger automatically detects the data format and uses appropriate methods:

#### 1. **Text Formats** (Text, JSON, XML)
- **Method**: Keyword-based detection
- **Process**: Searches for critical/high/low keywords
- **Example**: `"Alert: Fire detected"` → Critical priority

#### 2. **Binary Formats**
- **Method**: Pattern-based heuristics
- **Process**: 
  - Size analysis (small = control message, large = bulk)
  - Entropy calculation (high entropy = compressed/encrypted)
  - Magic number detection (ZIP, GZIP, PNG, etc.)
- **Example**: 2MB binary chunk → Bulk priority

#### 3. **Structured Binary**
- **Method**: Hybrid approach
- **Process**: Tries text-based first, falls back to binary heuristics
- **Example**: Protobuf with text headers → Format-aware detection

#### 4. **Embedding-Based** (Any Format)
- **Method**: Semantic analysis via embeddings
- **Process**: Uses embedding variance/patterns
- **Example**: Works with any format that can be embedded

### Scheduler - Format-Agnostic

The scheduler is **completely format-agnostic**:
- Stores `Vec<u8>` (raw bytes)
- No format-specific processing
- Works with any data type

## Format Detection

The priority tagger automatically detects formats:

```rust
pub enum DataFormat {
    Text,       // Plain text, logs
    Json,       // JSON format
    Xml,        // XML format
    Binary,     // Binary data
    Structured, // Structured binary (protobuf, msgpack, etc.)
    Unknown,    // Unknown format
}
```

### Detection Logic

1. **JSON**: Starts with `{` or `[`, contains JSON structure
2. **XML**: Starts with `<`
3. **Text**: >80% ASCII characters
4. **Structured**: 30-80% ASCII (mixed binary/text)
5. **Binary**: <30% ASCII

## Examples by Format

### Text Format
```rust
let chunk = b"Alert: System failure detected";
let priority = tagger.tag_priority(chunk, None);
// Format: Text
// Priority: Critical (keyword detected)
```

### JSON Format
```rust
let chunk = br#"{"type":"Alert","severity":"Critical","message":"Fire detected"}"#;
let priority = tagger.tag_priority(chunk, None);
// Format: Json
// Priority: Critical (keyword in JSON)
```

### XML Format
```rust
let chunk = br#"<alert><type>Critical</type><message>System down</message></alert>"#;
let priority = tagger.tag_priority(chunk, None);
// Format: Xml
// Priority: Critical (keyword in XML)
```

### Binary Format (Small Control Message)
```rust
let chunk = vec![0x01, 0x02, 0x03, 0x04]; // 4 bytes
let priority = tagger.tag_priority(&chunk, None);
// Format: Binary
// Priority: High (small size = control message)
```

### Binary Format (Large Bulk Data)
```rust
let chunk = vec![0u8; 2_000_000]; // 2MB
let priority = tagger.tag_priority(&chunk, None);
// Format: Binary
// Priority: Bulk (large size)
```

### Compressed Format
```rust
let chunk = b"PK\x03\x04..."; // ZIP file header
let priority = tagger.tag_priority(chunk, None);
// Format: Binary
// Priority: Bulk (compressed format detected)
```

### Encrypted Format
```rust
let chunk = vec![random_bytes; 10000]; // High entropy
let priority = tagger.tag_priority(&chunk, None);
// Format: Binary
// Priority: Bulk (high entropy = encrypted/compressed)
```

## Format-Specific Optimizations

### Text/JSON/XML
- ✅ Keyword detection (case-insensitive)
- ✅ Pattern matching (numeric ratios, array detection)
- ✅ Structure-aware (JSON arrays = bulk)

### Binary
- ✅ Size-based heuristics
- ✅ Entropy analysis
- ✅ Magic number detection
- ✅ Format-specific rules (ZIP, GZIP, PNG, etc.)

### Structured Binary
- ✅ Hybrid approach
- ✅ Text header detection
- ✅ Binary body analysis

## Scheduler Format Support

The scheduler is **completely format-agnostic**:

```rust
// Works with ANY format
scheduler.schedule(
    json_data,      // JSON
    priority,
    route,
)?;

scheduler.schedule(
    binary_data,    // Binary
    priority,
    route,
)?;

scheduler.schedule(
    text_data,      // Text
    priority,
    route,
)?;

scheduler.schedule(
    any_format,     // Any format
    priority,
    route,
)?;
```

## Limitations & Considerations

### Current Limitations

1. **Keyword Detection**: Works best with English keywords
   - **Solution**: Use `with_keywords()` to add custom keywords

2. **Binary Priority**: Less precise than text-based
   - **Solution**: Use embedding-based tagging for better accuracy

3. **Format Detection**: Heuristic-based (not 100% accurate)
   - **Solution**: Can be improved with ML-based format detection

### Best Practices

1. **For Text/JSON**: Use default keyword-based tagging
2. **For Binary**: Consider using embedding-based tagging
3. **For Custom Formats**: Add format-specific keywords
4. **For Mixed Formats**: System handles automatically

## Extending Format Support

### Add Custom Format Detection

```rust
// Extend detect_format() method
match format {
    DataFormat::CustomProtocol => {
        // Custom format-specific logic
    }
    // ... existing formats
}
```

### Add Format-Specific Keywords

```rust
let tagger = PriorityTagger::with_keywords(
    vec![b"CustomAlert", b"CustomCritical"],  // Critical
    vec![b"CustomWarning"],                    // High
    vec![b"CustomInfo"],                       // Low
);
```

### Use Embedding-Based Tagging

```rust
// Works with ANY format (embeddings are format-agnostic)
let embedding = ai_system.embed_chunk(chunk_data)?;
let priority = tagger.tag_priority_from_embedding(&embedding, severity);
```

## Summary

| Component | Format Support | Method |
|-----------|---------------|--------|
| **Priority Tagger** | ✅ **ALL formats** | Format-aware detection + embedding fallback |
| **Scheduler** | ✅ **ALL formats** | Format-agnostic (stores `Vec<u8>`) |
| **Text/JSON/XML** | ✅ **Full support** | Keyword-based detection |
| **Binary** | ✅ **Full support** | Pattern/heuristic-based |
| **Structured** | ✅ **Full support** | Hybrid approach |
| **Any Format** | ✅ **Full support** | Embedding-based fallback |

## Conclusion

**YES, the priority tagger and scheduler work with ALL data formats!**

- ✅ Format-agnostic design
- ✅ Automatic format detection
- ✅ Format-specific optimizations
- ✅ Embedding-based fallback
- ✅ Extensible for custom formats

The system intelligently adapts its priority tagging strategy based on the detected format, ensuring optimal priority assignment regardless of data type.

