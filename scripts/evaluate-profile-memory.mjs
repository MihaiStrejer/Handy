import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const mode = process.argv[2] ?? "fixture";
if (!["fixture", "endpoint"].includes(mode)) {
  throw new Error(
    "Usage: node scripts/evaluate-profile-memory.mjs [fixture|endpoint]",
  );
}
if (
  mode === "endpoint" &&
  (!process.env.PROFILE_MEMORY_EVAL_URL ||
    !process.env.PROFILE_MEMORY_EVAL_MODEL)
) {
  throw new Error(
    "Endpoint mode requires PROFILE_MEMORY_EVAL_URL and PROFILE_MEMORY_EVAL_MODEL; PROFILE_MEMORY_EVAL_KEY is optional.",
  );
}
const directory = resolve("design/proof/profile-memory");
mkdirSync(directory, { recursive: true });
const reportPath = resolve(directory, `evaluation-${mode}.json`);
const env = { ...process.env, PROFILE_MEMORY_EVAL_REPORT: reportPath };
if (mode === "fixture") {
  delete env.PROFILE_MEMORY_EVAL_URL;
  delete env.PROFILE_MEMORY_EVAL_MODEL;
  delete env.PROFILE_MEMORY_EVAL_KEY;
}
// Windows unit-test executables live in deps, outside the packaged DLL directory.
// Stage only existing build artifacts; never search or modify installed Handy.
if (process.platform === "win32") {
  for (const name of [
    "DirectML.dll",
    "transcribe.dll",
    "ggml.dll",
    "ggml-base.dll",
    "ggml-cpu.dll",
  ]) {
    const source = resolve("src-tauri/target/debug", name);
    const target = resolve("src-tauri/target/debug/deps", name);
    if (
      existsSync(source) &&
      existsSync(resolve("src-tauri/target/debug/deps"))
    )
      copyFileSync(source, target);
  }
}
const result = spawnSync(
  "cargo",
  [
    "test",
    "--manifest-path",
    "src-tauri/Cargo.toml",
    "--lib",
    "profile_memory_evaluation",
    "--",
    "--ignored",
    "--nocapture",
  ],
  { env, stdio: "inherit" },
);
if (result.error) throw result.error;
if (existsSync(reportPath)) {
  const report = JSON.parse(readFileSync(reportPath, "utf8"));
  console.log(
    JSON.stringify(
      {
        mode: report.mode,
        model: report.model,
        false_learning_eligible_cases: report.false_learning_eligible_cases,
        positive_recall: report.positive_recall,
        warm_requests_per_rewrite: report.warm_requests_per_rewrite,
        report: reportPath,
      },
      null,
      2,
    ),
  );
}
process.exitCode = result.status ?? 1;
