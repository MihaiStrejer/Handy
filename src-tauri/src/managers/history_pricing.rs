//! Pinned, exact-model USD estimates. Rates are nanodollars per token, so arithmetic is decimal and exact.
use serde_json::{json, Value};

const SOURCE: &str = "https://developers.openai.com/api/docs/pricing";
const VERIFIED_AT: &str = "2026-09-27";

#[derive(Clone, Copy)]
struct Rates {
    input: u128,
    cached: u128,
    cache_write: u128,
    output: u128,
}

fn calculate(input: u64, cached: u64, cache_write: u64, output: u64, rates: Rates) -> Option<u128> {
    let ordinary = input.checked_sub(cached)?.checked_sub(cache_write)?;
    (ordinary as u128)
        .checked_mul(rates.input)?
        .checked_add((cached as u128).checked_mul(rates.cached)?)?
        .checked_add((cache_write as u128).checked_mul(rates.cache_write)?)?
        .checked_add((output as u128).checked_mul(rates.output)?)
}

fn usd(nanodollars: u128) -> String {
    format!(
        "{}.{:09}",
        nanodollars / 1_000_000_000,
        nanodollars % 1_000_000_000
    )
}

/// Returns a rate snapshot even when known rates cannot yield a cost because usage is partial.
pub fn estimate(
    provider_id: &str,
    endpoint: &str,
    model: &str,
    usage_json: Option<&str>,
) -> (Option<String>, Option<String>) {
    if provider_id != "openai"
        || endpoint.trim_end_matches('/') != "https://api.openai.com/v1"
        || model != "gpt-6-luna"
    {
        return (None, None);
    }
    let Some(usage) = usage_json.and_then(|value| serde_json::from_str::<Value>(value).ok()) else {
        return (None, None);
    };
    let input = usage.get("input_tokens").and_then(Value::as_u64);
    let output = usage.get("output_tokens").and_then(Value::as_u64);
    let cached = usage.get("cached_input_tokens").and_then(Value::as_u64);
    let cache_write = usage.get("cache_write_tokens").and_then(Value::as_u64);
    let tier = usage.get("service_tier").and_then(Value::as_str);
    let Some(input) = input else {
        return (None, None);
    };
    if tier != Some("default") {
        return (None, None);
    }
    let long = input > 272_000;
    let rates = if long {
        Rates {
            input: 200,
            cached: 20,
            cache_write: 250,
            output: 750,
        }
    } else {
        Rates {
            input: 100,
            cached: 10,
            cache_write: 125,
            output: 500,
        }
    };
    let snapshot = json!({
        "version":1,"provider_id":"openai","endpoint":"https://api.openai.com/v1",
        "model":"gpt-6-luna","currency":"USD","service_tier":"default",
        "input_range":if long { "long_over_272000" } else { "short_at_most_272000" },
        "input_per_million_usd":if long { "0.20" } else { "0.10" },
        "cached_per_million_usd":if long { "0.02" } else { "0.01" },
        "cache_write_per_million_usd":if long { "0.25" } else { "0.125" },
        "output_per_million_usd":if long { "0.75" } else { "0.50" },
        "source":SOURCE,"verified_at":VERIFIED_AT,"provenance":"built_in_verified"
    })
    .to_string();
    let cost = cached
        .zip(cache_write)
        .zip(output)
        .and_then(|((cached, cache_write), output)| {
            calculate(input, cached, cache_write, output, rates)
        })
        .map(usd);
    (Some(snapshot), cost)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fictional_fixture_is_exact_and_cache_is_not_double_counted() {
        let rates = Rates {
            input: 500,
            cached: 50,
            cache_write: 500,
            output: 2_000,
        };
        let rewrite = calculate(14_860, 14_000, 0, 96, rates).unwrap();
        let probe = calculate(60, 0, 0, 12, rates).unwrap();
        assert_eq!(usd(rewrite + probe), "0.001376000");
    }

    #[test]
    fn luna_requires_exact_endpoint_model_tier_and_cache_categories() {
        let usage = r#"{"input_tokens":100,"output_tokens":10,"cached_input_tokens":20,"cache_write_tokens":5,"service_tier":"default"}"#;
        let (rate, cost) = estimate(
            "openai",
            "https://api.openai.com/v1",
            "gpt-6-luna",
            Some(usage),
        );
        assert!(rate.unwrap().contains(SOURCE));
        assert_eq!(cost.as_deref(), Some("0.000013325"));
        assert!(estimate(
            "custom",
            "https://api.openai.com/v1",
            "gpt-6-luna",
            Some(usage)
        )
        .1
        .is_none());
        assert!(estimate(
            "openai",
            "https://proxy.example/v1",
            "gpt-6-luna",
            Some(usage)
        )
        .1
        .is_none());
        assert!(estimate(
            "openai",
            "https://api.openai.com/v1",
            "gpt-6-luna-alias",
            Some(usage)
        )
        .1
        .is_none());
        assert!(estimate(
            "openai",
            "https://api.openai.com/v1",
            "gpt-6-luna",
            Some(r#"{"input_tokens":100,"output_tokens":10,"service_tier":"default"}"#)
        )
        .1
        .is_none());
        assert!(estimate(
            "openai",
            "https://api.openai.com/v1",
            "gpt-6-luna",
            Some(&usage.replace("default", "priority"))
        )
        .1
        .is_none());
    }

    #[test]
    fn long_range_applies_to_whole_request() {
        let usage = r#"{"input_tokens":272001,"output_tokens":1,"cached_input_tokens":0,"cache_write_tokens":0,"service_tier":"default"}"#;
        let (snapshot, cost) = estimate(
            "openai",
            "https://api.openai.com/v1",
            "gpt-6-luna",
            Some(usage),
        );
        assert!(snapshot.unwrap().contains("long_over_272000"));
        assert_eq!(cost.as_deref(), Some("0.054400950"));
    }
}
