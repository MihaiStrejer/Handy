use crate::managers::history_processing::RunGuard;
use crate::settings::PostProcessProvider;
use log::{debug, error, info};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, REFERER, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::error::Error as StdError;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct JsonSchema {
    name: String,
    strict: bool,
    schema: Value,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
    json_schema: JsonSchema,
}

#[derive(Debug, Serialize, Clone, Default, PartialEq)]
struct ReasoningConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exclude: Option<bool>,
}

/// Request fields used to ask an endpoint to skip reasoning/thinking.
/// Providers disagree on the field name and accepted values, so at most one of
/// these is set per request (see `reasoning_disable_params`).
#[derive(Debug, Serialize, Clone, Default, PartialEq)]
struct ReasoningParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<ReasoningConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<Value>,
}

impl ReasoningParams {
    fn is_empty(&self) -> bool {
        self.reasoning_effort.is_none() && self.reasoning.is_none() && self.thinking.is_none()
    }
}

/// Pick the reasoning-disable request fields an endpoint understands.
/// Unknown endpoints get the common OpenAI-style field; if they reject it,
/// the request is retried without it (see `send_chat_completion_with_schema`).
fn reasoning_disable_params(provider: &PostProcessProvider) -> ReasoningParams {
    let base_url = provider.base_url.to_lowercase();
    if base_url.contains("api.deepseek.com") {
        // DeepSeek rejects reasoning_effort "none" and uses its own field:
        // https://api-docs.deepseek.com/guides/thinking_mode
        ReasoningParams {
            thinking: Some(serde_json::json!({ "type": "disabled" })),
            ..Default::default()
        }
    } else if provider.id == "openrouter" {
        // OpenRouter nested object; exclude:true also keeps reasoning text out
        // of the response so it can't pollute structured-output JSON parsing
        ReasoningParams {
            reasoning: Some(ReasoningConfig {
                effort: Some("none".to_string()),
                exclude: Some(true),
            }),
            ..Default::default()
        }
    } else {
        ReasoningParams {
            reasoning_effort: Some("none".to_string()),
            ..Default::default()
        }
    }
}

/// Endpoints (base_url|model) that rejected the reasoning-disable fields with a
/// 4xx. Remembered for the lifetime of the process so every dictation after the
/// first skips the doomed attempt and goes straight to a plain request.
fn reasoning_rejections() -> &'static Mutex<HashSet<String>> {
    static REJECTED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    REJECTED.get_or_init(|| Mutex::new(HashSet::new()))
}

fn endpoint_key(provider: &PostProcessProvider, model: &str) -> String {
    format!("{}|{}", provider.base_url.trim_end_matches('/'), model)
}

fn is_known_rejected(key: &str) -> bool {
    reasoning_rejections()
        .lock()
        .map(|set| set.contains(key))
        .unwrap_or(false)
}

fn remember_rejection(key: String) {
    if let Ok(mut set) = reasoning_rejections().lock() {
        set.insert(key);
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parallel_tool_calls: Option<bool>,
    #[serde(flatten)]
    reasoning: ReasoningParams,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

/// A run is supplied by its caller; this client never selects mutable global history state.
pub struct CallContext<'a> {
    pub run: &'a RunGuard,
    pub purpose: &'a str,
    pub retry_of: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatMessageResponse {
    content: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
    refusal: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ToolCall {
    // Some local OpenAI-compatible servers omit `id` and `type` (or send null).
    // They must not fail the whole body parse; Handy never sends a second turn,
    // so the id is only bounded, and a present `type` must still be "function".
    #[serde(default)]
    id: Option<String>,
    #[serde(rename = "type", default)]
    kind: Option<String>,
    function: ToolFunction,
}
#[derive(Debug, Deserialize)]
struct ToolFunction {
    name: String,
    arguments: String,
}

/// Build headers for API requests based on provider type
fn build_headers(provider: &PostProcessProvider, api_key: &str) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();

    // Common headers
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        REFERER,
        HeaderValue::from_static("https://github.com/cjpais/Handy"),
    );
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("Handy/1.0 (+https://github.com/cjpais/Handy)"),
    );
    headers.insert("X-Title", HeaderValue::from_static("Handy"));

    // Provider-specific auth headers
    if !api_key.is_empty() {
        if provider.id == "anthropic" {
            headers.insert(
                "x-api-key",
                HeaderValue::from_str(api_key)
                    .map_err(|e| format!("Invalid API key header value: {}", e))?,
            );
            headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        } else {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", api_key))
                    .map_err(|e| format!("Invalid authorization header value: {}", e))?,
            );
        }
    }

    Ok(headers)
}

/// Create an HTTP client with provider-specific headers
fn create_client(provider: &PostProcessProvider, api_key: &str) -> Result<reqwest::Client, String> {
    let headers = build_headers(provider, api_key)?;
    reqwest::Client::builder()
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| report_reqwest_error("Failed to build HTTP client", &e))
}

