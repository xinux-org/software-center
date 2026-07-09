pub fn checkonline() -> bool {
    reqwest::blocking::get("https://nmcheck.gnome.org/check_network_status.txt").is_ok()
}

pub async fn checkonline_async() -> bool {
    reqwest::get("https://nmcheck.gnome.org/check_network_status.txt")
        .await
        .is_ok()
}
