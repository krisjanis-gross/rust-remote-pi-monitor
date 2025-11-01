
    use serde::{Deserialize, Serialize};
    use tokio_pg_mapper_derive::PostgresMapper;
    use chrono::{  Utc};

    #[derive(Deserialize, Serialize)]
    pub struct CheckinData {
        pub api_key: String,
        pub node_id: String,
        pub sensor_data: Option<Vec<SensorData>>,
    }

    #[derive(Deserialize, Serialize)]
    pub struct SensorData {
        pub id: String,
        pub sensor_name: String,
        pub value: f32,
    }

    #[derive(Deserialize, PostgresMapper, Serialize)]
    #[pg_mapper(table = "sensor_triggers")] // singular 'user' is a keyword..
    pub struct SensorTrigger {
        pub sensor_triggers_id: i32,
        pub node_id: i32,
        pub sensor_id: String,
        pub monitoring_enabled: bool,
        pub trigger_notification_sent: bool,
        pub  validation_function: String,
        pub validation_parameter_1: Option<f32>,
        pub validation_parameter_2: Option<f32>,
    }

    #[derive(Deserialize, PostgresMapper, Serialize)]
    #[pg_mapper(table = "nodes")] // singular 'user' is a keyword..
    pub struct Nodes {
        pub id: i32,
        pub node_id_external: String,
        pub fk_api_key_id: i32,
        pub monitoring_enabled: bool,
        pub last_checkin_timestamp: chrono::DateTime<Utc>,
        pub notification_email_list:  String,
        pub offline_notification_sent: bool,
    }

    #[derive(Debug, Default, Deserialize,Clone)]
    pub struct Email {
        pub smtp_server: String,
        pub username: String,
        pub password: String,
    }


    #[derive(Debug, Default, Deserialize,Clone)]
    pub struct TelegramConfig {
        pub bot_token: String,
        pub channel_id: String,

    }