/// Format a bounded error source chain.
///
/// `reqwest::Error`'s Display implementation intentionally gives only a short
/// summary. Nested causes contain the useful transport details, such as a
/// certificate validation failure, an HTTP/2 error, or a connection reset.
/// Callers must skip source types whose Display text can quote payload data.
fn error_source_chain(error: &(dyn StdError + 'static)) -> Vec<String> {
    let mut causes = Vec::new();
    let mut source = error.source();

    // Defensive cap in case a third-party error exposes a cyclic source chain.
    for _ in 0..16 {
        let Some(cause) = source else {
            break;
        };
        causes.push(cause.to_string());
        source = cause.source();
    }

    causes
}

fn reqwest_error_kinds(error: &reqwest::Error) -> String {
    let mut kinds = Vec::new();

    if error.is_builder() {
        kinds.push("builder");
    }
    if error.is_connect() {
        kinds.push("connect");
    }
    if error.is_request() {
        kinds.push("request");
    }
    if error.is_redirect() {
        kinds.push("redirect");
    }
    if error.is_timeout() {
        kinds.push("timeout");
    }
    if error.is_status() {
        kinds.push("status");
    }
    if error.is_body() {
        kinds.push("body");
    }
    if error.is_decode() {
        kinds.push("decode");
    }
    if error.is_upgrade() {
        kinds.push("upgrade");
    }

    if kinds.is_empty() {
        "unknown".to_string()
    } else {
        kinds.join(", ")
    }
}

fn sanitized_url(url: &reqwest::Url) -> String {
    let mut url = url.clone();

    // Custom endpoints should not contain credentials or query-string tokens,
    // but omit them from diagnostics in case one does.
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);

    url.to_string()
}

fn sanitized_url_for_log(url: &str) -> String {
    reqwest::Url::parse(url)
        .map(|url| sanitized_url(&url))
        // Do not echo an invalid URL: the parse failure might have been caused
        // by sensitive data entered in the custom endpoint field.
        .unwrap_or_else(|_| "<invalid URL>".to_string())
}

fn report_reqwest_error(context: &str, error: &reqwest::Error) -> String {
    let kinds = reqwest_error_kinds(error);
    let url = error
        .url()
        .map(sanitized_url)
        .map(|url| format!(", url: {url}"))
        .unwrap_or_default();

    // serde_json's error text can quote values from a malformed response. That
    // response may contain transcription content, so retain the useful decode
    // classification but never put its nested source in logs or UI errors.
    let causes = if error.is_decode() {
        Vec::new()
    } else {
        error_source_chain(error)
    };
    let cause_details = if !causes.is_empty() {
        format!(": caused by: {}", causes.join(" -> "))
    } else if error.url().is_none() {
        // Reqwest's short Display text is safe when it cannot append a raw URL.
        format!(": {error}")
    } else {
        // The sanitized URL is already included above. Avoid formatting the
        // original error because its Display implementation includes the raw URL.
        String::new()
    };

    let details = format!("{context} (kind: {kinds}{url}){cause_details}");
    error!("{details}");
    details
}

/// Send a chat completion request to an OpenAI-compatible API
/// Returns Ok(Some(content)) on success, Ok(None) if response has no content,
/// or Err on actual errors (HTTP, parsing, etc.)
pub async fn send_chat_completion(
    provider: &PostProcessProvider,
    api_key: String,
    model: &str,
    prompt: String,
    disable_reasoning: bool,
) -> Result<Option<String>, String> {
    send_chat_completion_with_schema(
        provider,
        api_key,
        model,
        prompt,
        None,
        None,
        disable_reasoning,
    )
    .await
}

/// Send a chat completion request with structured output support.
/// When json_schema is provided, uses structured outputs mode.
/// system_prompt is used as the system message when provided.
///
/// When disable_reasoning is set, the request carries the reasoning-disable
/// fields the endpoint is expected to understand. Not every OpenAI-compatible
/// endpoint accepts them (DeepSeek, Gemini's compat layer, and some OpenRouter
/// upstreams reject with 400), so a 400/422 answer to such a request triggers
/// one retry without the fields, and the rejection is remembered per
/// (base_url, model) so later requests skip the failing attempt entirely.
pub async fn send_chat_completion_with_schema(
    provider: &PostProcessProvider,
    api_key: String,
    model: &str,
    user_content: String,
    system_prompt: Option<String>,
    json_schema: Option<Value>,
    disable_reasoning: bool,
) -> Result<Option<String>, String> {
    send_chat_completion_observed(
        provider,
        api_key,
        model,
        user_content,
        system_prompt,
        json_schema,
        disable_reasoning,
        None,
    )
    .await
}

pub async fn send_chat_completion_observed(
    provider: &PostProcessProvider,
    api_key: String,
    model: &str,
    user_content: String,
    system_prompt: Option<String>,
    json_schema: Option<Value>,
    disable_reasoning: bool,
    observation: Option<CallContext<'_>>,
) -> Result<Option<String>, String> {
    send_chat_completion_configured(
        provider,
        api_key,
        model,
        user_content,
        system_prompt,
        json_schema,
        disable_reasoning,
        observation,
        CompletionOptions::default(),
    )
    .await
}

