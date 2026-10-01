import assert from "node:assert/strict";
import { formatUsd, summarizeCalls, type ProviderCall } from "./historyApi";

assert.equal(formatUsd("0.000150000"), "$0.0002");
assert.equal(formatUsd("0.000149999"), "$0.0001");
assert.equal(formatUsd("0.000099999"), "< $0.0001");
assert.equal(formatUsd("0.000000000"), "$0.0000");

const call = (usage: string | null, cost: string | null): ProviderCall => ({
  id: 1,
  run_id: 1,
  ordinal: 1,
  purpose: "rewrite",
  retry_of: null,
  provider_id: "openai",
  provider_name: "OpenAI",
  endpoint_label: "https://api.openai.com/",
  requested_model: "gpt-6-luna",
  reported_model: null,
  started_at: "",
  ended_at: null,
  elapsed_ms: null,
  http_status: 200,
  outcome: "succeeded",
  error_code: null,
  provider_request_id: null,
  usage_json: usage,
  rate_json: null,
  cost_usd: cost,
  archive_status: "retained",
  request_bytes: 10,
});
const result = summarizeCalls([
  call('{"total_tokens":10}', "0.000050000"),
  call(null, null),
]);
assert.equal(result.knownTokens, 10n);
assert.equal(result.usageComplete, false);
assert.equal(result.knownCostUsd, "0.000050000");
assert.equal(result.costComplete, false);
assert.equal(summarizeCalls([call(null, null)]).knownTokens, null);

console.log("history API: all assertions passed");
