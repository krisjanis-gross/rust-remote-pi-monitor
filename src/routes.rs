//use diesel::dsl::*;
use diesel::{self, prelude::*};
use rocket_contrib::json::Json;

use crate::models::ApiKey;
use crate::DbConn;

pub mod send_email;

use crate::models::SensorTriggersList;

use chrono::NaiveDateTime;

#[get("/")]
pub fn index() -> &'static str {
    "remote-pi-monitor successfully started!"
}

// input parameters to the /checkin web service
#[derive(Serialize, Deserialize)]
pub struct SensorData {
    id: String,
    sensor_name: String,
    value: f32,
}

#[derive(Serialize, Deserialize)]
pub struct CheckinData {
    api_key: String,
    node_id: String,
    sensor_data: Option<Vec<SensorData>>,
}

#[post("/checkin", data = "<checkin_data>")]
pub fn process_node_checkin(
    conn: DbConn,
    checkin_data: Json<CheckinData>,
) -> Result<String, String> {
    println!(
        "Checkin-data: API-key={} node_id={}",
        checkin_data.api_key, checkin_data.node_id
    );
    // check if API key is valid
    use crate::schema::api_keys::dsl::*;
    let api_key_check = api_keys
        .filter(api_key.eq(&checkin_data.api_key))
        .limit(1)
        .load::<ApiKey>(&conn.0)
        .expect("Error loading api keys");

    if api_key_check.len() > 0 {
        println!(
            "api_key OK. id={}  api_key={}",
            api_key_check[0].id, api_key_check[0].api_key
        );
        let api_key_id = api_key_check[0].id;

        use crate::models::Nodeslist;
        use crate::schema::nodes::dsl::*;
        // find node id in db
        let checkin_node = nodes
            .filter(fk_api_key_id.eq(api_key_id))
            .filter(node_id_external.eq(&checkin_data.node_id))
            .load::<Nodeslist>(&conn.0)
            .expect("error executing query");

        if checkin_node.len() > 0 {
            println!(
                "node_id found = {}. Updating checkin timestamp ",
                checkin_node[0].id
            );

            let node_checkin_timestamp = chrono::Utc::now().naive_utc();
            // check whether node has been offline and has came online now.
            if checkin_node[0].offline_notification_sent.eq(&1) {
                // node has been offline and now has came online. we need to send notification
                if let Some(x) = &checkin_node[0].notification_email_list {
                    send_email::send_node_online_notification_email(
                        &checkin_node[0].node_id_external,
                        &x,
                        &node_checkin_timestamp,
                        &checkin_node[0].last_checkin_timestamp,
                    );
                } else {
                    println!("Can not send notification. recipient list not set");
                }
            }

            let result_rows_count = diesel::update(nodes.filter(id.eq(checkin_node[0].id)))
                .set((
                    last_checkin_timestamp.eq(node_checkin_timestamp),
                    offline_notification_sent.eq(0),
                ))
                .execute(&conn.0)
                .expect("error executing query");
            println!("updated rows count  = {:?}", result_rows_count);

            sensor_trigger_check(
                &checkin_node[0].id,
                &checkin_data.sensor_data,
                &checkin_node[0].node_id_external,
                &checkin_node[0].notification_email_list,
                &node_checkin_timestamp,
                conn,
            );

        // check sensor values end
        } else {
            println!("node not found. need to instert node in db");
            let insert_result = diesel::insert_into(nodes)
                .values((
                    node_id_external.eq(&checkin_data.node_id),
                    fk_api_key_id.eq(api_key_id),
                    last_checkin_timestamp.eq(chrono::Utc::now().naive_utc()),
                ))
                .execute(&conn.0)
                .expect("error executing query");
            println!("inserted rows count  = {:?}", insert_result);
        }
    } else {
        println!("api_key not found");
    }

    // insert or update node checkin data

    Ok(format!("Checkin status: OK"))
}

