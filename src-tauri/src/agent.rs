//! Minimal ReAct agent loop over the NeuralDeep hub.
//!
//! Uses qwen3.6 (native tool-calls) via /v1/chat/completions with a single
//! `web_search` tool backed by the hub Search API. Runs in a background thread
//! and streams progress to the frontend via Tauri events:
//!   - "agent-progress" : String (a human-readable step line)
//!   - "agent-final"    : String (the final answer)
//!   - "agent-error"    : String

use serde_json::{json, Value};
use std::time::Duration;
use tauri::Emitter;

/// Model with the best native tool-calling on the hub.
const AGENT_MODEL: &str = "qwen3.6-35b-a3b";
const MAX_STEPS: usize = 6;

fn agent_http() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout(Duration::from_secs(90))
        .build()
}

fn normalize_base(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

/// Run the agent loop to completion, emitting progress events on `app`.
pub fn run(app: &tauri::AppHandle, base_url: &str, token: &str, query: &str) {
    if let Err(e) = run_inner(app, base_url, token, query) {
        let _ = app.emit("agent-error", e);
    }
}

fn run_inner(app: &tauri::AppHandle, base_url: &str, token: &str, query: &str) -> Result<(), String> {
    let base = normalize_base(base_url);
    if base.is_empty() || token.trim().is_empty() {
        return Err("Set the NeuralDeep hub URL and token in Settings".to_string());
    }
    let query = query.trim();
    if query.is_empty() {
        return Err("Empty question".to_string());
    }

    let tools = json!([{
        "type": "function",
        "function": {
            "name": "web_search",
            "description": "Search the web for current/factual information. Use it whenever the question needs fresh facts, news, prices, docs or anything you are unsure about.",
            "parameters": {
                "type": "object",
                "properties": { "query": { "type": "string", "description": "search query" } },
                "required": ["query"]
            }
        }
    }]);

    let mut messages: Vec<Value> = vec![
        json!({
            "role": "system",
            "content": "You are a research agent. Break the user's question into web searches, call the web_search tool as many times as needed, then synthesize a concise, well-structured answer in the user's language. Cite source URLs inline. Do not invent facts — search instead."
        }),
        json!({ "role": "user", "content": query }),
    ];

    let url = format!("{}/v1/chat/completions", base);

    for step in 0..MAX_STEPS {
        let _ = app.emit("agent-progress", format!("🤔 Думаю… (шаг {}/{})", step + 1, MAX_STEPS));

        let body = json!({
            "model": AGENT_MODEL,
            "messages": messages,
            "tools": tools,
            "tool_choice": "auto",
            "temperature": 0.2,
            "stream": false
        });

        let resp = agent_http()
            .post(&url)
            .set("Authorization", &format!("Bearer {}", token.trim()))
            .set("Content-Type", "application/json")
            .send_json(body)
            .map_err(|e| match e {
                ureq::Error::Status(code, _) => format!("Hub returned HTTP {}", code),
                other => format!("Hub request failed: {}", other),
            })?;

        let json: Value = resp.into_json().map_err(|e| format!("Bad hub response: {}", e))?;
        let message = &json["choices"][0]["message"];

        let tool_calls = message["tool_calls"].as_array().cloned().unwrap_or_default();

        if tool_calls.is_empty() {
            // Final answer.
            let content = message["content"].as_str().unwrap_or("").trim().to_string();
            if content.is_empty() {
                return Err("Agent returned an empty answer".to_string());
            }
            let _ = app.emit("agent-final", content);
            return Ok(());
        }

        // Record the assistant turn (with its tool_calls) verbatim.
        messages.push(message.clone());

        for tc in &tool_calls {
            let name = tc["function"]["name"].as_str().unwrap_or("");
            let id = tc["id"].as_str().unwrap_or("").to_string();
            let args_raw = tc["function"]["arguments"].as_str().unwrap_or("{}");
            let args: Value = serde_json::from_str(args_raw).unwrap_or(json!({}));

            let result = if name == "web_search" {
                let q = args["query"].as_str().unwrap_or("").to_string();
                let _ = app.emit("agent-progress", format!("🔎 Ищу: {}", q));
                crate::hub::web_search(&base, token, &q, 5)
                    .unwrap_or_else(|e| format!("(search failed: {})", e))
            } else {
                format!("(unknown tool: {})", name)
            };

            messages.push(json!({
                "role": "tool",
                "tool_call_id": id,
                "content": result
            }));
        }
    }

    let _ = app.emit(
        "agent-error",
        "Достигнут лимит шагов — попробуй переформулировать вопрос".to_string(),
    );
    Ok(())
}
