#[allow(non_snake_case)]
pub fn loadPakoJs() -> String {
    include_str!("pako.script.js").to_string()
}

#[allow(non_snake_case)]
pub fn loadCryptoJs() -> String {
    include_str!("CryptoJS.script.js").to_string()
}

#[allow(non_snake_case)]
pub fn loadJimpJs() -> String {
    include_str!("Jimp.script.js").to_string()
}
