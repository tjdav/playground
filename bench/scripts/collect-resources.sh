#!/bin/bash

if [ -z "$1" ] || [ -z "$2" ]; then
  echo "Usage: $0 <target> <output_path> [interval_secs]"
  exit 1
fi

TARGET="$1"
OUTPUT="$2"
INTERVAL="${3:-1}"

echo "timestamp_ms,rss_kb,cpu_percent,fd_count" > "$OUTPUT"

# Setup cleanup on termination
write_final_sample() {
  sample
  exit 0
}
trap write_final_sample SIGTERM SIGINT

sample() {
  local ts=$(date +%s%3N)
  local rss=0
  local cpu=0
  local fds=0

  if [[ "$TARGET" =~ ^[0-9]+$ ]]; then
    # Target is a PID
    if ! kill -0 "$TARGET" 2>/dev/null; then
      exit 0
    fi
    # Use standard ps output: rss in KB, pcpu
    # Convert output to array to safely extract values
    local ps_out=($(ps -o rss=,pcpu= -p "$TARGET"))
    rss=${ps_out[0]:-0}
    cpu=${ps_out[1]:-0}
    fds=$(ls /proc/"$TARGET"/fd 2>/dev/null | wc -l)
  else
    # Target is a container
    local stats_out=$(docker stats --no-stream --format '{{json .}}' "$TARGET" 2>/dev/null)
    if [ -n "$stats_out" ]; then
      local mem_usage=$(echo "$stats_out" | jq -r '.MemUsage')
      local cpu_perc=$(echo "$stats_out" | jq -r '.CPUPerc')

      # Strip % from CPU
      cpu=${cpu_perc%\%}

      # MemUsage looks like "12.3MiB / 1GiB". We take the first part.
      local mem_val=$(echo "$mem_usage" | awk '{print $1}')

      # Convert mem_val to KB based on suffix
      if [[ "$mem_val" == *GiB ]]; then
        local val=${mem_val%GiB}
        rss=$(awk "BEGIN {print int($val * 1024 * 1024)}")
      elif [[ "$mem_val" == *MiB ]]; then
        local val=${mem_val%MiB}
        rss=$(awk "BEGIN {print int($val * 1024)}")
      elif [[ "$mem_val" == *KiB ]]; then
        rss=${mem_val%KiB}
      elif [[ "$mem_val" == *B ]]; then
        local val=${mem_val%B}
        rss=$(awk "BEGIN {print int($val / 1024)}")
      fi

      local pid=$(docker inspect --format '{{.State.Pid}}' "$TARGET" 2>/dev/null)
      if [ -n "$pid" ] && [ "$pid" != "0" ]; then
        fds=$(sudo ls /proc/"$pid"/fd 2>/dev/null | wc -l || ls /proc/"$pid"/fd 2>/dev/null | wc -l)
      fi
    fi
  fi

  # Fallbacks if parsing fails
  [ -z "$rss" ] && rss=0
  [ -z "$cpu" ] && cpu=0
  [ -z "$fds" ] && fds=0

  echo "$ts,$rss,$cpu,$fds" >> "$OUTPUT"
}

while true; do
  sample
  sleep "$INTERVAL"
done