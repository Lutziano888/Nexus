pub fn color_for_label(label: &str) -> u32 {
    if label.contains("root")      { return 0x00_35_36_3A; }
    if label.contains("header")    { return 0x00_35_36_3A; }
    if label.contains("content")   { return 0x00_FF_FF_FF; }
    if label.contains("statusbar") { return 0x00_F1_F3_F4; }
    if label.contains("main")      { return 0x00_FF_FF_FF; }
    if label.starts_with('"')      { return 0x00_44_44_55; }
    hash_color(label)
}

fn hash_color(s: &str) -> u32 {
    let mut h: u32 = 0x811C_9DC5;
    for b in s.bytes() {
        h = h.wrapping_mul(0x0100_0193);
        h ^= b as u32;
    }
    let r = ((h >> 16) & 0xFF) | 0x40;
    let g = ((h >>  8) & 0xFF) | 0x40;
    let b = ( h        & 0xFF) | 0x40;
    (r << 16) | (g << 8) | b
}

pub fn lighten(color: u32, amount: u32) -> u32 {
    let r = (((color >> 16) & 0xFF) + amount).min(0xFF);
    let g = (((color >>  8) & 0xFF) + amount).min(0xFF);
    let b = (( color        & 0xFF) + amount).min(0xFF);
    (r << 16) | (g << 8) | b
}