mod config {
    use serde::Deserialize;
    #[derive(Debug, Default, Deserialize)]
    pub struct ExampleConfig {
        pub server_addr: String,
        pub pg: deadpool_postgres::Config,
        pub email: Email,
    }

    #[derive(Debug, Default, Deserialize,Clone)]
    pub struct Email {
        pub smtp_server: String,
        pub username: String,
        pub password: String,
    }
}

mod models {
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


}
mod errors {
    use actix_web::{HttpResponse, ResponseError};
    use deadpool_postgres::PoolError;
    use derive_more::{Display, From};
    use tokio_pg_mapper::Error as PGMError;
    use tokio_postgres::error::Error as PGError;

    #[derive(Display, From, Debug)]
    pub enum MyError {
        NotFound,
        PGError(PGError),
        PGMError(PGMError),
        PoolError(PoolError),
    }
    impl std::error::Error for MyError {}

    impl ResponseError for MyError {
        fn error_response(&self) -> HttpResponse {
            match *self {
                MyError::NotFound => HttpResponse::NotFound().finish(),
                MyError::PoolError(ref err) => {
                    HttpResponse::InternalServerError().body(err.to_string())
                }
                _ => HttpResponse::InternalServerError().finish(),
            }
        }
    }
}


mod handlers {
    use actix_web::{web, Error, HttpResponse};
    use deadpool_postgres::{Client, Pool};
    use log::debug;
    use log::error;
    use log::info;
    use crate::send_email;

    use chrono::{DateTime, Duration, Utc};

    use crate::{ models::CheckinData, models::SensorData, models::SensorTrigger,models::Nodes,config::Email};

