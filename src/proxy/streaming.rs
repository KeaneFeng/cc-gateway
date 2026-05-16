//! Streaming module
//!
//! Handles SSE streaming conversion from OpenAI to Anthropic format

use futures::stream::Stream;
use serde_json::{json, Value};
use std::pin::Pin;

/// Convert OpenAI SSE stream to Anthropic SSE stream
pub fn openai_to_anthropic_stream(
    stream: impl Stream<Item = Result<String, std::io::Error>> + Send + Unpin + 'static,
    model: String,
    message_id: String,
) -> Pin<Box<dyn Stream<Item = Result<String, std::io::Error>> + Send>> {
    let stream = futures::stream::unfold(
        (stream, model, message_id, false),
        |(mut stream, model, message_id, started)| async move {
            loop {
                match futures::StreamExt::next(&mut stream).await {
                    Some(Ok(line)) => {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }

                        if line == "data: [DONE]" {
                            // Send message_stop
                            let event = "event: message_stop\ndata: {}\n\n".to_string();
                            return Some((Ok(event), (stream, model, message_id, true)));
                        }

                        // Parse SSE line
                        let data = if let Some(d) = line.strip_prefix("data: ") {
                            d
                        } else {
                            continue;
                        };

                        let openai_chunk: Value = match serde_json::from_str(data) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };

                        let mut events = Vec::new();

                        // First chunk: send message_start
                        if !started {
                            let message_start = json!({
                                "type": "message_start",
                                "message": {
                                    "id": message_id,
                                    "type": "message",
                                    "role": "assistant",
                                    "model": model,
                                    "content": [],
                                    "stop_reason": null,
                                    "stop_sequence": null,
                                    "usage": {
                                        "input_tokens": 0,
                                        "output_tokens": 0
                                    }
                                }
                            });
                            events
                                .push(format!("event: message_start\ndata: {}\n\n", message_start));

                            // Start content block
                            let content_block_start = json!({
                                "type": "content_block_start",
                                "index": 0,
                                "content_block": {
                                    "type": "text",
                                    "text": ""
                                }
                            });
                            events.push(format!(
                                "event: content_block_start\ndata: {}\n\n",
                                content_block_start
                            ));
                        }

                        // Process delta
                        if let Some(choices) =
                            openai_chunk.get("choices").and_then(|c| c.as_array())
                        {
                            if let Some(choice) = choices.first() {
                                if let Some(delta) = choice.get("delta") {
                                    // Text content
                                    if let Some(content) =
                                        delta.get("content").and_then(|c| c.as_str())
                                    {
                                        if !content.is_empty() {
                                            let delta = json!({
                                                "type": "content_block_delta",
                                                "index": 0,
                                                "delta": {
                                                    "type": "text_delta",
                                                    "text": content
                                                }
                                            });
                                            events.push(format!(
                                                "event: content_block_delta\ndata: {}\n\n",
                                                delta
                                            ));
                                        }
                                    }

                                    // Tool calls
                                    if let Some(tool_calls) =
                                        delta.get("tool_calls").and_then(|t| t.as_array())
                                    {
                                        for tc in tool_calls {
                                            let index = tc
                                                .get("index")
                                                .and_then(|i| i.as_u64())
                                                .unwrap_or(0)
                                                as usize;
                                            let function = tc.get("function");
                                            let name = function
                                                .and_then(|f| f.get("name"))
                                                .and_then(|n| n.as_str());
                                            let arguments = function
                                                .and_then(|f| f.get("arguments"))
                                                .and_then(|a| a.as_str());

                                            if let Some(name) = name {
                                                // Start tool use block
                                                let content_block_start = json!({
                                                    "type": "content_block_start",
                                                    "index": index + 1,
                                                    "content_block": {
                                                        "type": "tool_use",
                                                        "id": format!("toolu_{}", uuid::Uuid::new_v4()),
                                                        "name": name,
                                                        "input": {}
                                                    }
                                                });
                                                events.push(format!(
                                                    "event: content_block_start\ndata: {}\n\n",
                                                    content_block_start
                                                ));
                                            }

                                            if let Some(args) = arguments {
                                                if !args.is_empty() {
                                                    let delta = json!({
                                                        "type": "content_block_delta",
                                                        "index": index + 1,
                                                        "delta": {
                                                            "type": "input_json_delta",
                                                            "partial_json": args
                                                        }
                                                    });
                                                    events.push(format!(
                                                        "event: content_block_delta\ndata: {}\n\n",
                                                        delta
                                                    ));
                                                }
                                            }
                                        }
                                    }

                                    // Check for finish reason
                                    if let Some(finish_reason) =
                                        choice.get("finish_reason").and_then(|f| f.as_str())
                                    {
                                        // Stop content block
                                        events.push(
                                            "event: content_block_stop\ndata: {}\n\n".to_string(),
                                        );

                                        // Send message_delta with stop_reason
                                        let stop_reason = match finish_reason {
                                            "stop" => "end_turn",
                                            "length" => "max_tokens",
                                            "tool_calls" => "tool_use",
                                            _ => "end_turn",
                                        };

                                        let message_delta = json!({
                                            "type": "message_delta",
                                            "delta": {
                                                "stop_reason": stop_reason,
                                                "stop_sequence": null
                                            },
                                            "usage": {
                                                "output_tokens": 0
                                            }
                                        });
                                        events.push(format!(
                                            "event: message_delta\ndata: {}\n\n",
                                            message_delta
                                        ));
                                        events
                                            .push("event: message_stop\ndata: {}\n\n".to_string());

                                        let output = events.join("");
                                        return Some((
                                            Ok(output),
                                            (stream, model, message_id, true),
                                        ));
                                    }
                                }
                            }
                        }

                        if !events.is_empty() {
                            let output = events.join("");
                            return Some((Ok(output), (stream, model, message_id, true)));
                        }
                    }
                    Some(Err(e)) => {
                        return Some((
                            Err(std::io::Error::other(e.to_string())),
                            (stream, model, message_id, started),
                        ));
                    }
                    None => {
                        // Stream ended without [DONE]
                        let mut events = Vec::new();
                        if !started {
                            // Empty response
                            let message_start = json!({
                                "type": "message_start",
                                "message": {
                                    "id": message_id,
                                    "type": "message",
                                    "role": "assistant",
                                    "model": model,
                                    "content": [],
                                    "stop_reason": null,
                                    "stop_sequence": null,
                                    "usage": {"input_tokens": 0, "output_tokens": 0}
                                }
                            });
                            events
                                .push(format!("event: message_start\ndata: {}\n\n", message_start));
                        }
                        events.push("event: message_stop\ndata: {}\n\n".to_string());
                        let output = events.join("");
                        return Some((Ok(output), (stream, model, message_id, true)));
                    }
                }
            }
        },
    );

    Box::pin(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stream_conversion() {
        let openai_chunks = vec![
            Ok(
                r#"data: {"choices":[{"delta":{"role":"assistant","content":"Hello"}}]}"#
                    .to_string(),
            ),
            Ok(r#"data: {"choices":[{"delta":{"content":" world"}}]}"#.to_string()),
            Ok(r#"data: {"choices":[{"delta":{},"finish_reason":"stop"}]}"#.to_string()),
            Ok("data: [DONE]".to_string()),
        ];

        let stream = futures::stream::iter(openai_chunks);
        let anthropic_stream =
            openai_to_anthropic_stream(stream, "test-model".to_string(), "msg_test".to_string());

        let results: Vec<_> = futures::StreamExt::collect(anthropic_stream).await;
        assert!(!results.is_empty());
    }
}
