-- Your SQL goes here
CREATE TABLE `rocket_app`.`sensor_triggers` (
  `sensor_triggers_id` INT NOT NULL AUTO_INCREMENT PRIMARY KEY,
  `node_id` INT NOT NULL,
  `sensor_id` VARCHAR(45) NOT NULL,
  `monitoring_enabled` TINYINT(4) NOT NULL DEFAULT 0,
  `trigger_notification_sent` TINYINT(4) NOT NULL DEFAULT 0,
  `validation_function` VARCHAR(3) NOT NULL,
  `validation_parameter_1` FLOAT NULL,
  `validation_parameter_2` FLOAT NULL,
  UNIQUE INDEX `sensor_triggers_id_UNIQUE` (`sensor_triggers_id` ASC),
  INDEX `i1` (`sensor_id` ASC));
