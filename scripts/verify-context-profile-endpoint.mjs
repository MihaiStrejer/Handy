// Historical enable-time probe: retained as an explicit migration notice.
console.error(
  "This probe is retired: enabling profiles is now local-only. Validate endpoint compatibility through a profile-processing request. The earlier native compatibility result remains recorded in plan.md.",
);
process.exitCode = 1;
