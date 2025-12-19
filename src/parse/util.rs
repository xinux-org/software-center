pub fn checkonline() -> bool {
    reqwest::blocking::get("https://nmcheck.gnome.org/check_network_status.txt").is_ok()
}

use gettextrs::gettext;

// pub fn get_translation(text: String) -> &'static str {
//   text.as_str().clone()
// }