pub(crate) struct CompletionOptions {
    pub schema_name: &'static str,
    pub response_limit: usize,
    pub tool_schema: Option<Value>,
    pub strict_response: bool,
}
impl Default for CompletionOptions {
    fn default() -> Self {
        Self {
            schema_name: "transcription_output",
            response_limit: 2 * 1024 * 1024,
            tool_schema: None,
            strict_response: false,
        }
    }
}

pub(crate) async fn send_chat_completion_configured(
    provider: &PostProcessProvider,
    api_key: String,
    model: &str,
    user_content: String,
    system_prompt: Option<String>,
    json_schema: Option<Value>,
    disable_reasoning: bool,
    observation: Option<CallContext<'_>>,
    options: CompletionOptions,
) -> Result<Option<String>, String> {
    let base_url = provider.base_url.trim_end_matches('/');
    let url = format!("{}/chat/completions", base_url);

    debug!(
        "Sending chat completion request to: {}",
        sanitized_url_for_log(&url)
    );

    let client = create_client(provider, &api_key)?;

    // Build messages vector
    let mut messages = Vec::new();

    // Add system prompt if provided
    if let Some(system) = system_prompt {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: system,
        });
    }

    // Add user message
    messages.push(ChatMessage {
        role: "user".to_string(),
        content: user_content,
    });

    // Build response_format if schema is provided
    let response_format = json_schema.map(|schema| ResponseFormat {
        format_type: "json_schema".to_string(),
        json_schema: JsonSchema {
            name: options.schema_name.to_string(),
            strict: true,
            schema,
        },
    });

    let key = endpoint_key(provider, model);
    let reasoning = if disable_reasoning && !is_known_rejected(&key) {
        reasoning_disable_params(provider)
    } else {
        ReasoningParams::default()
    };

    let mut request_body = ChatCompletionRequest {
        model: model.to_string(),
        messages,
        stream: false,
        response_format,
        reasoning,
        tools: options.tool_schema.as_ref().map(|schema| serde_json::json!([{
            "type":"function","function":{"name":"submit_rewrite","description":"Submit the final rewrite and staged short-term corrections.",
                "strict":true,"parameters":schema}
        }])),
        tool_choice: options.tool_schema.as_ref().map(|_| serde_json::json!({"type":"function","function":{"name":"submit_rewrite"}})),
        parallel_tool_calls: options.tool_schema.as_ref().map(|_| false),
    };

    let mut previous_call = observation.as_ref().and_then(|context| context.retry_of);
    let mut response = send_attempt(
        &client,
        &url,
        &request_body,
        provider,
        model,
        observation.as_ref(),
        &mut previous_call,
        options.response_limit,
    )
    .await?;
    let mut status = response.0;
    debug!(
        "Chat completion response received with status {} from {}",
        status,
        sanitized_url_for_log(&url)
    );

    // A 400/422 on a request carrying reasoning-disable fields is almost always
    // the endpoint rejecting those fields — retry once without them.
    if !status.is_success()
        && matches!(status.as_u16(), 400 | 422)
        && !request_body.reasoning.is_empty()
    {
        info!(
            "Endpoint rejected request with reasoning disabled (status {}). Retrying without reasoning fields",
            status
        );

        request_body.reasoning = ReasoningParams::default();
        response = send_attempt(
            &client,
            &url,
            &request_body,
            provider,
            model,
            observation.as_ref(),
            &mut previous_call,
            options.response_limit,
        )
        .await?;
        status = response.0;
        debug!(
            "Chat completion retry response received with status {} from {}",
            status,
            sanitized_url_for_log(&url)
        );

        if status.is_success() {
            info!(
                "Retry without reasoning fields succeeded; '{}' (model '{}') will skip them from now on",
                sanitized_url_for_log(base_url), model
            );
            remember_rejection(key);
        }
    }

    if !status.is_success() {
        // Endpoints may echo user input in their error bodies.
        return Err(format!("API request failed with status {}", status));
    }

    let completion: ChatCompletionResponse = serde_json::from_slice(&response.1)
        .map_err(|_| "Endpoint returned invalid response JSON".to_string())?;

    completion_content(
        completion,
        options.tool_schema.is_some(),
        options.strict_response,
    )
}

