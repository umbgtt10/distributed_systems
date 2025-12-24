use rand::Rng;

pub fn generate_random_string(rng: &mut impl Rng, max_length: usize) -> String {
    let length = rng.random_range(1..=max_length);
    (0..length)
        .map(|_| {
            let c = rng.random_range(b'a'..=b'z');
            c as char
        })
        .collect()
}

pub fn generate_target_word(rng: &mut impl Rng, length: usize) -> String {
    (0..length)
        .map(|_| {
            let c = rng.random_range(b'a'..=b'z');
            c as char
        })
        .collect()
}
