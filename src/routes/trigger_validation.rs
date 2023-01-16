use crate::models::SensorData;

pub fn validate_sensor_data(
    validation_function: &String,
    validation_parameter_1: &Option<f32>,
    validation_parameter_2: &Option<f32>,
    sensor_value: f32,
) -> (Option<bool>, String) {
    println!(
        "Sensor data validation. Function '{}' parameter1 '{:?}' parameter2 '{:?}' sensor value '{}'",
        *validation_function, validation_parameter_1, validation_parameter_2,sensor_value
    );

    let mut validation_result: (Option<bool>, String) = (None, "".to_string());
    let validation_delta: f32 = 0.05;

    match validation_function.as_str() {
        ">" => {
            println!("validation against > ");
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
                None => println!("can not validate. parameter missing"),
            }
        }
        "<" => {
            println!("validation against > ");
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
                None => println!("can not validate. parameter missing"),
            }
        }
        "==" => {
            println!("validation against == ");
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
                None => println!("can not validate. parameter missing"),
            }
        }
        "!=" => {
            println!("validation against != ");
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
                None => println!("can not validate. parameter missing"),
            }
        }
        "b" => {
            println!("validation against b- range ");
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

