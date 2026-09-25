import sys
import json
import csv
import os
import glob
from collections import defaultdict

def percentile(data, p):
    if not data:
        return 0
    idx = int(p * len(data))
    if idx >= len(data):
        idx = len(data) - 1
    return data[idx]

def main():
    results_dir = os.path.join(os.path.dirname(__file__), "..", "results")

    transports = ["ws", "sse", "sockudo"]
    scenarios = ["baseline", "small-group", "large-group", "welcome-burst", "mixed"]

    # Data structures to store results
    # latency: [scenario][transport] = {p50, p95, p99, max}
    # completeness: [scenario][transport] = float %
    # connection: [scenario][transport] = {p50, p95}
    # resources: [transport] = {rss, cpu, fds}

    latency_data = defaultdict(lambda: defaultdict(dict))
    completeness_data = defaultdict(lambda: defaultdict(float))
    connection_data = defaultdict(lambda: defaultdict(dict))
    resources_data = defaultdict(dict)

    # Track lowest/highest for verdict
    lowest_p95_latency = None # (transport, val)
    highest_completeness = None # (transport, val)
    lowest_memory_large = None # (transport, val)

    for scenario in scenarios:
        for transport in transports:
            json_file = os.path.join(results_dir, f"{transport}-{scenario}.jsonl")
            csv_file = os.path.join(results_dir, f"{transport}-{scenario}-resources.csv")

            if not os.path.exists(json_file):
                continue

            latencies = []
            received = 0
            unique_messages = set()
            unique_receivers = set()

            conn_start_times = []

            with open(json_file, 'r') as f:
                for line in f:
                    if not line.strip(): continue
                    try:
                        record = json.loads(line)
                    except json.JSONDecodeError:
                        continue

                    if record.get("phase") == "measured" and "latency_us" in record:
                        latencies.append(record["latency_us"])
                        received += 1
                        if "receiver_id" in record:
                            unique_receivers.add(record["receiver_id"])
                        if "client_message_id" in record:
                            unique_messages.add(record["client_message_id"])

                    if record.get("event") == "connection_established" and "ts_micros" in record:
                        conn_start_times.append(record["ts_micros"])

            # Latency Calculations
            if latencies:
                latencies.sort()
                # Store in ms
                latency_data[scenario][transport] = {
                    "p50": percentile(latencies, 0.50) / 1000.0,
                    "p95": percentile(latencies, 0.95) / 1000.0,
                    "p99": percentile(latencies, 0.99) / 1000.0,
                    "max": max(latencies) / 1000.0
                }

                # Global lowest p95 tracking (averaging across scenarios or just finding min)
                # We'll just collect them and figure out verdict later.

            # Completeness Calculations
            # We don't have the exact 'messages' from args here, but we can compute expected
            # if we trust the total unique messages sent. Actually, we should know the scenario target.
            # But the max msg ID + 1 is safe since generator sends sequentially.
            messages_count = len(unique_messages)
            if messages_count > 0:
                max_msg_id = max(unique_messages)
                messages_count = max(messages_count, max_msg_id + 1)

            expected = len(unique_receivers) * messages_count

            if expected > 0:
                completeness = (received / expected) * 100.0
            else:
                completeness = 0.0

            completeness_data[scenario][transport] = completeness

            # Connection Establishment Calculations
            if conn_start_times:
                conn_start_times.sort()
                first_ts = conn_start_times[0]
                deltas = [(t - first_ts) for t in conn_start_times]

                connection_data[scenario][transport] = {
                    "p50": percentile(deltas, 0.50) / 1000.0,
                    "p95": percentile(deltas, 0.95) / 1000.0
                }

            # Resources Parsing
            if os.path.exists(csv_file):
                rss_vals = []
                cpu_vals = []
                fd_vals = []
                with open(csv_file, 'r') as f:
                    reader = csv.DictReader(f)
                    for row in reader:
                        try:
                            rss_vals.append(float(row['rss_kb']))
                            cpu_vals.append(float(row['cpu_percent']))
                            fd_vals.append(int(row['fd_count']))
                        except (ValueError, KeyError):
                            pass

                if rss_vals:
                    # Convert to MB
                    peak_rss_mb = max(rss_vals) / 1024.0
                    mean_cpu = sum(cpu_vals) / len(cpu_vals)
                    peak_fds = max(fd_vals)

                    if scenario == "large-group":
                        resources_data[transport] = {
                            "peak_rss_mb": peak_rss_mb,
                            "mean_cpu": mean_cpu,
                            "peak_fds": peak_fds
                        }

    # Generate Markdown
    md_path = os.path.join(results_dir, "..", "COMPARISON.md")

    with open(md_path, 'w') as f:
        f.write("# Transport Comparison Report\n\n")

        # Check missing
        missing = False
        for s in scenarios:
            for t in transports:
                if t not in latency_data.get(s, {}):
                    missing = True
                    break
        if missing:
            f.write("> **Note:** Some combinations are missing and displayed as N/A.\n\n")

        f.write("## Section 1 — Per-scenario latency\n\n")

        for s in scenarios:
            f.write(f"### Scenario: {s.replace('-', ' ').title()}\n")
            f.write("| Transport | p50 (ms) | p95 (ms) | p99 (ms) | Max (ms) |\n")
            f.write("|---|---|---|---|---|\n")

            for t in transports:
                if t in latency_data.get(s, {}):
                    d = latency_data[s][t]
                    f.write(f"| {t} | {d['p50']:.2f} | {d['p95']:.2f} | {d['p99']:.2f} | {d['max']:.2f} |\n")
                else:
                    f.write(f"| {t} | N/A | N/A | N/A | N/A |\n")
            f.write("\n")

        f.write("## Section 2 — Delivery completeness\n\n")
        f.write("| Scenario | ws | sse | sockudo |\n")
        f.write("|---|---|---|---|\n")

        for s in scenarios:
            f.write(f"| {s.replace('-', ' ').capitalize()} ")
            for t in transports:
                if t in completeness_data.get(s, {}):
                    f.write(f"| {completeness_data[s][t]:.2f}% ")
                else:
                    f.write("| N/A ")
            f.write("|\n")
        f.write("\n")

        f.write("## Section 3 — Connection establishment (ms)\n\n")
        f.write("| Scenario | ws p50 | ws p95 | sse p50 | sse p95 | sockudo p50 | sockudo p95 |\n")
        f.write("|---|---|---|---|---|---|---|\n")

        for s in scenarios:
            f.write(f"| {s.replace('-', ' ').capitalize()} ")
            for t in transports:
                if t in connection_data.get(s, {}):
                    d = connection_data[s][t]
                    f.write(f"| {d['p50']:.2f} | {d['p95']:.2f} ")
                else:
                    f.write("| N/A | N/A ")
            f.write("|\n")
        f.write("\n")

        f.write("## Section 4 — Resource profile at 500 connections (Large group scenario)\n\n")
        f.write("| Transport | Peak RSS (MB) | Mean CPU (%) | Peak FDs |\n")
        f.write("|---|---|---|---|\n")

        for t in transports:
            if t in resources_data:
                d = resources_data[t]
                f.write(f"| {t} | {d['peak_rss_mb']:.2f} | {d['mean_cpu']:.2f} | {d['peak_fds']} |\n")
            else:
                f.write(f"| {t} | N/A | N/A | N/A |\n")
        f.write("\n")

        # Calculate Verdict
        f.write("## Section 5 — Verdict\n\n")

        # lowest p95 latency
        p95_wins = {t: 0 for t in transports}
        for s in scenarios:
            min_t = None
            min_val = float('inf')
            for t in transports:
                if t in latency_data.get(s, {}):
                    val = latency_data[s][t]['p95']
                    if val < min_val:
                        min_val = val
                        min_t = t
            if min_t:
                p95_wins[min_t] += 1

        # highest delivery completeness
        comp_scores = {t: 0 for t in transports}
        for s in scenarios:
            for t in transports:
                if t in completeness_data.get(s, {}):
                    comp_scores[t] += completeness_data[s][t]

        highest_comp_t = None
        highest_comp_val = -1
        for t, total in comp_scores.items():
            avg = total / len(scenarios)
            if avg > highest_comp_val:
                highest_comp_val = avg
                highest_comp_t = t

        # lowest memory at 500
        lowest_mem_t = None
        lowest_mem_val = float('inf')
        for t in transports:
            if t in resources_data:
                if resources_data[t]['peak_rss_mb'] < lowest_mem_val:
                    lowest_mem_val = resources_data[t]['peak_rss_mb']
                    lowest_mem_t = t

        overall_winner = max(p95_wins, key=p95_wins.get)

        # Check for scenarios where winner differed
        differed_scenarios = []
        for s in scenarios:
            min_t = None
            min_val = float('inf')
            for t in transports:
                if t in latency_data.get(s, {}):
                    val = latency_data[s][t]['p95']
                    if val < min_val:
                        min_val = val
                        min_t = t
            if min_t and min_t != overall_winner:
                differed_scenarios.append(s)

        if p95_wins[overall_winner] == 0:
             f.write("Not enough data to calculate a verdict.\n")
        else:
             f.write(f"{overall_winner.capitalize()} had the lowest p95 latency in {p95_wins[overall_winner]} of {len(scenarios)} scenarios. ")
             if highest_comp_t:
                 f.write(f"{highest_comp_t.capitalize()} had the highest average delivery completeness ({highest_comp_val:.2f}%). ")
             if lowest_mem_t:
                 diff_percent = 0
                 if 'sse' in resources_data and lowest_mem_t != 'sse':
                     diff_percent = ((resources_data['sse']['peak_rss_mb'] - lowest_mem_val) / resources_data['sse']['peak_rss_mb']) * 100
                     f.write(f"{lowest_mem_t.capitalize()} used {diff_percent:.0f}% less memory than PocketBase (sse) at 500 connections. ")
                 else:
                     f.write(f"{lowest_mem_t.capitalize()} used the least memory ({lowest_mem_val:.2f} MB) at 500 connections. ")

             if differed_scenarios:
                 f.write(f"However, in the {', '.join(differed_scenarios).replace('-', ' ')} scenario(s), a different transport had lower latency. ")

             f.write("All transports performed adequately, but ")
             f.write(f"{overall_winner.capitalize()} appears to be the overall winner for this workload.\n")

if __name__ == "__main__":
    main()