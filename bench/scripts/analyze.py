import sys
import json
import statistics
from collections import defaultdict

def main():
    if len(sys.argv) < 2:
        print("Usage: python analyze.py <results.jsonl>")
        sys.exit(1)

    filename = sys.argv[1]

    # Group measurements by payload_bytes
    # Store latencies and compute tracking for each group
    group_latencies = defaultdict(list)
    # count of received messages per group
    group_received = defaultdict(int)
    # We need to know expected per group. This script assumes we can compute
    # expected from the max client_message_id and unique receivers,
    # but the prompt says: completeness formula is `received / (receivers * messages) * 100`
    # Since we group by payload_bytes, let's keep track of unique sender messages and unique receivers per payload size
    group_receivers = defaultdict(set)
    group_messages = defaultdict(set)

    with open(filename, 'r') as f:
        for line in f:
            if not line.strip():
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue

            # We only care about measured phase messages with latency
            if record.get("phase") == "measured" and "latency_us" in record and "payload_bytes" in record:
                pb = record["payload_bytes"]
                group_latencies[pb].append(record["latency_us"])
                group_received[pb] += 1
                if "receiver_id" in record:
                    group_receivers[pb].add(record["receiver_id"])
                if "client_message_id" in record:
                    group_messages[pb].add(record["client_message_id"])

    if not group_latencies:
        print("No measured latency records found.")
        return

    # Print markdown table header
    print("| payload_bytes | received | expected | completeness | p50_us | p95_us | p99_us | mean_us | max_us |")
    print("|---------------|----------|----------|--------------|--------|--------|--------|---------|--------|")

    for pb in sorted(group_latencies.keys()):
        latencies = sorted(group_latencies[pb])
        received = group_received[pb]

        # Calculate expected.
        receivers_count = len(group_receivers[pb])
        messages_count = len(group_messages[pb])
        # If there are missing messages, max ID might give a better bound, but let's assume
        # max message ID + 1 is the total messages sent.
        if messages_count > 0:
            max_msg_id = max(group_messages[pb])
            messages_count = max(messages_count, max_msg_id + 1)

        expected = receivers_count * messages_count
        if expected == 0:
            completeness = 0.0
        else:
            completeness = (received / expected) * 100.0

        p50 = int(statistics.quantiles(latencies, n=100)[49]) if len(latencies) >= 100 else latencies[int(len(latencies)*0.50)]

        # for p95, p99 we can just use percentiles if we have enough elements, or manual calculation
        def percentile(data, p):
            if not data:
                return 0
            idx = int(p * len(data))
            if idx >= len(data):
                idx = len(data) - 1
            return data[idx]

        p95 = percentile(latencies, 0.95)
        p99 = percentile(latencies, 0.99)
        # re-calculate p50 with same method for consistency if few elements
        p50 = percentile(latencies, 0.50)

        mean_val = int(statistics.mean(latencies))
        max_val = max(latencies)

        print(f"| {pb:<13} | {received:<8} | {expected:<8} | {completeness:>6.2f}%       | {p50:<6} | {p95:<6} | {p99:<6} | {mean_val:<7} | {max_val:<6} |")

if __name__ == "__main__":
    main()
