import { commands, type ProviderCall, type Result } from "@/bindings";

export type {
  ProcessingRun,
  ProviderCall,
  RequestContents,
  RunDetail,
} from "@/bindings";

async function data<T>(request: Promise<Result<T, string>>): Promise<T> {
  const result = await request;
  if (result.status === "error") throw new Error(result.error);
  return result.data;
}

export const historyApi = {
  runs: (entryId: number) => data(commands.getHistoryProcessingRuns(entryId)),
  run: (runId: number) => data(commands.getHistoryProcessingRun(runId)),
  request: (callId: number) => data(commands.getHistoryRequestContents(callId)),
  clearRequests: () => data(commands.clearHistoryRequestContents()),
  setArchiveEnabled: (enabled: boolean) =>
    data(commands.setHistoryRequestContentsEnabled(enabled)),
};

export function parseUsage(
  call: ProviderCall,
): Record<string, number | string> | null {
  if (!call.usage_json) return null;
  try {
    const parsed: unknown = JSON.parse(call.usage_json);
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      return parsed as Record<string, number | string>;
    }
  } catch {
    return null;
  }
  return null;
}

export function formatUsd(usd: string): string {
  if (!/^\d+\.\d{9}$/.test(usd)) return usd;
  const [whole, fraction] = usd.split(".");
  const nanos = BigInt(whole) * 1_000_000_000n + BigInt(fraction);
  if (nanos > 0n && nanos < 100_000n) return "< $0.0001";
  const units = (nanos + 50_000n) / 100_000n;
  return `$${units / 10_000n}.${String(units % 10_000n).padStart(4, "0")}`;
}

export function summarizeCalls(calls: ProviderCall[]): {
  knownTokens: bigint | null;
  usageComplete: boolean;
  knownCostUsd: string | null;
  costComplete: boolean;
} {
  let tokens = 0n;
  let costs = 0n;
  let tokenCount = 0;
  let costCount = 0;
  for (const call of calls) {
    const total = parseUsage(call)?.total_tokens;
    if (
      typeof total === "number" &&
      Number.isSafeInteger(total) &&
      total >= 0
    ) {
      tokens += BigInt(total);
      tokenCount++;
    }
    if (call.cost_usd && /^\d+\.\d{9}$/.test(call.cost_usd)) {
      const [whole, fraction] = call.cost_usd.split(".");
      costs += BigInt(whole) * 1_000_000_000n + BigInt(fraction);
      costCount++;
    }
  }
  return {
    knownTokens: tokenCount ? tokens : null,
    usageComplete: calls.length > 0 && tokenCount === calls.length,
    knownCostUsd: costCount
      ? `${costs / 1_000_000_000n}.${String(costs % 1_000_000_000n).padStart(9, "0")}`
      : null,
    costComplete: calls.length > 0 && costCount === calls.length,
  };
}
