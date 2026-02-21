//! Strategy listing and explanation.

use tabled::{Table, Tabled};

use crate::cli::output;
use crate::error::Result;

#[derive(Tabled)]
struct StrategyRow {
    #[tabled(rename = "Name")]
    name: &'static str,
    #[tabled(rename = "Signal")]
    signal: &'static str,
    #[tabled(rename = "Typical Edge")]
    edge: &'static str,
}

/// List available strategies.
pub fn list() -> Result<()> {
    output::header(env!("CARGO_PKG_VERSION"));
    output::section("Available strategies");
    println!();

    let strategies = vec![
        StrategyRow {
            name: "single-condition",
            signal: "YES + NO < $1",
            edge: "2-5%",
        },
        StrategyRow {
            name: "market-rebalancing",
            signal: "sum(outcomes) < $1",
            edge: "1-3%",
        },
        StrategyRow {
            name: "combinatorial",
            signal: "cross-market constraints",
            edge: "<1%",
        },
    ];

    let table = Table::new(strategies).to_string();
    for line in table.lines() {
        println!("  {}", line);
    }

    println!();
    println!(
        "  Run {} for details",
        output::highlight("edgelord strategies explain <name>")
    );
    println!();

    Ok(())
}

/// Explain a specific strategy.
pub fn explain(name: &str) -> Result<()> {
    output::header(env!("CARGO_PKG_VERSION"));

    match name {
        "single-condition" => explain_single_condition(),
        "market-rebalancing" => explain_market_rebalancing(),
        "combinatorial" => explain_combinatorial(),
        _ => {
            output::error(&format!("Unknown strategy: {}", name));
            println!();
            println!("  Available: single-condition, market-rebalancing, combinatorial");
            return Ok(());
        }
    }

    Ok(())
}

fn explain_single_condition() {
    output::section("single-condition");
    println!();
    println!("  Detects arbitrage in binary (YES/NO) markets where:");
    println!("  YES price + NO price < $1.00 payout");
    println!();
    println!("  Example:");
    println!("    YES @ $0.45 + NO @ $0.52 = $0.97");
    println!("    Payout = $1.00");
    println!("    Edge = $0.03 (3%)");
    println!();
    println!("  Configuration:");
    println!("    [strategies.single-condition]");
    println!("    min_edge = 0.05    # 5% minimum edge");
    println!("    min_profit = 0.50  # $0.50 minimum profit");
    println!();
}

fn explain_market_rebalancing() {
    output::section("market-rebalancing");
    println!();
    println!("  Detects arbitrage in multi-outcome markets where:");
    println!("  sum(all outcome prices) < $1.00 payout");
    println!();
    println!("  Example (3-outcome market):");
    println!("    Option A @ $0.30 + Option B @ $0.35 + Option C @ $0.32 = $0.97");
    println!("    Payout = $1.00");
    println!("    Edge = $0.03 (3%)");
    println!();
    println!("  Configuration:");
    println!("    [strategies.market-rebalancing]");
    println!("    min_edge = 0.03");
    println!();
}

fn explain_combinatorial() {
    output::section("combinatorial");
    println!();
    println!("  Detects arbitrage across related markets using:");
    println!("  - LLM inference to identify market relationships");
    println!("  - LP/ILP optimization to find profitable combinations");
    println!();
    println!("  Example:");
    println!("    Market A: \"Will X happen in 2024?\"");
    println!("    Market B: \"Will X happen in Q4 2024?\"");
    println!("    Constraint: B implies A");
    println!();
    println!("  Requires:");
    println!("    [inference]");
    println!("    provider = \"anthropic\"  # or \"openai\"");
    println!();
    println!("  Configuration:");
    println!("    [strategies.combinatorial]");
    println!("    min_edge = 0.02");
    println!();
}
