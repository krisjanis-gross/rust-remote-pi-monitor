use chrono::{NaiveDateTime, TimeZone};
use chrono_tz::Europe::Riga;

use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::SmtpTransport;
use lettre::{
    message::{header, MultiPart, SinglePart},
    Message, Transport,
};

use compound_duration::format_dhms;
use std::env;

pub fn send_email_generic(
    notification_recipient_list: &String,
    subject: &String,
    body_plain: &String,
    body_html: &String,
) {
    let env_variable_name = "email_account_password";
    let email_account_password;

    match env::var(env_variable_name) {
        Ok(v) => email_account_password = v,
        Err(e) => panic!("${} is not set ({})", env_variable_name, e),
    }
    //    println!("Email account password = {}", email_account_password);
    let creds = Credentials::new(
        "rpi@betras.lv".to_string(),
        email_account_password.to_string(),
    );

    // Open a remote connection to gmail
    let mailer = SmtpTransport::starttls_relay("smtp.gmail.com")
        .unwrap()
        .credentials(creds)
        .build();

    let mut email: Message;

    for email_destionation in notification_recipient_list.split(";") {
        println!("sending email to {}", email_destionation);

        email = Message::builder()
            .from("rpi monitor<rpi@betras.lv>".parse().unwrap())
            .reply_to("rpi monitor<rpi@betras.lv>".parse().unwrap())
            .subject(subject)
            .to(email_destionation.parse().unwrap())
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
            Ok(_) => println!("Email sent successfully!"),
            Err(e) => panic!("Could not send email: {:?}", e),
        }
    }
}

pub fn send_node_offline_notification_email(
    node_id: &String,
    notification_recipient_list: &String,
    checkin_timestamp: &NaiveDateTime,
) {
    let subject = format!("Node OFF-line: {}", node_id);

    let timestamp_now = chrono::Utc::now().naive_utc();
    let last_seen_minutes_ago = timestamp_now
        .signed_duration_since(*checkin_timestamp)
        .num_minutes();
    let timestamp_riga_time = Riga.from_utc_datetime(&timestamp_now);

    let body_plain = format!(
        "Node - {} - is OFF-line. It was last seen {} minutes ago on {}.",
        node_id,
        last_seen_minutes_ago,
        timestamp_riga_time.format("%Y-%m-%d %H:%M:%S"),
    );
    let body_html = format!(
        "Node - <b>{}</b> - is <span style='color:red'><b>OFF-line</b></span>. It was last seen {} minutes ago on {}.",
        node_id,
        last_seen_minutes_ago,
        timestamp_riga_time.format("%Y-%m-%d %H:%M:%S"),
    );

    send_email_generic(
        notification_recipient_list,
        &subject,
        &body_plain,
        &body_html,
    );
}

pub fn send_node_online_notification_email(
    node_id: &String,
    notification_recipient_list: &String,
    checkin_timestamp: &NaiveDateTime,
    last_checkin_timestamp: &NaiveDateTime,
) {
    let offline_minutes = checkin_timestamp
        .signed_duration_since(*last_checkin_timestamp)
        .num_seconds();

    let checkin_timestamp_riga_time = Riga.from_utc_datetime(&checkin_timestamp);

    let subject = format!("Node ON-line: {}", node_id);

    let offline_duration_text = format_dhms(offline_minutes);

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
    );
}

pub fn sensor_validation_failed_email(
    node_id: &String,
    notification_recipient_list: &String,
    checkin_timestamp: &NaiveDateTime,
    validation_message: &String,
    sensor_id: &String
) {
    let checkin_timestamp_riga_time = Riga.from_utc_datetime(&checkin_timestamp);

    let subject = format!("sensor validation FAILED: {}-{}", node_id, sensor_id);

    let body_plain = format!(
        "Sensor validation FAILED:\n Node ID:{}\n Sensor ID: {}\n Timestamp: {}\n Validation: {}",
        node_id,
        sensor_id,
        checkin_timestamp_riga_time.format("%Y-%m-%d %H:%M:%S"),
        validation_message,
    );
    let body_html = format!(
        "Sensor validation <span style='color:red'>FAILED</span>.<br> Node ID:{}<br> Sensor ID: {}<br> Timestamp: {}<br> Validation: <b>{}</b>",
        node_id,
        sensor_id,
        checkin_timestamp_riga_time.format("%Y-%m-%d %H:%M:%S"),
        validation_message

    );

    send_email_generic(
        notification_recipient_list,
        &subject,
        &body_plain,
        &body_html,
    );
}

pub fn sensor_validation_ok_email(
    node_id: &String,
    notification_recipient_list: &String,
    checkin_timestamp: &NaiveDateTime,
    validation_message: &String,
    sensor_id: &String
) {
    let checkin_timestamp_riga_time = Riga.from_utc_datetime(&checkin_timestamp);

    let subject = format!("Sensor validation OK: {}-{}", node_id, sensor_id);

    let body_plain = format!(
        "Sensor validation SUCCESSFUL:\n Node ID:{}\n Sensor ID: {}\n Timestamp: {}\n Validation: {}",
        node_id,
        sensor_id,
        checkin_timestamp_riga_time.format("%Y-%m-%d %H:%M:%S"),
        validation_message
    );
    let body_html = format!(
        "Sensor validation <span style='color:green'>SUCCESSFUL</span>.<br> Node ID:{}<br> Sensor ID: {}<br> Timestamp: {}<br> Validation: <b>{}</b>",
        node_id,
        sensor_id,
        checkin_timestamp_riga_time.format("%Y-%m-%d %H:%M:%S"),
        validation_message

    );

    send_email_generic(
        notification_recipient_list,
        &subject,
        &body_plain,
        &body_html,
    );
}
