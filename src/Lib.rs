pub mod nt;
pub mod rzctl;

#[cfg(test)]
mod tests {
    use super::rzctl;

    #[test]
    fn test_mouse_move() {
        // Initialize the device
        if rzctl::init() {
            // Move the mouse to (100, 100) from the current position
            rzctl::mouse_move(100, 100, true);
        } else {
            panic!("Failed to initialize the device.");
        }
    }
}