#[get("/alert-sender")]
pub fn send_email_alerts(conn: DbConn) -> &'static str {
    // select nodes that are offline more than X minutes
    // send e-mail alerts
    // update nodes table- enable flag  offline_notification_sent
    use crate::models::Nodeslist;
    use crate::schema::nodes::dsl::*;

    let offline_nodes_list = nodes
        .filter(monitoring_enabled.eq(1))
        .filter(notification_email_list.ne(""))
        .filter(offline_notification_sent.ne(1))
        .filter(last_checkin_timestamp.lt(diesel::dsl::sql("UTC_TIMESTAMP() - interval 5 minute")))
        .load::<Nodeslist>(&conn.0)
        .expect("Error loading offline nodes list");

    for offline_node in offline_nodes_list {
        //println!("offline node: {:?}", offline_node);
        println!(
            "offline node: id={} ext_id={} last_seen={:?} email_list={:?}",
            offline_node.id,
            offline_node.node_id_external,
            offline_node.last_checkin_timestamp,
            offline_node.notification_email_list
        );

        // send e-mail alert
        if let Some(x) = offline_node.notification_email_list {
            send_email::send_node_offline_notification_email(
                &offline_node.node_id_external,
                &x,
                &offline_node.last_checkin_timestamp,
            );
        } else {
            println!("Can not send notification. recipient list not set");
        }

        // update nodes table- enable flag  offline_notification_sent
        diesel::update(nodes.filter(id.eq(offline_node.id)))
            .set(offline_notification_sent.eq(1))
            .execute(&conn.0)
            .expect("error executing query");
    }

    "hello"
}

#[get("/")]
pub fn index33() -> &'static str {
    "Application successfully started!"
}

pub fn sensor_trigger_check(
    node_id_internal: &i32,
    sensor_data: &Option<Vec<SensorData>>,
    node_id_external: &String,
    notification_email_list: &Option<String>,
    node_checkin_timestamp: &NaiveDateTime,
    dbconnection: DbConn,
) {
    // check sensor values
    // 1. find list of sensor that should be monitored from table sensor_triggers
    // 2. match against sensor data present in checkin data object
    //    2.1 send alerts if necessary

    use crate::schema::sensor_triggers::dsl::*;
    // find sensor data triggers defined for this node
    let sensor_trigger_list = sensor_triggers
        .filter(node_id.eq(*node_id_internal))
        .filter(monitoring_enabled.eq(1))
        .load::<SensorTriggersList>(&dbconnection.0)
        .expect("error executing query to find triggers");

    log_sensor_data(&sensor_data); // log to console

    // iterate sensor triggers
    for sensor_trigger in sensor_trigger_list {
        println!(
"Trigger check: sensor_triggers_id={} sensor_id={} monitoring_enabled={} trigger_notification_sent={} validation_function={} validation_parameter_1={:?} validation_parameter_2={:?} ",
                        sensor_trigger.sensor_triggers_id,
                        sensor_trigger.sensor_id,
                        sensor_trigger.monitoring_enabled,
                        sensor_trigger.trigger_notification_sent,
                        sensor_trigger.validation_function,
                        sensor_trigger.validation_parameter_1,
                        sensor_trigger.validation_parameter_2);

        // find sensor (or send error)
        let sensor_data_found = find_sensor_data_by_id(&sensor_trigger.sensor_id, sensor_data);
        let mut validation_result: (bool, String) = (false, "".to_string());
        let sensor_name_email;
        match sensor_data_found {
            None => {
                println!("sensor value not present. Sending notification");
                validation_result.0 = false;
                validation_result.1 = "Sensor value is missing".to_string();
                sensor_name_email = "".to_string()
            } // not found -> notification
            Some(x) => {
                validation_result = validate_sensor_data(
                    &sensor_trigger.validation_function,
                    &sensor_trigger.validation_parameter_1,
                    &sensor_trigger.validation_parameter_2,
                    x.value,
                );
                sensor_name_email = x.sensor_name.clone();
                println!("Validation result = {}", validation_result.0);
                println!("Validation email message = {}", validation_result.1);
            } // perform validation
        }
        // send notofications and update status in DB
        if (validation_result.0 == false) & (sensor_trigger.trigger_notification_sent == 0) {
            if let Some(x) = notification_email_list {
                send_email::sensor_validation_failed_email(
                    node_id_external,
                    x,
                    node_checkin_timestamp,
                    &validation_result.1,
                    &sensor_trigger.sensor_id,
                    &sensor_name_email
                );
                // update DB
                diesel::update(
                    sensor_triggers
                        .filter(sensor_triggers_id.eq(sensor_trigger.sensor_triggers_id)),
                )
                .set(trigger_notification_sent.eq(1))
                .execute(&dbconnection.0)
                .expect("error executing query");
            } else {
                println!("Can not send notification. recipient list not set");
            }
        } else if (validation_result.0 == true) & (sensor_trigger.trigger_notification_sent == 1) {
            println!("sensor value is OK (was not OK) -> send notification");
            if let Some(x) = notification_email_list {
                send_email::sensor_validation_ok_email(
                    node_id_external,
                    x,
                    node_checkin_timestamp,
                    &validation_result.1,
                    &sensor_trigger.sensor_id,
                    &sensor_name_email
                );
                // update DB
                diesel::update(
                    sensor_triggers
                        .filter(sensor_triggers_id.eq(sensor_trigger.sensor_triggers_id)),
                )
                .set(trigger_notification_sent.eq(0))
                .execute(&dbconnection.0)
                .expect("error executing query");
            } else {
                println!("Can not send notification. recipient list not set");
            }
        }
    }
    // switch/case based on validation validation_function
}

