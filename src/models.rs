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
#[derive(Serialize, Deserialize, Queryable)]
pub struct SensorTriggersList {
    pub sensor_triggers_id: i32,
    pub node_id: i32,
    pub sensor_id: String,
    pub monitoring_enabled: i8,
    pub trigger_notification_sent: i8,
    pub validation_function: String,
    pub validation_parameter_1: Option<f32>,
    pub validation_parameter_2: Option<f32>,
}


// input parameters to the /checkin web service
#[derive(Serialize, Deserialize)]
pub struct SensorData {
    pub(crate) id: String,
    pub(crate) sensor_name: String,
    pub(crate) value: f32,
}

#[derive(Serialize, Deserialize)]
pub struct CheckinData {
    pub(crate) api_key: String,
    pub(crate) node_id: String,
    pub(crate) sensor_data: Option<Vec<SensorData>>,
}