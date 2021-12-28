//use diesel::dsl::*;
use diesel::{self, prelude::*};
use rocket_contrib::json::Json;

use crate::models::ApiKey;
use crate::DbConn;

pub mod send_email;

#[get("/")]
pub fn index() -> &'static str {
    "Application successfully started!"
}

// input parameters to the /checkin web service
#[derive(Serialize, Deserialize)]
pub struct CheckinData {
    api_key: String,
    node_id: String,
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

            let node_checkin_timetamp = chrono::Utc::now().naive_utc();
            // check whether node has been offline and has came online now.
            if checkin_node[0].offline_notification_sent.eq(&1) {
                // node has been offline and now has came online. we need to send notification
                if let Some(x) = &checkin_node[0].notification_email_list {
                    send_email::send_node_online_notification_email(
                        &checkin_node[0].node_id_external,
                        &x,
                        &node_checkin_timetamp,
                        &checkin_node[0].last_checkin_timestamp,
                    );
                } else {
                    println!("Can not send notification. recipient list not set");
                }
            }

            let result_rows_count = diesel::update(nodes.filter(id.eq(checkin_node[0].id)))
                .set((
                    last_checkin_timestamp.eq(node_checkin_timetamp),
                    offline_notification_sent.eq(0),
                ))
                .execute(&conn.0)
                .expect("error executing query");
            println!("updated rows count  = {:?}", result_rows_count);
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