fn completion_content(
    completion: ChatCompletionResponse,
    native: bool,
    strict: bool,
) -> Result<Option<String>, String> {
    if (native || strict) && completion.choices.len() != 1 {
        return Err("Endpoint returned an ambiguous submission".into());
    }
    let Some(choice) = completion.choices.first() else {
        return Ok(None);
    };
    if choice
        .message
        .refusal
        .as_ref()
        .is_some_and(|text| !text.is_empty())
        || matches!(
            choice.finish_reason.as_deref(),
            Some("length" | "content_filter")
        )
    {
        return Err("Endpoint refused or truncated the submission".into());
    }
    if native {
        let Some(calls) = &choice.message.tool_calls else {
            return Err("Endpoint did not return the forced submission".into());
        };
        if calls.len() != 1 {
            return Err("Endpoint returned an ambiguous submission".into());
        }
        let call = &calls[0];
        // A forced named function yields "tool_calls" on some servers and "stop"
        // (or no reason) on OpenAI and others. Every other reason is rejected.
        if call.kind.as_deref().is_some_and(|kind| kind != "function")
            || call.function.name != "submit_rewrite"
            || call.id.as_ref().is_some_and(|id| id.len() > 128)
            || !matches!(
                choice.finish_reason.as_deref(),
                None | Some("tool_calls" | "stop")
            )
            || choice
                .message
                .content
                .as_ref()
                .is_some_and(|text| !text.trim().is_empty())
            || call.function.arguments.len() > 128_000
        {
            return Err("Endpoint returned an invalid submission function".into());
        }
        Ok(Some(call.function.arguments.clone()))
    } else {
        if choice
            .message
            .tool_calls
            .as_ref()
            .is_some_and(|calls| !calls.is_empty())
        {
            return Err("Endpoint returned an unexpected submission function".into());
        }
        Ok(choice.message.content.clone())
    }
}

async fn read_bounded_response(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<Vec<u8>, String> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err("Response body exceeds its limit".into());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Could not read endpoint response")?
    {
        if body.len().saturating_add(chunk.len()) > limit {
            return Err("Response body exceeds its limit".into());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

async fn send_attempt(
    client: &reqwest::Client,
    url: &str,
    body: &ChatCompletionRequest,
    provider: &PostProcessProvider,
    model: &str,
    observation: Option<&CallContext<'_>>,
    previous_call: &mut Option<i64>,
    response_limit: usize,
) -> Result<(reqwest::StatusCode, Vec<u8>), String> {
    // The archived string is passed unchanged as the HTTP body. It contains no headers.
    let body_json =
        serde_json::to_string(body).map_err(|_| "Could not serialize request".to_string())?;
    let mut guard = observation.and_then(|context| {
        match context.run.start_call(
            context.purpose,
            *previous_call,
            &provider.id,
            &provider.label,
            url,
            model,
            &body_json,
        ) {
            Ok(guard) => Some(guard),
            Err(error) => {
                error!("Details could not be saved: {error}");
                context.run.mark_incomplete();
                None
            }
        }
    });
    if let Some(ref guard) = guard {
        *previous_call = Some(guard.id());
    }
    let response = match client.post(url).body(body_json).send().await {
        Ok(response) => response,
        Err(error) => {
            if let Some(ref mut guard) = guard {
                let code = if error.is_timeout() {
                    "timeout"
                } else {
                    "transport_error"
                };
                if let Err(error) = guard.finish("failed", None, None, None, None, Some(code)) {
                    error!("Details could not be saved: {error}");
                    if let Some(context) = observation {
                        context.run.mark_incomplete();
                    }
                }
            }
            return Err(report_reqwest_error("HTTP request failed", &error));
        }
    };
    let status = response.status();
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 128
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        })
        .map(str::to_owned);
    let body_bytes = match read_bounded_response(response, response_limit).await {
        Ok(bytes) => bytes,
        Err(error) => {
            if let Some(ref mut guard) = guard {
                let code = if error == "Response body exceeds its limit" {
                    "response_limit"
                } else {
                    "body_error"
                };
                if let Err(error) = guard.finish(
                    "failed",
                    Some(status.as_u16()),
                    None,
                    request_id.as_deref(),
                    None,
                    Some(code),
                ) {
                    error!("Details could not be saved: {error}");
                    if let Some(context) = observation {
                        context.run.mark_incomplete();
                    }
                }
            }
            return Err(error);
        }
    };
    let metadata: Option<Value> = if body_bytes.len() <= 2 * 1024 * 1024 {
        serde_json::from_slice(&body_bytes).ok()
    } else {
        None
    };
    let parsed: Option<ChatCompletionResponse> = if status.is_success() {
        serde_json::from_slice(&body_bytes).ok()
    } else {
        None
    };
    let reported_model = metadata
        .as_ref()
        .and_then(|value| value.get("model"))
        .and_then(Value::as_str)
        .filter(|model| model.len() <= 256);
    let usage_json = metadata
        .as_ref()
        .and_then(|value| value.get("usage"))
        .and_then(|usage| {
            normalize_usage(
                usage,
                metadata
                    .as_ref()
                    .and_then(|value| value.get("service_tier"))
                    .and_then(Value::as_str),
            )
        });
    let error_code = if status.is_success() {
        if parsed.is_none() {
            Some("invalid_response")
        } else {
            None
        }
    } else {
        Some(classify_error(status, &body_bytes))
    };
    if let Some(ref mut guard) = guard {
        let outcome = if status.is_success() && parsed.is_some() {
            "succeeded"
        } else {
            "failed"
        };
        if let Err(error) = guard.finish(
            outcome,
            Some(status.as_u16()),
            reported_model,
            request_id.as_deref(),
            usage_json.as_deref(),
            error_code,
        ) {
            error!("Details could not be saved: {error}");
            if let Some(context) = observation {
                context.run.mark_incomplete();
            }
        }
    }
    Ok((status, body_bytes.to_vec()))
}

fn classify_error(status: reqwest::StatusCode, bytes: &[u8]) -> &'static str {
    let code = (bytes.len() <= 16 * 1024)
        .then(|| serde_json::from_slice::<Value>(bytes).ok())
        .flatten()
        .and_then(|value| {
            value
                .pointer("/error/code")
                .and_then(Value::as_str)
                .map(str::to_owned)
        });
    match (status.as_u16(), code.as_deref()) {
        (_, Some("invalid_api_key")) => "authentication",
        (_, Some("insufficient_quota")) => "quota",
        (_, Some("rate_limit_exceeded" | "rate_limit")) => "rate_limit",
        (401, _) => "authentication",
        (500..=599, _) => "provider_unavailable",
        _ => "http_error",
    }
}

