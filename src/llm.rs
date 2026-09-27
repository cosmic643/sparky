use std::{env, time::Duration};

use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

const GATEWAY_URL: &str = "https://ai-gateway.vercel.sh/v1/chat/completions";
const DEFAULT_MODEL: &str = "openai/gpt-6-luna";

#[derive(Serialize)]
struct GatewayRequest<'a> {
    model: &'a str,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u16
}

#[derive(Serialize)]
struct Message{
    role: &'static str,
    content: String,
}

#[derive(Deserialize)]
struct GatewayResponse{
    choices: Vec<Choice>    
}

#[derive(Deserialize)]
struct Choice{
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage{
    content: String,
}


pub fn generate_command(user_request: &str) -> Result<String> {
    if user_request.trim().is_empty(){
        bail!("The command description cannot be empty");
    }
    let api_key = env::var("AI_GATEWAY_API_KEY")
        .ok()
        .filter(|key| !key.trim().is_empty())
        .context("AI_GATEWAY_API_KEY is not set")?;

    let model = env::var("SPK_MODEL")
        .unwrap_or_else(|_| DEFAULT_MODEL.to_owned());

    let shell = env::var("SHELL")
        .unwrap_or_else(|_| "unknown".to_owned());

    let system_prompt = format!(
        "Translate natural-language requests into shell commands. \
        Return exactly one executable shell command and nothing else. \
        Do not include markdown, code fences, backticks, explanations, \
        alternatives, or a leading dollar sign. \
        Prefer safe, non-destructive commands and do not add sudo unless requested. \
        Target operating system: {}. Target shell: {}.",
        env::consts::OS,
        shell
    );

    let request = GatewayRequest{
        model: &model,
        messages: vec![
            Message {
                role: "system",
                content: system_prompt,
            },
            Message {
                role: "user",
                content: user_request.trim().to_owned(),
            },
        ],
        temperature: 0.0,
        max_tokens: 1000, // reasoning models spend tokens before answering
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("failed to create HTTP client")?;

    let response = client
        .post(GATEWAY_URL)
        .bearer_auth(api_key)
        .json(&request)
        .send()
        .context("failed to contact Vercel AI Gateway")?;

    let status = response.status();

    let response_body = response
        .text()
        .context("failed to read the Gateway response")?;

    if !status.is_success(){
        bail!("Gateway request failed with status {status}: {response_body}");
    }

    let gateway_response: GatewayResponse = serde_json::from_str(&response_body).context("failed to parse the Gateway Response")?;

    let content = gateway_response
        .choices
        .into_iter()
        .next()
        .context("the Gateway response contained no choices")?
        .message
        .content;

    validate_command(&content)
}

fn validate_command(content: &str) -> Result<String> {
    let lines: Vec<&str> = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("```"))
        .collect();

    if lines.len() != 1 {
        bail!("the model did not return exactly one command:\n{content}");
    }

    let command = lines[0];

    if command.starts_with("$ ") {
        bail!("the model included a shell prompt");
    }

    if command.chars().any(char::is_control) {
        bail!("the model returned control characters");
    }

    Ok(command.to_owned())
}