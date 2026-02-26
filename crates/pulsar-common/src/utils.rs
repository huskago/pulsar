use rand::RngExt;

const INVITE_CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz23456789";

pub fn generate_invite_code(len: usize) -> String {
    let mut rng = rand::rng();
    (0..len)
        .map(|_| {
            let idx = rng.random_range(0..INVITE_CHARSET.len());
            INVITE_CHARSET[idx] as char
        })
        .collect()
}
