//! Headroom — inline context compression for tool outputs.
//!
//! Port of the core, deterministic (ML-free) parts of
//! [headroom](https://github.com/chopratejas/headroom): compress what an agent
//! reads (tool outputs, build/test logs, grep results) *before* it reaches the
//! LLM, keeping the salient signal while cutting 60-95% of the tokens.
//!
//! A [`ContentRouter`] detects the content type of a tool output and routes it
//! to the best compressor:
//!
//! - build/test output → [`transforms::log_compressor::LogCompressor`]
//! - grep / ripgrep results → [`transforms::search_compressor::SearchCompressor`]
//! - unified diffs → [`transforms::diff_compressor::DiffCompressor`]
//! - everything else → passed through unchanged
//!
//! Compression is reversible in spirit: the compressed output always carries an
//! explicit marker (`[N lines omitted: ...]` / `[... and N more matches ...]`)
//! so the model knows content was elided and can re-run the tool for detail.

pub mod adaptive_sizer;
pub mod auth_mode;
pub mod ccr;
pub mod log_compressor;
pub mod search_compressor;
pub mod signals;
pub mod smart_crusher;
pub mod tokenizer;
pub mod transforms;

use transforms::content_detector;
use transforms::detection;
use transforms::diff_compressor::{DiffCompressor, DiffCompressorConfig};
use transforms::log_compressor::{LogCompressor, LogCompressorConfig};
use transforms::search_compressor::{SearchCompressor, SearchCompressorConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    SearchResults,
    BuildOutput,
    GitDiff,
    PlainText,
}

#[derive(Debug, Clone)]
pub struct CompressionResult {
    pub compressed: String,
    pub original_chars: usize,
    pub compressed_chars: usize,
    pub content_type: ContentType,
    pub strategy: &'static str,
}

impl CompressionResult {
    fn passthrough(content: &str, content_type: ContentType) -> Self {
        Self {
            compressed: content.to_string(),
            original_chars: content.len(),
            compressed_chars: content.len(),
            content_type,
            strategy: "passthrough",
        }
    }

    pub fn ratio(&self) -> f64 {
        if self.original_chars == 0 {
            1.0
        } else {
            self.compressed_chars as f64 / self.original_chars as f64
        }
    }

    /// Estimated tokens saved (rough: ~4 chars per token).
    pub fn tokens_saved_estimate(&self) -> usize {
        self.original_chars.saturating_sub(self.compressed_chars) / 4
    }

    pub fn did_compress(&self) -> bool {
        self.compressed_chars < self.original_chars
    }
}

/// Routes content to the appropriate compressor based on detected type.
pub struct ContentRouter {
    log: LogCompressor,
    search: SearchCompressor,
    diff: DiffCompressor,
    /// `bias` multiplier passed to the adaptive sizer (>1 keeps more).
    bias: f64,
}

impl Default for ContentRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentRouter {
    pub fn new() -> Self {
        Self {
            log: LogCompressor::new(LogCompressorConfig::default()),
            search: SearchCompressor::new(SearchCompressorConfig::default()),
            diff: DiffCompressor::new(DiffCompressorConfig::default()),
            bias: 1.0,
        }
    }

    pub fn with_bias(mut self, bias: f64) -> Self {
        self.bias = bias;
        self
    }

    /// Compress `content`, optionally biasing scoring toward `context`
    /// (e.g. the tool's arguments / the user's intent) for search results.
    pub fn compress(&self, content: &str, context: &str) -> CompressionResult {
        let original_chars = content.len();
        let detected_type = detection::detect(content);

        match detected_type {
            content_detector::ContentType::SearchResults => {
                let (result, _stats) = self.search.compress(content, context, self.bias);
                CompressionResult {
                    compressed_chars: result.compressed.len(),
                    compressed: result.compressed,
                    original_chars,
                    content_type: ContentType::SearchResults,
                    strategy: "search_compressor",
                }
            }
            content_detector::ContentType::BuildOutput => {
                let (result, _stats) = self.log.compress(content, self.bias);
                CompressionResult {
                    compressed_chars: result.compressed.len(),
                    compressed: result.compressed,
                    original_chars,
                    content_type: ContentType::BuildOutput,
                    strategy: "log_compressor",
                }
            }
            content_detector::ContentType::GitDiff => {
                let result = self.diff.compress(content, context);
                CompressionResult {
                    compressed_chars: result.compressed.len(),
                    compressed: result.compressed,
                    original_chars,
                    content_type: ContentType::GitDiff,
                    strategy: "diff_compressor",
                }
            }
            _ => {
                // JsonArray, SourceCode, Html, PlainText all pass through
                CompressionResult::passthrough(content, ContentType::PlainText)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_search_results() {
        let content = "src/a.py:1:foo\nsrc/b.py:2:bar\nsrc/c.py:3:baz";
        let r = ContentRouter::new().compress(content, "");
        assert_eq!(r.content_type, ContentType::SearchResults);
    }

    #[test]
    fn detects_build_output() {
        let content = "INFO starting\nERROR boom happened\nWARNING careful\nINFO done";
        let r = ContentRouter::new().compress(content, "");
        assert_eq!(r.content_type, ContentType::BuildOutput);
    }

    #[test]
    fn plain_prose_passes_through() {
        let content = "The quick brown fox jumps over the lazy dog. Nothing to compress here.";
        let r = ContentRouter::new().compress(content, "");
        assert_eq!(r.content_type, ContentType::PlainText);
        assert_eq!(r.strategy, "passthrough");
        assert_eq!(r.compressed, content);
    }

    #[test]
    fn router_compresses_noisy_log() {
        let mut lines: Vec<String> = (0..400).map(|i| format!("INFO step {i} ok")).collect();
        lines.push("ERROR: the build broke on widget".to_string());
        let content = lines.join("\n");
        let r = ContentRouter::new().compress(&content, "");
        assert_eq!(r.content_type, ContentType::BuildOutput);
        assert!(r.did_compress());
        assert!(r.compressed.contains("the build broke on widget"));
        assert!(r.tokens_saved_estimate() > 0);
    }

    #[test]
    fn router_compresses_search_results() {
        let mut lines: Vec<String> = vec![];
        for i in 0..50 {
            lines.push(format!("src/module.rs:{}:    let x = {};", i, i));
        }
        lines.push("src/target.rs:42:fn important() {".to_string());
        let content = lines.join("\n");
        let r = ContentRouter::new().compress(&content, "");
        assert_eq!(r.content_type, ContentType::SearchResults);
        assert!(r.did_compress());
        assert!(r.compressed.contains("important"));
    }
}
