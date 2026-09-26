# Transport Comparison Report

## Run Status

| Transport | Baseline | Small group | Large group | Welcome burst | Mixed |
|---|---|---|---|---|---|
| ws | complete | complete | complete | complete | complete |
| sse | pending | pending | pending | pending | pending |
| sockudo | pending | pending | pending | pending | pending |

> **Note:** Some combinations are missing and displayed as N/A.

## Section 1 — Per-scenario latency

### Scenario: Baseline
| Transport | p50 (ms) | p95 (ms) | p99 (ms) | Max (ms) |
|---|---|---|---|---|
| ws | 0.60 | 0.85 | 1.08 | 1.23 |
| sse | N/A | N/A | N/A | N/A |
| sockudo | N/A | N/A | N/A | N/A |

### Scenario: Small Group
| Transport | p50 (ms) | p95 (ms) | p99 (ms) | Max (ms) |
|---|---|---|---|---|
| ws | 0.88 | 1.24 | 1.60 | 2.74 |
| sse | N/A | N/A | N/A | N/A |
| sockudo | N/A | N/A | N/A | N/A |

### Scenario: Large Group
| Transport | p50 (ms) | p95 (ms) | p99 (ms) | Max (ms) |
|---|---|---|---|---|
| ws | 3.50 | 5.99 | 7.16 | 12.27 |
| sse | N/A | N/A | N/A | N/A |
| sockudo | N/A | N/A | N/A | N/A |

### Scenario: Welcome Burst
| Transport | p50 (ms) | p95 (ms) | p99 (ms) | Max (ms) |
|---|---|---|---|---|
| ws | 1.34 | 1.72 | 1.98 | 2.25 |
| sse | N/A | N/A | N/A | N/A |
| sockudo | N/A | N/A | N/A | N/A |

### Scenario: Mixed
| Transport | p50 (ms) | p95 (ms) | p99 (ms) | Max (ms) |
|---|---|---|---|---|
| ws | 1.20 | 1.89 | 2.35 | 5.48 |
| sse | N/A | N/A | N/A | N/A |
| sockudo | N/A | N/A | N/A | N/A |

## Section 2 — Delivery completeness

| Scenario | ws | sse | sockudo |
|---|---|---|---|
| Baseline | 100.00% | N/A | N/A |
| Small group | 100.00% | N/A | N/A |
| Large group | 100.00% | N/A | N/A |
| Welcome burst | 100.00% | N/A | N/A |
| Mixed | 100.00% | N/A | N/A |

## Section 3 — Connection establishment (ms)

| Scenario | ws p50 | ws p95 | sse p50 | sse p95 | sockudo p50 | sockudo p95 |
|---|---|---|---|---|---|---|
| Baseline | 0.97 | 1.13 | N/A | N/A | N/A | N/A |
| Small group | 1.75 | 4.25 | N/A | N/A | N/A | N/A |
| Large group | 34.32 | 53.59 | N/A | N/A | N/A | N/A |
| Welcome burst | 3.25 | 6.05 | N/A | N/A | N/A | N/A |
| Mixed | 12.51 | 14.13 | N/A | N/A | N/A | N/A |

## Section 4 — Resource profile at 500 connections (Large group scenario)

| Transport | Peak RSS (MB) | Mean CPU (%) | Peak FDs |
|---|---|---|---|
| ws | 71.26 | 1.29 | 511 |
| sse | N/A | N/A | N/A |
| sockudo | N/A | N/A | N/A |

## Section 5 — Verdict

**Insufficient data for a verdict.** Only 1 transport(s) have data for at least 3 scenarios. Complete the matrix and regenerate this report.
