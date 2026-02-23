#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[architecture] checking deprecated exchange_config and operator symbols"
if rg -n "exchange_config::|\bExchangeConfig\b|create_exchange_config|market_mapper|create_market_mapper|PolymarketExchangeConfig|infrastructure::operator::service|\bOperatorService\b" src tests >/dev/null; then
  echo "[architecture] ERROR: found deprecated symbols"
  exit 1
fi

echo "[architecture] checking monolithic operator file removal"
if [[ -f src/infrastructure/operator/service.rs ]]; then
  echo "[architecture] ERROR: src/infrastructure/operator/service.rs should not exist"
  exit 1
fi

echo "[architecture] checking inbound CLI does not import infrastructure directly"
INBOUND_INFRA_HITS="$(rg -n "use crate::infrastructure|crate::infrastructure::" src/adapter/inbound/cli || true)"
if [[ -n "$INBOUND_INFRA_HITS" ]]; then
  echo "$INBOUND_INFRA_HITS"
  echo "[architecture] ERROR: inbound CLI has direct infrastructure imports"
  exit 1
fi

echo "[architecture] checking orchestration processing placement"
if [[ ! -f src/application/orchestration/handler.rs || ! -f src/application/orchestration/execution.rs || ! -f src/application/orchestration/context.rs || ! -f src/application/orchestration/event.rs || ! -f src/application/orchestration/opportunity.rs || ! -f src/application/orchestration/slippage.rs || ! -f src/application/orchestration/position.rs ]]; then
  echo "[architecture] ERROR: application orchestration modules are incomplete"
  exit 1
fi
if [[ -f src/infrastructure/orchestration/handler.rs || -f src/infrastructure/orchestration/execution.rs ]]; then
  echo "[architecture] ERROR: infrastructure still owns application orchestration handlers"
  exit 1
fi
if [[ ! -f src/infrastructure/orchestration/context.rs || ! -f src/infrastructure/orchestration/health.rs || ! -f src/infrastructure/orchestration/runtime.rs || ! -f src/infrastructure/orchestration/startup.rs || ! -f src/infrastructure/orchestration/stream.rs || ! -f src/infrastructure/orchestration/inference.rs || ! -f src/infrastructure/orchestration/cluster.rs ]]; then
  echo "[architecture] ERROR: infrastructure orchestration modules are incomplete"
  exit 1
fi

echo "[architecture] checking orchestration façade boundaries"
if rg -n "pub\(crate\) fn get_max_slippage\(" src/application/orchestration/handler.rs >/dev/null; then
  echo "[architecture] ERROR: handler.rs should delegate slippage logic to slippage.rs"
  exit 1
fi
if rg -n "pub async fn run_with_shutdown\(|pub fn health_check\(" src/infrastructure/orchestration/orchestrator.rs >/dev/null; then
  echo "[architecture] ERROR: orchestrator.rs should be façade-only"
  exit 1
fi

echo "[architecture] checking inbound operator ports are transport-agnostic"
if rg -n "std::path::Path|PathBuf" src/port/inbound/operator >/dev/null; then
  echo "[architecture] ERROR: inbound operator ports expose filesystem path types"
  exit 1
fi

echo "[architecture] checking application layer adapter agnosticism"
APP_ADAPTER_HITS="$(rg -n "crate::adapter::" src/application || true)"
if [[ -n "$APP_ADAPTER_HITS" ]]; then
  echo "$APP_ADAPTER_HITS"
  echo "[architecture] ERROR: application layer imports adapters directly"
  exit 1
fi

echo "[architecture] checking infrastructure orchestrator context ownership"
if rg -n "pub use crate::application::orchestration::handler::MarketEventHandlingContext as EventProcessingContext;" \
  src/infrastructure/orchestration/orchestrator.rs >/dev/null; then
  echo "[architecture] ERROR: infrastructure orchestrator re-exports application context type"
  exit 1
fi

echo "[architecture] checking LLM port placement"
if [[ ! -f src/port/outbound/llm.rs ]]; then
  echo "[architecture] ERROR: src/port/outbound/llm.rs should exist"
  exit 1
fi
if rg -n "crate::adapter::outbound::llm" src/adapter/outbound/inference/inferrer.rs >/dev/null; then
  echo "[architecture] ERROR: inference adapter still depends directly on llm adapters"
  exit 1
fi

echo "[architecture] checking report query placement"
if [[ -f src/infrastructure/status.rs || -f src/infrastructure/stats.rs ]]; then
  echo "[architecture] ERROR: infrastructure status/stats query modules should not exist"
  exit 1
fi

echo "[architecture] checking remaining hotspot SRP splits"
if [[ ! -f src/infrastructure/exchange/pool/state.rs \
   || ! -f src/infrastructure/exchange/pool/spawn.rs \
   || ! -f src/infrastructure/exchange/pool/replace.rs \
   || ! -f src/infrastructure/exchange/pool/manage.rs \
   || ! -f src/infrastructure/exchange/pool/tests.rs ]]; then
  echo "[architecture] ERROR: exchange pool split modules are incomplete"
  exit 1
fi
if rg -n "fn spawn_connection\(|async fn replace_connection\(|async fn management_task\(" src/infrastructure/exchange/pool.rs >/dev/null; then
  echo "[architecture] ERROR: exchange pool root should be façade-only over split internals"
  exit 1
fi

if [[ ! -f src/adapter/outbound/notifier/telegram/control/runtime.rs \
   || ! -f src/adapter/outbound/notifier/telegram/control/dispatch.rs \
   || ! -f src/adapter/outbound/notifier/telegram/control/render.rs \
   || ! -f src/adapter/outbound/notifier/telegram/control/mutate.rs \
   || ! -f src/adapter/outbound/notifier/telegram/control/tests.rs ]]; then
  echo "[architecture] ERROR: telegram control split modules are incomplete"
  exit 1
fi
if rg -n "fn status_text\(|fn health_text\(|fn pause_text\(" src/adapter/outbound/notifier/telegram/control.rs >/dev/null; then
  echo "[architecture] ERROR: telegram control root should be façade-only over split internals"
  exit 1
fi

if [[ ! -f src/infrastructure/subscription/priority/state.rs \
   || ! -f src/infrastructure/subscription/priority/queue.rs \
   || ! -f src/infrastructure/subscription/priority/contract.rs \
   || ! -f src/infrastructure/subscription/priority/event.rs \
   || ! -f src/infrastructure/subscription/priority/tests.rs ]]; then
  echo "[architecture] ERROR: priority manager split modules are incomplete"
  exit 1
fi
if rg -n "fn read_lock<T>\(|while markets_added < count|while tokens_to_remove > 0" src/infrastructure/subscription/priority.rs >/dev/null; then
  echo "[architecture] ERROR: priority root should be façade-only over split internals"
  exit 1
fi

echo "[architecture] checking mod.rs files are export-only"
while IFS= read -r mod_file; do
  if awk '{
    line=$0
    sub(/^[[:space:]]+/, "", line)
    if (line=="" || line ~ /^\/\// || line ~ /^pub mod / || line ~ /^mod / || line ~ /^#\[cfg/) {
      next
    }
    print FILENAME ":" NR ":" $0
    bad=1
  }
  END { if (bad) exit 1 }' "$mod_file"; then
    :
  else
    echo "[architecture] ERROR: mod.rs export-only check failed"
    exit 1
  fi
done < <(find src -name mod.rs | sort)

echo "[architecture] all checks passed"
