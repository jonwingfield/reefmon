use super::controller::Alert;
use std::env;

use lettre::transport::smtp::{SecurityLevel, SmtpTransportBuilder};
use lettre::transport::smtp::error::Error;
use lettre::email::EmailBuilder;
use lettre::transport::EmailTransport;

pub struct Alerts {

}

pub fn alert(alerts: &Vec<Alert>) -> Result<(), Error>  {
    if alerts.len() == 0 { return Ok(()); }

    let alert_text = alerts.into_iter()
        .map(|a| format!("{:?}: {}", a.component, a.message))
        .fold(String::new(), |acc, s| acc + "\n" + &s);

    let email = EmailBuilder::new()
        .to("wingfield.jon@gmail.com")
        .from("aquamon@pi3b")
        .subject("Aquamon Alert")
        .text(&alert_text)
        .build()
        .unwrap();

    let pass = env::var("GMAIL_PWD").unwrap();

    let mut mailer = SmtpTransportBuilder::new(("smtp.gmail.com", 587))
        .unwrap()
        .credentials("wingfield.jon@gmail.com", &pass)
        .security_level(SecurityLevel::AlwaysEncrypt)
        .build();
    mailer.send(email).map(|_| ())
}
