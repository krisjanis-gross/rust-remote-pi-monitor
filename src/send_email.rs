use chrono::{ DateTime, Utc};
use chrono_tz::Europe::Riga;

use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::SmtpTransport;
use lettre::{
    message::{header, MultiPart, SinglePart},
    Message, Transport,
};


use compound_duration::format_dhms;
use actix_web::web;

use log::debug;
use crate::config::Email;
//use log::error;


pub fn send_email_generic(
    notification_recipient_list: &String,
    subject: &String,
    body_plain: &String,
    body_html: &String,
    email_config: &web::Data<Email>,
) {
    debug!("Email configuration: {} {} {}",email_config.smtp_server,email_config.username,email_config.password, ) ;

    //    println!("Email account password = {}", email_account_password);
    let creds = Credentials::new(
        email_config.username.to_string(),
        email_config.password.to_string(),
    );

    // Open a remote connection to gmail
    let mailer = SmtpTransport::starttls_relay(&email_config.smtp_server)
        .unwrap()
        .credentials(creds)
        .build();

    let mut email: Message;

    for email_destination in notification_recipient_list.split(";") {
        debug!("sending email to {}", email_destination);

        email = Message::builder()
            .from(email_config.username.to_string().parse().unwrap())
            .reply_to(email_config.username.to_string().parse().unwrap())
            .subject(subject)
            .to(email_destination.parse().unwrap())
            .multipart(
                MultiPart::alternative() // This is composed of two parts.
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_PLAIN)
                            .body(body_plain.clone()), // Every message should have a plain text fallback.
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_HTML)
                            .body(String::from(body_html.clone())),
                    ),
            )
            .unwrap();

        // Send the email(s)
        match mailer.send(&email) {
            Ok(_) => debug!("Email sent successfully!"),
            Err(e) => panic!("Could not send email: {:?}", e),
        }
    }
}

pub fn send_node_offline_notification_email(
    node_id: &String,
    notification_recipient_list: &String,
    last_checkin_timestamp: &DateTime<Utc>,
    email_config: &web::Data<Email>,
) {
    let subject = format!("Node OFF-line: {}", node_id);

    let checkin_timestamp  = Utc::now();
    let offline_minutes = checkin_timestamp
        .signed_duration_since(*last_checkin_timestamp)
        .num_seconds();

    let offline_duration_text = format_dhms(offline_minutes);


    let last_checkin_timestamp_riga_time = checkin_timestamp.with_timezone(&Riga);

    let body_plain = format!(
        "Node - {} - is OFF-line. It was last seen {} minutes ago on {}.",
        node_id,
        offline_duration_text,
        last_checkin_timestamp_riga_time.format("%Y-%m-%d %H:%M:%S"),
    );
    let body_html = format!(
        "Node - <b>{}</b> - is <span style='color:red'><b>OFF-line</b></span>. It was last seen {} minutes ago on {}.",
        node_id,
        offline_duration_text,
        last_checkin_timestamp_riga_time.format("%Y-%m-%d %H:%M:%S"),
    );

    send_email_generic(
        notification_recipient_list,
        &subject,
        &body_plain,
        &body_html,
        email_config,
    );
}

pub fn send_node_online_notification_email(
    node_id: &String,
    notification_recipient_list: &String,
    checkin_timestamp: &DateTime<Utc>,
    last_checkin_timestamp: &DateTime<Utc>,
    email_config: &web::Data<Email>,
) {

    let offline_minutes = checkin_timestamp
        .signed_duration_since(*last_checkin_timestamp)
        .num_seconds();

    let offline_duration_text = format_dhms(offline_minutes);

    debug!("offline_duration_text = {:?}" , &offline_duration_text);

    let checkin_timestamp_riga_time = checkin_timestamp.with_timezone(&Riga);

    let subject = format!("Node ON-line: {}", node_id);

    let body_plain = format!(
        "Node - {} - is ON-line since {}. It was offline for {}.",
        node_id,
        checkin_timestamp_riga_time.format("%Y-%m-%d %H:%M:%S"),
        offline_duration_text
    );
    let body_html = format!(
        "Node - <b>{}</b> - is <span style='color:green'><b>ON-line</b></span> since {}. It was offline for {}.",
        node_id,
        checkin_timestamp_riga_time.format("%Y-%m-%d %H:%M:%S"),
        offline_duration_text
    );

    send_email_generic(
        notification_recipient_list,
        &subject,
        &body_plain,
        &body_html,
        email_config,
    );
}

pub fn sensor_validation_failed_email(
    node_id: &String,
    notification_recipient_list: &String,
    checkin_timestamp: &DateTime<Utc>,
    validation_message: &String,
    sensor_id: &String,
    sensor_name: &String,
    email_config: &web::Data<Email>,
) {
    let checkin_timestamp_riga_time = checkin_timestamp.with_timezone(&Riga);
    let subject = format!("sensor validation FAILED: {}-{}", node_id, sensor_name);

    let body_plain = format!(
        "Sensor validation FAILED:\n Node ID:{}\n Sensor Name: {}\n Sensor ID: {}\n Timestamp: {}\n Validation: {}",
        node_id,
        sensor_name,
        sensor_id,
        checkin_timestamp_riga_time.format("%Y-%m-%d %H:%M:%S"),
        validation_message,
    );
    let body_html = format!(
        "Sensor validation <span style='color:red'>FAILED</span>.<br> Node ID:{}<br>Sensor Name: {}<br> Sensor ID: {}<br> Timestamp: {}<br> Validation: <b>{}</b>",
        node_id,
        sensor_name,
        sensor_id,
        checkin_timestamp_riga_time.format("%Y-%m-%d %H:%M:%S"),
        validation_message

    );

    send_email_generic(
        notification_recipient_list,
        &subject,
        &body_plain,
        &body_html,
        email_config,
    );
}

pub fn sensor_validation_ok_email(
    node_id: &String,
    notification_recipient_list: &String,
    checkin_timestamp: &DateTime<Utc>,
    validation_message: &String,
    sensor_id: &String,
    sensor_name: &String,
    email_config: &web::Data<Email>,
) {
    let checkin_timestamp_riga_time = checkin_timestamp.with_timezone(&Riga);

    let subject = format!("Sensor validation OK: {}-{}", node_id, sensor_name);

    let body_plain = format!(
        "Sensor validation SUCCESSFUL:\n Node ID:{}\n Sensor Name: {}\n Sensor ID: {}\n Timestamp: {}\n Validation: {}",
        node_id,
        sensor_name,
        sensor_id,
        checkin_timestamp_riga_time.format("%Y-%m-%d %H:%M:%S"),
        validation_message
    );
    let body_html = format!(
        "Sensor validation <span style='color:green'>SUCCESSFUL</span>.<br> Node ID:{}<br>Sensor Name: {} <br> Sensor ID: {}<br> Timestamp: {}<br> Validation: <b>{}</b>",
        node_id,
        sensor_name,
        sensor_id,
        checkin_timestamp_riga_time.format("%Y-%m-%d %H:%M:%S"),
        validation_message

    );

    send_email_generic(
        notification_recipient_list,
        &subject,
        &body_plain,
        &body_html,
        email_config,
    );
}
