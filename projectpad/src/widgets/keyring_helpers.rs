pub fn get_pass_from_keyring() -> Option<String> {
    let service = "projectpad-cli";
    let kr = keyring::Keyring::new(service, service);
    kr.get_password().ok()
}

pub fn set_pass_in_keyring(pass: &str) -> Result<(), String> {
    let service = "projectpad-cli";
    let kr = keyring::Keyring::new(service, service);
    kr.set_password(pass).map_err(|e| e.to_string())
}

pub fn clear_pass_from_keyring() -> Result<(), String> {
    let service = "projectpad-cli";
    let kr = keyring::Keyring::new(service, service);
    kr.delete_password().map_err(|e| e.to_string())
}
