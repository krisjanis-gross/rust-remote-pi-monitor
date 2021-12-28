table! {
    api_keys (id) {
        id -> Integer,
        api_key -> Varchar,
    }
}

table! {
    nodes (id) {
        id -> Integer,
        node_id_external -> Varchar,
        fk_api_key_id -> Integer,
        monitoring_enabled -> Tinyint,
        last_checkin_timestamp -> Datetime,
        notification_email_list -> Nullable<Varchar>,
        offline_notification_sent -> Tinyint,
    }
}

allow_tables_to_appear_in_same_query!(
    api_keys,
    nodes,
);
