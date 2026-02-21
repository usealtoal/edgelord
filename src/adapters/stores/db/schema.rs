// @generated automatically by Diesel CLI.

diesel::table! {
    clusters (id) {
        id -> Text,
        market_ids -> Text,
        relation_ids -> Text,
        constraints_json -> Text,
        updated_at -> Text,
    }
}

diesel::table! {
    daily_stats (date) {
        date -> Text,
        opportunities_detected -> Integer,
        opportunities_executed -> Integer,
        opportunities_rejected -> Integer,
        trades_opened -> Integer,
        trades_closed -> Integer,
        profit_realized -> Float,
        loss_realized -> Float,
        win_count -> Integer,
        loss_count -> Integer,
        total_volume -> Float,
        peak_exposure -> Float,
        latency_sum_ms -> Integer,
        latency_count -> Integer,
    }
}

diesel::table! {
    opportunities (id) {
        id -> Nullable<Integer>,
        strategy -> Text,
        market_ids -> Text,
        edge -> Float,
        expected_profit -> Float,
        detected_at -> Text,
        executed -> Integer,
        rejected_reason -> Nullable<Text>,
    }
}

diesel::table! {
    relations (id) {
        id -> Text,
        kind -> Text,
        confidence -> Float,
        reasoning -> Text,
        inferred_at -> Text,
        expires_at -> Text,
        market_ids -> Text,
    }
}

diesel::table! {
    strategy_daily_stats (date, strategy) {
        date -> Text,
        strategy -> Text,
        opportunities_detected -> Integer,
        opportunities_executed -> Integer,
        trades_opened -> Integer,
        trades_closed -> Integer,
        profit_realized -> Float,
        win_count -> Integer,
        loss_count -> Integer,
    }
}

diesel::table! {
    trades (id) {
        id -> Nullable<Integer>,
        opportunity_id -> Integer,
        strategy -> Text,
        market_ids -> Text,
        legs -> Text,
        size -> Float,
        expected_profit -> Float,
        realized_profit -> Nullable<Float>,
        status -> Text,
        opened_at -> Text,
        closed_at -> Nullable<Text>,
        close_reason -> Nullable<Text>,
    }
}

diesel::joinable!(trades -> opportunities (opportunity_id));

diesel::allow_tables_to_appear_in_same_query!(
    clusters,
    daily_stats,
    opportunities,
    relations,
    strategy_daily_stats,
    trades,
);
