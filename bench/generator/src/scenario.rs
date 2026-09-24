use rand::RngCore;

pub fn generate_payload(size: usize) -> Vec<u8> {
    if size < 16 {
        panic!("Payload size must be at least 16 bytes for header");
    }
    let mut payload = vec![0u8; size];
    // Fill the rest of the payload with random bytes.
    // The first 16 bytes will be overwritten with the header (client_message_id and sender_timestamp_micros).
    rand::thread_rng().fill_bytes(&mut payload[16..]);
    payload
}
