use std::error::Error;

pub fn get_keyring_pass() -> Result<Option<String>, Box<dyn Error>> {
    let service = "projectpad-cli";
    let kr = keyring::Keyring::new(&service, &service);
    Ok(kr.get_password().ok())
}