fn normalize_usage(raw: &Value, service_tier: Option<&str>) -> Option<String> {
    let pick = |pointer: &str| raw.pointer(pointer).and_then(Value::as_u64);
    let mut usage = serde_json::Map::new();
    for (field, path) in [
        ("input_tokens", "/prompt_tokens"),
        ("output_tokens", "/completion_tokens"),
        ("total_tokens", "/total_tokens"),
        (
            "cached_input_tokens",
            "/prompt_tokens_details/cached_tokens",
        ),
        (
            "cache_write_tokens",
            "/prompt_tokens_details/cache_write_tokens",
        ),
        (
            "reasoning_tokens",
            "/completion_tokens_details/reasoning_tokens",
        ),
    ] {
        if let Some(count) = pick(path) {
            usage.insert(field.into(), Value::from(count));
        }
    }
    if !usage.contains_key("total_tokens") {
        if let (Some(input), Some(output)) = (pick("/prompt_tokens"), pick("/completion_tokens")) {
            if let Some(total) = input.checked_add(output) {
                usage.insert("total_tokens".into(), Value::from(total));
                usage.insert("total_source".into(), Value::from("derived"));
            }
        }
    } else {
        usage.insert("total_source".into(), Value::from("reported"));
    }
    if let Some(tier) = service_tier.filter(|tier| tier.len() <= 32) {
        usage.insert("service_tier".into(), Value::from(tier));
    }
    (!usage.is_empty()).then(|| Value::Object(usage).to_string())
}

