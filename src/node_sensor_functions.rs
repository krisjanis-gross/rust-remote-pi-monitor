use crate::models::TelegramConfig;
use crate::{ models::SensorData, models::SensorTrigger,models::Email};
use chrono::{DateTime,Utc};
use deadpool_postgres::{Client};
use actix_web::{web};

use log::debug;
use log::error;

use crate::send_email;


 pub async fn sensor_trigger_check(
        node_id_db: &i32,
        sensor_data: &Option<Vec<SensorData>>,
        node_id_external: &String,
        notification_email_list: &String,
        node_checkin_timestamp: &DateTime<Utc>,
        dbconnection: &Client,
        email_config: &web::Data<Email>,
        telegram_config:&web::Data<TelegramConfig>,
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
                                &telegram_config
                            ).await;
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
                                &email_config,
                                &telegram_config
                            ).await;
                            // update DB
                            let stmt_trigger_notification_status_update2 = dbconnection.prepare_cached("UPDATE remote_pi_monitor.sensor_triggers SET trigger_notification_sent=false WHERE sensor_triggers_id= $1;").await.unwrap();
                            let _result = dbconnection.query(&stmt_trigger_notification_status_update2, &[&sensor_trigger.sensor_triggers_id] ).await.unwrap();

                        } else {
                            error!("Can not send notification. recipient list not set");
                        }
                    }
                }


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



        