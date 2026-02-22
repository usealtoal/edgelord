//! Architecture contract tests.

mod support;

use support::architecture::{
    find_lines_containing, find_non_export_lines_in_mod_files, path_exists, read_relative,
};

#[test]
fn cli_has_no_direct_infrastructure_imports() {
    let hits = find_lines_containing(
        "src/adapter/inbound/cli",
        &["use crate::infrastructure", "crate::infrastructure::"],
    );

    assert!(
        hits.is_empty(),
        "found direct infrastructure imports in inbound CLI adapters: {hits:#?}"
    );
}

#[test]
fn domain_has_no_framework_or_outer_layer_imports() {
    let hits = find_lines_containing(
        "src/domain",
        &[
            "crate::adapter",
            "crate::infrastructure",
            "crate::application",
            "tokio::",
            "reqwest::",
            "diesel::",
        ],
    );

    assert!(
        hits.is_empty(),
        "found forbidden imports in domain layer: {hits:#?}"
    );
}

#[test]
fn legacy_exchange_config_port_is_removed() {
    assert!(
        !path_exists("src/port/outbound/exchange_config.rs"),
        "legacy exchange_config port file should be removed"
    );
}

#[test]
fn mod_rs_is_export_only() {
    let violations = find_non_export_lines_in_mod_files("src");
    assert!(
        violations.is_empty(),
        "found non-export content in mod.rs files: {violations:#?}"
    );
}

#[test]
fn cli_operator_bridge_uses_operator_name() {
    let source = read_relative("src/adapter/inbound/cli/operator.rs");
    assert!(
        source.contains("pub fn operator() -> &'static dyn OperatorPort"),
        "operator bridge should expose `operator()` capability accessor"
    );
}

#[test]
fn opportunity_handler_uses_context_parameter() {
    let source = read_relative("src/application/orchestration/handler.rs");
    assert!(
        source.contains("pub(crate) struct OpportunityHandlingContext<'a>"),
        "opportunity handler should define an OpportunityHandlingContext"
    );
    assert!(
        source.contains("pub(crate) fn handle_opportunity("),
        "opportunity handler function should exist"
    );
    assert!(
        source.contains("context: OpportunityHandlingContext<'_>"),
        "handle_opportunity should accept a single context parameter"
    );
}

#[test]
fn market_event_handler_uses_context_parameter() {
    let source = read_relative("src/application/orchestration/handler.rs");
    assert!(
        source.contains("pub struct MarketEventHandlingContext<'a>"),
        "market event handler should define MarketEventHandlingContext"
    );
    assert!(
        source.contains("pub(crate) fn handle_market_event("),
        "market event handler function should exist"
    );
    assert!(
        source.contains("context: MarketEventHandlingContext<'_>"),
        "handle_market_event should accept one context object"
    );
    assert!(
        !source.contains("#[allow(clippy::too_many_arguments)]"),
        "market event handler should not need too_many_arguments allow"
    );
}

#[test]
fn orchestration_processing_lives_in_application_layer() {
    assert!(
        path_exists("src/application/orchestration/handler.rs"),
        "market event processing handler should live under application/orchestration"
    );
    assert!(
        path_exists("src/application/orchestration/execution.rs"),
        "execution coordination should live under application/orchestration"
    );
    assert!(
        path_exists("src/application/orchestration/context.rs"),
        "detection context should live under application/orchestration"
    );
    assert!(
        !path_exists("src/infrastructure/orchestration/handler.rs"),
        "infrastructure layer should not own market event processing handler"
    );
    assert!(
        !path_exists("src/infrastructure/orchestration/execution.rs"),
        "infrastructure layer should not own execution coordination module"
    );
}

#[test]
fn llm_contract_lives_in_outbound_port() {
    assert!(
        path_exists("src/port/outbound/llm.rs"),
        "LLM trait contract should live under port/outbound"
    );

    let inferrer = read_relative("src/adapter/outbound/inference/inferrer.rs");
    assert!(
        !inferrer.contains("crate::adapter::outbound::llm"),
        "inference adapter should not depend on llm adapter modules directly"
    );
    assert!(
        inferrer.contains("crate::port::outbound::llm::Llm"),
        "inference adapter should depend on outbound llm port contract"
    );
}

#[test]
fn operator_ports_are_transport_agnostic() {
    let hits = find_lines_containing("src/port/inbound/operator", &["std::path::Path", "PathBuf"]);
    assert!(
        hits.is_empty(),
        "operator inbound ports should not expose filesystem path types: {hits:#?}"
    );
}

#[test]
fn application_layer_has_no_direct_adapter_imports() {
    let hits = find_lines_containing("src/application", &["crate::adapter::"]);
    assert!(
        hits.is_empty(),
        "application layer should not import adapters directly: {hits:#?}"
    );
}

#[test]
fn infrastructure_orchestrator_does_not_reexport_application_context_type() {
    let source = read_relative("src/infrastructure/orchestration/orchestrator.rs");
    assert!(
        !source.contains(
            "pub use crate::application::orchestration::handler::MarketEventHandlingContext as EventProcessingContext;"
        ),
        "infrastructure orchestrator should not re-export application context types"
    );
}

