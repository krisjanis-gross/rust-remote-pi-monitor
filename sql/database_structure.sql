-- SCHEMA: remote_pi_monitor

-- DROP SCHEMA IF EXISTS remote_pi_monitor ;

CREATE SCHEMA IF NOT EXISTS remote_pi_monitor
    AUTHORIZATION postgresuser;

GRANT ALL ON SCHEMA remote_pi_monitor TO postgresuser;
GRANT ALL ON SCHEMA remote_pi_monitor TO remote_pi_monitor_user;


-- Table: remote_pi_monitor.api_keys

-- DROP TABLE IF EXISTS remote_pi_monitor.api_keys;

CREATE TABLE IF NOT EXISTS remote_pi_monitor.api_keys
(
    id integer NOT NULL,
    api_key character varying(100) COLLATE pg_catalog."default" NOT NULL,
    CONSTRAINT api_keys_pkey PRIMARY KEY (id),
    CONSTRAINT "apy key is unique" UNIQUE (api_key)
)

TABLESPACE pg_default;

ALTER TABLE IF EXISTS remote_pi_monitor.api_keys
    OWNER to remote_pi_monitor_user;




-- Table: remote_pi_monitor.nodes

-- DROP TABLE IF EXISTS remote_pi_monitor.nodes;

CREATE TABLE IF NOT EXISTS remote_pi_monitor.nodes
(
    id serial,
    node_id_external character varying(100) COLLATE pg_catalog."default" NOT NULL,
    fk_api_key_id integer NOT NULL,
    monitoring_enabled boolean NOT NULL DEFAULT 'false',
    last_checkin_timestamp timestamp with time zone NOT NULL,
    notification_email_list character varying(255) COLLATE pg_catalog."default",
    offline_notification_sent boolean NOT NULL DEFAULT 'false',
    CONSTRAINT nodes_pkey PRIMARY KEY (id)
)

TABLESPACE pg_default;

ALTER TABLE IF EXISTS remote_pi_monitor.nodes
    OWNER to remote_pi_monitor_user;



-- Table: remote_pi_monitor.sensor_triggers

-- DROP TABLE IF EXISTS remote_pi_monitor.sensor_triggers;

CREATE TABLE IF NOT EXISTS remote_pi_monitor.sensor_triggers
(
    sensor_triggers_id serial,
    node_id integer NOT NULL,
    sensor_id character varying(100) COLLATE pg_catalog."default" NOT NULL,
    monitoring_enabled boolean NOT NULL,
    trigger_notification_sent boolean NOT NULL,
    validation_function character varying(3) COLLATE pg_catalog."default" NOT NULL,
    validation_parameter_1 real,
    validation_parameter_2 real,
    CONSTRAINT sensor_triggers_pkey PRIMARY KEY (sensor_triggers_id)
)

TABLESPACE pg_default;

ALTER TABLE IF EXISTS remote_pi_monitor.sensor_triggers
    OWNER to remote_pi_monitor_user;

