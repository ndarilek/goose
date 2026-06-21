#[test]
fn test_e2e_headroom_compression_on_build_log() {
    use goose::agents::headroom::ContentRouter;

    // Simulate a realistic CI log: timestamped lines with INFO/ERROR/WARN spread
    // throughout so the content detector's 10% signal threshold is comfortably met.
    let mut lines = vec![];
    lines.push("[INFO] Build starting — 2026-06-21 09:00:00".to_string());
    lines.push("[INFO] Loading configuration".to_string());
    for i in 0..60 {
        lines.push(format!("[INFO] Compiling module_{:03}", i));
    }
    for i in 0..60 {
        lines.push(format!("[DEBUG] Linking step {:03}", i));
    }
    // Errors that MUST be preserved
    lines.push("[ERROR] Compilation failed in payment_processor.rs:42 — type mismatch".to_string());
    lines.push("[ERROR] Linker error: undefined symbol `process_refund`".to_string());
    lines.push("[WARN] Deprecated API call in legacy_auth.rs:11".to_string());
    for i in 0..100 {
        lines.push(format!("[INFO] Post-build check {:03} ok", i));
    }
    lines.push("[INFO] Build finished in 43.2s".to_string());

    let log_content = lines.join("\n");
    let original_size = log_content.len();

    let router = ContentRouter::new();
    let result = router.compress(&log_content, "");

    println!("Original size: {} bytes", original_size);
    println!("Compressed size: {} bytes", result.compressed_chars);
    println!("Compression ratio: {:.2}%", result.ratio() * 100.0);
    println!("Strategy: {}", result.strategy);
    println!("Content type: {:?}", result.content_type);
    println!("Did compress: {}", result.did_compress());
    println!("Tokens saved (est): {}", result.tokens_saved_estimate());

    assert!(
        result.did_compress(),
        "Expected compression on noisy CI log"
    );
    assert_eq!(
        result.strategy, "log_compressor",
        "Should use log compressor"
    );
    assert!(
        result.compressed.contains("payment_processor"),
        "Error line must be preserved"
    );
    assert!(
        result.compressed.contains("process_refund"),
        "Error line must be preserved"
    );
    assert!(result.tokens_saved_estimate() > 0);
    println!("\n✓ End-to-end build log compression test passed");
}

#[test]
fn test_e2e_headroom_compression_on_grep_results() {
    use goose::agents::headroom::ContentRouter;

    // Simulate grep output with many matches
    let mut lines = vec![];
    for i in 0..100 {
        lines.push(format!("src/utils.rs:{}:    let x = {};", 20 + i, i));
    }
    lines.insert(42, "src/target.rs:15:fn important_function() {".to_string());
    lines.insert(43, "src/target.rs:16:    // This is critical".to_string());

    let grep_content = lines.join("\n");
    let original_size = grep_content.len();

    let router = ContentRouter::new();
    let result = router.compress(&grep_content, "important_function");

    println!("Grep original size: {} bytes", original_size);
    println!("Grep compressed size: {} bytes", result.compressed_chars);
    println!("Grep compression ratio: {:.2}%", result.ratio() * 100.0);
    println!("Strategy: {}", result.strategy);

    assert!(
        result.did_compress(),
        "Expected compression on grep results"
    );
    assert_eq!(
        result.strategy, "search_compressor",
        "Should use search compressor"
    );
    assert!(result.compressed.contains("important_function"));
    println!("✓ Grep compression test passed");
}

#[test]
fn test_e2e_headroom_compression_on_diff() {
    use goose::agents::headroom::ContentRouter;

    let diff_content = r#"diff --git a/src/main.rs b/src/main.rs
index abc123..def456 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,10 +1,15 @@
 fn main() {
-    println!("old");
+    println!("new");
     let x = 1;
 }
 
 fn helper() {
     // lots of context
+    // some new important change
 }

@@ -50,3 +55,7 @@
 line 50
 line 51
+new line after 51
+another new line
+important fix here
"#;

    let router = ContentRouter::new();
    let result = router.compress(diff_content, "");

    println!("Diff original size: {} bytes", result.original_chars);
    println!("Diff compressed size: {} bytes", result.compressed_chars);
    println!("Diff compression ratio: {:.2}%", result.ratio() * 100.0);
    println!("Strategy: {}", result.strategy);

    assert_eq!(
        result.strategy, "diff_compressor",
        "Should use diff compressor"
    );
    assert_eq!(
        result.content_type,
        goose::agents::headroom::ContentType::GitDiff
    );
    println!("✓ Diff compression test passed");
}


