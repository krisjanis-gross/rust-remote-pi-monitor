use chrono::NaiveDateTime;

#[derive(Serialize, Deserialize, Queryable)]
pub struct ApiKey {
    pub id: i32,
    pub api_key: String,
}

#[derive(Serialize, Deserialize, Queryable)]
pub struct Nodeslist {
    pub id: i32,
    pub node_id_external: String,
    pub fk_api_key_id: i32,
    pub monitoring_enabled: i8,
    pub last_checkin_timestamp: NaiveDateTime,
    pub notification_email_list: Option<String>,
    pub offline_notification_sent: i8,
}
