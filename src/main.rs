mod models; // tells Rust to look for models.rs
mod send_telegram;


mod errors {
    //use actix_web::{HttpResponse, ResponseError};
   // use deadpool_postgres::PoolError;
 //   use derive_more::{Display, From};
 //   use tokio_pg_mapper::Error as PGMError;
//    use tokio_postgres::error::Error as PGError;
/* 
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
    */
}


mod handlers {
    use actix_web::{web, Error, HttpResponse};
    use deadpool_postgres::{ Pool};
    use log::debug;
    use log::error;
    use log::info;
    use crate::send_email;
    use crate::node_sensor_functions;

    use chrono::{DateTime, Duration, Utc};

    use crate::{ models::CheckinData,models::Nodes,models::Email,models::TelegramConfig};

    pub async fn status_check( ) -> &'static str {
        "Remote-pi-monitor has started!"
    }

    pub async fn checkin_node (
        checkin_data: web::Json<CheckinData>,
        db_pool: web::Data<Pool>,
        email_config: web::Data<Email>,
        telegram_config: web::Data<TelegramConfig>,
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
            status_message = format!("api_key_id = {}", checkin_data.api_key);
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
                            &email_config,
                            &telegram_config,
                        ).await;
                    }
                    else {
                        error!("Can not send notification. recipient list not defined");
                    }

                }

                // perform sensor data validation
                node_sensor_functions::sensor_trigger_check(
                    &node_id_db,
                    &checkin_data.sensor_data,
                    &checkin_data.node_id,
                    &email_notification_list,
                    &node_checkin_timestamp,
                    &client,
                    &email_config,
                    &telegram_config,
                ).await;

            }



        }

        info!("/checkin done. {} ",log_status_message );

        Ok(HttpResponse::Ok().json(checkin_data))
    }

    pub async fn alert_sender (
        db_pool: web::Data<Pool>,
        email_config: web::Data<Email>,
        telegram_config: web::Data<TelegramConfig>,
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
                &email_config,
                &telegram_config,
            ).await;

            // udpate status in db
            let stmt_node_status_update = client.prepare_cached("UPDATE remote_pi_monitor.nodes SET offline_notification_sent = true WHERE id = $1").await.unwrap();
            let _update = client.query(&stmt_node_status_update, &[&offline_node.id] ).await.unwrap();


        }

        info!("/alert-sender done. offline_nodes_count = {:?}.", offline_nodes_count );

        Ok(HttpResponse::Ok().body("OK"))
    }

}

pub mod send_email;
pub mod node_sensor_functions;


use actix_web::{ web, App, HttpServer};
use dotenv::dotenv;
use ::config::Config;
use tokio_postgres::NoTls;
use handlers::status_check;
use handlers::checkin_node;
use handlers::alert_sender;
use env_logger::{Builder, Target};
use log::{info};
use crate::models::TelegramConfig;
use crate::models::Email;


#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let mut builder = Builder::from_default_env();
    builder.target(Target::Stdout);
    builder.init();

/*  debug print env. values
for (key, value) in std::env::vars() {
    println!("{} = {}", key, value);
}
*/


    let config_ = Config::builder()
        .add_source(::config::Environment::default())
        .build()
        .unwrap();


 //   println!("config: {:?} ", config_);

  
    let email_config = Email {
    smtp_server: config_.get("email_smtp_server").unwrap(),
    username:  config_.get("email_username").unwrap(),
    password: config_.get("email_password").unwrap(),
};

  // println!("email config: {:?} ", email_config);


    let telegram_config = TelegramConfig {
        bot_token: config_.get("telegram_config_bot_token").unwrap(),
        channel_id: config_.get("telegram_config_channel_id").unwrap(),
    };

 // println!("tel config: {:?} ", telegram_config);

  let server_addr:String = config_.get("server_addr").unwrap();
 
 let pgconfig = deadpool_postgres::Config {
        user:config_.get("pg_user").unwrap(),
        password:config_.get("pg_password").unwrap(),
        host:config_.get("pg_host").unwrap(),
        port:config_.get("pg_port").unwrap(),
        dbname:config_.get("pg_dbname").unwrap(),
        ..Default::default()
       // pool.max_size:config_.get("pg_pool_max_size").unwrap(),
 };

    //let config: AppConfig = config_.try_deserialize().unwrap();
    info!("Email configuration: {} {}",email_config.smtp_server,email_config.username ) ;

    let telegram_config_parameter: TelegramConfig = telegram_config.clone();

    let pool = pgconfig.create_pool(None, NoTls).unwrap();

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data( web::Data::new( email_config.clone()))
            .app_data( web::Data::new( telegram_config.clone()))
            .service(web::resource("/").route(web::get().to(status_check)))
            .service(web::resource("/checkin").route(web::post().to(checkin_node)))
            .service(web::resource("/alert-sender").route(web::get().to(alert_sender)))
    })
        .bind(server_addr.clone())?
        .run();
    info!("Server running at http://{}/", server_addr);
    
   
    let startup_message:String = format!("Server startup complete");
    send_telegram::send_telegram_msg(&startup_message,&telegram_config_parameter).await;

    
    server.await
}