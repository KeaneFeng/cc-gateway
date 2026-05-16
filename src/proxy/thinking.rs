//! Thinking block sanitizer for proxy forwarding.
//!
//! Claude Code uses extended thinking (thinking blocks in assistant messages).
//! Some providers (e.g. MiMo) require ALL assistant messages to carry reasoning_content
//! when thinking mode is enabled — missing thinking blocks in tool_use chains cause 400.
//!
//! Strategy:
//! 1. Strip existing thinking/redacted_thinking blocks from all assistant messages
//! 2. For any assistant message that starts with non-thinking content (tool_use, text, etc.),
//!    prepend an empty thinking block to satisfy provider requirements
//! 3. Keep top-level `thinking` config intact
//!
//! This is more robust than cc-switch's strip-only approach because MiMo rejects
//! requests even WITHOUT a top-level `thinking` field when assistant messages
//! contain tool_use without thinking blocks.

use serde_json::{json, Value};

/// Sanitize thinking blocks in assistant messages for provider compatibility.
///
/// 1. Strips existing thinking/redacted_thinking blocks (they may have invalid signatures)
/// 2. Prepends empty thinking blocks to assistant messages that don't start with one
///    (MiMo requires ALL assistant messages to have thinking when in thinking mode)
///
/// Modifies body in place and returns true if any changes were made.
pub fn sanitize_thinking_blocks(body: &mut Value) -> bool {
    let messages = match body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        Some(m) => m,
        None => return false,
    };

    let mut changes = 0usize;
    let empty_thinking = json!({"type": "thinking", "thinking": "", "signature": ""});

    for msg in messages.iter_mut() {
        if msg.get("role").and_then(|r| r.as_str()) != Some("assistant") {
            continue;
        }

        let content = match msg.get_mut("content").and_then(|c| c.as_array_mut()) {
            Some(c) => c,
            None => continue,
        };

        // Step 1: Remove existing thinking/redacted_thinking blocks
        let before = content.len();
        content.retain(|block| {
            !matches!(
                block.get("type").and_then(|t| t.as_str()),
                Some("thinking") | Some("redacted_thinking")
            )
        });
        let stripped = before - content.len();
        changes += stripped;

        // Step 2: If content is non-empty and doesn't start with thinking,
        // prepend an empty thinking block
        if !content.is_empty() {
            let first_type = content[0].get("type").and_then(|t| t.as_str());
            if first_type != Some("thinking") && first_type != Some("redacted_thinking") {
                content.insert(0, empty_thinking.clone());
                changes += 1;
            }
        }
    }

    if changes > 0 {
        tracing::info!(
            "🧹 Sanitized thinking blocks in assistant messages ({} changes)",
            changes
        );
    }

    changes > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_sanitize_strips_and_replaces_thinking() {
        let mut body = json!({
            "messages": [
                {"role": "user", "content": "hi"},
                {"role": "assistant", "content": [
                    {"type": "thinking", "thinking": "old thinking", "signature": "sig"},
                    {"type": "text", "text": "Hello!"}
                ]},
                {"role": "user", "content": "bye"}
            ]
        });
        assert!(sanitize_thinking_blocks(&mut body));
        let content = body["messages"][1]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        // First block should be fresh empty thinking
        assert_eq!(content[0]["type"], "thinking");
        assert_eq!(content[0]["thinking"], "");
        // Second block preserved
        assert_eq!(content[1]["type"], "text");
        assert_eq!(content[1]["text"], "Hello!");
    }

    #[test]
    fn test_sanitize_prepends_to_tool_use_only() {
        let mut body = json!({
            "messages": [
                {"role": "user", "content": "do it"},
                {"role": "assistant", "content": [
                    {"type": "tool_use", "name": "bash", "id": "t1", "input": {"cmd": "ls"}}
                ]},
                {"role": "user", "content": "ok"}
            ]
        });
        assert!(sanitize_thinking_blocks(&mut body));
        let content = body["messages"][1]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "thinking");
        assert_eq!(content[0]["thinking"], "");
        assert_eq!(content[1]["type"], "tool_use");
    }

    #[test]
    fn test_sanitize_no_change_when_no_assistant() {
        let mut body = json!({
            "messages": [
                {"role": "user", "content": "hi"},
                {"role": "user", "content": "bye"}
            ]
        });
        assert!(!sanitize_thinking_blocks(&mut body));
    }

    #[test]
    fn test_sanitize_tool_use_chain() {
        let mut body = json!({
            "thinking": {"type": "enabled", "budget_tokens": 50},
            "messages": [
                {"role": "user", "content": "list files"},
                {"role": "assistant", "content": [
                    {"type": "thinking", "thinking": "plan", "signature": ""},
                    {"type": "tool_use", "name": "bash", "id": "t1", "input": {"cmd": "ls"}}
                ]},
                {"role": "user", "content": [{"type": "tool_result", "tool_use_id": "t1", "content": "f1"}]},
                {"role": "assistant", "content": [
                    {"type": "tool_use", "name": "bash", "id": "t2", "input": {"cmd": "cat f1"}}
                ]},
                {"role": "user", "content": "continue"}
            ]
        });
        sanitize_thinking_blocks(&mut body);
        // thinking field preserved
        assert!(body.get("thinking").is_some());
        // First assistant: thinking stripped + empty prepended + tool_use preserved
        let c1 = body["messages"][1]["content"].as_array().unwrap();
        assert_eq!(c1[0]["type"], "thinking");
        assert_eq!(c1[0]["thinking"], "");
        assert_eq!(c1[1]["type"], "tool_use");
        // Second assistant: empty thinking prepended + tool_use preserved
        let c2 = body["messages"][3]["content"].as_array().unwrap();
        assert_eq!(c2[0]["type"], "thinking");
        assert_eq!(c2[0]["thinking"], "");
        assert_eq!(c2[1]["type"], "tool_use");
    }

    #[test]
    fn test_sanitize_preserves_thinking_field() {
        let mut body = json!({
            "thinking": {"type": "enabled", "budget_tokens": 10000},
            "messages": [
                {"role": "user", "content": "hi"},
                {"role": "assistant", "content": [{"type": "text", "text": "ok"}]},
                {"role": "user", "content": "bye"}
            ]
        });
        sanitize_thinking_blocks(&mut body);
        // Top-level thinking should be preserved
        assert!(body.get("thinking").is_some());
        assert_eq!(body["thinking"]["type"], "enabled");
    }
}
