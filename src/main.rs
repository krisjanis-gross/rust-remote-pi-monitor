#![feature(proc_macro_hygiene, decl_macro)]

extern crate chrono;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

pub mod cors;
pub mod models;
pub mod routes;
pub mod schema;

// This registers your database with Rocket, returning a `Fairing` that can be `.attach`'d to your
// Rocket application to set up a connection pool for it and automatically manage it for you.
#[database("rocket_app")]
pub struct DbConn(diesel::MysqlConnection);

fn main() {
    env_logger::init();
    rocket::ignite()
        .mount(
            "/",
            routes![
                routes::index,
                routes::process_node_checkin,
                routes::send_email_alerts,
            ],
        )
        .attach(DbConn::fairing())
        .attach(cors::CorsFairing)
        .launch();
}
