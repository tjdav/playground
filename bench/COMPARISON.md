# Transport Comparison Report

> **Note:** Some combinations are missing and displayed as N/A.

## Section 1 — Per-scenario latency

### Scenario: Baseline
| Transport | p50 (ms) | p95 (ms) | p99 (ms) | Max (ms) |
|---|---|---|---|---|
| ws | 0.66 | 0.87 | 1.04 | 1.08 |
| sse | N/A | N/A | N/A | N/A |
| sockudo | N/A | N/A | N/A | N/A |

### Scenario: Small Group
| Transport | p50 (ms) | p95 (ms) | p99 (ms) | Max (ms) |
|---|---|---|---|---|
| ws | 0.92 | 1.24 | 1.46 | 1.91 |
| sse | N/A | N/A | N/A | N/A |
| sockudo | N/A | N/A | N/A | N/A |

### Scenario: Large Group
| Transport | p50 (ms) | p95 (ms) | p99 (ms) | Max (ms) |
|---|---|---|---|---|
| ws | 3.76 | 6.58 | 8.07 | 11.40 |
| sse | N/A | N/A | N/A | N/A |
| sockudo | N/A | N/A | N/A | N/A |

### Scenario: Welcome Burst
| Transport | p50 (ms) | p95 (ms) | p99 (ms) | Max (ms) |
|---|---|---|---|---|
| ws | N/A | N/A | N/A | N/A |
| sse | 14.04 | 21.77 | 27.62 | 33.83 |
| sockudo | 3.95 | 4.94 | 6.15 | 6.75 |

### Scenario: Mixed
| Transport | p50 (ms) | p95 (ms) | p99 (ms) | Max (ms) |
|---|---|---|---|---|
| ws | N/A | N/A | N/A | N/A |
| sse | 8.58 | 23.02 | 35.56 | 103.68 |
| sockudo | 3.31 | 5.80 | 7.26 | 11.69 |

## Section 2 — Delivery completeness

| Scenario | ws | sse | sockudo |
|---|---|---|---|
| Baseline | 100.00% | N/A | N/A |
| Small group | 100.00% | N/A | N/A |
| Large group | 100.00% | N/A | N/A |
| Welcome burst | N/A | 100.00% | 100.00% |
| Mixed | N/A | 100.00% | 100.00% |

## Section 3 — Connection establishment (ms)

| Scenario | ws p50 | ws p95 | sse p50 | sse p95 | sockudo p50 | sockudo p95 |
|---|---|---|---|---|---|---|
| Baseline | 0.10 | 1.48 | N/A | N/A | N/A | N/A |
| Small group | 2.19 | 3.67 | N/A | N/A | N/A | N/A |
| Large group | 28.59 | 42.01 | N/A | N/A | N/A | N/A |
| Welcome burst | N/A | N/A | 21.04 | 24.99 | 1922.17 | 3673.90 |
| Mixed | N/A | N/A | 36.54 | 38.91 | 3805.42 | 8215.75 |

## Section 4 — Resource profile at 500 connections (Large group scenario)

| Transport | Peak RSS (MB) | Mean CPU (%) | Peak FDs |
|---|---|---|---|
| ws | 70.83 | 1.34 | 511 |
| sse | N/A | N/A | N/A |
| sockudo | N/A | N/A | N/A |

## Section 5 — Verdict

Ws had the lowest p95 latency in 3 of 5 scenarios. Ws had the highest average delivery completeness (60.00%). Ws used the least memory (70.83 MB) at 500 connections. However, in the welcome burst, mixed scenario(s), a different transport had lower latency. All transports performed adequately, but Ws appears to be the overall winner for this workload.