pub fn validate_sensor_data(
    validation_function: &String,
    validation_parameter_1: &Option<f32>,
    validation_parameter_2: &Option<f32>,
    sensor_value: f32,
) -> (bool, String) {
    println!(
        "Sensor data validation. Function '{}' parameter1 '{:?}' parameter2 '{:?}' sensor value '{}'",
        *validation_function, validation_parameter_1, validation_parameter_2,sensor_value
    );
    let mut validation_result: (bool, String) = (false, "".to_string());

    match validation_function.as_str() {
        ">" => {
            println!("validation against > ");
            match *validation_parameter_1 {
                Some(x) => {
                    if sensor_value > x {
                        validation_result.0 = true;
                    } else {
                        validation_result.0 = false;
                    }
                    validation_result.1 = format!(
                        "expected value {} {:?}. Got {}",
                        validation_function, x, sensor_value
                    );
                }
                None => println!("can not validate. parameter missing"),
            }
        }
        ">=" => {
            println!("validation against >= ");
            match *validation_parameter_1 {
                Some(x) => {
                    if sensor_value >= x {
                        validation_result.0 = true;
                    } else {
                        validation_result.0 = false;
                    }
                    validation_result.1 = format!(
                        "expected value {} {:?}. Got {}",
                        validation_function, x, sensor_value
                    );
                }
                None => println!("can not validate. parameter missing"),
            }
        }
        "<" => {
            println!("validation against < ");
            match *validation_parameter_1 {
                Some(x) => {
                    if sensor_value < x {
                        validation_result.0 = true;
                    } else {
                        validation_result.0 = false;
                    }
                    validation_result.1 = format!(
                        "expected value {} {:?}. Got {}",
                        validation_function, x, sensor_value
                    );
                }
                None => println!("can not validate. parameter missing"),
            }
        }
        "<=" => {
            println!("validation against <= ");
            match *validation_parameter_1 {
                Some(x) => {
                    if sensor_value <= x {
                        validation_result.0 = true;
                    } else {
                        validation_result.0 = false;
                    }
                    validation_result.1 = format!(
                        "expected value {} {:?}. Got {}",
                        validation_function, x, sensor_value
                    );
                }
                None => println!("can not validate. parameter missing"),
            }
        }
        "==" => {
            println!("validation against == ");
            match *validation_parameter_1 {
                Some(x) => {
                    if sensor_value == x {
                        validation_result.0 = true;
                    } else {
                        validation_result.0 = false;
                    }
                    validation_result.1 = format!(
                        "expected value {} {:?}. Got {}",
                        validation_function, x, sensor_value
                    );
                }
                None => println!("can not validate. parameter missing"),
            }
        }
        "!=" => {
            println!("validation against != ");
            match *validation_parameter_1 {
                Some(x) => {
                    if sensor_value != x {
                        validation_result.0 = true;
                    } else {
                        validation_result.0 = false;
                    }
                    validation_result.1 = format!(
                        "expected value {} {:?}. Got {}",
                        validation_function, x, sensor_value
                    );
                }
                None => println!("can not validate. parameter missing"),
            }
        }
        "b" => {
            println!("validation against b ");
            match *validation_parameter_1 {
                Some(x) => match *validation_parameter_2 {
                    Some(y) => {
                        if (sensor_value > x) & (sensor_value < y) {
                            validation_result.0 = true;
                        } else {
                            validation_result.0 = false;
                        }
                        validation_result.1 =
                            format!("expected {} < value > {}. Got {}", x, y, sensor_value);
                    }
                    None => println!("can not validate. parameter missing"),
                },
                None => println!("can not validate. parameter missing"),
            }
        }
        &_ => println!("Validation function unknown"),
    }
    validation_result
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
                    println!(
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

pub fn     log_sensor_data(sensor_data: &Option<Vec<SensorData>>)
{
    match sensor_data {
        // The division was valid
        Some(x) => {
            // debug print sensor_values present in API call
            for sensor_value in x {
                println!(
                    "received sensor data : id={} sensor_name={} value={:?}",
                    sensor_value.id, sensor_value.sensor_name, sensor_value.value
                );
            }
        }
        // The division was invalid
        None => println!("No sensor data present in API call"),
    }
}
