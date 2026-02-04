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

diesel::allow_tables_to_appear_in_same_query!(clusters, relations,);
