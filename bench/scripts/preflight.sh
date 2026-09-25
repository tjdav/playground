#!/bin/bash

# 1. Compose files use named volumes
for f in bench/pocketbase/docker-compose.yml bench/sockudo/docker-compose.yml; do
  if grep -E "^\s*-\s*\./" "$f"; then
    echo "FAIL: $f uses a host-path bind mount"
    exit 1
  fi
done

# 2. No pb_data/ directory exists in the repo
if find . -type d -name pb_data -not -path "*/node_modules/*" -not -path "*/target/*" | grep -q .; then
  echo "FAIL: pb_data/ directory found. It should never exist when using Docker named volumes."
  exit 1
fi

# 3. Required ports are free (or held by the expected container)
for port in 8090 6001 8080; do
  if lsof -i :$port -sTCP:LISTEN -t >/dev/null 2>&1; then
    # lsof output means something is listening
    container=$(docker ps --format '{{.Names}} {{.Ports}}' | grep ":$port->" | awk '{print $1}')
    if [ -z "$container" ]; then
      echo "FAIL: port $port is held by a non-Docker process"
      exit 1
    fi
  fi
done

# 4. Binaries exist
test -x bench/target/release/relay || { echo "FAIL: relay not built"; exit 1; }
test -x bench/target/release/generator || { echo "FAIL: generator not built"; exit 1; }

# 5. Docker is available
docker info >/dev/null 2>&1 || { echo "FAIL: Docker daemon not reachable"; exit 1; }

exit 0