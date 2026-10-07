pub async fn check_online() -> bool {
    reqwest::get("https://nmcheck.gnome.org/check_network_status.txt")
        .await
        .is_ok()
}
