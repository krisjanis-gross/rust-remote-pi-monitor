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

table! {
    sensor_triggers (sensor_triggers_id) {
        sensor_triggers_id -> Integer,
        node_id -> Integer,
        sensor_id -> Varchar,
        monitoring_enabled -> Tinyint,
        trigger_notification_sent -> Tinyint,
        validation_function -> Varchar,
        validation_parameter_1 -> Nullable<Float>,
        validation_parameter_2 -> Nullable<Float>,
    }
}

allow_tables_to_appear_in_same_query!(
    api_keys,
    nodes,
    sensor_triggers,
);
