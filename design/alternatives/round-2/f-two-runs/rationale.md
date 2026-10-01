# Alternative F: compare two runs inside ownership lanes

The page keeps the five owners from alternative C and aligns two synthetic utterances under them. Its main question is what the second run can reuse. A reader sees the repeated prompt and dictionary beside the new transcript and input context before reaching the proposed prefix arrangement.

The comparison deliberately separates message role from change frequency. The dictionary is currently untrusted user data and may remain identical between utterances. The system message can change when the effective profile prompt changes. Both messages are assembled and sent on every run. The process cache in `request.rs` tracks endpoint compatibility; it gives no evidence of prompt reuse. Local ASR model reuse is a separate concern.

The orange panel is a design proposal. It groups stable material before dynamic material and calls for provider reported usage and measured latency before claiming a benefit. Reshaping the current single JSON user envelope needs behavioral validation and must preserve the data authority of dictionary entries. The page states no guaranteed cache hit, cost reduction, or provider support.

The source links and five stop conditions remain available below the comparison. G1 retains the earlier learning decision and stable option IDs. Current output dispatch does not prove destination readback, and automatic learning still lacks a writer. The example does not claim a recorded run.

This layout transfers to order fulfillment: compare two orders in aligned lanes for customer input, inventory, pricing, carrier, and fulfillment. A repeated catalog or shipping policy may be stable, while stock, address, and order lines change each time. The comparison would expose a possible reuse boundary without claiming that the system already caches those inputs.

## Review

The parent checked the page in Edge at desktop and mobile sizes with JavaScript disabled. Local files loaded with no external network requests, and source links resolved. No full voice run or endpoint cache behavior was verified.
