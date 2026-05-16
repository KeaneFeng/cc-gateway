//! Format conversion module
//!
//! Implements Anthropic ↔ OpenAI format conversion for multi-provider routing

use serde_json::{json, Value};

/// Anthropic request → OpenAI Chat Completions request
pub fn anthropic_to_openai(body: Value) -> Result<Value, String> {
    let mut result = json!({});

    // Copy model
    if let Some(model) = body.get("model").and_then(|m| m.as_str()) {
        result["model"] = json!(model);
    }

    let mut messages = Vec::new();

    // Handle system prompt
    if let Some(system) = body.get("system") {
        if let Some(text) = system.as_str() {
            if !text.is_empty() {
                messages.push(json!({"role": "system", "content": text}));
            }
        } else if let Some(arr) = system.as_array() {
            for msg in arr {
                if let Some(text) = msg.get("text").and_then(|t| t.as_str()) {
                    if !text.is_empty() {
                        let mut sys_msg = json!({"role": "system", "content": text});
                        if let Some(cc) = msg.get("cache_control") {
                            sys_msg["cache_control"] = cc.clone();
                        }
                        messages.push(sys_msg);
                    }
                }
            }
        }
    }

    // Convert messages
    if let Some(msgs) = body.get("messages").and_then(|m| m.as_array()) {
        for msg in msgs {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
            let content = msg.get("content");
            let converted = convert_message_to_openai(role, content)?;
            messages.extend(converted);
        }
    }

    // Normalize system messages
    normalize_system_messages(&mut messages);
    result["messages"] = json!(messages);

    // Convert parameters
    if let Some(v) = body.get("max_tokens") {
        result["max_tokens"] = v.clone();
    }
    if let Some(v) = body.get("temperature") {
        result["temperature"] = v.clone();
    }
    if let Some(v) = body.get("top_p") {
        result["top_p"] = v.clone();
    }
    if let Some(v) = body.get("stop_sequences") {
        result["stop"] = v.clone();
    }
    if let Some(v) = body.get("stream") {
        result["stream"] = v.clone();
    }

    // Convert tools
    if let Some(tools) = body.get("tools").and_then(|t| t.as_array()) {
        let openai_tools: Vec<Value> = tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.get("name").unwrap_or(&json!("")),
                        "description": tool.get("description").unwrap_or(&json!("")),
                        "parameters": tool.get("input_schema").unwrap_or(&json!({}))
                    }
                })
            })
            .collect();
        result["tools"] = json!(openai_tools);
    }

    // Convert tool_choice
    if let Some(tool_choice) = body.get("tool_choice") {
        if let Some(tc_type) = tool_choice.get("type").and_then(|t| t.as_str()) {
            match tc_type {
                "auto" => result["tool_choice"] = json!("auto"),
                "any" => result["tool_choice"] = json!("required"),
                "tool" => {
                    if let Some(name) = tool_choice.get("name").and_then(|n| n.as_str()) {
                        result["tool_choice"] = json!({
                            "type": "function",
                            "function": {"name": name}
                        });
                    }
                }
                _ => {}
            }
        }
    }

    Ok(result)
}

/// Convert a single message from Anthropic to OpenAI format
fn convert_message_to_openai(role: &str, content: Option<&Value>) -> Result<Vec<Value>, String> {
    let mut messages = Vec::new();

    match role {
        "user" | "assistant" => {
            if let Some(content) = content {
                if let Some(text) = content.as_str() {
                    messages.push(json!({"role": role, "content": text}));
                } else if let Some(arr) = content.as_array() {
                    let mut text_parts = Vec::new();
                    let mut tool_calls = Vec::new();
                    let mut tool_results = Vec::new();

                    for part in arr {
                        match part.get("type").and_then(|t| t.as_str()) {
                            Some("text") => {
                                if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                    text_parts.push(text.to_string());
                                }
                            }
                            Some("tool_use") => {
                                let id = part.get("id").and_then(|i| i.as_str()).unwrap_or("");
                                let name = part.get("name").and_then(|n| n.as_str()).unwrap_or("");
                                let default_input = json!({});
                                let input = part.get("input").unwrap_or(&default_input);
                                tool_calls.push(json!({
                                    "id": id,
                                    "type": "function",
                                    "function": {
                                        "name": name,
                                        "arguments": serde_json::to_string(input).unwrap_or_default()
                                    }
                                }));
                            }
                            Some("tool_result") => {
                                let tool_use_id = part
                                    .get("tool_use_id")
                                    .and_then(|i| i.as_str())
                                    .unwrap_or("");
                                let content = part.get("content");
                                let result_content =
                                    if let Some(text) = content.and_then(|c| c.as_str()) {
                                        text.to_string()
                                    } else if let Some(arr) = content.and_then(|c| c.as_array()) {
                                        arr.iter()
                                            .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                                            .collect::<Vec<_>>()
                                            .join("\n")
                                    } else {
                                        String::new()
                                    };
                                tool_results.push(json!({
                                    "role": "tool",
                                    "tool_call_id": tool_use_id,
                                    "content": result_content
                                }));
                            }
                            _ => {}
                        }
                    }

                    if role == "assistant" && !tool_calls.is_empty() {
                        let msg = json!({
                            "role": "assistant",
                            "content": if text_parts.is_empty() { Value::Null } else { json!(text_parts.join("\n")) },
                            "tool_calls": tool_calls
                        });
                        messages.push(msg);
                    } else if !text_parts.is_empty() {
                        messages.push(json!({"role": role, "content": text_parts.join("\n")}));
                    }

                    messages.extend(tool_results);
                }
            }
        }
        _ => {}
    }

    Ok(messages)
}

