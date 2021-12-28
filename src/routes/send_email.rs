use chrono::{NaiveDateTime, TimeZone};
use chrono_tz::Europe::Riga;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::SmtpTransport;
use lettre::{Message, Transport};

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

    let email = Message::builder()
        .from("rpi monitor<rpi@betras.lv>".parse().unwrap())
        .reply_to("rpi monitor<rpi@betras.lv>".parse().unwrap())
        .to(notification_recipient_list.parse().unwrap())
        .subject(subject)
        .body(body_plain)
        .unwrap();

    let creds = Credentials::new("rpi@betras.lv".to_string(), "rB*&ZQ9>rB*&ZQ9>".to_string());

    // Open a remote connection to gmail
    let mailer = SmtpTransport::starttls_relay("smtp.gmail.com")
        .unwrap()
        .credentials(creds)
        .build();

    // Send the email
    match mailer.send(&email) {
        Ok(_) => println!("Email sent successfully!"),
        Err(e) => panic!("Could not send email: {:?}", e),
    }
}

pub fn send_node_online_notification_email(
    node_id: &String,
    notification_recipient_list: &String,
    checkin_timestamp: &NaiveDateTime,
    last_checkin_timestamp: &NaiveDateTime,
) {
    let offline_minutes = checkin_timestamp
        .signed_duration_since(*last_checkin_timestamp)
        .num_minutes();

    let checkin_timestamp_riga_time = Riga.from_utc_datetime(&checkin_timestamp);

    let subject = format!("Node ON-line: {}", node_id);

    let body_plain = format!(
        "Node - {} - is ON-line since {}. It was offline for {} minutes.",
        node_id,
        checkin_timestamp_riga_time.format("%Y-%m-%d %H:%M:%S"),
        offline_minutes
    );

    let email = Message::builder()
        .from("rpi monitor<rpi@betras.lv>".parse().unwrap())
        .reply_to("rpi monitor<rpi@betras.lv>".parse().unwrap())
        .to(notification_recipient_list.parse().unwrap())
        .subject(subject)
        .body(body_plain)
        .unwrap();

    let creds = Credentials::new("rpi@betras.lv".to_string(), "rB*&ZQ9>rB*&ZQ9>".to_string());

    // Open a remote connection to gmail
    let mailer = SmtpTransport::starttls_relay("smtp.gmail.com")
        .unwrap()
        .credentials(creds)
        .build();

    // Send the email
    match mailer.send(&email) {
        Ok(_) => println!("Email sent successfully!"),
        Err(e) => panic!("Could not send email: {:?}", e),
    }
}
