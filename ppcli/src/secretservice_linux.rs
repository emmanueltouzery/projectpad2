use serde::{Deserialize, Serialize};
use std::error::Error;
use zvariant_derive::Type;

use zbus::dbus_proxy;
use zvariant::{Str, Value};

// busctl --user --xml-interface introspect org.freedesktop.secrets /org/freedesktop/secrets > ~/secrets.xml
// cargo run --bin zbus-xmlgen ~/secrets.xml
#[dbus_proxy(
    interface = "org.freedesktop.Secret.Service",
    default_service = "org.freedesktop.secrets",
    default_path = "/org/freedesktop/secrets"
)]
trait Service {
    fn get_secrets(
        &self,
        items: &[zvariant::ObjectPath],
        session: &zvariant::ObjectPath,
    ) -> zbus::Result<SecretsResponse>;

    fn open_session(
        &self,
        algorithm: &str,
        input: &zvariant::Value,
    ) -> zbus::Result<(zvariant::OwnedValue, zvariant::OwnedObjectPath)>;

    fn search_items(
        &self,
        attributes: std::collections::HashMap<&str, &str>,
    ) -> zbus::Result<(
        Vec<zvariant::OwnedObjectPath>,
        Vec<zvariant::OwnedObjectPath>,
    )>;
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct SecretsResponse(
    Vec<(
        zvariant::OwnedObjectPath,
        (zvariant::OwnedObjectPath, Vec<u8>, Vec<u8>, String),
    )>,
);

pub fn get_keyring_pass() -> Result<Option<String>, Box<dyn Error>> {
    let connection = zbus::Connection::new_session()?;

    let proxy = ServiceProxy::new(&connection)?;
    let (_val, session_path) = proxy.open_session("plain", &Value::Str(Str::from("")))?;

    let (unlocked, _locked) =
        proxy.search_items([("service", "projectpad-cli")].iter().cloned().collect())?;

    let p: &zvariant::ObjectPath = &unlocked[0];
    let SecretsResponse(secrets) = proxy.get_secrets(&[p.clone()][..], &session_path)?;
    Ok(secrets.first()
       .and_then(|s| std::str::from_utf8(&s.1.2).ok())
       .map(|s| s.to_string()))
}