    pub async fn status_check( ) -> &'static str {
        "Remote-pi-monitor has started!"
    }

    pub async fn checkin_node (
        checkin_data: web::Json<CheckinData>,
        db_pool: web::Data<Pool>,
        email_config: web::Data<Email>,
    ) -> Result<HttpResponse, Error> {

        debug!(
        "checkin_data: API-key={} node_id={}",
        checkin_data.api_key, checkin_data.node_id
        );
        let mut log_status_message = "".to_string();
        let mut status_message;

        let client = db_pool.get().await.unwrap();
        let stmt = client.prepare_cached("SELECT id, api_key	FROM remote_pi_monitor.api_keys where api_key = $1").await.unwrap();
        let rows = client.query(&stmt, &[&checkin_data.api_key] ).await.unwrap();
        if rows.is_empty() {
            error!("API key not found. api_key = {} " , checkin_data.api_key);
            status_message = format!("api_key = {} is not found", checkin_data.api_key);
            log_status_message.push_str(&status_message );
        }
        else { // API key is found. Continue with node checkin
            let api_key_id: i32 = rows[0].get( 0);
            status_message = format!("api_key_id = {}", checkin_data.api_key, api_key_id);
            log_status_message.push_str(&status_message );

            // find node in nodes table
            let stmt_nodes = client.prepare_cached("SELECT id, node_id_external, fk_api_key_id, monitoring_enabled, last_checkin_timestamp, notification_email_list, offline_notification_sent
	FROM remote_pi_monitor.nodes where fk_api_key_id= $1 AND node_id_external = $2").await.unwrap();
            let rows = client.query(&stmt_nodes, &[&api_key_id,&checkin_data.node_id] ).await.unwrap();
            if rows.is_empty() { // node ID is not found. Needs to be added to DB
                debug!("Node id = {} not found. Adding new node to db" , &checkin_data.node_id);

                let stmt_node_insert = client.prepare_cached("INSERT INTO remote_pi_monitor.nodes(
	id, node_id_external, fk_api_key_id, monitoring_enabled, last_checkin_timestamp, notification_email_list, offline_notification_sent)
	VALUES (DEFAULT, $1, $2, DEFAULT, now(), '', DEFAULT);").await.unwrap();
                let _rows = client.query(&stmt_node_insert, &[&checkin_data.node_id, &api_key_id] ).await.unwrap();

                status_message = format!(" node id = {} added to db", &checkin_data.node_id);
                log_status_message.push_str(&status_message );

            } else {  // node is found. Need to update checkin timestamp and send online notification in case it was offline
                debug!("Node id = {} is found. Updating checkin timestamp" , &checkin_data.node_id);

                // update checkin timestamp
                let node_id_db: i32 = rows[0].get( 0);
                debug!("nodes.id = {}" , &node_id_db);

                let email_notification_list: String =  rows[0].get( 5);


                let node_checkin_timestamp = Utc::now();

                let stmt_timestamp_update = client.prepare_cached("UPDATE remote_pi_monitor.nodes SET last_checkin_timestamp= $2, offline_notification_sent=false WHERE id= $1;").await.unwrap();
                let _rows = client.query(&stmt_timestamp_update, &[&node_id_db,&node_checkin_timestamp] ).await.unwrap();

                status_message = format!(" nodes.id = {} nodes.node_id_external = {}", &node_id_db, &checkin_data.node_id);
                log_status_message.push_str(&status_message );

                // send notification in case node was offline before
                let node_monitoring_enabled: bool= rows[0].get( 3);
                let node_offline_notification_sent: bool= rows[0].get( 6);
                debug!("nodes.monitoring_enabled = {} nodes.node_offline_notification_sent = {}" , &node_monitoring_enabled, &node_offline_notification_sent);

                if node_monitoring_enabled && node_offline_notification_sent {
                    // node was offline and is now online -> send notification
                    debug!("Sending node online notification");

                    let node_last_checkin_timestamp:  DateTime<Utc> = rows[0].get( 4);

                    debug!("email_notification_list = {:?}", email_notification_list);
                    if email_notification_list != "" {
                        send_email::send_node_online_notification_email(
                            &checkin_data.node_id,
                            &email_notification_list,
                            &node_checkin_timestamp,
                            &node_last_checkin_timestamp,
                            &email_config
                        );
                    }
                    else {
                        error!("Can not send notification. recipient list not defined");
                    }

                }

                // perform sensor data validation
                sensor_trigger_check(
                    &node_id_db,
                    &checkin_data.sensor_data,
                    &checkin_data.node_id,
                    &email_notification_list,
                    &node_checkin_timestamp,
                    &client,
                    &email_config,
                ).await;

            }



        }

        info!("/checkin done. {} ",log_status_message );

        Ok(HttpResponse::Ok().json(checkin_data))
    }


    pub async fn sensor_trigger_check(
        node_id_db: &i32,
        sensor_data: &Option<Vec<SensorData>>,
        node_id_external: &String,
        notification_email_list: &String,
        node_checkin_timestamp: &DateTime<Utc>,
        dbconnection: &Client,
        email_config: &web::Data<Email>
    ) {
        // check sensor values
        // 1. find list of sensor that should be monitored from table sensor_triggers
        // 2. match against sensor data present in checkin data object
        //    2.1 send alerts if necessary

        use tokio_pg_mapper::FromTokioPostgresRow;

        log_sensor_data(&sensor_data); // log to console

        let stmt_trigger_list = dbconnection.prepare_cached("SELECT sensor_triggers_id, node_id, sensor_id, monitoring_enabled, trigger_notification_sent, validation_function, validation_parameter_1, validation_parameter_2
	FROM remote_pi_monitor.sensor_triggers where node_id = $1 AND monitoring_enabled = true ;").await.unwrap();
        let rows_trigger_list = dbconnection.query(&stmt_trigger_list, &[&node_id_db] ).await.unwrap();



        for sensor_trigger_row in rows_trigger_list {
            let sensor_trigger = SensorTrigger::from_row(sensor_trigger_row).unwrap();
                    debug!(
                    "Trigger check: sensor_triggers_id={} sensor_id={} monitoring_enabled={} trigger_notification_sent={} validation_function={} validation_parameter_1={:?} validation_parameter_2={:?} ",
                    sensor_trigger.sensor_triggers_id,
                    sensor_trigger.sensor_id,
                    sensor_trigger.monitoring_enabled,
                    sensor_trigger.trigger_notification_sent,
                    sensor_trigger.validation_function,
                    sensor_trigger.validation_parameter_1,
                    sensor_trigger.validation_parameter_2);

                    // find sensor data in sensor_data list that matches the sensor_trigger and perform validation
                    let mut validation_result: (Option<bool>, String) = (None, "".to_string());
                    let mut sensor_name_email= "".to_string();

                    // find sensor data in sensor_data list
                    let sensor_data_found = find_sensor_data_by_id(&sensor_trigger.sensor_id, sensor_data);

                    match sensor_data_found {
                        None => { // case when sensor data is not present for this trigger
                            if sensor_trigger.trigger_notification_sent == false { // checking if notification has not been sent already
                                println!("sensor value not present. Sending notification");
                                validation_result.0 = Some(false);
                                validation_result.1 = "Sensor value is missing ".to_string();
                                sensor_name_email = "".to_string()
                            }
                        }
                        Some(x) => { // case when sensor data IS found and we need to validate the data against trigger validation function + parameters
                            validation_result = validate_sensor_data(
                                &sensor_trigger.validation_function,
                                &sensor_trigger.validation_parameter_1,
                                &sensor_trigger.validation_parameter_2,
                                x.value,
                            );
                            sensor_name_email = x.sensor_name.clone();
                            debug!("Validation result = {:?}", validation_result.0);
                            debug!("Validation email message = {}", validation_result.1);
                        }
                    }
                    // send e-mail notifications (if needed)  and update status in DB
                    if (validation_result.0 == Some(false)) & (sensor_trigger.trigger_notification_sent == false) {
                        if  notification_email_list != "" {
                            send_email::sensor_validation_failed_email(
                                node_id_external,
                                notification_email_list,
                                node_checkin_timestamp,
                                &validation_result.1,
                                &sensor_trigger.sensor_id,
                                &sensor_name_email,
                                &email_config,
                            );
                            // update DB
                            let stmt_trigger_notification_status_update = dbconnection.prepare_cached("UPDATE remote_pi_monitor.sensor_triggers SET trigger_notification_sent=true WHERE sensor_triggers_id= $1;").await.unwrap();
                            let _result = dbconnection.query(&stmt_trigger_notification_status_update, &[&sensor_trigger.sensor_triggers_id] ).await.unwrap();

                        } else {
                            error!("Can not send notification. recipient list not set");
                        }
                    } else if (validation_result.0 == Some(true)) & (sensor_trigger.trigger_notification_sent == true) {
                        debug!("sensor value is OK (was not OK) -> send notification");
                        if notification_email_list != "" {
                            send_email::sensor_validation_ok_email(
                                node_id_external,
                                notification_email_list,
                                node_checkin_timestamp,
                                &validation_result.1,
                                &sensor_trigger.sensor_id,
                                &sensor_name_email,
                                &email_config
                            );
                            // update DB
                            let stmt_trigger_notification_status_update2 = dbconnection.prepare_cached("UPDATE remote_pi_monitor.sensor_triggers SET trigger_notification_sent=false WHERE sensor_triggers_id= $1;").await.unwrap();
                            let _result = dbconnection.query(&stmt_trigger_notification_status_update2, &[&sensor_trigger.sensor_triggers_id] ).await.unwrap();

                        } else {
                            error!("Can not send notification. recipient list not set");
                        }
                    }
                }


    }


    pub fn find_sensor_data_by_id<'a>(
        trigger_sensor_id: &String,
        sensor_data: &'a Option<Vec<SensorData>>,
    ) -> Option<&'a SensorData> {
        let mut function_return: Option<&SensorData> = None;
        match sensor_data {
            Some(x) => {
                for sensor_data_iter in x {
                    if sensor_data_iter.id == *trigger_sensor_id {
                        debug!(
                        "trigger-sensor-id match. Sensor value = {:?}",
                        sensor_data_iter.value
                    );
                        function_return = Some(&sensor_data_iter);
                        break;
                    }
                }
            }
            // The division was invalid
            None => function_return = None,
        }
        function_return
    }

    pub fn log_sensor_data(sensor_data: &Option<Vec<SensorData>>)
    {
        match sensor_data {
            Some(x) => {
                // debug print sensor_values present in API call
                for sensor_value in x {
                    debug!(
                    "received sensor data : id={} sensor_name={} value={:?}",
                    sensor_value.id, sensor_value.sensor_name, sensor_value.value
                );
                }
            }
            // The division was invalid
            None => debug!("No sensor data present in API call"),
        }
    }



    pub fn validate_sensor_data(
        validation_function: &String,
        validation_parameter_1: &Option<f32>,
        validation_parameter_2: &Option<f32>,
        sensor_value: f32,
    ) -> (Option<bool>, String) {
        debug!(
        "Sensor data validation. Function '{}' parameter1 '{:?}' parameter2 '{:?}' sensor value '{}'",
        *validation_function, validation_parameter_1, validation_parameter_2,sensor_value
    );

        let mut validation_result: (Option<bool>, String) = (None, "".to_string());
        let validation_delta: f32 = 0.05;

        match validation_function.as_str() {
            ">" => {
                debug!("validation against > ");
                match *validation_parameter_1 {
                    Some(x) => {
                        if sensor_value > ( x + validation_delta) {
                            validation_result.0 = Some(true); // OK result
                            validation_result.1 = format!(
                                "expected sensor value {} {:?}. Got sensor value =  {}",
                                validation_function, x + validation_delta, sensor_value
                            );
                        } else if sensor_value < ( x - validation_delta) {
                            validation_result.0 = Some(false); // validation failed
                            validation_result.1 = format!(
                                "expected sensor value {} {:?}. Got sensor value =  {}",
                                validation_function, x - validation_delta, sensor_value
                            );
                        }
                    }
                    None => error!("can not validate. parameter missing"),
                }
            }
            "<" => {
                debug!("validation against > ");
                match *validation_parameter_1 {
                    Some(x) => {
                        if sensor_value < ( x - validation_delta) {
                            validation_result.0 = Some(true); // OK result
                            validation_result.1 = format!(
                                "expected sensor value {} {:?}. Got sensor value =  {}",
                                validation_function, x - validation_delta, sensor_value
                            );
                        } else if sensor_value > ( x + validation_delta) {
                            validation_result.0 = Some(false); // validation failed
                            validation_result.1 = format!(
                                "expected sensor value {} {:?}. Got sensor value =  {}",
                                validation_function, x + validation_delta, sensor_value
                            );
                        }
                    }
                    None => error!("can not validate. parameter missing"),
                }
            }
            "==" => {
                debug!("validation against == ");
                match *validation_parameter_1 {
                    Some(x) => {
                        if sensor_value == x {
                            validation_result.0 = Some(true);  // OK result
                            validation_result.1 = format!(
                                "expected value {} {:?}. Got {}",
                                validation_function, x, sensor_value
                            );
                        } else {
                            validation_result.0 = Some(false); // validation failed
                            validation_result.1 = format!(
                                "expected value {} {:?}. Got {}",
                                validation_function, x, sensor_value
                            );
                        }

                    }
                    None => debug!("can not validate. parameter missing"),
                }
            }
            "!=" => {
                debug!("validation against != ");
                match *validation_parameter_1 {
                    Some(x) => {
                        if sensor_value != x {
                            validation_result.0 = Some(true);  // OK result
                            validation_result.1 = format!(
                                "expected value {} {:?}. Got {}",
                                validation_function, x, sensor_value
                            );
                        } else {
                            validation_result.0 = Some(false); // validation failed
                            validation_result.1 = format!(
                                "expected value {} {:?}. Got {}",
                                validation_function, x, sensor_value
                            );
                        }

                    }
                    None => error!("can not validate. parameter missing"),
                }
            }
            "b" => {
                debug!("validation against b- range ");
                match *validation_parameter_1 {
                    Some(x) => match *validation_parameter_2 {
                        Some(y) => {
                            if (sensor_value > (x + validation_delta )) & (sensor_value < (y - validation_delta) ) {
                                validation_result.0 = Some(true);  // value OK
                                validation_result.1 = format!("expected {} < Sensor value < {}. Got sensor value = {}", x + validation_delta , y - validation_delta, sensor_value);
                            } else if (sensor_value < (x - validation_delta)) | (sensor_value > (y + validation_delta ))  {
                                validation_result.0 = Some(false);  // check failed
                                validation_result.1 = format!("Sensor value {} is  < {} OR  > {}", sensor_value, x - validation_delta , y + validation_delta );
                            }

                        }
                        None => error!("can not validate. parameter missing"),
                    },
                    None => error!("can not validate. parameter missing"),
                }
            }
            &_ => error!("Validation function unknown"),
        }
        validation_result
    }

    pub async fn alert_sender (
        db_pool: web::Data<Pool>,
        email_config: web::Data<Email>,
    ) -> Result<HttpResponse, Error>
    {
        use tokio_pg_mapper::FromTokioPostgresRow;

        let client = db_pool.get().await.unwrap();

        let offline_select_timestamp =  Utc::now() - Duration::minutes(5);
        debug!("selecting nodes with monitoring enabled and checkin time < than {:?}" , &offline_select_timestamp);

        let stmt_offline_nodes_list = client.prepare_cached("SELECT id, node_id_external, fk_api_key_id, monitoring_enabled, last_checkin_timestamp, notification_email_list, offline_notification_sent \
        FROM remote_pi_monitor.nodes where monitoring_enabled = true AND notification_email_list <> '' AND offline_notification_sent = false AND last_checkin_timestamp < $1;").await.unwrap();
        let rows_offline_nodes_list = client.query(&stmt_offline_nodes_list, &[&offline_select_timestamp] ).await.unwrap();

        let offline_nodes_count = rows_offline_nodes_list.len();

        for row_offline_nodes in rows_offline_nodes_list {
            let offline_node = Nodes::from_row(row_offline_nodes).unwrap();
            debug!("Offline node: id = {} last_checkin_timestamp= {:?}", &offline_node.id, &offline_node.last_checkin_timestamp);

            send_email::send_node_offline_notification_email(
                &offline_node.node_id_external,
                &offline_node.notification_email_list,
                &offline_node.last_checkin_timestamp,
                &email_config
            );

            // udpate status in db
            let stmt_node_status_update = client.prepare_cached("UPDATE remote_pi_monitor.nodes SET offline_notification_sent = true WHERE id = $1").await.unwrap();
            let _update = client.query(&stmt_node_status_update, &[&offline_node.id] ).await.unwrap();


        }

        info!("/alert-sender done. offline_nodes_count = {:?}.", offline_nodes_count );

        Ok(HttpResponse::Ok().body("OK"))
    }





}

pub mod send_email;

use actix_web::{ web, App, HttpServer};
use dotenv::dotenv;
use ::config::Config;
use tokio_postgres::NoTls;
use handlers::status_check;
use handlers::checkin_node;
use handlers::alert_sender;
use env_logger::{Builder, Target};
use log::{info};

use crate::config::ExampleConfig;


#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let mut builder = Builder::from_default_env();
    builder.target(Target::Stdout);
    builder.init();


    let config_ = Config::builder()
        .add_source(::config::Environment::default())
        .build()
        .unwrap();

    let config: ExampleConfig = config_.try_deserialize().unwrap();
    info!("Email configuration: {} {} {}",config.email.smtp_server,config.email.username,config.email.password, ) ;

    let pool = config.pg.create_pool(None, NoTls).unwrap();

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data( web::Data::new( config.email.clone()))
            .service(web::resource("/").route(web::get().to(status_check)))
            .service(web::resource("/checkin").route(web::post().to(checkin_node)))
            .service(web::resource("/alert-sender").route(web::get().to(alert_sender)))
    })
        .bind(config.server_addr.clone())?
        .run();
    info!("Server running at http://{}/", config.server_addr);

    server.await

}