#[test]
fn application_orchestration_is_srp_split() {
    for file in [
        "src/application/orchestration/event.rs",
        "src/application/orchestration/opportunity.rs",
        "src/application/orchestration/slippage.rs",
        "src/application/orchestration/position.rs",
    ] {
        assert!(
            path_exists(file),
            "expected application orchestration module `{file}`"
        );
    }
}

#[test]
fn infrastructure_orchestration_is_srp_split() {
    for file in [
        "src/infrastructure/orchestration/context.rs",
        "src/infrastructure/orchestration/health.rs",
        "src/infrastructure/orchestration/runtime.rs",
        "src/infrastructure/orchestration/startup.rs",
        "src/infrastructure/orchestration/stream.rs",
        "src/infrastructure/orchestration/inference.rs",
        "src/infrastructure/orchestration/cluster.rs",
    ] {
        assert!(
            path_exists(file),
            "expected infrastructure orchestration module `{file}`"
        );
    }
}

#[test]
fn application_handler_is_a_faade() {
    let source = read_relative("src/application/orchestration/handler.rs");
    assert!(
        source.contains("event::handle_market_event"),
        "handler should delegate event flow to event module"
    );
    assert!(
        source.contains("opportunity::handle_opportunity"),
        "handler should delegate opportunity flow to opportunity module"
    );
    assert!(
        !source.contains("pub(crate) fn get_max_slippage("),
        "handler should not own slippage calculation implementation"
    );
}

#[test]
fn infrastructure_orchestrator_is_a_faade() {
    let source = read_relative("src/infrastructure/orchestration/orchestrator.rs");
    assert!(
        source.contains("runtime::run_with_shutdown"),
        "orchestrator should delegate runtime loop to runtime module"
    );
    assert!(
        !source.contains("pub async fn run_with_shutdown("),
        "orchestrator should not own run_with_shutdown implementation"
    );
    assert!(
        !source.contains("pub fn health_check("),
        "orchestrator should not own health checks"
    );
}

#[test]
fn exchange_pool_is_srp_split() {
    for file in [
        "src/infrastructure/exchange/pool/state.rs",
        "src/infrastructure/exchange/pool/spawn.rs",
        "src/infrastructure/exchange/pool/replace.rs",
        "src/infrastructure/exchange/pool/manage.rs",
        "src/infrastructure/exchange/pool/tests.rs",
    ] {
        assert!(path_exists(file), "expected exchange pool module `{file}`");
    }

    let source = read_relative("src/infrastructure/exchange/pool.rs");
    assert!(
        source.contains("mod manage;")
            && source.contains("mod replace;")
            && source.contains("mod spawn;")
            && source.contains("mod state;"),
        "pool root should declare split submodules"
    );
    assert!(
        !source.contains("fn spawn_connection("),
        "pool root should not own connection spawn internals"
    );
    assert!(
        !source.contains("async fn replace_connection("),
        "pool root should not own replacement internals"
    );
    assert!(
        !source.contains("async fn management_task("),
        "pool root should not own management loop internals"
    );
}

#[test]
fn telegram_control_is_srp_split() {
    for file in [
        "src/adapter/outbound/notifier/telegram/control/runtime.rs",
        "src/adapter/outbound/notifier/telegram/control/dispatch.rs",
        "src/adapter/outbound/notifier/telegram/control/render.rs",
        "src/adapter/outbound/notifier/telegram/control/mutate.rs",
        "src/adapter/outbound/notifier/telegram/control/tests.rs",
    ] {
        assert!(
            path_exists(file),
            "expected telegram control module `{file}`"
        );
    }

    let source = read_relative("src/adapter/outbound/notifier/telegram/control.rs");
    assert!(
        source.contains("mod runtime;")
            && source.contains("mod dispatch;")
            && source.contains("mod render;")
            && source.contains("mod mutate;"),
        "telegram control root should declare split submodules"
    );
    assert!(
        !source.contains("fn status_text("),
        "telegram control root should not own status rendering"
    );
    assert!(
        !source.contains("fn health_text("),
        "telegram control root should not own health rendering"
    );
    assert!(
        !source.contains("fn pause_text("),
        "telegram control root should not own mutation logic"
    );
}

#[test]
fn priority_manager_is_srp_split() {
    for file in [
        "src/infrastructure/subscription/priority/state.rs",
        "src/infrastructure/subscription/priority/queue.rs",
        "src/infrastructure/subscription/priority/contract.rs",
        "src/infrastructure/subscription/priority/event.rs",
        "src/infrastructure/subscription/priority/tests.rs",
    ] {
        assert!(path_exists(file), "expected priority module `{file}`");
    }

    let source = read_relative("src/infrastructure/subscription/priority.rs");
    assert!(
        source.contains("mod state;")
            && source.contains("mod queue;")
            && source.contains("mod contract;")
            && source.contains("mod event;"),
        "priority root should declare split submodules"
    );
    assert!(
        !source.contains("fn read_lock<T>("),
        "priority root should not own lock helper internals"
    );
    assert!(
        !source.contains("while markets_added < count"),
        "priority root should not own expand queue internals"
    );
    assert!(
        !source.contains("while tokens_to_remove > 0"),
        "priority root should not own contract internals"
    );
}