/// Normalize system messages (merge consecutive system messages)
fn normalize_system_messages(messages: &mut Vec<Value>) {
    let mut i = 0;
    while i < messages.len() - 1 {
        if messages[i].get("role").and_then(|r| r.as_str()) == Some("system")
            && messages[i + 1].get("role").and_then(|r| r.as_str()) == Some("system")
        {
            let content1 = messages[i]
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("");
            let content2 = messages[i + 1]
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("");
            let merged = format!("{}\n{}", content1, content2);
            messages[i]["content"] = json!(merged);
            messages.remove(i + 1);
        } else {
            i += 1;
        }
    }
}

/// OpenAI response → Anthropic response
pub fn openai_to_anthropic_response(openai_response: Value, model: &str) -> Value {
    let id = format!("msg_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));

    let mut anthropic_response = json!({
        "id": id,
        "type": "message",
        "role": "assistant",
        "model": model,
        "stop_reason": null,
        "stop_sequence": null,
        "usage": {
            "input_tokens": 0,
            "output_tokens": 0
        }
    });

    // Extract content from OpenAI response
    if let Some(choices) = openai_response.get("choices").and_then(|c| c.as_array()) {
        if let Some(choice) = choices.first() {
            if let Some(message) = choice.get("message") {
                let mut content = Vec::new();

                // Text content
                if let Some(text) = message.get("content").and_then(|t| t.as_str()) {
                    if !text.is_empty() {
                        content.push(json!({"type": "text", "text": text}));
                    }
                }

                // Tool calls
                if let Some(tool_calls) = message.get("tool_calls").and_then(|t| t.as_array()) {
                    for tc in tool_calls {
                        let id = tc.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        let name = tc
                            .get("function")
                            .and_then(|f| f.get("name"))
                            .and_then(|n| n.as_str())
                            .unwrap_or("");
                        let arguments = tc
                            .get("function")
                            .and_then(|f| f.get("arguments"))
                            .and_then(|a| a.as_str())
                            .unwrap_or("{}");
                        let input: Value = serde_json::from_str(arguments).unwrap_or(json!({}));
                        content.push(json!({
                            "type": "tool_use",
                            "id": id,
                            "name": name,
                            "input": input
                        }));
                    }
                }

                anthropic_response["content"] = json!(content);
            }

            // Map finish reason
            if let Some(finish_reason) = choice.get("finish_reason").and_then(|f| f.as_str()) {
                anthropic_response["stop_reason"] = match finish_reason {
                    "stop" => json!("end_turn"),
                    "length" => json!("max_tokens"),
                    "tool_calls" => json!("tool_use"),
                    _ => json!("end_turn"),
                };
            }
        }
    }

    // Extract usage
    if let Some(usage) = openai_response.get("usage") {
        anthropic_response["usage"] = json!({
            "input_tokens": usage.get("prompt_tokens").and_then(|t| t.as_u64()).unwrap_or(0),
            "output_tokens": usage.get("completion_tokens").and_then(|t| t.as_u64()).unwrap_or(0)
        });
    }

    anthropic_response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_to_openai_basic() {
        let anthropic = json!({
            "model": "claude-sonnet-4",
            "max_tokens": 1024,
            "system": "You are a helpful assistant.",
            "messages": [
                {"role": "user", "content": "Hello!"}
            ]
        });

        let openai = anthropic_to_openai(anthropic).unwrap();
        assert_eq!(openai["model"], "claude-sonnet-4");
        assert_eq!(openai["max_tokens"], 1024);
        assert!(openai["messages"].as_array().unwrap().len() == 2); // system + user
    }

    #[test]
    fn test_openai_to_anthropic_response() {
        let openai = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help you?"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20
            }
        });

        let anthropic = openai_to_anthropic_response(openai, "claude-sonnet-4");
        assert_eq!(anthropic["type"], "message");
        assert_eq!(anthropic["role"], "assistant");
        assert_eq!(anthropic["stop_reason"], "end_turn");
    }
}