/// Fetch available models from an OpenAI-compatible API
/// Returns a list of model IDs
pub async fn fetch_models(
    provider: &PostProcessProvider,
    api_key: String,
) -> Result<Vec<String>, String> {
    let base_url = provider.base_url.trim_end_matches('/');
    let url = format!("{}/models", base_url);

    debug!("Fetching models from: {}", sanitized_url_for_log(&url));

    let client = create_client(provider, &api_key)?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| report_reqwest_error("Failed to fetch models", &e))?;

    let status = response.status();
    debug!(
        "Model list response received with status {} over {:?} from {}",
        status,
        response.version(),
        sanitized_url(response.url())
    );
    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|e| report_reqwest_error("Failed to read model list error", &e));
        return Err(format!(
            "Model list request failed ({}): {}",
            status, error_text
        ));
    }

    let parsed: serde_json::Value = response
        .json()
        .await
        .map_err(|e| report_reqwest_error("Failed to parse model list response", &e))?;

    let mut models = Vec::new();

    // Handle OpenAI format: { data: [ { id: "..." }, ... ] }
    if let Some(data) = parsed.get("data").and_then(|d| d.as_array()) {
        for entry in data {
            if let Some(id) = entry.get("id").and_then(|i| i.as_str()) {
                models.push(id.to_string());
            } else if let Some(name) = entry.get("name").and_then(|n| n.as_str()) {
                models.push(name.to_string());
            }
        }
    }
    // Handle array format: [ "model1", "model2", ... ]
    else if let Some(array) = parsed.as_array() {
        for entry in array {
            if let Some(model) = entry.as_str() {
                models.push(model.to_string());
            }
        }
    }

    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_submission_accepts_null_content_and_rejects_ambiguous_authority() {
        let args = r#"{"text":"BOM","operation":"insert","memory_changes":[]}"#;
        let valid = serde_json::json!({"choices":[{"finish_reason":"tool_calls","message":{"content":null,
            "tool_calls":[{"id":"call-1","type":"function","function":{"name":"submit_rewrite","arguments":args}}]}}]});
        assert_eq!(
            completion_content(serde_json::from_value(valid.clone()).unwrap(), true, true)
                .unwrap()
                .as_deref(),
            Some(args)
        );
        let mut refused = valid.clone();
        refused["choices"][0]["message"]["refusal"] = serde_json::json!("No");
        assert!(completion_content(serde_json::from_value(refused).unwrap(), true, true).is_err());
        for path in ["name", "arguments"] {
            let mut bad = valid.clone();
            bad["choices"][0]["message"]["tool_calls"][0]["function"][path] =
                serde_json::json!(if path == "name" {
                    "write_long_term_memory".into()
                } else {
                    "x".repeat(128001)
                });
            assert!(completion_content(serde_json::from_value(bad).unwrap(), true, true).is_err());
        }
        let mut multiple = valid.clone();
        let call = multiple["choices"][0]["message"]["tool_calls"][0].clone();
        multiple["choices"][0]["message"]["tool_calls"]
            .as_array_mut()
            .unwrap()
            .push(call);
        assert!(completion_content(serde_json::from_value(multiple).unwrap(), true, true).is_err());
        let mut truncated = valid.clone();
        truncated["choices"][0]["finish_reason"] = serde_json::json!("length");
        assert!(
            completion_content(serde_json::from_value(truncated).unwrap(), true, true).is_err()
        );
        assert!(completion_content(serde_json::from_value(valid).unwrap(), false, true).is_err());
    }

    #[test]
    fn native_submission_accepts_forced_function_finish_reasons_and_rejects_others() {
        let args = r#"{"text":"BOM","operation":"insert","memory_changes":[]}"#;
        let valid = serde_json::json!({"choices":[{"finish_reason":"tool_calls","message":{"content":null,
            "tool_calls":[{"id":"call-1","type":"function","function":{"name":"submit_rewrite","arguments":args}}]}}]});
        for reason in [
            serde_json::json!("tool_calls"),
            serde_json::json!("stop"),
            serde_json::Value::Null,
        ] {
            let mut response = valid.clone();
            response["choices"][0]["finish_reason"] = reason;
            assert_eq!(
                completion_content(serde_json::from_value(response).unwrap(), true, true)
                    .unwrap()
                    .as_deref(),
                Some(args)
            );
        }
        let mut omitted = valid.clone();
        omitted["choices"][0]
            .as_object_mut()
            .unwrap()
            .remove("finish_reason");
        assert!(completion_content(serde_json::from_value(omitted).unwrap(), true, true).is_ok());
        for reason in ["length", "content_filter", "function_call", "error"] {
            let mut response = valid.clone();
            response["choices"][0]["finish_reason"] = serde_json::json!(reason);
            assert!(
                completion_content(serde_json::from_value(response).unwrap(), true, true).is_err()
            );
        }
    }

    #[test]
    fn tool_call_id_and_type_are_optional_but_a_present_type_must_be_function() {
        let args = r#"{"text":"BOM","operation":"insert","memory_changes":[]}"#;
        let bare = serde_json::json!({"choices":[{"finish_reason":"stop","message":{"content":null,
            "tool_calls":[{"function":{"name":"submit_rewrite","arguments":args}}]}}]});
        assert_eq!(
            completion_content(serde_json::from_value(bare.clone()).unwrap(), true, true)
                .unwrap()
                .as_deref(),
            Some(args)
        );
        let mut nulls = bare.clone();
        nulls["choices"][0]["message"]["tool_calls"][0]["id"] = serde_json::Value::Null;
        nulls["choices"][0]["message"]["tool_calls"][0]["type"] = serde_json::Value::Null;
        assert!(completion_content(serde_json::from_value(nulls).unwrap(), true, true).is_ok());
        let mut wrong_kind = bare.clone();
        wrong_kind["choices"][0]["message"]["tool_calls"][0]["type"] =
            serde_json::json!("code_interpreter");
        assert!(
            completion_content(serde_json::from_value(wrong_kind).unwrap(), true, true).is_err()
        );
        let mut long_id = bare.clone();
        long_id["choices"][0]["message"]["tool_calls"][0]["id"] =
            serde_json::json!("x".repeat(129));
        assert!(completion_content(serde_json::from_value(long_id).unwrap(), true, true).is_err());
        // Ordinary post-processing still parses such a body; it only rejects
        // the unexpected function call, as before.
        let ordinary = serde_json::json!({"choices":[{"finish_reason":"stop","message":{"content":"Hello",
            "tool_calls":[{"function":{"name":"other","arguments":"{}"}}]}}]});
        let parsed: ChatCompletionResponse = serde_json::from_value(ordinary).unwrap();
        assert!(completion_content(parsed, false, false).is_err());
        let plain = serde_json::json!({"choices":[{"message":{"content":"Hello"}}]});
        assert_eq!(
            completion_content(serde_json::from_value(plain).unwrap(), false, false)
                .unwrap()
                .as_deref(),
            Some("Hello")
        );
    }
    use std::fmt;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[derive(Debug)]
    struct TestError {
        message: &'static str,
        source: Option<Box<TestError>>,
    }

    impl fmt::Display for TestError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.message)
        }
    }

    impl StdError for TestError {
        fn source(&self) -> Option<&(dyn StdError + 'static)> {
            self.source
                .as_deref()
                .map(|source| source as &(dyn StdError + 'static))
        }
    }

    fn provider(id: &str, base_url: &str) -> PostProcessProvider {
        PostProcessProvider {
            id: id.to_string(),
            label: id.to_string(),
            base_url: base_url.to_string(),
            allow_base_url_edit: true,
            models_endpoint: None,
            supports_structured_output: false,
        }
    }

    fn request_json(reasoning: ReasoningParams) -> Value {
        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "hi".to_string(),
            }],
            stream: false,
            response_format: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            reasoning,
        };
        serde_json::to_value(&request).unwrap()
    }

    async fn serve_one_response(status: &str, body: &str) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 2048];
            let _ = stream.read(&mut request).await.unwrap();
            stream.write_all(response.as_bytes()).await.unwrap();
        });

        format!("http://{address}")
    }

    #[test]
    fn error_source_chain_includes_all_nested_causes() {
        let error = TestError {
            message: "request failed",
            source: Some(Box::new(TestError {
                message: "TLS handshake failed",
                source: Some(Box::new(TestError {
                    message: "unknown certificate authority",
                    source: None,
                })),
            })),
        };

        assert_eq!(
            error_source_chain(&error),
            vec!["TLS handshake failed", "unknown certificate authority"]
        );
    }

    #[test]
    fn log_url_sanitization_removes_credentials_and_tokens() {
        let url = "https://user:password@example.com/v1/models?api_key=secret#private";
        assert_eq!(sanitized_url_for_log(url), "https://example.com/v1/models");
    }

    #[test]
    fn invalid_log_urls_are_not_echoed() {
        assert_eq!(
            sanitized_url_for_log("not a URL containing secret"),
            "<invalid URL>"
        );
    }

    #[tokio::test]
    async fn decode_error_does_not_echo_response_values() {
        let base_url =
            serve_one_response("200 OK", r#"{"choices":"PRIVATE TRANSCRIPTION CONTENT"}"#).await;
        let error = reqwest::get(base_url)
            .await
            .unwrap()
            .json::<ChatCompletionResponse>()
            .await
            .unwrap_err();

        let details = report_reqwest_error("Failed to parse API response", &error);
        assert!(details.contains("kind: decode"));
        assert!(!details.contains("PRIVATE TRANSCRIPTION CONTENT"));
    }

    #[tokio::test]
    async fn raw_error_url_is_not_reintroduced_without_a_source() {
        let base_url = serve_one_response("400 Bad Request", "bad request").await;
        let error = reqwest::get(format!(
            "{base_url}/private?api_key=SECRET_QUERY_TOKEN#private"
        ))
        .await
        .unwrap()
        .error_for_status()
        .unwrap_err();

        let details = report_reqwest_error("Request failed", &error);
        assert!(details.contains(&format!("url: {base_url}/private")));
        assert!(!details.contains("SECRET_QUERY_TOKEN"));
        assert!(!details.contains("#private"));
    }

    #[test]
    fn requests_explicitly_disable_streaming() {
        let json = request_json(ReasoningParams::default());
        assert_eq!(json["stream"], false);
    }

    #[test]
    fn default_reasoning_params_serialize_to_no_fields() {
        let json = request_json(ReasoningParams::default());
        assert!(json.get("reasoning_effort").is_none());
        assert!(json.get("reasoning").is_none());
        assert!(json.get("thinking").is_none());
    }

    #[test]
    fn custom_provider_uses_top_level_reasoning_effort() {
        let params = reasoning_disable_params(&provider("custom", "http://localhost:11434/v1"));
        let json = request_json(params);
        assert_eq!(json["reasoning_effort"], "none");
        assert!(json.get("reasoning").is_none());
        assert!(json.get("thinking").is_none());
    }

    #[test]
    fn openrouter_uses_nested_reasoning_object() {
        let params =
            reasoning_disable_params(&provider("openrouter", "https://openrouter.ai/api/v1"));
        let json = request_json(params);
        assert!(json.get("reasoning_effort").is_none());
        assert_eq!(json["reasoning"]["effort"], "none");
        assert_eq!(json["reasoning"]["exclude"], true);
        assert!(json.get("thinking").is_none());
    }

    #[test]
    fn deepseek_base_url_uses_thinking_disabled() {
        let params = reasoning_disable_params(&provider("custom", "https://api.deepseek.com"));
        let json = request_json(params);
        assert!(json.get("reasoning_effort").is_none());
        assert!(json.get("reasoning").is_none());
        assert_eq!(json["thinking"]["type"], "disabled");
    }

    #[test]
    fn reasoning_params_is_empty_tracks_all_fields() {
        assert!(ReasoningParams::default().is_empty());
        assert!(!ReasoningParams {
            reasoning_effort: Some("none".to_string()),
            ..Default::default()
        }
        .is_empty());
        assert!(!ReasoningParams {
            thinking: Some(serde_json::json!({ "type": "disabled" })),
            ..Default::default()
        }
        .is_empty());
    }

    #[test]
    fn rejection_memo_is_keyed_by_base_url_and_model() {
        let deepseek = provider("custom", "https://api.deepseek.com/");
        let key = endpoint_key(&deepseek, "deepseek-chat");
        assert_eq!(key, "https://api.deepseek.com|deepseek-chat");
        assert!(!is_known_rejected(&key));
        remember_rejection(key.clone());
        assert!(is_known_rejected(&key));
        // A different model on the same endpoint is tracked separately
        assert!(!is_known_rejected(&endpoint_key(&deepseek, "other-model")));
    }

    #[test]
    fn unknown_http_429_does_not_claim_a_rate_limit() {
        assert_eq!(
            classify_error(
                reqwest::StatusCode::TOO_MANY_REQUESTS,
                br#"{"error":{"message":"PRIVATE"}}"#
            ),
            "http_error"
        );
        assert_eq!(
            classify_error(
                reqwest::StatusCode::TOO_MANY_REQUESTS,
                br#"{"error":{"code":"rate_limit_exceeded"}}"#
            ),
            "rate_limit"
        );
        assert_eq!(
            classify_error(
                reqwest::StatusCode::FORBIDDEN,
                br#"{"error":{"code":"insufficient_quota"}}"#
            ),
            "quota"
        );
    }

    #[test]
    fn usage_normalization_retains_only_known_numeric_fields() {
        let raw = serde_json::json!({"prompt_tokens":14860,"completion_tokens":96,"total_tokens":14956,"prompt_tokens_details":{"cached_tokens":14000,"cache_write_tokens":0,"private":"SECRET"},"completion_tokens_details":{"reasoning_tokens":20},"secret":"SECRET"});
        let value: Value =
            serde_json::from_str(&normalize_usage(&raw, Some("default")).unwrap()).unwrap();
        assert_eq!(value["cached_input_tokens"], 14000);
        assert_eq!(value["cache_write_tokens"], 0);
        assert_eq!(value["reasoning_tokens"], 20);
        assert_eq!(value["service_tier"], "default");
        assert!(!value.to_string().contains("SECRET"));
    }

    #[tokio::test]
    async fn retry_archive_matches_each_dispatched_body_and_keeps_reported_usage() {
        use crate::managers::{
            history::MIGRATIONS,
            history_processing::{get_request, get_run, RunGuard, RunSnapshot},
        };
        use rusqlite_migration::Migrations;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.db");
        let mut conn = rusqlite::Connection::open(&path).unwrap();
        Migrations::new(MIGRATIONS.to_vec())
            .to_latest(&mut conn)
            .unwrap();
        conn.execute("INSERT INTO transcription_history (file_name,timestamp,title,transcription_text) VALUES ('test.wav',1,'Test','spoken')", []).unwrap();
        let run = RunGuard::start(
            &path,
            conn.last_insert_rowid(),
            "spoken",
            RunSnapshot::default(),
            true,
        )
        .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let mut bodies = Vec::new();
            for index in 0..2 {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut received = Vec::new();
                let boundary = loop {
                    let mut chunk = [0u8; 4096];
                    let count = socket.read(&mut chunk).await.unwrap();
                    assert!(count > 0);
                    received.extend_from_slice(&chunk[..count]);
                    if let Some(pos) = received.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                        break pos + 4;
                    }
                };
                let header = String::from_utf8_lossy(&received[..boundary]);
                let length: usize = header
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse().ok())
                            .flatten()
                    })
                    .unwrap();
                while received.len() < boundary + length {
                    let mut chunk = [0u8; 4096];
                    let count = socket.read(&mut chunk).await.unwrap();
                    assert!(count > 0);
                    received.extend_from_slice(&chunk[..count]);
                }
                bodies.push(
                    String::from_utf8(received[boundary..boundary + length].to_vec()).unwrap(),
                );
                let (status, body) = if index == 0 {
                    ("400 Bad Request", r#"{"error":{"message":"PRIVATE ECHO"}}"#)
                } else {
                    (
                        "200 OK",
                        r#"{"model":"reported-model","choices":[],"usage":{"prompt_tokens":21,"completion_tokens":3,"total_tokens":24}}"#,
                    )
                };
                let response = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                socket.write_all(response.as_bytes()).await.unwrap();
            }
            let _ = tx.send(bodies);
        });
        let result = send_chat_completion_observed(
            &provider("custom", &base),
            String::new(),
            "unique-retry-model",
            "spoken".into(),
            Some("instruction".into()),
            None,
            true,
            Some(CallContext {
                run: &run,
                purpose: "rewrite",
                retry_of: None,
            }),
        )
        .await
        .unwrap();
        assert!(result.is_none());
        let sent = rx.await.unwrap();
        let detail = get_run(&path, run.id()).unwrap().unwrap();
        assert_eq!(detail.calls.len(), 2);
        assert_eq!(detail.calls[1].retry_of, Some(detail.calls[0].id));
        assert_eq!(
            detail.calls[1].reported_model.as_deref(),
            Some("reported-model")
        );
        assert!(detail.calls[1]
            .usage_json
            .as_deref()
            .unwrap()
            .contains("21"));
        assert_eq!(
            get_request(&path, detail.calls[0].id)
                .unwrap()
                .unwrap()
                .request_json
                .as_deref(),
            Some(sent[0].as_str())
        );
        assert_eq!(
            get_request(&path, detail.calls[1].id)
                .unwrap()
                .unwrap()
                .request_json
                .as_deref(),
            Some(sent[1].as_str())
        );
        assert!(sent[0].contains("reasoning_effort"));
        assert!(!sent[1].contains("reasoning_effort"));
        assert!(!format!("{detail:?}").contains("PRIVATE ECHO"));
    }
}
